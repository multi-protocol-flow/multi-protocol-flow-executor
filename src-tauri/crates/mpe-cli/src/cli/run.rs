//! mpe run subcommand — execute flow file
//!
//! Supports variable overrides, NDJSON streaming events, and report save control.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use flow_engine::{ExecutionEvent, ExecutionEventSender};

use super::output::CliOutput;
use super::{config, output, CliError};

// ── Variable parsing ────────────────────────────────────────────────

/// Parse `--var key=value` arguments into a `HashMap`
///
/// Values are attempted as JSON first (numbers, booleans, etc.), falling back to string.
fn parse_vars(vars: &[String]) -> Result<HashMap<String, serde_json::Value>, CliError> {
    let mut map = HashMap::new();
    for v in vars {
        let (key, val) = v.split_once('=').ok_or_else(|| {
            CliError::InvalidArgs(format!("--var format error (expected key=value): {}", v))
        })?;
        let json_val =
            serde_json::from_str(val).unwrap_or(serde_json::Value::String(val.to_string()));
        map.insert(key.to_string(), json_val);
    }
    Ok(map)
}

// ── CLI execution event sender ───────────────────────────────────────

/// CLI mode execution event sender
///
/// Sends `ExecutionEvent` through `tokio::sync::mpsc` unbounded channel to receiver,
/// which outputs events as NDJSON to stdout.
struct CliExecutionEventSender {
    tx: tokio::sync::mpsc::UnboundedSender<ExecutionEvent>,
}

impl ExecutionEventSender for CliExecutionEventSender {
    fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
        self.tx
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))
    }
}

// ── Main entrypoint ──────────────────────────────────────────────────

