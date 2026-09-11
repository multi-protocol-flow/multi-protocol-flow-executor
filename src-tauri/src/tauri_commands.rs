//! Tauri 命令模块
//!
//! 提供 #[`tauri::command`] 包装器，调用 flow-engine 的业务逻辑。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::tauri_adapters::{create_log_sender, TauriDebugEventSender, TauriExecutionEventSender};
use crate::{NodeRegistryState, RegistrySnapshot};
use flow_engine::command::{
    CreateFlowCommandInput, CreateFlowCommandOutput, ExecuteFlowCommandInput,
    ExecuteFlowCommandOutput,
};
use flow_engine::debug::DebugEvent;
use flow_engine::execution_event::ExecutionEvent;
use flow_engine::flow::Flow;
use flow_engine::log_event::LogEvent;
use flow_engine::node::Node;
use flow_engine::plugin::NodeMetadata;
use flow_engine::report::{FlowExecutionReport, NodeExecutionReport, ReportMetadata};
use flow_engine::traits::DebugEventSender;
use flow_engine::traits::ExecutionEventSender;

// ============================================================================
// 执行会话管理
// ============================================================================

/// 执行会话管理器
///
/// 管理当前流程执行会话的取消信号发送端。
/// 使用 `std::sync::Mutex` 保护，因为锁持有时间极短（仅在存取 sender 时）。
pub struct ExecutionSessionManager {
    /// 取消信号发送端映射（会话 ID -> watch channel sender）
    cancel_senders: std::sync::Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
}

