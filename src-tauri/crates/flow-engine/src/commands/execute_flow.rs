//! 执行流程命令

use std::sync::Arc;

use crate::command::{ExecuteFlowCommandInput, ExecuteFlowCommandOutput};
use crate::executor::FlowExecutor;
use crate::flow::Flow;
use crate::log_event::{LogEvent, LogLevel};
use crate::traits::LogSender;

/// 执行流程命令
///
/// 执行指定的流程，调用 FlowExecutor 执行引擎
///
/// # 参数
/// - `input`: 包含流程 ID 和可选变量的输入参数
/// - `flow`: 要执行的流程对象
/// - `log_sender`: 日志发送器（可选），用于发送日志事件
///
/// # 返回
/// - `ExecuteFlowCommandOutput`: 包含执行结果
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    log_sender: Option<Arc<dyn LogSender>>,
) -> ExecuteFlowCommandOutput {
    let flow_uuid = flow.uuid.clone();
    let flow_name = flow.name.clone();

    // 发送流程开始事件
    let start_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;

    if let Some(ref sender) = log_sender {
        let _ = sender
            .send(LogEvent {
                level: LogLevel::Info,
                message: format!("流程 '{}' 开始执行", flow_name),
                timestamp: start_timestamp,
                node_uuid: None,
                node_name: None,
                flow_uuid: flow_uuid.clone(),
            })
            .await;
    }

    // 调用执行器，传递 log_sender
    let executor = FlowExecutor::new();
    let result = executor
        .execute_flow(&flow, input.variables, log_sender.clone())
        .await;

    // 发送流程结束事件
    let end_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;

    if let Some(sender) = log_sender {
        let _ = sender
            .send(LogEvent {
                level: if result.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                },
                message: format!(
                    "流程执行{}",
                    if result.success { "成功" } else { "失败" }
                ),
                timestamp: end_timestamp,
                node_uuid: None,
                node_name: None,
                flow_uuid: flow_uuid.clone(),
            })
            .await;
    }

    // 先计算 execution_time，避免 move 后借用
    let execution_time = result
        .flow_report
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or(0);

    ExecuteFlowCommandOutput {
        success: result.success,
        data: result.data,
        flow_report: result.flow_report,
        error: result.error,
        execution_time,
    }
}
