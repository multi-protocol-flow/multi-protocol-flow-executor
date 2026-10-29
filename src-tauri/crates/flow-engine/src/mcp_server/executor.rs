//! Embedded Flow Executor runner for MCP Tools calls.

use std::time::{Duration, Instant};

use flow_engine_core::report::ExecutionStatus;
use flow_engine_core::utils::unix_timestamp_ms;
use flow_engine_executor::executor::{ExecutionResult, FlowExecutor};
use serde_json::Value;

use super::protocol::CallToolResult;
use super::telemetry::{emit_opportunistic_telemetry, McpTelemetryEvent};
use super::tools::{is_system_variable, RegisteredFlowTool};

/// Execute a flow tool with sandboxed context and timeout guard
pub async fn execute_flow_tool(
    registered: &RegisteredFlowTool,
    arguments: Option<Value>,
    timeout_ms: u64,
) -> CallToolResult {
    let tool_name = registered.tool_def.name.clone();
    let start_time = Instant::now();
    let flow = registered.flow.clone();

    // Prepare variables: merge arguments into initial variables
    let mut vars_map = serde_json::Map::new();
    if let Some(ref initial) = flow.initial_variables {
        for (k, v) in initial {
            vars_map.insert(k.clone(), v.clone());
        }
    }

    if let Some(args_val) = arguments {
        if let Some(args_obj) = args_val.as_object() {
            for (k, v) in args_obj {
                vars_map.insert(k.clone(), v.clone());
            }
        }
    }

    let input_variables = Some(Value::Object(vars_map));
    let timeout_duration = Duration::from_millis(timeout_ms);

    // Spawn execution in an isolated task
    let exec_task = tokio::spawn(async move {
        let executor = FlowExecutor::new();
        executor
            .execute_flow(&flow, input_variables, None, None)
            .await
    });

    let timeout_res = tokio::time::timeout(timeout_duration, exec_task).await;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    match timeout_res {
        Err(_) => {
            // Execution timed out
            let err_msg = format!(
                "Execution timeout after {timeout_ms}ms (threshold exceeded)"
            );
            emit_opportunistic_telemetry(McpTelemetryEvent {
                event: "mcp_tool_call".to_string(),
                tool_name: tool_name.clone(),
                execution_id: uuid::Uuid::new_v4().to_string(),
                duration_ms,
                success: false,
                error: Some(err_msg.clone()),
                timestamp: unix_timestamp_ms(),
            });

            CallToolResult::error(err_msg)
        }
        Ok(Err(join_err)) => {
            let err_msg = format!("Internal task join error: {join_err}");
            emit_opportunistic_telemetry(McpTelemetryEvent {
                event: "mcp_tool_call".to_string(),
                tool_name: tool_name.clone(),
                execution_id: uuid::Uuid::new_v4().to_string(),
                duration_ms,
                success: false,
                error: Some(err_msg.clone()),
                timestamp: unix_timestamp_ms(),
            });

            CallToolResult::error(err_msg)
        }
        Ok(Ok(result)) => {
            let (output_text, is_error) = extract_refined_output(&result);

            emit_opportunistic_telemetry(McpTelemetryEvent {
                event: "mcp_tool_call".to_string(),
                tool_name,
                execution_id: result.execution_id.clone(),
                duration_ms,
                success: !is_error,
                error: if is_error { Some(output_text.clone()) } else { None },
                timestamp: unix_timestamp_ms(),
            });

            if is_error {
                CallToolResult::error(output_text)
            } else {
                CallToolResult::success(output_text)
            }
        }
    }
}

/// Refined output extraction following the hierarchy:
/// 1. `$output`, `output`, `result` variable
/// 2. End node `output_data`
/// 3. `ExecutionResult.data`
/// 4. Last executed node `output_data`
/// 5. Filtered variables dictionary
pub fn extract_refined_output(result: &ExecutionResult) -> (String, bool) {
    if !result.success {
        // Find failed node for diagnostics
        let mut failed_node = None;
        for nr in &result.node_reports {
            if nr.status == ExecutionStatus::Failed {
                let err_msg = nr
                    .error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Node execution failed".to_string());
                failed_node = Some((nr.node_name.clone(), err_msg));
                break;
            }
        }

        let (node_name, err_msg) = failed_node.unwrap_or_else(|| {
            (
                "flow".to_string(),
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "Unknown flow execution failure".to_string()),
            )
        });

        return (
            format!("Execution failed at node '{node_name}': {err_msg}"),
            true,
        );
    }

    // 1. $output, output, or result in final_variables
    if let Some(ref flow_report) = result.flow_report {
        if let Some(ref final_vars) = flow_report.final_variables {
            if let Some(obj) = final_vars.as_object() {
                if let Some(val) = obj
                    .get("$output")
                    .or_else(|| obj.get("output"))
                    .or_else(|| obj.get("result"))
                {
                    return (serialize_val(val), false);
                }
            }
        }
    }

    // 2. End node output_data
    for nr in result.node_reports.iter().rev() {
        if nr.node_type == "end" {
            if let Some(ref out) = nr.output_data {
                if !out.is_null() {
                    return (serialize_val(out), false);
                }
            }
        }
    }

    // 3. ExecutionResult.data
    if let Some(ref data) = result.data {
        if !data.is_null() {
            return (serialize_val(data), false);
        }
    }

    // 4. Last executed node output_data
    if let Some(last_report) = result.node_reports.last() {
        if let Some(ref out) = last_report.output_data {
            if !out.is_null() {
                return (serialize_val(out), false);
            }
        }
    }

    // 5. Filtered variables dictionary
    if let Some(ref flow_report) = result.flow_report {
        if let Some(ref final_vars) = flow_report.final_variables {
            if let Some(obj) = final_vars.as_object() {
                let mut filtered = serde_json::Map::new();
                for (k, v) in obj {
                    if !is_system_variable(k) {
                        filtered.insert(k.clone(), v.clone());
                    }
                }
                if !filtered.is_empty() {
                    return (
                        serialize_val(&Value::Object(filtered)),
                        false,
                    );
                }
            }
        }
    }

    (
        "Flow executed successfully (no output produced)".to_string(),
        false,
    )
}

fn serialize_val(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        _ => serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string()),
    }
}