impl ExecutionSessionManager {
    /// 创建新的执行会话管理器
    pub fn new() -> Self {
        Self {
            cancel_senders: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for ExecutionSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

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
/// 执行指定的流程，调用 `FlowExecutor` 执行引擎，并通过 Channel 发送日志事件。
/// 创建取消信号通道并存储 sender，允许前端通过 `cancel_flow` 命令取消执行。
///
/// # 日志发送策略
/// - **调试模式**: 同步发送日志，确保日志顺序和完整性，便于调试
/// - **发布模式**: 队列异步发送日志，不阻塞执行流程，提升性能
///
/// # 取消机制
/// - 创建 `watch::channel(false)` 取消信号通道
/// - sender 存储在 `ExecutionSessionManager` 中
/// - receiver 传递给执行引擎用于取消检查
/// - execution complete后自动清理 sender
#[tauri::command]
pub async fn execute_flow(
    app: tauri::AppHandle,
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
    on_execution_event: Channel<ExecutionEvent>,
    on_ws_event: Channel<DebugEvent>,
    state: State<'_, ExecutionSessionManager>,
    node_registry: State<'_, NodeRegistryState>,
) -> Result<ExecuteFlowCommandOutput, String> {
    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let session_id = uuid::Uuid::new_v4().to_string();

    // 子流程按 ID 解析需要与前端列表同一流程目录
    crate::sync_flows_dir_override(&app);

    log::info!(
        "[IPC::execute_flow] Start: uuid={}, name='{}', nodes={}, session={}",
        flow.uuid,
        flow.name,
        flow.nodes.len(),
        session_id
    );

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
        guard.insert(session_id.clone(), cancel_tx);
    }
    // 创建队列日志发送器（调试模式同步，发布模式异步）
    let (log_sender, log_worker) = create_log_sender(on_event);

    // 在后台启动日志消费任务
    tokio::spawn(log_worker);

    // 创建执行事件发送器（包装 Tauri Channel）
    let execution_event_sender: Option<Arc<dyn ExecutionEventSender>> =
        Some(Arc::new(TauriExecutionEventSender::new(on_execution_event)));

    // 创建 WS 流事件发送器（复用 DebugEvent 类型，用于普通执行中的 WS 消息流）
    let debug_event_sender: Option<Arc<dyn DebugEventSender>> =
        Some(Arc::new(TauriDebugEventSender::new(on_ws_event)));

    // 执行流程
    let flow_uuid = flow.uuid.clone();
    let flow_name = flow.name.clone();
    let input_variables = input.variables.clone();
    let snap = node_registry.snapshot();

    let default_timeout = crate::resolve_default_execution_timeout(&app);
    let mut executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(Arc::clone(&snap.registry))
        .with_fallback_timeout(default_timeout);
    if let Some(sender) = execution_event_sender {
        executor = executor.with_execution_event_sender(sender);
    }
    if let Some(sender) = debug_event_sender {
        executor = executor.with_debug_event_sender(sender);
    }
    if let Some(rx) = Some(cancel_rx) {
        executor = executor.with_cancel_receiver(rx);
    }
    executor = executor.with_hook_executor(std::sync::Arc::new(
        flow_engine::hook_runner::DefaultHookExecutor::new(),
    ));

    let result = executor
        .execute_flow(&flow, input_variables, None, Some(log_sender))
        .await;

    // 发送流程结束日志
    let _ = (flow_uuid, flow_name);

    // 先计算 execution_time，避免 move 后借用
    let execution_time = result
        .flow_report
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or(0);
    let output = flow_engine::command::ExecuteFlowCommandOutput {
        success: result.success,
        data: result.data,
        flow_report: result.flow_report,
        error: result.error,
        execution_time,
    };

    // execution complete后清理会话（take sender 使其 drop，后续 cancel_flow 调用将返回"没有活跃的执行会话"）
    // execution complete后清理会话
    {
        let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
        guard.remove(&session_id);
    }
    // 插件据此释放 per-execution 资源（如 Mongo 连接池）。
    // 幂等：仅通知已在运行的插件，不拉起空闲插件。
    #[cfg(feature = "plugin-manager")]
    {
        snap.manager.notify_flow_ended(&result.execution_id).await;
    }
    if output.success {
        log::info!(
            "[IPC::execute_flow] Succeeded: uuid={}, duration={}ms",
            output
                .flow_report
                .as_ref()
                .map(|r| r.flow_uuid.as_str())
                .unwrap_or(""),
            execution_time
        );
    } else {
        log::warn!(
            "[IPC::execute_flow] Failed: uuid={}, duration={}ms, error={:?}",
            output
                .flow_report
                .as_ref()
                .map(|r| r.flow_uuid.as_str())
                .unwrap_or(""),
            execution_time,
            output.error
        );
    }

    Ok(output)
}

/// 取消流程执行
///
/// 向当前正在执行的流程发送取消信号。
/// 幂等操作：第二次调用返回"没有活跃的执行会话"Error。
#[tauri::command]
pub fn cancel_flow(state: State<'_, ExecutionSessionManager>) -> Result<(), String> {
    let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
    let senders: Vec<_> = guard.drain().collect();
    if senders.is_empty() {
        log::warn!("[IPC::cancel_flow] No active execution session to cancel");
        return Err("No active execution session".to_string());
    }
    log::info!(
        "[IPC::cancel_flow] Sending cancel signal to {} session(s)",
        senders.len()
    );
    for (_id, tx) in senders {
        let _ = tx.send(true);
    }
    Ok(())
}

/// 执行单个节点（连接性验证）
///
/// 单独执行一个协议节点，用于前端"测试"功能验证连接性。
/// 与后端 executor 共用 `supports_single_node` 能力检查（内置白名单 +
/// 插件 capabilities.single_node），无独立清单。执行超时时间为 30 秒。
#[tauri::command]
pub async fn execute_single_node(
    app: tauri::AppHandle,
    node: Node,
    variables: Option<serde_json::Value>,
    state: State<'_, ExecutionSessionManager>,
    node_registry: State<'_, NodeRegistryState>,
) -> Result<NodeExecutionReport, String> {
    // 能力检查（纵深防御）——与 executor 内部同一函数，杜绝两份白名单漂移：
    // 内置节点走 SUPPORTED_SINGLE_NODE_TYPES，插件节点走 describe 的
    // capabilities.single_node 声明。
    let type_id = node.data.type_name();
    let snap = node_registry.snapshot();
    if !flow_engine::executor::supports_single_node(&snap.registry, &type_id) {
        return Err(format!(
            "Unsupported node type for single-node execution: {}",
            type_id
        ));
    }

    log::info!(
        "[SingleNode] Execute: type={}, name={}, uuid={}",
        type_id,
        node.name,
        node.uuid
    );

    // 子流程按 ID 解析需要与前端列表同一流程目录
    crate::sync_flows_dir_override(&app);

    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let session_id = uuid::Uuid::new_v4().to_string();

    // 存储 sender 到会话管理器（允许前端通过 cancel_flow 取消）
    {
        let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
        guard.insert(session_id.clone(), cancel_tx);
    }
    let default_timeout = crate::resolve_default_execution_timeout(&app);
    let executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(Arc::clone(&snap.registry))
        .with_fallback_timeout(default_timeout)
        .with_cancel_receiver(cancel_rx)
        .with_hook_executor(std::sync::Arc::new(
            flow_engine::hook_runner::DefaultHookExecutor::new(),
        ));

    let result = executor.execute_single_node(&node, variables, None).await;

    // execution complete后清理会话
    {
        let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
        guard.remove(&session_id);
    }
    log::info!(
        "[SingleNode] Complete: type={}, success={}",
        type_id,
        result.success
    );

    result
        .node_reports
        .into_iter()
        .next()
        .ok_or_else(|| "No execution result".to_string())
}

/// 验证流程
///
/// 检查流程结构是否正确，返回带有Error码的结构化验证结果。
/// 每个Error项包含 `code`（Error码，前端用于 i18n 映射）、`message`（详细描述）和可选的 `field`。
/// 插件节点（`plugin-manager` 特性）额外执行插件的 `validate` RPC。
#[tauri::command]
pub async fn validate_flow(
    flow: Flow,
    app: tauri::AppHandle,
) -> flow_engine::validator::ValidationResult {
    crate::sync_flows_dir_override(&app);
    let mut search_dirs = Vec::new();
    if let Some(dir) = crate::resolve_flows_dir(&app) {
        search_dirs.push(dir);
    }
    if let Ok(dir) = flow_engine::storage::get_flows_dir() {
        if !search_dirs.contains(&dir) {
            search_dirs.push(dir);
        }
    }

    let validator = flow_engine::validator::FlowValidator::new();
    // 插件 validate RPC 是异步的；async 命令直接在 Tauri 运行时内 await，
    // 不阻塞线程，IPC 返回形状（直接返回 ValidationResult）不变。
    // 注：本版本 Tauri 的 async 命令要求引用型入参（State）必须返回 Result，
    // 因此经 AppHandle 取 State，保持非 Result 返回类型不被包装。
    let node_registry = app.state::<NodeRegistryState>();
    let snap = node_registry.snapshot();
    #[cfg(feature = "plugin-manager")]
    {
        validator
            .validate_with_registry_async_and_dirs(
                &flow,
                &snap.registry,
                Some(snap.manager.as_ref()),
                &search_dirs,
            )
            .await
    }
    #[cfg(not(feature = "plugin-manager"))]
    {
        validator.validate_with_registry_and_dirs(&flow, &snap.registry, &search_dirs)
    }
}

// ============================================================================
// 报告存储命令
// ============================================================================

/// 保存流程执行报告
#[tauri::command]
pub fn save_flow_report(report: FlowExecutionReport, reports_dir: String) -> Result<(), String> {
    log::info!(
        "[IPC::save_flow_report] uuid={}, dir={}",
        report.uuid,
        reports_dir
    );
    let res = flow_engine::commands::save_flow_report(report, &reports_dir);
    if let Err(ref e) = res {
        log::error!("[IPC::save_flow_report] FAILED: {}", e);
    }
    res
}

/// 加载流程执行报告
#[tauri::command]
pub fn load_flow_report(uuid: String, reports_dir: String) -> Result<FlowExecutionReport, String> {
    log::debug!("[IPC::load_flow_report] uuid={}, dir={}", uuid, reports_dir);
    let res = flow_engine::commands::load_flow_report(&uuid, &reports_dir);
    if let Err(ref e) = res {
        log::error!("[IPC::load_flow_report] FAILED: uuid={}, error={}", uuid, e);
    }
    res
}

/// 列出所有报告元数据
#[tauri::command]
pub fn list_flow_reports(reports_dir: String) -> Result<Vec<ReportMetadata>, String> {
    log::debug!("[IPC::list_flow_reports] dir={}", reports_dir);
    let res = flow_engine::commands::list_flow_reports(&reports_dir);
    if let Err(ref e) = res {
        log::error!(
            "[IPC::list_flow_reports] FAILED: dir={}, error={}",
            reports_dir,
            e
        );
    }
    res
}

/// 删除流程执行报告
#[tauri::command]
pub fn delete_flow_report(uuid: String, reports_dir: String) -> Result<(), String> {
    log::info!(
        "[IPC::delete_flow_report] uuid={}, dir={}",
        uuid,
        reports_dir
    );
    let res = flow_engine::commands::delete_flow_report(&uuid, &reports_dir);
    if let Err(ref e) = res {
        log::error!(
            "[IPC::delete_flow_report] FAILED: uuid={}, error={}",
            uuid,
            e
        );
    }
    res
}
// ============================================================================
// SSE 连接测试命令
// ============================================================================

/// 测试 SSE 连接
///
/// 设计时调用：验证 SSE 端点 URL 格式有效性。
#[cfg(feature = "protocol-sse")]
#[tauri::command]
pub async fn sse_test_connection(url: String) -> Result<bool, String> {
    // URL 格式验证
    if url.is_empty() {
        return Err("URL must not be empty".to_string());
    }
    // 基本 URL 格式验证
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    Ok(true)
}

// ============================================================================
// TCP 协议模板命令
// ============================================================================

/// 获取所有 TCP 协议内置模板
///
/// 返回所有预置的协议模板（Redis RESP、TLV、固定头部等）。
#[cfg(feature = "protocol-tcp")]
#[tauri::command]
pub fn tcp_list_templates() -> Vec<flow_engine::templates::ProtocolTemplate> {
    flow_engine::templates::builtin_templates()
}

/// 根据 ID 获取指定 TCP 协议模板
///
/// 返回与 ID 匹配的模板，不存在则返回 null。
#[cfg(feature = "protocol-tcp")]
#[tauri::command]
pub fn tcp_get_template(id: String) -> Option<flow_engine::templates::ProtocolTemplate> {
    flow_engine::templates::get_template(&id)
}

// ============================================================================
// GraphQL IPC 命令
// ============================================================================

/// GraphQL 内省请求参数
#[cfg(feature = "protocol-graphql")]
#[derive(Debug, serde::Deserialize)]
pub struct GraphqlIntrospectInput {
    /// GraphQL 端点 URL
    pub endpoint: String,
    /// 请求头
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// GraphQL Schema 内省
///
/// 设计时调用：向 GraphQL 端点发送内省查询，返回完整的 Schema 信息。
#[cfg(feature = "protocol-graphql")]
#[tauri::command]
pub async fn graphql_introspect_schema(
    input: GraphqlIntrospectInput,
) -> flow_engine::traits::GraphqlIntrospectOutput {
    flow_engine::flow_engine_graphql::introspect_endpoint::introspect_endpoint(
        &input.endpoint,
        input.headers.as_ref(),
    )
    .await
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
        .map_err(|e| format!("Failed to get app data directory: {}", e))
}

/// 在文件管理器中打开目录
#[tauri::command]
pub async fn open_directory(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&path, None::<String>)
        .map_err(|e| format!("Failed to open directory: {}", e))
}

/// 保存 CSV 模板文件到应用数据目录下的 templates 子目录
/// 返回写入的文件路径
#[tauri::command]
pub fn save_csv_template(
    app: tauri::AppHandle,
    filename: String,
    content: String,
) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let templates_dir = app_data_dir.join("templates");
    std::fs::create_dir_all(&templates_dir)
        .map_err(|e| format!("Failed to create template directory: {}", e))?;
    let file_path = templates_dir.join(&filename);
    // 写入 UTF-8 BOM + 内容，确保 Excel 正确识别中文编码
    let bom = "\u{FEFF}";
    let bytes = format!("{}{}", bom, content);
    std::fs::write(&file_path, bytes.as_bytes())
        .map_err(|e| format!("Failed to write template file: {}", e))?;
    Ok(file_path.to_string_lossy().to_string())
}

// ============================================================================
// 调试命令
// ============================================================================

use flow_engine::debug::DebugCommand;
use tauri::State;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// ============================================================================
// 压力测试命令
// ============================================================================

use flow_engine::stress_test::{
    GetStressTestStatusOutput, StartStressTestInput, StressTestEvent, StressTestSessionManager,
};

// ============================================================================
// 数据集执行命令
// ============================================================================

use flow_engine::csv_parser;
use flow_engine::dataset_executor::{DatasetEvent, DatasetExecutionConfig};

/// 数据驱动执行会话管理器
///
/// 管理数据驱动执行的取消信号发送端。
pub struct DatasetExecutionSessionManager {
    /// 取消信号发送端映射（会话 ID -> watch channel sender）
    cancel_senders:
        std::sync::Arc<std::sync::Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
}

impl DatasetExecutionSessionManager {
    /// 创建新的数据驱动执行会话管理器
    pub fn new() -> Self {
        Self {
            cancel_senders: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}
impl Default for DatasetExecutionSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 数据驱动执行请求参数
#[derive(serde::Deserialize)]
pub struct ExecuteDatasetInput {
    /// 流程定义
    pub flow: Flow,
    /// 执行配置
    pub config: DatasetExecutionConfig,
}

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
    node_registry: State<'_, NodeRegistryState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 检查是否已有调试会话
    let mut manager = state.lock().await;
    if manager.command_sender.is_some() {
        return Err("A debug session is already running".to_string());
    }

    // 子流程按 ID 解析需要与前端列表同一流程目录
    crate::sync_flows_dir_override(&app);

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
    let debug_registry = Arc::clone(&node_registry.snapshot().registry);
    tokio::spawn(async move {
        let executor =
            flow_engine::debug::DebuggableFlowExecutor::new().with_registry(debug_registry);
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
            .map_err(|e| format!("Failed to send debug command: {}", e))?;
        Ok(())
    } else {
        Err("No running debug session".to_string())
    }
}

/// 结束调试会话
///
/// 终止当前调试会话
#[tauri::command]
pub async fn end_debug(state: State<'_, Arc<Mutex<DebugSessionManager>>>) -> Result<(), String> {
    let mut manager = state.lock().await;

    if let Some(sender) = &manager.command_sender {
        // 发送结束调试命令
        let _ = sender.send(DebugCommand::EndDebug);
        manager.command_sender = None;
        Ok(())
    } else {
        Err("No running debug session".to_string())
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
    log::info!("[Debug] Force end debug session");
    Ok(())
}

// ============================================================================
// 压力测试命令
// ============================================================================

/// 启动压力测试
///
/// 开始压力测试指定的流程，通过 Tauri 事件系统发送压测事件。
/// 使用 `app_handle.emit` 代替 `Channel`，因为压测是长时间后台任务——
/// IPC Channel 在命令返回后会被清理，导致事件无法送达前端。
#[tauri::command]
pub async fn start_stress_test(
    app_handle: tauri::AppHandle,
    input: StartStressTestInput,
    state: State<'_, Arc<StressTestSessionManager>>,
) -> Result<(), String> {
    // 子流程按 ID 解析需要与前端列表同一流程目录
    crate::sync_flows_dir_override(&app_handle);

    // 创建事件发送器
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<StressTestEvent>();

    // 启动事件转发任务：mpsc 事件 → Tauri 全局事件
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let _ = app_handle.emit("stress-test-event", event);
        }
    });

