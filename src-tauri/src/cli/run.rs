//! mpe run 子命令 — 执行流程文件
//!
//! 支持变量覆盖、NDJSON 流式事件输出、报告保存控制。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{ExecutionEventSender, ExecutionEvent};

use super::{CliError, config, output};
use super::output::CliOutput;

// ── 变量解析 ──────────────────────────────────────────────────────

/// 将 `--var key=value` 参数解析为 `HashMap`
///
/// 值优先尝试解析为 JSON（数字、布尔等），失败则作为字符串处理。
fn parse_vars(vars: &[String]) -> Result<HashMap<String, serde_json::Value>, CliError> {
    let mut map = HashMap::new();
    for v in vars {
        let (key, val) = v.split_once('=').ok_or_else(|| {
            CliError::InvalidArgs(format!("--var 格式错误 (需要 key=value): {}", v))
        })?;
        let json_val = serde_json::from_str(val)
            .unwrap_or(serde_json::Value::String(val.to_string()));
        map.insert(key.to_string(), json_val);
    }
    Ok(map)
}

// ── CLI 执行事件发送器 ─────────────────────────────────────────────

/// CLI 模式的执行事件发送器
///
/// 将 `ExecutionEvent` 通过 `tokio::sync::mpsc` 无界通道发送到接收端，
/// 接收端负责以 NDJSON 格式输出到 stdout。
struct CliExecutionEventSender {
    tx: tokio::sync::mpsc::UnboundedSender<ExecutionEvent>,
}

impl ExecutionEventSender for CliExecutionEventSender {
    fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
        self.tx.send(event).map_err(|e| format!("发送事件失败: {}", e))
    }
}

// ── 主入口 ────────────────────────────────────────────────────────

/// 执行流程
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
            "流程文件不存在: {}",
            flow_path
        )));
    }

    // 加载配置（获取 reports_dir）
    let app_config = config::AppConfig::load().map_err(CliError::Config)?;

    // 加载流程
    let flow = crate::load_flow(flow_path)
        .map_err(|e| CliError::Execution(format!("加载流程失败: {}", e)))?;

    // 解析变量覆盖
    let variables = if vars.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            parse_vars(vars)?
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        ))
    };

    // 执行流程
    let result = if stream {
        execute_with_streaming(path, &flow, variables, cancel_rx).await?
    } else {
        execute_simple(path, &flow, variables, cancel_rx).await?
    };

    // 构建输出数据
    let total_time_ms: u64 = result
        .flow_report
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or(0);

    // 提取节点级别的执行结果（用于测试验证）
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

    // 保存报告
    if !no_report {
        if let Some(ref flow_report) = result.flow_report {
            if let Err(e) = crate::save_report(flow_report, &app_config.reports_dir) {
                eprintln!("保存报告失败: {}", e);
            }
        }
    }

    if !result.success {
        return Err(CliError::Execution(
            result
                .error
                .unwrap_or_else(|| "执行失败".to_string()),
        ));
    }

    Ok(())
}

/// 非流式执行 — 直接调用 `FlowExecutor`
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

/// 流式执行 — 注册事件发送器，NDJSON 实时输出
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

    // 在后台启动执行，同时消费事件流
    let flow = flow.clone();
    let handle = tokio::spawn(async move {
        executor.execute_flow(&flow, variables, None, None).await
    });

    // 实时输出 NDJSON 事件
    while let Some(event) = event_rx.recv().await {
        output::output_ndjson(&event);
    }

    // 等待执行完成
    handle
        .await
        .map_err(|e| CliError::Execution(format!("执行任务失败: {}", e)))
}