/// Execute flow
pub async fn execute(
    format: &str,
    flow_path: &str,
    env: Option<&str>,
    vars: &[String],
    stream: bool,
    no_report: bool,
    report_format: &str,
    report_file: Option<&std::path::Path>,
    registry: Arc<flow_engine::registry::NodeRegistry>,
    plugin_manager: super::PluginManagerHandle,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), CliError> {
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(CliError::InvalidArgs(format!(
            "Flow file not found: {}",
            flow_path
        )));
    }

    // Load config (get reports_dir)
    let app_config = config::AppConfig::load().map_err(CliError::Config)?;

    // 子流程按 ID 解析使用配置中的流程目录（默认数据目录），
    // 与 `mpe flow list` 等命令的可见范围保持一致
    flow_engine::storage::set_flows_dir_override(Some(app_config.flows_dir.clone()));
    flow_engine::storage::set_environments_dir_override(Some(app_config.environments_dir.clone()));

    // Load flow
    let flow = flow_engine::storage::load_flow(flow_path)
        .map_err(|e| CliError::execution(format!("Failed to load flow: {}", e)))?;

    // Load environment if specified
    let env_set = if let Some(env_name) = env {
        if !env_name.trim().is_empty() {
            let env_dir = flow_engine::storage::get_environments_dir().map_err(|e| {
                CliError::execution(format!("Failed to get environments directory: {}", e))
            })?;
            Some(
                flow_engine::storage::load_environment_file(env_name.trim(), &env_dir).map_err(
                    |e| {
                        CliError::execution(format!(
                            "Failed to load environment '{}': {}",
                            env_name, e
                        ))
                    },
                )?,
            )
        } else {
            None
        }
    } else {
        None
    };

    // Parse variable overrides from CLI --var
    let cli_vars = if vars.is_empty() {
        None
    } else {
        Some(parse_vars(vars)?)
    };

    // Merge variable layers: fallback (flow.initial_variables) -> env -> runtime (--var)
    let merged = flow_engine::flow::merge_variable_layers(
        flow.initial_variables.as_ref(),
        env_set.as_ref(),
        None,
        cli_vars.as_ref(),
    );

    let variables = if merged.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            merged.into_iter().collect::<serde_json::Map<_, _>>(),
        ))
    };

    // Execute flow
    let result = if stream {
        execute_with_streaming(path, &flow, variables, Arc::clone(&registry), cancel_rx).await?
    } else {
        execute_simple(path, &flow, variables, registry, cancel_rx).await?
    };

    // 通知所有活跃插件：本次执行已结束（fire-and-forget）。
    // 插件据此释放 per-execution 资源（如 Mongo 连接池）；
    // 仅通知已在运行的插件，不拉起空闲插件。
    #[cfg(feature = "plugin-manager")]
    if let Some(manager) = plugin_manager {
        manager.notify_flow_ended(&result.execution_id).await;
    }
    #[cfg(not(feature = "plugin-manager"))]
    let _ = plugin_manager;

    // Build output data
    let total_time_ms: u64 = result
        .flow_report
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or(0);

    // Extract node-level execution results (for test validation)
    let node_results: Vec<serde_json::Value> = result
        .node_reports
        .iter()
        .map(|r| {
            serde_json::json!({
                "uuid": r.node_uuid,
                "node_type": r.node_type,
                "name": r.node_name,
                "status": r.status,
                "output": r.output_data,
                "error": r.error,
            })
        })
        .collect();

    let output_data = serde_json::json!({
        "success": result.success,
        "data": result.data,
        "error": result.error,
        "execution_time_ms": total_time_ms,
        "executed_count": result.executed_count,
        "node_results": node_results,
    });

    // Export report file (--report-file): junit → JUnit XML, json → full report JSON
    if let Some(report_path) = report_file {
        if let Some(parent) = report_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create report directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }
        let content = match report_format {
            "junit" => {
                let report = result.flow_report.as_ref().ok_or_else(|| {
                    CliError::execution("No flow report available for JUnit export".to_string())
                })?;
                super::junit::format_junit_xml(report)
            }
            "json" => match &result.flow_report {
                Some(report) => serde_json::to_string_pretty(report).map_err(|e| {
                    CliError::execution(format!("Failed to serialize report: {}", e))
                })?,
                None => serde_json::to_string_pretty(&output_data).map_err(|e| {
                    CliError::execution(format!("Failed to serialize report: {}", e))
                })?,
            },
            other => {
                return Err(CliError::InvalidArgs(format!(
                    "Unsupported --report-format: {} (expected json or junit)",
                    other
                )));
            }
        };
        std::fs::write(report_path, content).map_err(|e| {
            CliError::Io(format!(
                "Failed to write report file {}: {}",
                report_path.display(),
                e
            ))
        })?;
    }

    // Save report
    if !no_report {
        if let Some(ref flow_report) = result.flow_report {
            if let Err(e) = flow_engine::storage::save_report(flow_report, &app_config.reports_dir)
            {
                eprintln!("Failed to save report: {}", e);
            }
        }
    }

    if report_format == "junit" && report_file.is_none() {
        if let Some(ref flow_report) = result.flow_report {
            print!("{}", super::junit::format_junit_xml(flow_report));
        }
    } else if format == "table" {
        let status_str = if result.success {
            "✔ SUCCESS"
        } else {
            "✖ FAILED"
        };
        let flow_name = result
            .flow_report
            .as_ref()
            .map(|r| r.flow_name.as_str())
            .unwrap_or(flow.name.as_str());
        println!(
            "\nFlow: {} | Status: {} | Duration: {}ms | Executed: {} nodes\n",
            flow_name, status_str, total_time_ms, result.executed_count
        );

        let headers = [
            "Status",
            "Node Name",
            "Type",
            "Duration",
            "Port",
            "Error / Details",
        ];
        let rows: Vec<Vec<String>> = result
            .node_reports
            .iter()
            .map(|r| {
                let status_icon = match r.status {
                    flow_engine::report::ExecutionStatus::Success => "✔ PASSED",
                    flow_engine::report::ExecutionStatus::Failed => "✖ FAILED",
                    flow_engine::report::ExecutionStatus::Skipped => "- SKIPPED",
                    flow_engine::report::ExecutionStatus::Cancelled => "■ CANCELLED",
                };
                let port = r.used_port_id.as_deref().unwrap_or("-");
                let detail = if let Some(ref err) = r.error {
                    err.message.clone()
                } else if r.node_type == "assertion" {
                    if let Some(ref out) = r.output_data {
                        if let (Some(passed), Some(total)) = (
                            out.get("passed_count").and_then(|v| v.as_u64()),
                            out.get("total").and_then(|v| v.as_u64()),
                        ) {
                            format!("{}/{} assertions passed", passed, total)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                vec![
                    status_icon.to_string(),
                    r.node_name.clone(),
                    r.node_type.clone(),
                    format!("{}ms", r.duration_ms),
                    port.to_string(),
                    detail,
                ]
            })
            .collect();
        print!("{}", output::format_table(&headers, &rows));
    } else {
        let cli_output = CliOutput::success(output_data.clone());
        output::output_json(&cli_output);
    }

    if !result.success {
        return Err(CliError::Execution {
            message: result
                .error
                .unwrap_or_else(|| "Execution failed".to_string()),
            data: if format == "table" {
                None
            } else {
                Some(output_data)
            },
        });
    }

    Ok(())
}

/// Non-streaming execution — directly calls `FlowExecutor`
async fn execute_simple(
    path: PathBuf,
    flow: &flow_engine::flow::Flow,
    variables: Option<serde_json::Value>,
    registry: Arc<flow_engine::registry::NodeRegistry>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<flow_engine::executor::ExecutionResult, CliError> {
    let default_timeout = std::env::var("MPE_DEFAULT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .map(std::time::Duration::from_millis);
    let mut executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(registry)
        .with_flow_file_path(path)
        .with_hook_executor(Arc::new(
            flow_engine::hook_runner::DefaultHookExecutor::new(),
        ))
        .with_cancel_receiver(cancel_rx);
    if let Some(timeout) = default_timeout {
        executor = executor.with_fallback_timeout(timeout);
    }
    Ok(executor.execute_flow(flow, variables, None, None).await)
}

/// Streaming execution — registers event sender, real-time NDJSON output
async fn execute_with_streaming(
    path: PathBuf,
    flow: &flow_engine::flow::Flow,
    variables: Option<serde_json::Value>,
    registry: Arc<flow_engine::registry::NodeRegistry>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<flow_engine::executor::ExecutionResult, CliError> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

    let sender: Arc<dyn ExecutionEventSender> = Arc::new(CliExecutionEventSender { tx: event_tx });

    let default_timeout = std::env::var("MPE_DEFAULT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .map(std::time::Duration::from_millis);
    let mut executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(registry)
        .with_flow_file_path(path)
        .with_execution_event_sender(sender)
        .with_hook_executor(Arc::new(
            flow_engine::hook_runner::DefaultHookExecutor::new(),
        ))
        .with_cancel_receiver(cancel_rx);
    if let Some(timeout) = default_timeout {
        executor = executor.with_fallback_timeout(timeout);
    }
    // Start execution in background, consume event stream simultaneously
    let flow = flow.clone();
    let handle =
        tokio::spawn(async move { executor.execute_flow(&flow, variables, None, None).await });

    // Output NDJSON events in real time
    while let Some(event) = event_rx.recv().await {
        output::output_ndjson(&event);
    }

    // Wait for execution to complete
    handle
        .await
        .map_err(|e| CliError::execution(format!("Execution task failed: {}", e)))
}