    // 调用命令处理函数
    flow_engine::stress_test::handle_start_stress_test(input, event_sender, state.inner().clone())
        .await
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

/// 保存压力测试报告
#[tauri::command]
pub fn save_stress_test_report(
    report: flow_engine::stress_test::StressTestReport,
    reports_dir: String,
) -> Result<String, String> {
    let path =
        flow_engine::storage::save_stress_report(&report, std::path::Path::new(&reports_dir))?;
    Ok(path.to_string_lossy().to_string())
}

/// 加载压力测试报告
#[tauri::command]
pub fn load_stress_test_report(
    file_path: String,
) -> Result<flow_engine::stress_test::StressTestReport, String> {
    let path = std::path::Path::new(&file_path);
    flow_engine::storage::load_stress_report(path)
}

/// 列出所有压力测试报告摘要
#[tauri::command]
pub fn list_stress_test_reports(
    reports_dir: String,
) -> Result<Vec<flow_engine::storage::StressTestReportSummary>, String> {
    flow_engine::storage::list_stress_reports(std::path::Path::new(&reports_dir))
}

/// 删除压力测试报告
#[tauri::command]
pub fn delete_stress_test_report(file_path: String) -> Result<(), String> {
    flow_engine::storage::delete_stress_report(std::path::Path::new(&file_path))
}

// ============================================================================
// 流程文件存储命令
// ============================================================================

/// 列出所有流程文件元数据
#[tauri::command]
pub fn list_flow_files(
    flows_dir: String,
) -> Result<Vec<flow_engine::storage::FlowMetadata>, String> {
    log::debug!("[IPC::list_flow_files] flows_dir={}", flows_dir);
    let res = flow_engine::storage::list_flow_files(std::path::Path::new(&flows_dir));
    if let Err(ref e) = res {
        log::error!(
            "[IPC::list_flow_files] FAILED: dir={}, error={}",
            flows_dir,
            e
        );
    }
    res
}

/// 加载流程文件（包含 _meta 元数据）
#[tauri::command]
pub fn load_flow_file(
    uuid: String,
    flows_dir: String,
) -> Result<flow_engine::storage::FlowFile, String> {
    log::debug!(
        "[IPC::load_flow_file] uuid={}, flows_dir={}",
        uuid,
        flows_dir
    );
    let res = flow_engine::storage::load_flow_file_raw(&uuid, std::path::Path::new(&flows_dir));
    if let Err(ref e) = res {
        log::error!("[IPC::load_flow_file] FAILED: uuid={}, error={}", uuid, e);
    }
    res
}

/// 保存流程文件（可选携带 _meta 元数据）
#[tauri::command]
pub fn save_flow_file(
    flow: Flow,
    flows_dir: String,
    meta: Option<serde_json::Value>,
) -> Result<(), String> {
    log::info!(
        "[IPC::save_flow_file] uuid={}, name='{}', flows_dir={}",
        flow.uuid,
        flow.name,
        flows_dir
    );
    let res = flow_engine::storage::save_flow_file_with_meta(
        &flow,
        std::path::Path::new(&flows_dir),
        meta,
    );
    if let Err(ref e) = res {
        log::error!(
            "[IPC::save_flow_file] FAILED: uuid={}, error={}",
            flow.uuid,
            e
        );
    }
    res
}

/// 删除流程文件
#[tauri::command]
pub fn delete_flow_file(uuid: String, flows_dir: String) -> Result<(), String> {
    log::info!(
        "[IPC::delete_flow_file] uuid={}, flows_dir={}",
        uuid,
        flows_dir
    );
    let res = flow_engine::storage::delete_flow_file(&uuid, std::path::Path::new(&flows_dir));
    if let Err(ref e) = res {
        log::error!("[IPC::delete_flow_file] FAILED: uuid={}, error={}", uuid, e);
    }
    res
}

/// 复制流程文件
#[tauri::command]
pub fn copy_flow_file(uuid: String, flows_dir: String) -> Result<Flow, String> {
    log::info!(
        "[IPC::copy_flow_file] uuid={}, flows_dir={}",
        uuid,
        flows_dir
    );
    let res = flow_engine::storage::copy_flow_file(&uuid, std::path::Path::new(&flows_dir));
    if let Err(ref e) = res {
        log::error!("[IPC::copy_flow_file] FAILED: uuid={}, error={}", uuid, e);
    }
    res
}

// ============================================================================
// 文件夹管理命令
// ============================================================================

/// 列出所有文件夹
#[tauri::command]
pub fn list_folders(flows_dir: String) -> Result<Vec<flow_engine::storage::FolderInfo>, String> {
    log::debug!("[IPC::list_folders] flows_dir={}", flows_dir);
    let res = flow_engine::storage::list_folders(std::path::Path::new(&flows_dir));
    if let Err(ref e) = res {
        log::error!("[IPC::list_folders] FAILED: dir={}, error={}", flows_dir, e);
    }
    res
}

/// 创建文件夹
#[tauri::command]
pub fn create_folder(
    flows_dir: String,
    folder_path: String,
) -> Result<flow_engine::storage::FolderInfo, String> {
    log::info!(
        "[IPC::create_folder] path={}, flows_dir={}",
        folder_path,
        flows_dir
    );
    let res = flow_engine::storage::create_folder(std::path::Path::new(&flows_dir), &folder_path);
    if let Err(ref e) = res {
        log::error!(
            "[IPC::create_folder] FAILED: path={}, error={}",
            folder_path,
            e
        );
    }
    res
}

/// 删除文件夹
#[tauri::command]
pub fn delete_folder(flows_dir: String, folder_path: String) -> Result<(), String> {
    log::info!(
        "[IPC::delete_folder] path={}, flows_dir={}",
        folder_path,
        flows_dir
    );
    let res = flow_engine::storage::delete_folder(std::path::Path::new(&flows_dir), &folder_path);
    if let Err(ref e) = res {
        log::error!(
            "[IPC::delete_folder] FAILED: path={}, error={}",
            folder_path,
            e
        );
    }
    res
}

/// 重命名文件夹
#[tauri::command]
pub fn rename_folder(
    flows_dir: String,
    folder_path: String,
    new_name: String,
) -> Result<flow_engine::storage::FolderInfo, String> {
    log::info!(
        "[IPC::rename_folder] path={}, new_name={}, flows_dir={}",
        folder_path,
        new_name,
        flows_dir
    );
    let res = flow_engine::storage::rename_folder(
        std::path::Path::new(&flows_dir),
        &folder_path,
        &new_name,
    );
    if let Err(ref e) = res {
        log::error!(
            "[IPC::rename_folder] FAILED: path={}, error={}",
            folder_path,
            e
        );
    }
    res
}

/// 移动流程到指定文件夹
#[tauri::command]
pub fn move_flow_to_folder(
    uuid: String,
    flows_dir: String,
    target_folder_path: String,
) -> Result<(), String> {
    log::info!(
        "[IPC::move_flow_to_folder] uuid={}, target={}, flows_dir={}",
        uuid,
        target_folder_path,
        flows_dir
    );
    let res = flow_engine::storage::move_flow_to_folder(
        &uuid,
        std::path::Path::new(&flows_dir),
        &target_folder_path,
    );
    if let Err(ref e) = res {
        log::error!(
            "[IPC::move_flow_to_folder] FAILED: uuid={}, error={}",
            uuid,
            e
        );
    }
    res
}

/// 移动文件夹到另一个文件夹（或根目录）
#[tauri::command]
pub fn move_folder(
    flows_dir: String,
    source_folder_path: String,
    target_folder_path: String,
) -> Result<(), String> {
    flow_engine::storage::move_folder(
        std::path::Path::new(&flows_dir),
        &source_folder_path,
        &target_folder_path,
    )
}

/// 获取所有已注册的节点类型元数据
///
/// 从共享节点注册表（内置 + 插件节点）获取全部节点类型的元数据信息，
/// 供前端动态渲染节点面板和画布。
#[tauri::command]
pub fn get_registered_node_types(node_registry: State<'_, NodeRegistryState>) -> Vec<NodeMetadata> {
    node_registry.snapshot().registry.all_metadata()
}

/// 获取指定节点类型的配置 JSON Schema
///
/// 通过 `type_id` 查找已注册的节点插件，返回其配置 schema。
/// 若 `type_id` 不存在或插件未提供 schema，返回 None。
#[tauri::command]
pub fn get_node_config_schema(
    type_id: String,
    node_registry: State<'_, NodeRegistryState>,
) -> Option<serde_json::Value> {
    node_registry
        .snapshot()
        .registry
        .get(&type_id)
        .and_then(|plugin| plugin.config_schema())
}

/// 转发插件配置面板的 `uiCall` 发现查询到插件进程
///
/// 配置面板 iframe 无法直接与插件进程通信；运行时发现类查询（如 MCP 工具/
/// 资源/提示词列表）经宿主转发：按 `node_type` 解析插件连接（未知类型即报错，
/// 天然防止一个插件的面板触达另一插件的进程），发起 `uiCall` JSON-RPC 请求并
/// 返回结果。超时用连接自带 request_timeout（默认 30s，与 execute 一致）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn plugin_ui_call(
    node_registry: State<'_, NodeRegistryState>,
    node_type: String,
    method: String,
    payload: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let manager = node_registry.snapshot().manager;
    manager
        .ui_call(
            &node_type,
            &method,
            payload.unwrap_or(serde_json::Value::Null),
        )
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// 数据源插件命令（列表 / browse / get_item / get_templates / validate_config）
// ============================================================================

/// 列出全部已注册数据源（插件 describe 时注册；供左侧栏数据源面板渲染）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub fn list_datasources(
    app: tauri::AppHandle,
) -> Vec<flow_engine::datasource::DatasourceDescription> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    snap.manager
        .list_datasources()
        .into_iter()
        .cloned()
        .collect()
}

