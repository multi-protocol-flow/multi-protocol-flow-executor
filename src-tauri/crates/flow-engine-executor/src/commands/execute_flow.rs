//! 执行流程命令

use std::sync::Arc;

use crate::command::{ExecuteFlowCommandInput, ExecuteFlowCommandOutput};
use crate::executor::FlowExecutor;
use flow_engine_core::flow::Flow;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::traits::{ExecutionEventSender, LogSender, DebugEventSender};

/// 执行流程命令
///
/// 执行指定的流程，调用 `FlowExecutor` 执行引擎
///
/// # 参数
/// - `input`: 包含流程 ID 和可选变量的输入参数
/// - `flow`: 要执行的流程对象
/// - `log_sender`: 日志发送器（可选），用于发送日志事件
/// - `execution_event_sender`: 执行事件发送器（可选），Tauri 模式有值，CLI 模式为 None
/// - `debug_event_sender`: 调试事件发送器（可选），普通模式下用于 WS 流事件广播
/// - `cancel_receiver`: 取消信号接收器（可选），用于外部取消流程执行
///
/// # 返回
/// - `ExecuteFlowCommandOutput`: 包含执行结果
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    log_sender: Option<Arc<dyn LogSender>>,
    execution_event_sender: Option<Arc<dyn ExecutionEventSender>>,
    debug_event_sender: Option<Arc<dyn DebugEventSender>>,
    cancel_receiver: Option<tokio::sync::watch::Receiver<bool>>,
) -> ExecuteFlowCommandOutput {
    let flow_uuid = flow.uuid.clone();
    let flow_name = flow.name.clone();

    // 发送流程开始事件
    let start_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    if let Some(ref sender) = log_sender {
        let _ = sender
            .send(LogEvent {
                level: LogLevel::Info,
                message: format!("流程 '{}' start execution", flow_name),
                timestamp: start_timestamp,
                node_uuid: None,
                node_name: None,
                flow_uuid: flow_uuid.clone(),
            })
            .await;
    }

    // 如果没有传入覆盖变量，使用流程的激活环境变量（环境管理功能）
    let resolved_variables = input.variables.or_else(|| {
        flow.get_active_variables().map(|vars| serde_json::json!(vars))
    });

    // 调用执行器，传递 log_sender
    let mut executor = FlowExecutor::new();
    if let Some(sender) = execution_event_sender {
        executor = executor.with_execution_event_sender(sender);
    }
    if let Some(sender) = debug_event_sender {
        executor = executor.with_debug_event_sender(sender);
    }
    if let Some(rx) = cancel_receiver {
        executor = executor.with_cancel_receiver(rx);
    }
    // 挂载默认钩子执行器
    executor = executor.with_hook_executor(Arc::new(crate::hook_runner::DefaultHookExecutor::new()));
    let result = executor
        .execute_flow(&flow, resolved_variables, None, log_sender.clone())
        .await;

    // 发送流程结束事件
    let end_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
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
                    if result.success { "Success" } else { "Failed" }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ExecuteFlowCommandInput;
    use flow_engine_core::flow::Flow;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;
    use flow_engine_core::connection::Connection;
    use flow_engine_core::traits::NoopLogSender;
    use serde_json::json;
    use uuid::Uuid;
    use std::sync::Arc;

    /// 测试正常流程执行
    #[tokio::test]
    async fn test_execute_flow_normal() {
        // 创建简单的 Entry node流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        // 执行流程（不使用日志发送器，不使用执行事件发送器）
        let result = execute_flow(input, flow, None, None, None, None).await;

        // 验证结果
        assert!(result.success, "流程应该Success执行");
        assert!(result.error.is_none(), "不应该有Error");
        assert!(result.flow_report.is_some(), "应该有执行报告");
        // duration_ms 是 u64 类型，总是 >= 0，所以移除无用的比较

        let report = result.flow_report.unwrap();
        assert_eq!(report.executed_count, 1, "应该执行了 1 个节点");
        assert_eq!(report.node_reports.len(), 1);
    }

    /// 测试带变量的流程执行
    #[tokio::test]
    async fn test_execute_flow_with_variables() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "变量Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        // 传入变量
        let variables = json!({
            "api_key": "test-key-123",
            "base_url": "https://api.example.com",
            "retry_count": 3
        });

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: Some(variables.clone()),
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        // 验证结果
        assert!(result.success);
        
        // 验证变量被正确传递到报告中
        let report = result.flow_report.unwrap();
        assert!(report.initial_variables.is_some());
        let init_vars = report.initial_variables.unwrap();
        assert_eq!(init_vars["api_key"], "test-key-123");
        assert_eq!(init_vars["base_url"], "https://api.example.com");
        assert_eq!(init_vars["retry_count"], 3);
    }

    /// 测试缺少开始节点的Error处理
    #[tokio::test]
    async fn test_execute_flow_missing_entry_node() {
        let flow_id = Uuid::new_v4().to_string();
        
        // 创建没有 Entry node的流程（只有 HTTP 节点）
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(flow_engine_core::node_config::HttpConfig {
                url: "https://example.com".to_string(),
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "无效流程".to_string(), description: None, nodes: vec![http_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        // 验证Error结果
        assert!(!result.success, "流程应该Failed");
        assert!(result.error.is_some(), "应该有Error信息");
        assert!(result.error.unwrap().contains("流程验证Failed"), "Error信息应该说明验证Failed");
        // Error情况下 executed_count 为 0（没有 flow_report 或报告中的 executed_count 为 0）
        if let Some(ref report) = result.flow_report {
            assert_eq!(report.executed_count, 0, "不应该执行任何节点");
        }
    }

    /// 测试日志发送器正确传递
    #[tokio::test]
    async fn test_execute_flow_with_log_sender() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "日志Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        // 使用 NoopLogSender
        let log_sender: Arc<dyn LogSender> = Arc::new(NoopLogSender);

        // 执行流程（应该不会因日志发送而Failed）
        let result = execute_flow(input, flow, Some(log_sender), None, None, None).await;

        assert!(result.success, "流程应该Success执行");
    }

    /// 测试多节点流程执行
    #[tokio::test]
    async fn test_execute_flow_multiple_nodes() {
        let flow_id = Uuid::new_v4().to_string();
        
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),  // 使用 Entry 作为第二个节点进行测试
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "多节点流程".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        // 验证结果
        assert!(result.success);
        let report = result.flow_report.unwrap();
        assert_eq!(report.executed_count, 2, "应该执行了 2 个节点");
        assert_eq!(report.node_reports.len(), 2);
    }

    /// 测试执行时间计算
    #[tokio::test]
    async fn test_execute_flow_execution_time() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "时间Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        // 验证执行时间字段存在
        // execution_time 来自 flow_report.duration_ms
        // duration_ms 是 u64 类型，总是 >= 0
        if let Some(ref report) = result.flow_report {
            // 访问 duration_ms 字段确保其存在
            let _ = report.duration_ms;
        }
    }

    /// Test Flow报告包含正确的流程信息
    #[tokio::test]
    async fn test_execute_flow_report_contains_flow_info() {
        let flow_id = Uuid::new_v4().to_string();
        let flow_name = "报告信息Test Flow".to_string();
        
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: flow_name.clone(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        assert!(result.success);
        let report = result.flow_report.unwrap();
        
        // 验证报告包含正确的流程信息
        assert_eq!(report.flow_uuid, flow_id);
        assert_eq!(report.flow_name, flow_name);
        assert_eq!(report.total_nodes, 1);
    }

    /// 测试空变量输入
    #[tokio::test]
    async fn test_execute_flow_with_empty_variables() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "空变量Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        // 传入空对象
        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: Some(json!({})),
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        // 应该Success执行
        assert!(result.success);
        
        // 验证报告中的初始变量为空对象
        let report = result.flow_report.unwrap();
        let init_vars = report.initial_variables.unwrap();
        assert!(init_vars.is_object());
        assert_eq!(init_vars.as_object().unwrap().len(), 0);
    }

    /// Test Node报告状态正确记录
    #[tokio::test]
    async fn test_execute_flow_node_report_status() {
        let flow_id = Uuid::new_v4().to_string();
        let node_name = "Test Node".to_string();
        
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: node_name.clone(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "状态Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let input = ExecuteFlowCommandInput {
                    flow_id: flow_id.clone(),
                    variables: None,
                };

        let result = execute_flow(input, flow, None, None, None, None).await;

        assert!(result.success);
        let report = result.flow_report.unwrap();
        let node_report = &report.node_reports[0];
        
        // 验证节点报告
        assert_eq!(node_report.node_name, node_name);
        assert_eq!(node_report.status, flow_engine_core::report::ExecutionStatus::Success);
    }
}
