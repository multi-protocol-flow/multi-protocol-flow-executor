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
use flow_engine::execution_event::ExecutionEvent;
use flow_engine::traits::ExecutionEventSender;
use flow_engine::traits::DebugEventSender;
use flow_engine::report::{FlowExecutionReport, ReportMetadata};
use flow_engine::flow::Flow;
use flow_engine::debug::DebugEvent;
use flow_engine::plugin::NodeMetadata;
use crate::tauri_adapters::{create_log_sender, TauriExecutionEventSender, TauriDebugEventSender};

// ============================================================================
// 执行会话管理
// ============================================================================

/// 执行会话管理器
///
/// 管理当前流程执行会话的取消信号发送端。
/// 使用 `std::sync::Mutex` 保护，因为锁持有时间极短（仅在存取 sender 时）。
pub struct ExecutionSessionManager {
    /// 取消信号发送端（watch channel）
    cancel_sender: std::sync::Mutex<Option<tokio::sync::watch::Sender<bool>>>,
}

impl ExecutionSessionManager {
    /// 创建新的执行会话管理器
    pub fn new() -> Self {
        Self {
            cancel_sender: std::sync::Mutex::new(None),
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
/// - 执行完成后自动清理 sender
#[tauri::command]
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
    on_execution_event: Channel<ExecutionEvent>,
    on_ws_event: Channel<DebugEvent>,
    state: State<'_, ExecutionSessionManager>,
) -> Result<ExecuteFlowCommandOutput, String> {
    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel_tx);
    }

    // 创建队列日志发送器（调试模式同步，发布模式异步）
    let (log_sender, log_worker) = create_log_sender(on_event);

    // 在后台启动日志消费任务
    tokio::spawn(log_worker);

    // 创建执行事件发送器（包装 Tauri Channel）
    let execution_event_sender: Option<Arc<dyn ExecutionEventSender>> = Some(
        Arc::new(TauriExecutionEventSender::new(on_execution_event))
    );

    // 创建 WS 流事件发送器（复用 DebugEvent 类型，用于普通执行中的 WS 消息流）
    let debug_event_sender: Option<Arc<dyn DebugEventSender>> = Some(
        Arc::new(TauriDebugEventSender::new(on_ws_event))
    );

    // 执行流程（传入取消信号 receiver）
    let result = flow_engine::commands::execute_flow(
        input, flow, Some(log_sender), execution_event_sender, debug_event_sender, Some(cancel_rx),
    ).await;

    // 执行完成后清理会话（take sender 使其 drop，后续 cancel_flow 调用将返回"没有活跃的执行会话"）
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = None;
    }

    Ok(result)
}