/// 浏览数据源某资源（项目列表 / 接口列表）。
///
/// `page`/`size` 缺省为 1/100（与 SDK `BrowseParams` serde default 一致）。
/// `config` 是保存的连接配置（api_key 等），由宿主转发给插件，插件无状态。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn datasource_browse(
    app: tauri::AppHandle,
    datasource_id: String,
    resource_id: String,
    parent_id: Option<String>,
    page: Option<u32>,
    size: Option<u32>,
    config: serde_json::Value,
) -> Result<Vec<flow_engine::datasource::DataItem>, String> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    let params = serde_json::to_value(flow_engine::datasource::BrowseParams {
        config,
        resource_id,
        parent_id,
        page: page.unwrap_or(1),
        size: size.unwrap_or(100),
    })
    .map_err(|e| format!("browse params serialization failed: {e}"))?;
    let value = snap
        .manager
        .datasource_call(
            &datasource_id,
            flow_engine::datasource::BROWSE_METHOD,
            params,
        )
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| format!("invalid browse result: {e}"))
}

/// 获取数据源单个条目详情（`get_item`）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn datasource_get_item(
    app: tauri::AppHandle,
    datasource_id: String,
    resource_id: String,
    item_id: String,
    config: serde_json::Value,
) -> Result<flow_engine::datasource::DataItem, String> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    let params = serde_json::to_value(flow_engine::datasource::GetItemParams {
        config,
        resource_id,
        item_id,
    })
    .map_err(|e| format!("get_item params serialization failed: {e}"))?;
    let value = snap
        .manager
        .datasource_call(
            &datasource_id,
            flow_engine::datasource::GET_ITEM_METHOD,
            params,
        )
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| format!("invalid get_item result: {e}"))
}

/// 把平台条目转换成可落画布的节点模板（`get_templates`）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn datasource_get_templates(
    app: tauri::AppHandle,
    datasource_id: String,
    item_ids: Vec<String>,
    config: serde_json::Value,
) -> Result<Vec<flow_engine::datasource::NodeTemplate>, String> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    let params =
        serde_json::to_value(flow_engine::datasource::GetTemplatesParams { config, item_ids })
            .map_err(|e| format!("get_templates params serialization failed: {e}"))?;
    let value = snap
        .manager
        .datasource_call(
            &datasource_id,
            flow_engine::datasource::GET_TEMPLATES_METHOD,
            params,
        )
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| format!("invalid get_templates result: {e}"))
}

