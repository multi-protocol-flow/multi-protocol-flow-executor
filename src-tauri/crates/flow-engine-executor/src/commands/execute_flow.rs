//! 执行流程命令

use std::sync::Arc;

use crate::command::{ExecuteFlowCommandInput, ExecuteFlowCommandOutput};
use crate::executor::FlowExecutor;
use flow_engine_core::flow::Flow;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::traits::{DebugEventSender, ExecutionEventSender, LogSender};

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
/// - `registry`: 节点插件注册表（共享），由调用方注入以复用插件进程
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
    registry: Arc<flow_engine_core::registry::NodeRegistry>,
) -> ExecuteFlowCommandOutput {
    let flow_uuid = flow.uuid.clone();
    let flow_name = flow.name.clone();

    // 发送流程开始事件
    let start_timestamp = flow_engine_core::utils::unix_timestamp_ms();

    if let Some(ref sender) = log_sender {
        let _ = sender
            .send(LogEvent {
                level: LogLevel::Info,
                message: format!("Flow '{}' start execution", flow_name),
                timestamp: start_timestamp,
                node_uuid: None,
                node_name: None,
                flow_uuid: flow_uuid.clone(),
            })
            .await;
    }

    // 多层变量合并：
    // 1. 基底兜底: flow.initial_variables
    // 2. 环境配置: input.environment_name (load_environment_file 会自动叠加 .local.yaml)
    // 3. 显式变量覆盖: input.variables (CLI --var 或调试注入)
    //
    // 显式指定了环境却加载失败时必须直接报错：静默退回流程内置变量会让流程在
    // 错误的端点上执行（例如 prod 流程跑在 fallback 指向的 dev/staging 地址）。
    let env_set = match input.environment_name.as_deref().map(str::trim) {
        Some(env_name) if !env_name.is_empty() => {
            let env_dir = match crate::storage::get_environments_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    return environment_failure_output(
                        &flow_uuid,
                        format!("Failed to resolve environments directory: {}", e),
                    );
                }
            };
            match crate::storage::load_environment_file(env_name, &env_dir) {
                Ok(env) => Some(env),
                Err(e) => {
                    return environment_failure_output(
                        &flow_uuid,
                        format!("Failed to load environment '{}': {}", env_name, e),
                    );
                }
            }
        }
        _ => None,
    };

    let runtime_vars_map = input.variables.as_ref().and_then(|v| {
        v.as_object().map(|obj| {
            obj.iter()
                .map(|(k, val)| (k.clone(), val.clone()))
                .collect::<std::collections::HashMap<_, _>>()
        })
    });

    let merged_variables = flow_engine_core::flow::merge_variable_layers(
        flow.initial_variables.as_ref(),
        env_set.as_ref(),
        None,
        runtime_vars_map.as_ref(),
    );

    let resolved_variables = if merged_variables.is_empty() {
        input.variables
    } else {
        Some(serde_json::json!(merged_variables))
    };

    // 调用执行器，传递 log_sender
    let mut executor = FlowExecutor::new().with_registry(registry);
    if let Some(sender) = execution_event_sender {
        executor = executor.with_execution_event_sender(sender);
    }
    if let Some(sender) = debug_event_sender {
        executor = executor.with_debug_event_sender(sender);
    }
    if let Some(rx) = cancel_receiver {
        executor = executor.with_cancel_receiver(rx);
    }
    let result = executor
        .execute_flow(&flow, resolved_variables, None, log_sender.clone())
        .await;

    // 发送流程结束事件
    let end_timestamp = flow_engine_core::utils::unix_timestamp_ms();

    if let Some(sender) = log_sender {
        let _ = sender
            .send(LogEvent {
                level: if result.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                },
                message: format!(
                    "flow execution {}",
                    if result.success {
                        "succeeded"
                    } else {
                        "failed"
                    }
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

/// 构造「环境解析失败」的失败输出（流程不执行）
///
/// 显式请求的环境无法加载时不能静默降级到流程内置变量，
/// 否则流程可能在错误的端点上执行。
fn environment_failure_output(flow_uuid: &str, message: String) -> ExecuteFlowCommandOutput {
    log::error!("[execute_flow] flow {} aborted: {}", flow_uuid, message);
    ExecuteFlowCommandOutput {
        success: false,
        data: None,
        flow_report: None,
        error: Some(message),
        execution_time: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ExecuteFlowCommandInput;
    use flow_engine_core::connection::Connection;
    use flow_engine_core::flow::Flow;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;
    use flow_engine_core::traits::NoopLogSender;
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    /// 测试用默认共享注册表
    fn test_registry() -> Arc<flow_engine_core::registry::NodeRegistry> {
        Arc::new(crate::default_registry::default_registry())
    }

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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        // 执行流程（不使用日志发送器，不使用执行事件发送器）
        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        // 验证结果
        assert!(result.success, "流程应该Success执行");
        assert!(result.error.is_none(), "不应该有Error");
        assert!(result.flow_report.is_some(), "应该有执行报告");
        // duration_ms 是 u64 类型，总是 >= 0，所以移除无用的比较

        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));
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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "变量Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        // 传入变量
        let variables = json!({
            "api_key": "test-key-123",
            "base_url": "https://api.example.com",
            "retry_count": 3
        });

        let input =
            ExecuteFlowCommandInput::new(flow_id.clone()).with_variables(Some(variables.clone()));

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        // 验证结果
        assert!(result.success);

        // 验证变量被正确传递到报告中
        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(report.initial_variables.is_some());
        let init_vars = report
            .initial_variables
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(init_vars["api_key"], "test-key-123");
        assert_eq!(init_vars["base_url"], "https://api.example.com");
        assert_eq!(init_vars["retry_count"], 3);
    }

    /// 测试缺少开始节点的Error处理
    #[tokio::test]
    async fn test_execute_flow_missing_entry_node() {
        let flow_id = Uuid::new_v4().to_string();

        // 创建空节点的流程
        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "空流程".to_string(),
            description: None,
            nodes: vec![],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        // 验证Error结果
        assert!(!result.success, "流程应该Failed");
        assert!(result.error.is_some(), "应该有Error信息");
        assert_eq!(
            result.error.as_deref(),
            Some("Flow missing start node"),
            "Error信息应该说明缺失开始节点"
        );
        // Error情况下 executed_count 为 0（没有 flow_report 或报告中的 executed_count 为 0）
        if let Some(report) = &result.flow_report {
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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "日志Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        // 使用 NoopLogSender
        let log_sender: Arc<dyn LogSender> = Arc::new(NoopLogSender);

        // 执行流程（应该不会因日志发送而Failed）
        let result = execute_flow(
            input,
            flow,
            Some(log_sender),
            None,
            None,
            None,
            test_registry(),
        )
        .await;

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
            node_type: "entry".to_string(), // 使用 Entry 作为第二个节点进行测试
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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "多节点流程".to_string(),
            description: None,
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        // 验证结果
        assert!(result.success);
        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));
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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "时间Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: flow_name.clone(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        assert!(result.success);
        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));

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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "空变量Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        // 传入空对象
        let input = ExecuteFlowCommandInput::new(flow_id.clone()).with_variables(Some(json!({})));

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        // 应该Success执行
        assert!(result.success);

        // 验证报告中的初始变量为空对象
        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));
        let init_vars = report
            .initial_variables
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(init_vars.is_object());
        assert_eq!(
            init_vars
                .as_object()
                .unwrap_or_else(|| panic!("expected Some"))
                .len(),
            0
        );
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

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "状态Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id.clone());

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        assert!(result.success);
        let report = result
            .flow_report
            .unwrap_or_else(|| panic!("expected Some"));
        let node_report = &report.node_reports[0];

        // 验证节点报告
        assert_eq!(node_report.node_name, node_name);
        assert_eq!(
            node_report.status,
            flow_engine_core::report::ExecutionStatus::Success
        );
    }

    /// 环境目录覆盖是进程级全局状态，并行执行会互相污染 —— 涉及它的测试必须串行
    static ENV_DIR_OVERRIDE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 测试环境层级合并在流程执行时的正确性
    #[tokio::test]
    async fn test_execute_flow_with_environment_layering() {
        use tempfile::TempDir;
        let _guard = ENV_DIR_OVERRIDE_LOCK
            .lock()
            .unwrap_or_else(|e| panic!("{e}"));
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        // 设置环境目录覆盖
        crate::storage::set_environments_dir_override(Some(env_dir.to_path_buf()));

        // 1. 创建 dev.yaml
        let mut dev_vars = std::collections::HashMap::new();
        dev_vars.insert("API_URL".to_string(), json!("http://dev.api"));
        dev_vars.insert("TIMEOUT".to_string(), json!(1000));
        let dev_env = flow_engine_core::flow::EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: dev_vars,
        };
        crate::storage::save_environment_file(&dev_env, false, env_dir).unwrap();

        // 2. 创建 dev.local.yaml
        let mut local_vars = std::collections::HashMap::new();
        local_vars.insert("TIMEOUT".to_string(), json!(9999));
        let local_env = flow_engine_core::flow::EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: local_vars,
        };
        crate::storage::save_environment_file(&local_env, true, env_dir).unwrap();

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

        let mut init_vars = std::collections::HashMap::new();
        init_vars.insert("API_URL".to_string(), json!("http://fallback.api"));
        init_vars.insert("FALLBACK_KEY".to_string(), json!("fallback-val"));

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: Some(init_vars),
            name: "环境分层Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input = ExecuteFlowCommandInput::new(flow_id)
            .with_environment_name(Some("dev".to_string()))
            .with_variables(Some(json!({
                // CLI_KEY 为新增键；API_URL 与 dev.yaml 冲突 → 必须由运行时层胜出
                "CLI_KEY": "cli-val",
                "API_URL": "http://cli.api",
            })));

        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        crate::storage::set_environments_dir_override(None);

        assert!(result.success);
        let report = result.flow_report.unwrap();
        let final_vars = report.final_variables.unwrap();

        // 验证四层优先级：
        // 1. FALLBACK_KEY 仅存在于 initial_variables
        assert_eq!(final_vars["FALLBACK_KEY"], json!("fallback-val"));
        // 2. API_URL 被运行时层（input.variables）覆盖，而不是停在 dev.yaml
        assert_eq!(
            final_vars["API_URL"],
            json!("http://cli.api"),
            "运行时变量必须覆盖环境层"
        );
        // 3. TIMEOUT 来自 dev.local.yaml，覆盖 dev.yaml
        assert_eq!(final_vars["TIMEOUT"], json!(9999));
        // 4. CLI_KEY 为运行时变量新增键
        assert_eq!(final_vars["CLI_KEY"], json!("cli-val"));
    }

    /// 显式请求的环境不存在时必须直接失败，绝不静默退回流程内置变量
    #[tokio::test]
    async fn test_execute_flow_fails_when_environment_missing() {
        use tempfile::TempDir;
        let _guard = ENV_DIR_OVERRIDE_LOCK
            .lock()
            .unwrap_or_else(|e| panic!("{e}"));
        let temp_dir = TempDir::new().unwrap();
        crate::storage::set_environments_dir_override(Some(temp_dir.path().to_path_buf()));

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
        let mut init_vars = std::collections::HashMap::new();
        init_vars.insert("API_URL".to_string(), json!("http://fallback.api"));

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: Some(init_vars),
            name: "缺失环境Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            setup: None,
            teardown: None,
        };

        let input =
            ExecuteFlowCommandInput::new(flow_id).with_environment_name(Some("nope".to_string()));
        let result = execute_flow(input, flow, None, None, None, None, test_registry()).await;

        crate::storage::set_environments_dir_override(None);

        assert!(!result.success, "环境缺失时必须失败，不能退回兜底变量执行");
        let error = result.error.unwrap_or_else(|| panic!("expected error"));
        assert!(error.contains("nope"), "错误信息应包含环境名: {error}");
        assert!(result.flow_report.is_none(), "失败时不应产生执行报告");
    }
}