/// 取消流程执行
///
/// 向当前正在执行的流程发送取消信号。
/// 幂等操作：第二次调用返回"没有活跃的执行会话"错误。
#[tauri::command]
pub fn cancel_flow(state: State<'_, ExecutionSessionManager>) -> Result<(), String> {
    let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
    match guard.take() {
        Some(tx) => {
            let _ = tx.send(true);
            Ok(())
        }
        None => Err("没有活跃的执行会话".to_string()),
    }
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
/// 当前使用 `MockPool` 实现（Wave 2），后续会接入真实数据库驱动。
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
// gRPC 服务发现命令
// ============================================================================

use flow_engine::grpc_connection_manager;
use flow_engine::traits::GrpcServiceInfo;

/// gRPC 服务发现请求参数
#[derive(serde::Deserialize)]
pub struct GrpcDiscoverServicesInput {
    /// gRPC 服务器地址
    pub url: String,
    /// 是否使用 TLS
    pub use_tls: Option<bool>,
    /// 是否跳过 TLS 证书验证
    pub tls_skip_verify: Option<bool>,
    /// 是否启用 Server Reflection
    pub enable_reflection: Option<bool>,
    /// Proto 文件列表 (path, content)
    pub proto_files: Option<Vec<(String, String)>>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
}

/// gRPC 服务发现响应
#[derive(serde::Serialize)]
pub struct GrpcDiscoverServicesOutput {
    /// 是否成功
    pub success: bool,
    /// 发现的服务列表
    pub services: Vec<GrpcServiceInfo>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 发现 gRPC 服务
///
/// 设计时调用：通过 Server Reflection 或解析 Proto 文件发现可用服务。
/// 不创建持久连接，仅返回服务/方法列表供前端缓存。
#[tauri::command]
pub async fn grpc_discover_services(
    input: GrpcDiscoverServicesInput,
) -> GrpcDiscoverServicesOutput {
    let enable_reflection = input.enable_reflection.unwrap_or(false);
    let connect_timeout_ms = input.connect_timeout_ms.unwrap_or(30000);
    let use_tls = input.use_tls.unwrap_or(false);
    let tls_skip_verify = input.tls_skip_verify.unwrap_or(false);
    let proto_files = input.proto_files.unwrap_or_default();

    log::info!(
        "[gRPC] 发现服务: url={}, reflection={}, proto_files={}",
        input.url,
        enable_reflection,
        proto_files.len()
    );

    let result = if enable_reflection {
        // 通过 Server Reflection 发现
        grpc_connection_manager::discover_services_via_reflection(
            &input.url,
            use_tls,
            tls_skip_verify,
            connect_timeout_ms,
        )
        .await
    } else if !proto_files.is_empty() {
        // 通过解析 Proto 文件发现
        grpc_connection_manager::discover_services_via_proto(&proto_files)
    } else {
        Err("未提供 proto 文件且未启用 Server Reflection".to_string())
    };

    match result {
        Ok(services) => {
            log::info!("[gRPC] 服务发现成功: 发现 {} 个服务", services.len());
            GrpcDiscoverServicesOutput {
                success: true,
                services,
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] 服务发现失败: {}", e);
            GrpcDiscoverServicesOutput {
                success: false,
                services: vec![],
                error: Some(e),
            }
        }
    }
}

// ============================================================================
// MCP 工具发现命令
// ============================================================================

use flow_engine::traits::McpProvider;

/// MCP 工具发现请求参数
#[derive(Debug, serde::Deserialize)]
pub struct McpDiscoverToolsInput {
    /// MCP 服务器地址
    pub url: String,
    /// 额外请求头
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// MCP 协议版本
    pub protocol_version: Option<String>,
}

/// MCP 工具发现响应
#[derive(Debug, serde::Serialize)]
pub struct McpDiscoverToolsOutput {
    /// 是否成功
    pub success: bool,
    /// 发现的工具列表
    pub tools: Vec<McpToolInfoOutput>,
    /// 发现的资源列表
    pub resources: Vec<McpResourceInfoOutput>,
    /// 发现的提示词列表
    pub prompts: Vec<McpPromptInfoOutput>,
    /// 服务器信息
    pub server_info: Option<McpServerInfoOutput>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// MCP 工具信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct McpToolInfoOutput {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: Option<String>,
    /// 工具输入 schema
    pub input_schema: Option<serde_json::Value>,
}

/// MCP 资源信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct McpResourceInfoOutput {
    /// 资源 URI
    pub uri: String,
    /// 资源名称
    pub name: String,
    /// 资源描述
    pub description: Option<String>,
    /// MIME 类型
    pub mime_type: Option<String>,
}

/// MCP 服务器信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct McpServerInfoOutput {
    /// 服务器名称
    pub name: String,
    /// 服务器版本
    pub version: Option<String>,
}

/// MCP 提示词参数信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct McpPromptArgumentOutput {
    /// 参数名称
    pub name: String,
    /// 参数描述
    pub description: Option<String>,
    /// 是否必填
    pub required: Option<bool>,
}

/// MCP 提示词信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct McpPromptInfoOutput {
    /// 提示词名称
    pub name: String,
    /// 提示词描述
    pub description: Option<String>,
    /// 提示词参数
    pub arguments: Vec<McpPromptArgumentOutput>,
}