/// 验证数据源连接配置（`validate_config`；保存连接前调用）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn datasource_validate_config(
    app: tauri::AppHandle,
    datasource_id: String,
    config: serde_json::Value,
) -> Result<flow_engine::datasource::ValidateConfigResult, String> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    let params = serde_json::to_value(flow_engine::datasource::ValidateConfigParams { config })
        .map_err(|e| format!("validate_config params serialization failed: {e}"))?;
    let value = snap
        .manager
        .datasource_call(
            &datasource_id,
            flow_engine::datasource::VALIDATE_CONFIG_METHOD,
            params,
        )
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| format!("invalid validate_config result: {e}"))
}

// ============================================================================
// 插件设置命令（目录信息 / 热重载 / 已安装列表 / 市场列表）
// ============================================================================

/// 插件目录来源
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginDirSource {
    /// `MPE_PLUGIN_DIR` 环境变量指定
    Env,
    /// 设置文件 `pluginDir` 键指定
    Config,
    /// 默认目录
    Default,
}

/// 插件目录信息（`get_plugin_dir_info` 返回）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginDirInfo {
    /// 当前生效的插件目录（绝对路径字符串）
    pub effective_dir: String,
    /// 目录来源（env / config / default）
    pub source: PluginDirSource,
    /// 设置文件中的 `pluginDir` 值（未配置则为 None）
    pub configured: Option<String>,
}

/// 插件重载结果（`reload_plugins` 返回）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReloadPluginsResult {
    /// 新快照中已登记（describe 成功）的插件数
    pub plugins_registered: usize,
    /// 新快照的节点类型总数（内置 + 插件）
    pub node_types: usize,
}

/// 已安装插件条目（`list_installed_plugins` 返回）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    /// 插件名
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 描述
    pub description: Option<String>,
    /// 安装目录
    pub path: String,
    /// 状态：`loaded`（已登记进注册表）| `error`
    pub status: String,
    /// 错误/跳过原因（`status == "error"` 时通常非空）
    pub error: Option<String>,
}

/// 获取当前生效的插件目录信息（含来源与设置文件中的配置值）
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub fn get_plugin_dir_info(app: tauri::AppHandle) -> PluginDirInfo {
    let (effective_dir, source) = crate::resolve_plugin_dir(&app);
    let configured = app.store("store/settings.bin").ok().and_then(|store| {
        store
            .get("pluginDir")
            .and_then(|v| v.as_str().map(String::from))
    });
    PluginDirInfo {
        effective_dir: effective_dir.display().to_string(),
        source,
        configured,
    }
}

/// 重载插件目录（运行时热重载，立即生效）
///
/// 用新目录构建全新快照（新 `PluginManager` + describe + 注册表）后原子替换
/// `NodeRegistryState`；旧快照的插件管理器在锁释放后统一 shutdown（终止旧插件
/// 子进程，防止旧注册表 Arc 令其常驻）。重载瞬间正在执行的插件节点可能报错，
/// 属预期行为。压测管理器同步切换到新注册表（新会话生效）。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn reload_plugins(
    app: tauri::AppHandle,
    dir: String,
) -> Result<ReloadPluginsResult, String> {
    log::info!("[IPC::reload_plugins] Start reload from dir: {}", dir);
    let host_locale = app.store("store/settings.bin").ok().and_then(|store| {
        store
            .get("locale")
            .and_then(|v| v.as_str().map(String::from))
    });

    // 新目录 → 新管理器 → describe → 新注册表
    let default_timeout = crate::resolve_default_execution_timeout(&app);
    let conn_config = crate::resolve_plugin_connection_config(&app);
    let mut manager = flow_engine::PluginManager::new(PathBuf::from(&dir))
        .with_host_locale(host_locale)
        .with_connection_config(conn_config)
        .with_default_execution_timeout(default_timeout);
    manager.scan_and_describe().await;

    // 瞬时 spawn 失败容错：安装完成后立刻热重载时，刚解压的二进制可能
    // 仍被文件系统占用（Windows 杀软扫描 / 防病毒锁），首次 describe
    // spawn 失败。对"扫描到但未登记"的插件延迟重试一次 describe，
    // 避免新装的插件一直显示 error 直到重启。
    let registered: std::collections::HashSet<String> = manager
        .registrations()
        .iter()
        .map(|r| r.name.clone())
        .collect();
    let quarantined: std::collections::HashSet<String> =
        manager.quarantined().into_iter().collect();
    let needs_retry = flow_engine::PluginScan::scan(std::path::Path::new(&dir))
        .into_iter()
        .any(|plugin| !registered.contains(&plugin.name) && !quarantined.contains(&plugin.name));
    if needs_retry {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        manager.scan_and_describe().await;
    }

    let manager = Arc::new(manager);
    let registry = Arc::new(flow_engine::default_registry_with_plugins(&manager));

    let plugins_registered = manager.registrations().len();
    let node_types = registry.all_metadata().len();

    let new_snapshot = RegistrySnapshot {
        registry: Arc::clone(&registry),
        plugins_dir: PathBuf::from(&dir),
        manager: Arc::clone(&manager),
    };

    // 原子替换快照（短临界区；Poisoned 锁用 into_inner 恢复）
    let old_snapshot = {
        let state = app.state::<NodeRegistryState>();
        let mut guard = state
            .inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::replace(&mut *guard, new_snapshot)
    };

    // 锁已释放：停掉旧插件进程（防止旧注册表 Arc 令插件子进程常驻）
    old_snapshot
        .manager
        .shutdown_all(Duration::from_secs(5))
        .await;

    // 压测管理器切换到新注册表（新启动的压测会话使用；旧会话沿用旧 Arc 直至结束）
    let stress = app.state::<Arc<StressTestSessionManager>>();
    stress.set_registry(registry);

    log::info!(
        "[IPC::reload_plugins] Succeeded: plugins_registered={}, node_types={}",
        plugins_registered,
        node_types
    );
    Ok(ReloadPluginsResult {
        plugins_registered,
        node_types,
    })
}

/// 列出已安装插件（扫描插件目录 + 加载状态）
///
/// `loaded` 表示该插件已通过 describe 并登记进当前注册表；`Skipped` 条目
/// 以 `error` 状态返回，跳过原因进 `error` 字段。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub fn list_installed_plugins(app: tauri::AppHandle) -> Vec<InstalledPlugin> {
    let snap = app.state::<NodeRegistryState>().snapshot();
    // 与 flow-engine-plugin discovery 的 host_version() 对齐：
    // MPE_HOST_VERSION 覆盖，否则取本 crate 包版本
    let host_version =
        std::env::var("MPE_HOST_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let loaded: HashSet<&str> = snap
        .manager
        .registrations()
        .iter()
        .map(|registration| registration.name.as_str())
        .collect();
    flow_engine::PluginScan::scan_with_errors(&snap.plugins_dir, &host_version)
        .into_iter()
        .filter_map(|entry| match entry {
            flow_engine::ScanEntry::Discovered(plugin) => Some(InstalledPlugin {
                name: plugin.name.clone(),
                version: plugin.version.clone(),
                description: plugin.manifest.description.clone(),
                path: plugin.dir.display().to_string(),
                status: if loaded.contains(plugin.name.as_str()) {
                    "loaded".to_string()
                } else {
                    "error".to_string()
                },
                // describe 失败原因（未 loaded）：来自 PluginManager 最近一次
                // describe 失败记录；扫描阶段即 log::warn，这里同步到 UI。
                error: if loaded.contains(plugin.name.as_str()) {
                    None
                } else {
                    snap.manager.last_error(&plugin.name)
                },
            }),
            flow_engine::ScanEntry::Skipped { dir, reason } => {
                if matches!(
                    reason,
                    flow_engine::ScanSkipReason::MissingManifest
                        | flow_engine::ScanSkipReason::NotADirectory
                ) {
                    None
                } else {
                    Some(InstalledPlugin {
                        name: dir
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        version: String::new(),
                        description: None,
                        path: dir.display().to_string(),
                        status: "error".to_string(),
                        error: Some(format!("{reason:?}")),
                    })
                }
            }
        })
        .collect()
}

