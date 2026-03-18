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
use crate::tauri_adapters::create_log_sender;



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
///
/// # 日志发送策略
/// - **调试模式**: 同步发送日志，确保日志顺序和完整性，便于调试
/// - **发布模式**: 队列异步发送日志，不阻塞执行流程，提升性能
#[tauri::command]
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
) -> ExecuteFlowCommandOutput {
    // 创建队列日志发送器（调试模式同步，发布模式异步）
    let (log_sender, log_worker) = create_log_sender(on_event);
    
    // 在后台启动日志消费任务
    tokio::spawn(log_worker);
    
    // 执行流程
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
// 数据库连接命令
// ============================================================================

use flow_engine::db::{DbConnection, DatabasePool, MockPool, TestDbConnectionResult};

/// 测试数据库连接
///
/// 接收前端传来的数据库连接配置，执行连接测试。
/// 当前使用 MockPool 实现（Wave 2），后续会接入真实数据库驱动。
#[tauri::command]
pub async fn test_db_connection(connection: DbConnection) -> TestDbConnectionResult {
    log::info!(
        "[数据库] 测试连接: id={}, name={}, driver={}",
        connection.id,
        connection.name,
        connection.driver
    );

    // Wave 2: 使用 MockPool 测试连接
    // 后续 Wave 会根据 driver 类型创建真实的数据库连接
    let pool = MockPool::new(connection.id.clone());

    match pool.test_connection().await {
        Ok(()) => {
            log::info!("[数据库] 连接测试成功: id={}", connection.id);
            TestDbConnectionResult::success()
        }
        Err(e) => {
            log::error!("[数据库] 连接测试失败: id={}, error={}", connection.id, e);
            TestDbConnectionResult::failure(e)
        }
    }
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
// 调试命令
// ============================================================================

use flow_engine::debug::{DebugCommand, DebugEvent};
use tauri::State;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// ============================================================================
// 压力测试命令
// ============================================================================

use flow_engine::stress_test::{
    GetStressTestStatusOutput, StartStressTestInput, StressTestEvent,
    StressTestSessionManager,
};

/// 调试会话管理器状态
#[derive(Default)]
pub struct DebugSessionManager {
    /// 当前调试会话的命令发送器
    pub command_sender: Option<mpsc::UnboundedSender<DebugCommand>>,
}

/// 启动调试会话
///
/// 开始调试指定的流程，设置断点，并通过 Channel 发送调试事件和日志
#[tauri::command]
pub async fn start_debug(
    flow: Flow,
    breakpoints: Vec<String>,
    on_event: Channel<DebugEvent>,
    on_log: Channel<LogEvent>,
    state: State<'_, Arc<Mutex<DebugSessionManager>>>,
    _app: tauri::AppHandle,
) -> Result<(), String> {
    // 检查是否已有调试会话
    let mut manager = state.lock().await;
    if manager.command_sender.is_some() {
        return Err("已有调试会话正在运行".to_string());
    }

    // 创建命令通道
    let (command_sender, command_receiver) = mpsc::unbounded_channel();

    // 创建事件发送器（包装 Channel）
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<DebugEvent>();
    
    // 克隆 state 的 Arc 用于在 spawn 中清理
    let state_clone = state.inner().clone();
    
    // 启动事件转发任务
    let on_event_for_spawn = on_event;
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let _ = on_event_for_spawn.send(event);
        }
    });

    // 保存命令发送器
    manager.command_sender = Some(command_sender);

    // 创建日志发送器（调试模式也需要日志）
    let (log_sender, log_worker) = create_log_sender(on_log);
    
    // 在后台启动日志消费任务
    tokio::spawn(log_worker);

    // 启动调试会话（在后台任务中）
    tokio::spawn(async move {
        let executor = flow_engine::debug::DebuggableFlowExecutor::new();
        let _ = executor
            .start_debug(
                &flow,
                None,
                breakpoints,
                Some(log_sender),
                event_sender,
                command_receiver,
            )
            .await;
        
        // 清理 command_sender
        let mut manager = state_clone.lock().await;
        manager.command_sender = None;
    });

    Ok(())
}

/// 发送调试命令
///
/// 向当前调试会话发送控制命令（继续、单步、跳过等）
#[tauri::command]
pub async fn send_debug_command(
    command: DebugCommand,
    state: State<'_, Arc<Mutex<DebugSessionManager>>>,
) -> Result<(), String> {
    let manager = state.lock().await;

    if let Some(sender) = &manager.command_sender {
        sender
            .send(command)
            .map_err(|e| format!("发送调试命令失败: {}", e))?;
        Ok(())
    } else {
        Err("没有正在运行的调试会话".to_string())
    }
}

/// 结束调试会话
///
/// 终止当前调试会话
#[tauri::command]
pub async fn end_debug(
    state: State<'_, Arc<Mutex<DebugSessionManager>>>,
) -> Result<(), String> {
    let mut manager = state.lock().await;

    if let Some(sender) = &manager.command_sender {
        // 发送结束调试命令
        let _ = sender.send(DebugCommand::EndDebug);
        manager.command_sender = None;
        Ok(())
    } else {
        Err("没有正在运行的调试会话".to_string())
    }
}

/// 获取调试会话状态
///
/// 检查是否有活跃的调试会话
#[tauri::command]
pub async fn get_debug_status(
    state: State<'_, Arc<Mutex<DebugSessionManager>>>,
) -> Result<bool, String> {
    let manager = state.lock().await;
    Ok(manager.command_sender.is_some())
}

/// 强制结束调试会话
///
/// 强制清理残留的调试会话状态（用于页面刷新后恢复）
#[tauri::command]
pub async fn force_end_debug(
    state: State<'_, Arc<Mutex<DebugSessionManager>>>,
) -> Result<(), String> {
    let mut manager = state.lock().await;
    
    if let Some(sender) = &manager.command_sender {
        // 尝试优雅地结束调试会话
        let _ = sender.send(DebugCommand::EndDebug);
    }
    
    manager.command_sender = None;
    log::info!("[调试] 强制结束调试会话");
    Ok(())
}

// ============================================================================
// 压力测试命令
// ============================================================================

/// 启动压力测试
///
/// 开始压力测试指定的流程，并通过 Channel 发送压测事件
#[tauri::command]
pub async fn start_stress_test(
    input: StartStressTestInput,
    on_event: Channel<StressTestEvent>,
    state: State<'_, Arc<StressTestSessionManager>>,
) -> Result<(), String> {
    // 创建事件发送器（包装 Channel）
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<StressTestEvent>();

    // 启动事件转发任务
    let on_event_for_spawn = on_event;
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let _ = on_event_for_spawn.send(event);
        }
    });

    // 调用命令处理函数
    flow_engine::stress_test::handle_start_stress_test(input, event_sender, state.inner().clone()).await
}

/// 停止压力测试
///
/// 停止当前正在运行的压力测试
#[tauri::command]
pub async fn stop_stress_test(
    state: State<'_, Arc<StressTestSessionManager>>,
) -> Result<(), String> {
    flow_engine::stress_test::handle_stop_stress_test(state.inner().clone()).await
}

/// 获取压力测试状态
///
/// 获取当前压力测试的状态和进度信息
#[tauri::command]
pub async fn get_stress_test_status(
    state: State<'_, Arc<StressTestSessionManager>>,
) -> Result<GetStressTestStatusOutput, String> {
    flow_engine::stress_test::handle_get_stress_test_status(state.inner().clone()).await
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