/// 发现 MCP 工具
///
/// 设计时调用：连接 MCP 服务器，获取可用工具和资源列表。
/// 不创建持久连接，仅返回信息供前端缓存。
#[tauri::command]
pub async fn mcp_discover_tools(
    input: McpDiscoverToolsInput,
) -> McpDiscoverToolsOutput {
    log::info!(
        "[MCP] 发现工具: url={}, protocol_version={}",
        input.url,
        input.protocol_version.as_deref().unwrap_or("default")
    );

    // 创建临时连接管理器
    let manager = flow_engine::flow_engine_mcp::McpConnectionManager::new();
    let headers = input.headers.unwrap_or_default();

    match manager.connect(
        "temp-discover".to_string(),
        input.url,
        headers,
        input.protocol_version.unwrap_or_else(|| "2025-11-25".to_string()),
        std::collections::HashMap::new(),
    ).await {
        Ok(conn_info) => {
            // 关闭临时连接
            let _ = manager.close(&conn_info.connection_id).await;

            log::info!(
                "[MCP] 工具发现成功: 发现 {} 个工具, {} 个资源, {} 个提示词",
                conn_info.tools.len(),
                conn_info.resources.len(),
                conn_info.prompts.len()
            );

            McpDiscoverToolsOutput {
                success: true,
                tools: conn_info.tools.into_iter().map(|t| McpToolInfoOutput {
                    name: t.name,
                    description: t.description,
                    input_schema: t.input_schema,
                }).collect(),
                resources: conn_info.resources.into_iter().map(|r| McpResourceInfoOutput {
                    uri: r.uri,
                    name: r.name,
                    description: r.description,
                    mime_type: r.mime_type,
                }).collect(),
                prompts: conn_info.prompts.into_iter().map(|p| McpPromptInfoOutput {
                    name: p.name,
                    description: p.description,
                    arguments: p.arguments.into_iter().map(|a| McpPromptArgumentOutput {
                        name: a.name,
                        description: a.description,
                        required: a.required,
                    }).collect(),
                }).collect(),
                server_info: Some(McpServerInfoOutput {
                    name: conn_info.server_info.name,
                    version: conn_info.server_info.version,
                }),
                error: None,
            }
        }
        Err(e) => {
            log::error!("[MCP] 工具发现失败: {}", e);
            McpDiscoverToolsOutput {
                success: false,
                tools: vec![],
                resources: vec![],
                prompts: vec![],
                server_info: None,
                error: Some(e),
            }
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

use flow_engine::debug::DebugCommand;
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

/// 保存压力测试报告
#[tauri::command]
pub fn save_stress_test_report(
    report: flow_engine::stress_test::StressTestReport,
    reports_dir: String,
) -> Result<String, String> {
    let path = flow_engine::storage::save_stress_report(&report, std::path::Path::new(&reports_dir))?;
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
pub fn delete_stress_test_report(
    file_path: String,
) -> Result<(), String> {
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
    flow_engine::storage::list_flow_files(std::path::Path::new(&flows_dir))
}

/// 加载流程文件（包含 _meta 元数据）
#[tauri::command]
pub fn load_flow_file(
    uuid: String,
    flows_dir: String,
) -> Result<flow_engine::storage::FlowFile, String> {
    flow_engine::storage::load_flow_file_raw(&uuid, std::path::Path::new(&flows_dir))
}

/// 保存流程文件（可选携带 _meta 元数据）
#[tauri::command]
pub fn save_flow_file(
    flow: Flow,
    flows_dir: String,
    meta: Option<serde_json::Value>,
) -> Result<(), String> {
    flow_engine::storage::save_flow_file_with_meta(&flow, std::path::Path::new(&flows_dir), meta)
}

/// 删除流程文件
#[tauri::command]
pub fn delete_flow_file(
    uuid: String,
    flows_dir: String,
) -> Result<(), String> {
    flow_engine::storage::delete_flow_file(&uuid, std::path::Path::new(&flows_dir))
}

/// 复制流程文件
#[tauri::command]
pub fn copy_flow_file(
    uuid: String,
    flows_dir: String,
) -> Result<Flow, String> {
    flow_engine::storage::copy_flow_file(&uuid, std::path::Path::new(&flows_dir))
}

// ============================================================================
// 文件夹管理命令
// ============================================================================

/// 列出所有文件夹
#[tauri::command]
pub fn list_folders(
    flows_dir: String,
) -> Result<Vec<flow_engine::storage::FolderInfo>, String> {
    flow_engine::storage::list_folders(std::path::Path::new(&flows_dir))
}

/// 创建文件夹
#[tauri::command]
pub fn create_folder(
    flows_dir: String,
    folder_path: String,
) -> Result<flow_engine::storage::FolderInfo, String> {
    flow_engine::storage::create_folder(std::path::Path::new(&flows_dir), &folder_path)
}

/// 删除文件夹
#[tauri::command]
pub fn delete_folder(
    flows_dir: String,
    folder_path: String,
) -> Result<(), String> {
    flow_engine::storage::delete_folder(std::path::Path::new(&flows_dir), &folder_path)
}

/// 重命名文件夹
#[tauri::command]
pub fn rename_folder(
    flows_dir: String,
    folder_path: String,
    new_name: String,
) -> Result<flow_engine::storage::FolderInfo, String> {
    flow_engine::storage::rename_folder(std::path::Path::new(&flows_dir), &folder_path, &new_name)
}

/// 移动流程到指定文件夹
#[tauri::command]
pub fn move_flow_to_folder(
    uuid: String,
    flows_dir: String,
    target_folder_path: String,
) -> Result<(), String> {
    flow_engine::storage::move_flow_to_folder(&uuid, std::path::Path::new(&flows_dir), &target_folder_path)
}

/// 移动文件夹到另一个文件夹（或根目录）
#[tauri::command]
pub fn move_folder(
    flows_dir: String,
    source_folder_path: String,
    target_folder_path: String,
) -> Result<(), String> {
    flow_engine::storage::move_folder(std::path::Path::new(&flows_dir), &source_folder_path, &target_folder_path)
}

/// 获取所有已注册的节点类型元数据
///
/// 从默认节点注册表获取全部 14 种节点类型的元数据信息，
/// 供前端动态渲染节点面板和画布。
#[tauri::command]
pub fn get_registered_node_types() -> Vec<NodeMetadata> {
    let registry = flow_engine::default_registry::default_registry();
    registry.all_metadata()
}

/// 获取指定节点类型的配置 JSON Schema
///
/// 通过 type_id 查找已注册的节点插件，返回其配置 schema。
/// 若 type_id 不存在或插件未提供 schema，返回 None。
#[tauri::command]
pub fn get_node_config_schema(type_id: String) -> Option<serde_json::Value> {
    let registry = flow_engine::default_registry::default_registry();
    registry.get(&type_id).and_then(|plugin| plugin.config_schema())
}

/// 获取所有已注册插件的元数据列表
///
/// 与 get_registered_node_types 相同，语义上更侧重"插件"概念。
/// 返回全部插件的 NodeMetadata 列表。
#[tauri::command]
pub fn list_registered_plugins() -> Vec<NodeMetadata> {
    let registry = flow_engine::default_registry::default_registry();
    registry.all_metadata()
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
    use flow_engine::node::{Node, NodePosition};
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
        assert_eq!(output.flow.nodes[0].node_type, "entry");

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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
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
            node_type: "entry".to_string(),
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
            node_type: "http".to_string(),
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
            node_type: "entry".to_string(),
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
            node_type: "entry".to_string(),
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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
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
            node_type: "http".to_string(),
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
            node_type: "entry".to_string(),
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