/// 从插件市场拉取插件列表（`GET {base}/plugins`，契约见 docs/plugin-marketplace.md）
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn list_marketplace_plugins(
    url: String,
) -> Result<Vec<flow_engine::flow_engine_plugin_marketplace::MarketplacePlugin>, String> {
    flow_engine::flow_engine_plugin_marketplace::MarketplaceClient::new(url)
        .list_plugins()
        .await
        .map_err(|e| e.to_string())
}

/// 从插件市场安装插件（下载 + sha256 校验 + 解压到生效插件目录 `<dir>/<name>/`）
///
/// 安装目录解析与 `get_plugin_dir_info` 一致（env > 设置文件 > 默认）。
/// `version` 缺省取市场列表第一个匹配项（服务端按版本降序返回）；
/// `force` 覆盖已存在的安装。
#[cfg(feature = "plugin-manager")]
#[tauri::command]
pub async fn install_marketplace_plugin(
    app: tauri::AppHandle,
    url: String,
    name: String,
    version: Option<String>,
    force: bool,
) -> Result<flow_engine::flow_engine_plugin_marketplace::InstalledPluginInfo, String> {
    log::info!(
        "[IPC::install_marketplace_plugin] Installing plugin '{}', version={:?}, force={}, url={}",
        name,
        version,
        force,
        url
    );
    let install_dir = crate::resolve_plugin_dir(&app).0;
    let client = flow_engine::flow_engine_plugin_marketplace::MarketplaceClient::new(url);
    let plugins = client.list_plugins().await.map_err(|e| e.to_string())?;
    let plugin = plugins
        .iter()
        .find(|plugin| {
            plugin.name == name && version.as_deref().is_none_or(|v| plugin.version == v)
        })
        .ok_or_else(|| {
            format!(
                "plugin {name}{} not found in marketplace",
                version.map(|v| format!("@{v}")).unwrap_or_default()
            )
        })?;
    let res = flow_engine::flow_engine_plugin_marketplace::install_plugin(
        &client,
        plugin,
        &install_dir,
        force,
    )
    .await
    .map_err(|e| e.to_string());
    match &res {
        Ok(info) => log::info!(
            "[IPC::install_marketplace_plugin] Succeeded: name={}, path={:?}",
            info.name,
            info.path
        ),
        Err(e) => log::error!(
            "[IPC::install_marketplace_plugin] FAILED: name={}, error={}",
            name,
            e
        ),
    }
    res
}

// ============================================================================
// 数据集执行命令
// ============================================================================

/// 验证 CSV 文件元信息
///
/// 读取 CSV 文件并返回行数、列名和检测到的编码信息。
#[tauri::command]
pub fn validate_dataset_csv(
    path: String,
    encoding: Option<String>,
    delimiter: Option<char>,
) -> Result<serde_json::Value, String> {
    let info = csv_parser::validate(&path, encoding.as_deref(), delimiter)?;
    Ok(serde_json::json!({
        "row_count": info.row_count,
        "column_names": info.column_names,
        "detected_encoding": info.detected_encoding,
    }))
}

/// 启动数据驱动执行
///
/// 从 CSV 文件逐行读取数据，每行注入为 `runtime_variables` 执行流程。
/// 通过 Tauri 事件系统实时向前端发送执行事件。
/// 使用 `app.emit` 代替 `Channel`，因为数据集执行是长时间后台任务。
#[tauri::command]
pub async fn execute_flow_with_dataset(
    app: tauri::AppHandle,
    input: ExecuteDatasetInput,
    state: State<'_, DatasetExecutionSessionManager>,
    node_registry: State<'_, NodeRegistryState>,
) -> Result<(), String> {
    // 子流程按 ID 解析需要与前端列表同一流程目录
    crate::sync_flows_dir_override(&app);

    // 获取应用数据目录（用于保存Failed行详情）
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let session_id = uuid::Uuid::new_v4().to_string();

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
        guard.insert(session_id.clone(), cancel_tx);
    }
    // 创建事件通道（mpsc → Tauri 全局事件）
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<DatasetEvent>();

    // 启动事件转发任务：mpsc → Tauri 全局事件
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let _ = app_clone.emit("dataset-event", event);
        }
    });

    // 克隆必要数据用于后台任务
    let flow = input.flow;
    let config = input.config;
    let state_cancel = state.cancel_senders.clone();
    let session_id_clone = session_id.clone();
    let registry = Arc::clone(&node_registry.snapshot().registry);
    // 在后台启动执行
    tokio::spawn(async move {
        let executor = flow_engine::dataset_executor::DatasetExecutor::new(registry);
        let result = executor
            .execute(&config, &flow, Some(cancel_rx), Some(event_sender))
            .await;

        // 保存所有行详情到磁盘
        if let Ok((_summaries, row_details, report_uuid)) = result {
            if !row_details.is_empty() {
                // 使用 executor 返回的 report UUID 直接定位目录
                let report_dir = app_data_dir
                    .join("reports")
                    .join(format!("dataset_{}", report_uuid));
                let rows_dir = report_dir.join("rows");
                if let Ok(()) = std::fs::create_dir_all(&rows_dir) {
                    for (row_index, flow_report) in &row_details {
                        let row_path = rows_dir.join(format!("row_{}.json", row_index));
                        if let Ok(row_json) = serde_json::to_string_pretty(flow_report) {
                            let _ = std::fs::write(&row_path, row_json);
                        }
                    }
                }
            }
        }

        // execution complete后清理 cancel sender
        if let Ok(mut guard) = state_cancel.lock() {
            guard.remove(&session_id_clone);
        }
    });

    Ok(())
}

/// 取消数据驱动执行
///
/// 向当前正在运行的数据驱动执行发送取消信号。
/// 幂等操作：第二次调用返回"没有活跃的执行会话"Error。
#[tauri::command]
pub fn cancel_dataset_execution(
    state: State<'_, DatasetExecutionSessionManager>,
) -> Result<(), String> {
    let mut guard = state.cancel_senders.lock().map_err(|e| e.to_string())?;
    let senders: Vec<_> = guard.drain().collect();
    if senders.is_empty() {
        return Err("No active data-driven execution session".to_string());
    }
    for (_id, tx) in senders {
        let _ = tx.send(true);
    }
    Ok(())
}
/// 列出数据集报告
///
/// 扫描报告目录中所有 dataset_* 子目录，读取 summary.json 返回摘要列表。
#[tauri::command]
pub fn list_dataset_reports(reports_dir: String) -> Result<Vec<serde_json::Value>, String> {
    let reports_path = std::path::Path::new(&reports_dir);
    if !reports_path.exists() {
        return Ok(vec![]);
    }

    let mut summaries: Vec<serde_json::Value> = Vec::new();

    let entries = std::fs::read_dir(reports_path)
        .map_err(|e| format!("Failed to read report directory: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // 只处理 dataset_* 目录
        if !dir_name.starts_with("dataset_") {
            continue;
        }

        let summary_path = path.join("summary.json");
        if !summary_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&summary_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let report: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // 提取摘要字段
        let report_id = dir_name.strip_prefix("dataset_").unwrap_or(&dir_name);
        summaries.push(serde_json::json!({
            "report_id": report_id,
            "flow_name": report["flow_name"].as_str().unwrap_or(""),
            "dataset_name": report["dataset_name"].as_str().unwrap_or(""),
            "executed_at": report["executed_at"].as_u64().unwrap_or(0),
            "total_rows": report["total_rows"].as_u64().unwrap_or(0),
            "success_count": report["success_count"].as_u64().unwrap_or(0),
            "fail_count": report["fail_count"].as_u64().unwrap_or(0),
            "error_count": report["error_count"].as_u64().unwrap_or(0),
        }));
    }

    Ok(summaries)
}

/// 加载数据集报告
///
/// 读取指定报告 ID 的 summary.json 文件。
#[tauri::command]
pub fn load_dataset_report(
    report_id: String,
    reports_dir: String,
) -> Result<serde_json::Value, String> {
    let summary_path = std::path::Path::new(&reports_dir)
        .join(format!("dataset_{}", report_id))
        .join("summary.json");

    if !summary_path.exists() {
        return Err(format!("Report not found: {}", report_id));
    }

    let content = std::fs::read_to_string(&summary_path)
        .map_err(|e| format!("Failed to read report: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse report: {}", e))
}

/// 加载Failed行详情
///
/// 读取行的详细执行报告。
#[tauri::command]
pub fn load_dataset_row_detail(
    report_id: String,
    row_index: u32,
    reports_dir: String,
) -> Result<Option<serde_json::Value>, String> {
    let row_path = std::path::Path::new(&reports_dir)
        .join(format!("dataset_{}", report_id))
        .join("rows")
        .join(format!("row_{}.json", row_index));

    if !row_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&row_path)
        .map_err(|e| format!("Failed to read row details: {}", e))?;

    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse row details: {}", e))?;

    Ok(Some(value))
}

