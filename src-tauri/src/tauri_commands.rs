//! Tauri 命令模块
//!
//! 提供 #[`tauri::command`] 包装器，调用 flow-engine 的业务逻辑。

use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::Manager;

use flow_engine::command::{
    CreateFlowCommandInput, CreateFlowCommandOutput,
    DeleteNodeCommandInput, DeleteNodeCommandOutput,
    ExecuteFlowCommandInput, ExecuteFlowCommandOutput,
    UpdateNodeCommandInput, UpdateNodeCommandOutput,
};
use flow_engine::log_event::LogEvent;
use flow_engine::report::{FlowExecutionReport, ReportMetadata};
use flow_engine::flow::Flow;
use crate::tauri_adapters::TauriLogSender;



// ============================================================================
// 流程命令
// ============================================================================

/// 创建流程命令
///
/// 创建一个新的流程对象，自动生成唯一流程 ID，并创建开始节点
#[tauri::command]
pub fn create_flow(input: CreateFlowCommandInput) -> CreateFlowCommandOutput {
    flow_engine::commands::create_flow(input)
}

/// 执行流程命令
///
/// 执行指定的流程，调用 `FlowExecutor` 执行引擎，并通过 Channel 发送日志事件
#[tauri::command]
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
) -> ExecuteFlowCommandOutput {
    let log_sender = Arc::new(TauriLogSender::new(on_event));
    flow_engine::commands::execute_flow(input, flow, Some(log_sender)).await
}

/// 更新节点命令（桩实现）
#[tauri::command]
pub fn update_node(input: UpdateNodeCommandInput, flow: Flow) -> UpdateNodeCommandOutput {
    flow_engine::commands::update_node(input, flow)
}

/// 删除节点命令（桩实现）
#[tauri::command]
pub fn delete_node(input: DeleteNodeCommandInput, flow: Flow) -> DeleteNodeCommandOutput {
    flow_engine::commands::delete_node(input, flow)
}

// ============================================================================
// 报告存储命令
// ============================================================================

/// 保存流程执行报告
#[tauri::command]
pub fn save_flow_report(
    report: FlowExecutionReport,
    reports_dir: String,
) -> Result<(), String> {
    flow_engine::commands::save_flow_report(report, &reports_dir)
}

/// 加载流程执行报告
#[tauri::command]
pub fn load_flow_report(
    uuid: String,
    reports_dir: String,
) -> Result<FlowExecutionReport, String> {
    flow_engine::commands::load_flow_report(&uuid, &reports_dir)
}

/// 列出所有报告元数据
#[tauri::command]
pub fn list_flow_reports(reports_dir: String) -> Result<Vec<ReportMetadata>, String> {
    flow_engine::commands::list_flow_reports(&reports_dir)
}

/// 删除流程执行报告
#[tauri::command]
pub fn delete_flow_report(
    uuid: String,
    reports_dir: String,
) -> Result<(), String> {
    flow_engine::commands::delete_flow_report(&uuid, &reports_dir)
}

// ============================================================================
// 应用设置命令
// ============================================================================

/// 获取应用数据目录路径
#[tauri::command]
pub fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("获取应用数据目录失败: {}", e))
}

/// 在文件管理器中打开目录
#[tauri::command]
pub async fn open_directory(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&path, None::<String>)
        .map_err(|e| format!("打开目录失败: {}", e))
}

// ============================================================================
// 单元测试
// ============================================================================

