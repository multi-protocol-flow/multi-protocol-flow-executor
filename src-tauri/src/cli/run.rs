//! mpe run subcommand — execute flow file
//!
//! Supports variable overrides, NDJSON streaming events, and report save control.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{ExecutionEventSender, ExecutionEvent};

use super::{CliError, config, output};
use super::output::CliOutput;

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
        let json_val = serde_json::from_str(val)
            .unwrap_or(serde_json::Value::String(val.to_string()));
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
        self.tx.send(event).map_err(|e| format!("Failed to send event: {}", e))
    }
}

// ── Main entrypoint ──────────────────────────────────────────────────

/// Execute flow
pub async fn execute(
    _format: &str,
    flow_path: &str,
    vars: &[String],
    stream: bool,
    no_report: bool,
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

    // Load flow
    let flow = crate::load_flow(flow_path)
        .map_err(|e| CliError::Execution(format!("Failed to load flow: {}", e)))?;

    // Parse variable overrides
    let variables = if vars.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            parse_vars(vars)?
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        ))
    };

    // Execute flow
    let result = if stream {
        execute_with_streaming(path, &flow, variables, cancel_rx).await?
    } else {
        execute_simple(path, &flow, variables, cancel_rx).await?
    };

    // Build output data
    let total_time_ms: u64 = result
        .flow_report
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or(0);

    // Extract node-level execution results (for test validation)
    let node_results: Vec<serde_json::Value> = result.node_reports.iter().map(|r| {
        serde_json::json!({
            "uuid": r.node_uuid,
            "node_type": r.node_type,
            "name": r.node_name,
            "status": r.status,
            "output": r.output_data,
            "error": r.error,
        })
    }).collect();

    let output_data = serde_json::json!({
        "success": result.success,
        "data": result.data,
        "error": result.error,
        "execution_time_ms": total_time_ms,
        "executed_count": result.executed_count,
        "node_results": node_results,
    });

    let cli_output = if result.success {
        CliOutput::success(output_data)
    } else {
        CliOutput {
            success: false,
            data: Some(output_data),
            error: result.error.clone(),
        }
    };

    output::output_json(&cli_output);

    // Save report
    if !no_report {
        if let Some(ref flow_report) = result.flow_report {
            if let Err(e) = crate::save_report(flow_report, &app_config.reports_dir) {
                eprintln!("Failed to save report: {}", e);
            }
        }
    }

    if !result.success {
        return Err(CliError::Execution(
            result
                .error
                .unwrap_or_else(|| "Execution failed".to_string()),
        ));
    }

    Ok(())
}

/// Non-streaming execution — directly calls `FlowExecutor`
async fn execute_simple(
    path: PathBuf,
    flow: &crate::flow::Flow,
    variables: Option<serde_json::Value>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<crate::ExecutionResult, CliError> {
    let executor = crate::FlowExecutor::new()
        .with_flow_file_path(path)
        .with_hook_executor(Arc::new(crate::hook_runner::DefaultHookExecutor::new()))
        .with_cancel_receiver(cancel_rx);
    Ok(executor.execute_flow(flow, variables, None, None).await)
}

/// Streaming execution — registers event sender, real-time NDJSON output
async fn execute_with_streaming(
    path: PathBuf,
    flow: &crate::flow::Flow,
    variables: Option<serde_json::Value>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<crate::ExecutionResult, CliError> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

    let sender: Arc<dyn ExecutionEventSender> = Arc::new(CliExecutionEventSender { tx: event_tx });

    let executor = crate::FlowExecutor::new()
        .with_flow_file_path(path)
        .with_execution_event_sender(sender)
        .with_hook_executor(Arc::new(crate::hook_runner::DefaultHookExecutor::new()))
        .with_cancel_receiver(cancel_rx);

    // Start execution in background, consume event stream simultaneously
    let flow = flow.clone();
    let handle = tokio::spawn(async move {
        executor.execute_flow(&flow, variables, None, None).await
    });

    // Output NDJSON events in real time
    while let Some(event) = event_rx.recv().await {
        output::output_ndjson(&event);
    }

    // Wait for execution to complete
    handle
        .await
        .map_err(|e| CliError::Execution(format!("Execution task failed: {}", e)))
}