/// 删除数据集报告
///
/// 删除指定报告 ID 的整个目录。
#[tauri::command]
pub fn delete_dataset_report(report_id: String, reports_dir: String) -> Result<(), String> {
    let report_dir = std::path::Path::new(&reports_dir).join(format!("dataset_{}", report_id));

    if !report_dir.exists() {
        return Err(format!("Report not found: {}", report_id));
    }

    std::fs::remove_dir_all(&report_dir).map_err(|e| format!("Failed to delete report: {}", e))
}

/// 保存数据集报告
///
/// 将数据驱动执行报告保存到磁盘。
/// 注意: `reports_dir` 是报告目录 (如 `{appDataDir}/reports/`)，
/// `save_report` 内部会拼接 `reports/`，所以需要传入父目录。
#[tauri::command]
pub fn save_dataset_report(
    report: serde_json::Value,
    reports_dir: String,
) -> Result<String, String> {
    use flow_engine::dataset::DatasetExecutionReport;
    use flow_engine::report::FlowExecutionReport;
    use std::collections::HashMap;

    let parsed_report: DatasetExecutionReport =
        serde_json::from_value(report).map_err(|e| format!("Failed to parse report: {}", e))?;

    // save_report expects app_data_dir (parent of reports/)，所以去掉末尾的 "reports" 组件
    let reports_path = std::path::Path::new(&reports_dir);
    let app_data_dir = reports_path
        .parent()
        .ok_or_else(|| "Cannot get parent directory of report directory".to_string())?;

    let failed_details: HashMap<u32, FlowExecutionReport> = HashMap::new();
    flow_engine::dataset_executor::save_report(&parsed_report, app_data_dir, &failed_details)
}

// ============================================================================
// AI Copilot IPC 命令
// ============================================================================

/// AI 连通性测试结果
#[derive(Debug, Serialize, Deserialize)]
pub struct AiTestConnectionResult {
    /// 是否连接成功
    pub success: bool,
    /// 响应延迟毫秒数
    pub latency_ms: u64,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

fn normalize_ai_base_url(url: &str) -> String {
    let mut cleaned = url.trim().trim_end_matches('/').to_string();
    if cleaned.ends_with("/chat/completions") {
        cleaned = cleaned[..cleaned.len() - "/chat/completions".len()].to_string();
    }
    if !cleaned.ends_with("/v1") && !cleaned.contains("/v1/") && !cleaned.ends_with("/api") {
        if let Some(pos) = cleaned.find("://") {
            let rest = &cleaned[pos + 3..];
            if !rest.contains('/') {
                cleaned = format!("{}/v1", cleaned);
            }
        }
    }
    cleaned
}

fn build_ai_reqwest_client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn build_ai_streaming_client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(180))
        .tcp_keepalive(Some(Duration::from_secs(15)))
        .tcp_nodelay(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn format_reqwest_error(e: &reqwest::Error) -> String {
    use std::error::Error;
    let mut messages = Vec::new();
    let mut current: Option<&dyn Error> = Some(e);
    while let Some(err) = current {
        let msg = err.to_string();
        if !messages
            .iter()
            .any(|m: &String| m == &msg || msg.contains(m.as_str()))
        {
            messages.push(msg);
        }
        current = err.source();
    }
    let detail = messages.join(" -> ");
    let lower = detail.to_lowercase();

    let hint = if e.is_timeout() || lower.contains("timed out") || lower.contains("timeout") {
        "Request timed out"
    } else if lower.contains("connection reset")
        || lower.contains("connection closed")
        || lower.contains("broken pipe")
        || e.is_body()
    {
        "Data stream interrupted (connection broken or reset by remote server or proxy)"
    } else if e.is_connect()
        || lower.contains("dns")
        || lower.contains("connection refused")
        || lower.contains("proxy")
    {
        "Failed to connect to host (check network, DNS, or proxy settings)"
    } else if e.is_decode() {
        "Response decoding error"
    } else {
        "Network request failed"
    };

    format!("[{}] {}", hint, detail)
}

/// 测试 AI Provider 连通性
#[tauri::command]
pub async fn ai_test_connection(
    base_url: String,
    api_key: String,
    model: String,
    custom_headers: Option<HashMap<String, String>>,
) -> Result<AiTestConnectionResult, String> {
    let start_time = std::time::Instant::now();
    let cleaned_base = normalize_ai_base_url(&base_url);
    let endpoint = format!("{}/chat/completions", cleaned_base);
    log::info!(
        "[IPC::ai_test_connection] Testing AI provider: endpoint={}, model={}, has_key={}",
        endpoint,
        model,
        !api_key.trim().is_empty()
    );
    let client = build_ai_reqwest_client();
    let mut req = client.post(&endpoint).json(&serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 5
    }));

    if !api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
    }
    if let Some(headers) = custom_headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    match req.send().await {
        Ok(resp) => {
            let latency_ms = start_time.elapsed().as_millis() as u64;
            if resp.status().is_success() {
                log::info!("[IPC::ai_test_connection] Succeeded in {}ms", latency_ms);
                Ok(AiTestConnectionResult {
                    success: true,
                    latency_ms,
                    error: None,
                })
            } else {
                let status = resp.status();
                let err_body = resp.text().await.unwrap_or_default();
                let err_msg = format!("HTTP {}: {}", status, err_body);
                log::warn!(
                    "[IPC::ai_test_connection] Failed in {}ms: {}",
                    latency_ms,
                    err_msg
                );
                Ok(AiTestConnectionResult {
                    success: false,
                    latency_ms,
                    error: Some(err_msg),
                })
            }
        }
        Err(e) => {
            let latency_ms = start_time.elapsed().as_millis() as u64;
            let err_msg = format_reqwest_error(&e);
            log::error!(
                "[IPC::ai_test_connection] Network error in {}ms: {}",
                latency_ms,
                err_msg
            );
            Ok(AiTestConnectionResult {
                success: false,
                latency_ms,
                error: Some(err_msg),
            })
        }
    }
}

/// 拉取 AI 模型列表
#[tauri::command]
pub async fn ai_fetch_models(
    base_url: String,
    api_key: String,
    custom_headers: Option<HashMap<String, String>>,
) -> Result<Vec<String>, String> {
    let cleaned_base = normalize_ai_base_url(&base_url);
    let endpoint = format!("{}/models", cleaned_base);
    log::info!(
        "[IPC::ai_fetch_models] Fetching models from: {}, has_key={}",
        endpoint,
        !api_key.trim().is_empty()
    );
    let client = build_ai_reqwest_client();
    let mut req = client.get(&endpoint);
    if !api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
    }
    if let Some(headers) = custom_headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let err_msg = format_reqwest_error(&e);
            log::error!("[IPC::ai_fetch_models] Network request failed: {}", err_msg);
            return Err(err_msg);
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "[IPC::ai_fetch_models] Non-success response: HTTP {} - {}",
            status,
            body
        );
        return Err(format!("HTTP {}: {}", status, body));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| {
        log::error!("[IPC::ai_fetch_models] JSON decode error: {}", e);
        e.to_string()
    })?;
    let mut models = Vec::new();
    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            }
        }
    }

    Ok(models)
}