// Windows 上跳过测试：cargo test 会因 Tauri 依赖导致 STATUS_ENTRYPOINT_NOT_FOUND 错误
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;
    use flow_engine::node::{Node, NodePosition, NodeType};
    use flow_engine::node_config::NodeData;
    use flow_engine::connection::Connection;
    use flow_engine::executor::FlowExecutor;


    #[test]
    fn test_create_flow() {
        let input = CreateFlowCommandInput {
            name: "测试流程".to_string(),
        };

        let output = create_flow(input);

        // 验证流程创建成功
        assert_eq!(output.flow.name, "测试流程");

        // 验证流程有唯一 ID
        assert!(!output.flow.uuid.is_empty());

        // 验证流程包含一个开始节点
        assert_eq!(output.flow.nodes.len(), 1);

        // 验证开始节点类型
        assert!(matches!(output.flow.nodes[0].node_type, NodeType::Entry));

        // 验证开始节点名称
        assert_eq!(output.flow.nodes[0].name, "开始");

        // 验证开始节点 ID 格式
        assert!(output.flow.nodes[0].uuid.starts_with("start-"));
        assert!(output.flow.nodes[0].uuid.contains(&output.flow.uuid));

        // 验证连接列表为空
        assert_eq!(output.flow.connections.len(), 0);
    }

    #[test]
    fn test_unique_flow_ids() {
        let input1 = CreateFlowCommandInput {
            name: "流程1".to_string(),
        };
        let input2 = CreateFlowCommandInput {
            name: "流程2".to_string(),
        };

        let output1 = create_flow(input1);
        let output2 = create_flow(input2);

        // 验证两个流程有不同的 ID
        assert_ne!(output1.flow.uuid, output2.flow.uuid);

        // 验证两个开始节点有不同的 ID
        assert_ne!(output1.flow.nodes[0].uuid, output2.flow.nodes[0].uuid);
    }

    #[tokio::test]
    async fn test_execute_flow_success() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None).await;

        // 验证执行成功
        assert!(result.success, "流程应该执行成功");
        assert!(result.error.is_none(), "成功时不应该有错误信息");
        assert!(result.data.is_some(), "成功时应该有返回数据");
        assert_eq!(
            result.data.unwrap()["flow_id"],
            flow_id,
            "返回数据应该包含流程 ID"
        );
    }

    #[tokio::test]
    async fn test_execute_flow_no_connection() {
        // 创建一个没有连接的测试流程
        // 真实执行器认为这是正常流程终止，不是错误
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None).await;

        // 真实执行器认为没有连接是正常终止，不是错误
        assert!(result.success, "流程应该执行成功（Entry 节点无连接是正常终止）");
        assert!(result.error.is_none(), "成功时不应该有错误信息");
    }

    #[tokio::test]
    async fn test_execute_flow_no_start_node() {
        // 创建一个没有开始节点的测试流程
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
            initial_variables: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None).await;

        // 验证执行失败
        assert!(!result.success, "流程应该执行失败（无开始节点）");
        assert!(result.error.is_some(), "失败时应该有错误信息");
        assert!(
            result.error.unwrap().contains("缺少开始节点"),
            "错误信息应该包含 '缺少开始节点'"
        );
    }

    #[test]
    fn test_update_node_not_implemented() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = UpdateNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
            data: json!({ "updated": true }),
        };

        // 更新节点
        let output = update_node(input, flow);

        // 验证更新失败（持久化未实现）
        assert!(!output.success, "update_node 应该返回失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("暂未实现持久化"),
            "错误信息应该包含 '暂未实现持久化'，实际: {}",
            error
        );
        assert!(
            error.contains("update_node 需要先加载流程、更新节点、再保存流程"),
            "错误信息应该说明原因"
        );
    }

    #[test]
    fn test_delete_start_node() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
        };

        // 尝试删除开始节点
        let output = delete_node(input, flow);

        // 验证删除失败
        assert!(!output.success, "删除开始节点应该失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("不能删除开始节点"),
            "错误信息应该包含 '不能删除开始节点'，实际: {}",
            error
        );
    }

    #[test]
    fn test_delete_node_not_implemented() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, http_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // 尝试删除非开始节点
        let output = delete_node(input, flow);

        // 验证删除失败（持久化未实现）
        assert!(!output.success, "delete_node 应该返回失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("暂未实现持久化"),
            "错误信息应该包含 '暂未实现持久化'，实际: {}",
            error
        );
        assert!(
            error.contains("delete_node 需要先加载流程、删除节点和相关连接、再保存流程"),
            "错误信息应该说明原因"
        );
    }

    #[test]
    fn test_create_flow_generates_unique_uuid() {
        let input = CreateFlowCommandInput {
            name: "测试流程".to_string(),
        };

        let output = create_flow(input);

        // 验证 flow.uuid 是有效的 UUID 格式
        let parsed_uuid = Uuid::parse_str(&output.flow.uuid);
        assert!(parsed_uuid.is_ok(), "flow.uuid 应该是有效的 UUID 格式");

        // 验证 start_node.uuid 基于 flow.uuid
        assert!(
            output.flow.nodes[0].uuid.starts_with("start-"),
            "start_node.uuid 应该以 'start-' 开头"
        );
        assert!(
            output.flow.nodes[0].uuid.contains(&output.flow.uuid),
            "start_node.uuid 应该包含 flow.uuid"
        );
    }

    #[tokio::test]
    async fn test_execute_flow_uses_connection_source_node_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        // 验证 conn.source_node_uuid 使用正确的 uuid 字段
        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None).await;

        // 验证使用 conn.source_node_uuid 检查连接
        assert!(result.success, "流程应该执行成功");
    }

    #[test]
    fn test_delete_node_uses_node_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // 验证 delete_node 使用 node.uuid 字段查找节点
        let output = delete_node(input, flow);

        // 验证找到了节点（通过 uuid）
        assert!(
            !output.error.unwrap().contains("不存在"),
            "应该通过 uuid 找到节点"
        );
    }

    #[test]
    fn test_delete_node_nonexistent_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: "non-existent-uuid".to_string(),
        };

        let output = delete_node(input, flow);

        // 验证找不到节点（通过 uuid）
        assert!(!output.success, "删除不存在的节点应该失败");
        assert!(
            output.error.unwrap().contains("不存在"),
            "错误信息应该包含 '不存在'"
        );
    }

    #[test]
    fn test_get_app_data_dir_signature() {
        // 验证 get_app_data_dir 函数签名正确
        // 该命令需要 AppHandle，无法在单元测试中直接调用
        // 通过函数指针类型检查验证签名
        fn _assert_command_signature(_: fn(tauri::AppHandle) -> Result<String, String>) {}
        _assert_command_signature(get_app_data_dir);
    }
}