/// 流式 AI 对话请求 (通过 Channel 实时推送 SSE 数据块)
#[tauri::command]
pub async fn ai_chat_stream(
    base_url: String,
    api_key: String,
    request_body: serde_json::Value,
    custom_headers: Option<HashMap<String, String>>,
    on_chunk: Channel<String>,
) -> Result<(), String> {
    let cleaned_base = normalize_ai_base_url(&base_url);
    let endpoint = format!("{}/chat/completions", cleaned_base);

    let client = build_ai_streaming_client();
    let mut req = client.post(&endpoint).json(&request_body);

    if !api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
    }
    if let Some(headers) = custom_headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    let mut resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let err_msg = format_reqwest_error(&e);
            log::error!("[IPC::ai_chat_stream] Failed to send request: {}", err_msg);
            return Err(err_msg);
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        log::warn!(
            "[IPC::ai_chat_stream] HTTP non-success: {} - {}",
            status,
            err_body
        );
        return Err(format!("HTTP {}: {}", status, err_body));
    }

    while let Some(chunk_result) = resp.chunk().await.map_err(|e| {
        let err_msg = format_reqwest_error(&e);
        log::error!("[IPC::ai_chat_stream] Stream chunk error: {}", err_msg);
        err_msg
    })? {
        let text = String::from_utf8_lossy(&chunk_result).to_string();
        if on_chunk.send(text).is_err() {
            // Frontend listener channel disconnected, stop streaming
            break;
        }
    }
    Ok(())
}

// ============================================================================
// AI 工作区感知与代码分析命令
// ============================================================================

/// 扫描本地工程目录，提取技术栈、配置文件、接口规范与核心代码目录
#[tauri::command]
pub fn ai_scan_workspace(
    workspace_dir: String,
) -> Result<flow_engine::workspace::WorkspaceScanResult, String> {
    let path = std::path::Path::new(&workspace_dir);
    flow_engine::workspace::scan_workspace(path, 6)
}

/// 基于正则表达式检索工作区中的源代码，按文件与行号返回匹配结果
#[tauri::command]
pub fn ai_search_code(
    workspace_dir: String,
    pattern: String,
    ext: Option<String>,
    max_results: Option<usize>,
) -> Result<Vec<flow_engine::workspace::CodeMatch>, String> {
    let path = std::path::Path::new(&workspace_dir);
    flow_engine::workspace::search_workspace_code(
        path,
        &pattern,
        ext.as_deref(),
        max_results.unwrap_or(20),
    )
}

/// 按行范围切片读取代码文件内容，带防溢出安全截断保护
#[tauri::command]
pub fn ai_read_code_slice(
    file_path: String,
    start_line: usize,
    max_lines: usize,
) -> Result<String, String> {
    let path = std::path::Path::new(&file_path);
    flow_engine::workspace::read_workspace_slice(path, start_line, max_lines)
}

/// 解析接口契约规范文件 (OpenAPI JSON/YAML 或 Protobuf .proto)
#[tauri::command]
pub fn ai_inspect_api_spec(file_path: String) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(&file_path);
    flow_engine::workspace::inspect_api_spec(path)
}

// ============================================================================
// 版本检查与更新命令
// ============================================================================

/// 获取当前应用编译包版本号
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 检查应用新版本
#[tauri::command]
pub async fn check_app_update(
    include_prerelease: Option<bool>,
) -> Result<crate::updater::UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    crate::updater::check_for_update(current_version, include_prerelease).await
}

/// 下载安装包并通过 `app-update-progress` 事件实时推送下载进度
#[tauri::command]
pub async fn download_update_package(app: tauri::AppHandle, url: String) -> Result<String, String> {
    use tauri::Emitter;
    let app_clone = app.clone();
    let file_path: std::path::PathBuf =
        crate::updater::download_update_package(&url, move |progress| {
            let _ = app_clone.emit("app-update-progress", progress);
        })
        .await?;
    Ok(file_path.display().to_string())
}

/// 执行下载好的安装包，并在 Windows 上退出主程序完成覆盖安装
#[tauri::command]
pub fn install_update_package(app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    crate::updater::launch_installer(path, &app)
}

// ============================================================================
// 单元测试
// ============================================================================

// Windows 上跳过测试：cargo test 会因 Tauri 依赖导致 STATUS_ENTRYPOINT_NOT_FOUND Error
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use flow_engine::connection::Connection;
    use flow_engine::executor::FlowExecutor;
    use flow_engine::node::{Node, NodePosition};
    use flow_engine::node_config::NodeData;
    use uuid::Uuid;

    #[test]
    fn test_create_flow() {
        let input = CreateFlowCommandInput {
            name: "Test Flow".to_string(),
        };

        let output = create_flow(input);

        // 验证流程创建Success
        assert_eq!(output.flow.name, "Test Flow");

        // 验证流程有唯一 ID
        assert!(!output.flow.uuid.is_empty());

        // 验证流程包含开始节点和结束节点
        assert_eq!(output.flow.nodes.len(), 2);

        // 验证开始节点类型
        assert_eq!(output.flow.nodes[0].node_type, "entry");

        // 验证开始节点名称
        assert_eq!(output.flow.nodes[0].name, "Start");

        // 验证开始节点 ID 格式
        assert!(output.flow.nodes[0].uuid.starts_with("start-"));
        assert!(output.flow.nodes[0].uuid.contains(&output.flow.uuid));

        // 验证结束节点
        assert_eq!(output.flow.nodes[1].node_type, "end");
        assert_eq!(output.flow.nodes[1].name, "End");

        // 验证连接列表为空
        assert_eq!(output.flow.connections.len(), 0);
    }

    #[test]
    fn test_unique_flow_ids() {
        let input1 = CreateFlowCommandInput {
            name: "Flow 1".to_string(),
        };
        let input2 = CreateFlowCommandInput {
            name: "Flow 2".to_string(),
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
        // 创建一个Test Flow
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "Start".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
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
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行Success
        assert!(result.success, "flow should execute successfully");
        assert!(result.error.is_none(), "should have no error on success");
        assert!(result.data.is_some(), "should have data on success");
        assert_eq!(
            result.data.unwrap()["flow_id"],
            flow_id,
            "return data should contain flow ID"
        );
    }

    #[tokio::test]
    async fn test_execute_flow_no_connection() {
        // 创建一个没有连接的Test Flow
        // 真实执行器认为这是正常流程终止，不是Error
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "Start".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 真实执行器认为没有连接是正常终止，不是Error
        assert!(
            result.success,
            "flow should succeed (Entry with no connections is normal termination)"
        );
        assert!(result.error.is_none(), "should have no error on success");
    }

    #[tokio::test]
    async fn test_execute_flow_no_start_node() {
        // 创建一个没有开始节点的Test Flow
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![http_node],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行Failed（校验先于执行，报缺少 Entry 节点）
        assert!(!result.success, "flow should fail (no start node)");
        assert!(result.error.is_some(), "should have error on failure");
        let error = result.error.unwrap();
        assert!(
            error.contains("missing an Entry node"),
            "error should contain 'missing an Entry node', got: {}",
            error
        );
    }

    #[test]
    fn test_create_flow_generates_unique_uuid() {
        let input = CreateFlowCommandInput {
            name: "Test Flow".to_string(),
        };

        let output = create_flow(input);

        // verify flow.uuid is valid UUID format
        let parsed_uuid = Uuid::parse_str(&output.flow.uuid);
        assert!(parsed_uuid.is_ok(), "flow.uuid should be valid UUID format");

        // verify start_node.uuid based on flow.uuid
        assert!(
            output.flow.nodes[0].uuid.starts_with("start-"),
            "start_node.uuid should start with 'start-'"
        );
        assert!(
            output.flow.nodes[0].uuid.contains(&output.flow.uuid),
            "start_node.uuid should contain flow.uuid"
        );
    }

    #[tokio::test]
    async fn test_execute_flow_uses_connection_source_node_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "Start".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        // 直接使用 FlowExecutor 测试（避免 Tauri Channel 依赖）
        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证使用 conn.source_node_uuid 检查连接
        assert!(result.success, "flow should execute successfully");
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
