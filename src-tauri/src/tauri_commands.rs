//! Tauri 命令模块
//!
//! 提供 #[`tauri::command`] 包装器，调用 flow-engine 的业务逻辑。

use std::collections::HashSet;
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
    CreateFlowCommandInput, CreateFlowCommandOutput, DeleteNodeCommandInput,
    DeleteNodeCommandOutput, ExecuteFlowCommandInput, ExecuteFlowCommandOutput,
    UpdateNodeCommandInput, UpdateNodeCommandOutput,
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

/// gRPC 取消会话管理器
///
/// 管理当前流程执行中的 gRPC 连接管理器引用，
/// 允许前端通过 `grpc_cancel_stream` 命令取消进行中的流式调用，
/// 并通过 `grpc_channelz_info` 命令查询连接状态。
pub struct GrpcCancelSessionManager {
    grpc_manager: std::sync::Mutex<Option<Arc<dyn GrpcProvider>>>,
    /// 具体类型的 GrpcConnectionManager（用于 Channelz 内省）
    channelz_manager:
        std::sync::Mutex<Option<Arc<flow_engine::grpc_connection_manager::GrpcConnectionManager>>>,
}

impl GrpcCancelSessionManager {
    /// 创建新的 gRPC 取消会话管理器
    pub fn new() -> Self {
        Self {
            grpc_manager: std::sync::Mutex::new(None),
            channelz_manager: std::sync::Mutex::new(None),
        }
    }

    /// 设置当前活跃的 gRPC manager
    pub fn set_manager(&self, manager: Arc<dyn GrpcProvider>) -> Result<(), String> {
        let mut guard = self
            .grpc_manager
            .lock()
            .map_err(|e| format!("gRPC manager lock poisoned: {}", e))?;
        *guard = Some(manager);
        Ok(())
    }

    /// 设置当前活跃的具体类型 GrpcConnectionManager（用于 Channelz）
    pub fn set_channelz_manager(
        &self,
        manager: Arc<flow_engine::grpc_connection_manager::GrpcConnectionManager>,
    ) -> Result<(), String> {
        let mut guard = self
            .channelz_manager
            .lock()
            .map_err(|e| format!("gRPC channelz manager lock poisoned: {}", e))?;
        *guard = Some(manager);
        Ok(())
    }

    /// 清除当前活跃的 gRPC manager（执行结束时清理）
    pub fn clear(&self) -> Result<(), String> {
        {
            let mut guard = self
                .grpc_manager
                .lock()
                .map_err(|e| format!("gRPC manager lock poisoned: {}", e))?;
            *guard = None;
        }
        {
            let mut guard = self
                .channelz_manager
                .lock()
                .map_err(|e| format!("gRPC channelz manager lock poisoned: {}", e))?;
            *guard = None;
        }
        Ok(())
    }

    /// 取消指定调用
    pub fn cancel_stream(&self, connection_id: &str, call_id: &str) -> Result<(), String> {
        let guard = self
            .grpc_manager
            .lock()
            .map_err(|e| format!("gRPC manager lock poisoned: {}", e))?;
        match guard.as_ref() {
            Some(manager) => manager.cancel_stream(connection_id, call_id),
            None => Err("No active execution session".to_string()),
        }
    }
}

impl Default for GrpcCancelSessionManager {
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
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
    on_execution_event: Channel<ExecutionEvent>,
    on_ws_event: Channel<DebugEvent>,
    state: State<'_, ExecutionSessionManager>,
    grpc_state: State<'_, GrpcCancelSessionManager>,
    node_registry: State<'_, NodeRegistryState>,
) -> Result<ExecuteFlowCommandOutput, String> {
    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel_tx);
    }

    // 创建共享的 gRPC manager 并注册到取消会话管理器
    let concrete_grpc_manager =
        Arc::new(flow_engine::grpc_connection_manager::GrpcConnectionManager::new());
    let grpc_manager: Arc<dyn GrpcProvider> = concrete_grpc_manager.clone();
    grpc_state.set_manager(grpc_manager.clone())?;
    // 注册具体类型引用（用于 Channelz 内省，共享同一个连接池）
    grpc_state.set_channelz_manager(concrete_grpc_manager)?;

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

    let mut executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(Arc::clone(&snap.registry));
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
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = None;
    }
    // 清理 gRPC 取消会话
    grpc_state.clear()?;

    // 通知所有活跃插件：本次执行已结束（fire-and-forget）。
    // 插件据此释放 per-execution 资源（如 Mongo 连接池）。
    // 幂等：仅通知已在运行的插件，不拉起空闲插件。
    #[cfg(feature = "plugin-manager")]
    {
        snap.manager
            .notify_flow_ended(&result.execution_id)
            .await;
    }

    Ok(output)
}

/// 取消流程执行
///
/// 向当前正在执行的流程发送取消信号。
/// 幂等操作：第二次调用返回"没有活跃的执行会话"Error。
#[tauri::command]
pub fn cancel_flow(state: State<'_, ExecutionSessionManager>) -> Result<(), String> {
    let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
    match guard.take() {
        Some(tx) => {
            let _ = tx.send(true);
            Ok(())
        }
        None => Err("No active execution session".to_string()),
    }
}

/// 执行单个节点（连接性验证）
///
/// 单独执行一个协议节点，用于前端"测试"功能验证连接性。
/// 与后端 executor 共用 `supports_single_node` 能力检查（内置白名单 +
/// 插件 capabilities.single_node），无独立清单。执行超时时间为 30 秒。
#[tauri::command]
pub async fn execute_single_node(
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

    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 存储 sender 到会话管理器（允许前端通过 cancel_flow 取消）
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel_tx);
    }

    let executor = flow_engine::executor::FlowExecutor::new()
        .with_registry(Arc::clone(&snap.registry))
        .with_cancel_receiver(cancel_rx)
        .with_hook_executor(std::sync::Arc::new(
            flow_engine::hook_runner::DefaultHookExecutor::new(),
        ));

    let result = executor.execute_single_node(&node, variables, None).await;

    // execution complete后清理会话
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = None;
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
            .validate_with_registry_async(
                &flow,
                &snap.registry,
                Some(snap.manager.as_ref()),
            )
            .await
    }
    #[cfg(not(feature = "plugin-manager"))]
    {
        validator.validate_with_registry(&flow, &snap.registry)
    }
}

// ============================================================================
// 报告存储命令
// ============================================================================

/// 保存流程执行报告
#[tauri::command]
pub fn save_flow_report(report: FlowExecutionReport, reports_dir: String) -> Result<(), String> {
    flow_engine::commands::save_flow_report(report, &reports_dir)
}

/// 加载流程执行报告
#[tauri::command]
pub fn load_flow_report(uuid: String, reports_dir: String) -> Result<FlowExecutionReport, String> {
    flow_engine::commands::load_flow_report(&uuid, &reports_dir)
}

/// 列出所有报告元数据
#[tauri::command]
pub fn list_flow_reports(reports_dir: String) -> Result<Vec<ReportMetadata>, String> {
    flow_engine::commands::list_flow_reports(&reports_dir)
}

/// 删除流程执行报告
#[tauri::command]
pub fn delete_flow_report(uuid: String, reports_dir: String) -> Result<(), String> {
    flow_engine::commands::delete_flow_report(&uuid, &reports_dir)
}

// ============================================================================
// gRPC 服务发现命令
// ============================================================================

#[cfg(feature = "protocol-grpc")]
use flow_engine::grpc_connection_manager;
use flow_engine::traits::GrpcProvider;
#[cfg(feature = "protocol-grpc")]
use flow_engine::traits::{GrpcMethodInfo, GrpcServiceInfo};

/// gRPC 服务发现请求参数
#[cfg(feature = "protocol-grpc")]
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
    /// TLS CA 证书文件路径
    pub tls_ca_cert: Option<String>,
    /// TLS 客户端证书文件路径
    pub tls_client_cert: Option<String>,
    /// TLS 客户端私钥文件路径
    pub tls_client_key: Option<String>,
    /// TLS SNI 覆盖
    pub tls_server_name_override: Option<String>,
    /// Reflection 请求附加 metadata（用于认证的反射服务）
    pub reflection_metadata: Option<Vec<(String, String)>>,
}

/// gRPC 服务发现响应
#[cfg(feature = "protocol-grpc")]
#[derive(serde::Serialize)]
pub struct GrpcDiscoverServicesOutput {
    /// 是否Success
    pub success: bool,
    /// 发现的服务列表
    pub services: Vec<GrpcServiceInfo>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

/// 发现 gRPC 服务
///
/// 设计时调用：通过 Server Reflection 或解析 Proto 文件发现可用服务。
/// 不创建持久连接，仅返回服务/方法列表供前端缓存。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_discover_services(
    input: GrpcDiscoverServicesInput,
) -> GrpcDiscoverServicesOutput {
    let enable_reflection = input.enable_reflection.unwrap_or(false);
    let connect_timeout_ms = input.connect_timeout_ms.unwrap_or(30000);
    let use_tls = input.use_tls.unwrap_or(false);
    let tls_skip_verify = input.tls_skip_verify.unwrap_or(false);
    let proto_files = input.proto_files.unwrap_or_default();
    let tls_ca_cert = input.tls_ca_cert.as_deref();
    let tls_client_cert = input.tls_client_cert.as_deref();
    let tls_client_key = input.tls_client_key.as_deref();
    let tls_server_name_override = input.tls_server_name_override.as_deref();
    let reflection_metadata = input.reflection_metadata.unwrap_or_default();

    log::info!(
        "[gRPC] Discover services: url={}, reflection={}, proto_files={}, metadata_count={}",
        input.url,
        enable_reflection,
        proto_files.len(),
        reflection_metadata.len()
    );

    let result = if enable_reflection {
        // 通过 Server Reflection 发现（v1 优先，自动回退到 v1alpha）
        grpc_connection_manager::discover_services_via_reflection(
            &input.url,
            use_tls,
            tls_skip_verify,
            connect_timeout_ms,
            tls_ca_cert,
            tls_client_cert,
            tls_client_key,
            tls_server_name_override,
            reflection_metadata,
        )
        .await
    } else if !proto_files.is_empty() {
        // 通过解析 Proto 文件发现
        grpc_connection_manager::discover_services_via_proto(&proto_files)
    } else {
        Err("No proto files provided and Server Reflection not enabled".to_string())
    };

    match result {
        Ok(services) => {
            log::info!(
                "[gRPC] Service discovery succeeded: found {} services",
                services.len()
            );
            GrpcDiscoverServicesOutput {
                success: true,
                services,
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] Service discovery failed: {}", e);
            GrpcDiscoverServicesOutput {
                success: false,
                services: vec![],
                error: Some(e),
            }
        }
    }
}

// ============================================================================
// gRPC 请求骨架生成命令
// ============================================================================

#[cfg(feature = "protocol-grpc")]
use flow_engine::proto_parser;

// ============================================================================
// gRPC Proto 目录扫描和 Descriptor Set 命令
// ============================================================================

/// Proto 文件信息（IPC 输出）
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct ProtoFileOutput {
    /// 文件名（相对路径）
    pub path: String,
    /// 文件内容
    pub content: String,
}

/// 扫描 Proto 目录响应
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct GrpcScanProtoDirectoryOutput {
    /// 是否Success
    pub success: bool,
    /// 发现的 proto 文件列表
    pub files: Vec<ProtoFileOutput>,
    /// 目录根路径（用作 proto 编译的 include 路径）
    pub import_path: Option<String>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

/// 扫描目录，发现所有 .proto 文件
///
/// 递归遍历指定目录及其子目录，收集所有 `.proto` 文件。
/// 跳过以 `.` 开头的隐藏目录。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_scan_proto_directory(dir_path: String) -> GrpcScanProtoDirectoryOutput {
    log::info!("[gRPC] Scan Proto directory: {}", dir_path);

    match proto_parser::discover_proto_files_in_dir(&dir_path) {
        Ok((files, import_path)) => {
            log::info!("[gRPC] Scan complete: found {} proto files", files.len());
            GrpcScanProtoDirectoryOutput {
                success: true,
                files: files
                    .into_iter()
                    .map(|(path, content)| ProtoFileOutput { path, content })
                    .collect(),
                import_path: Some(import_path),
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] Directory scan failed: {}", e);
            GrpcScanProtoDirectoryOutput {
                success: false,
                files: vec![],
                import_path: None,
                error: Some(format!("{}", e)),
            }
        }
    }
}

/// 读取 Proto 文件列表响应
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct GrpcReadProtoFilesOutput {
    /// 是否Success
    pub success: bool,
    /// 文件列表 (filename, content)
    pub files: Vec<ProtoFileOutput>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

/// 读取多个 .proto 文件的内容
///
/// 接收文件路径列表，读取每个文件的内容并返回。
/// 用于文件选择器选择 .proto 文件后批量读取内容。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_read_proto_files(file_paths: Vec<String>) -> GrpcReadProtoFilesOutput {
    log::info!("[gRPC] Reading {} Proto files", file_paths.len());

    let mut files = Vec::new();
    for file_path in &file_paths {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                // 提取文件名
                let file_name = std::path::Path::new(file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.clone());
                files.push(ProtoFileOutput {
                    path: file_name,
                    content,
                });
            }
            Err(e) => {
                log::error!("[gRPC] Failed to read file: {} - {}", file_path, e);
                return GrpcReadProtoFilesOutput {
                    success: false,
                    files: vec![],
                    error: Some(format!("Failed to read file {}: {}", file_path, e)),
                };
            }
        }
    }

    log::info!("[gRPC] Successfully read {} Proto files", files.len());
    GrpcReadProtoFilesOutput {
        success: true,
        files,
        error: None,
    }
}

/// 解析 Descriptor Set 响应
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct GrpcParseDescriptorSetOutput {
    /// 是否Success
    pub success: bool,
    /// 发现的服务列表
    pub services: Vec<GrpcServiceInfo>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

/// 解析 `FileDescriptorSet` 二进制文件
///
/// 读取 `.pb` / `.fdset` / `.desc` 格式的 `FileDescriptorSet` 文件，
/// 解析并返回其中的服务和方法信息。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_parse_descriptor_set(file_path: String) -> GrpcParseDescriptorSetOutput {
    log::info!("[gRPC] Parse Descriptor Set file: {}", file_path);

    // 读取文件内容
    let data = match std::fs::read(&file_path) {
        Ok(data) => data,
        Err(e) => {
            let err_msg = format!("Failed to read file: {}", e);
            log::error!("[gRPC] {}", err_msg);
            return GrpcParseDescriptorSetOutput {
                success: false,
                services: vec![],
                error: Some(err_msg),
            };
        }
    };

    // 解析 FileDescriptorSet
    let pool = match proto_parser::parse_descriptor_set(&data) {
        Ok(pool) => pool,
        Err(e) => {
            let err_msg = format!("{}", e);
            log::error!("[gRPC] {}", err_msg);
            return GrpcParseDescriptorSetOutput {
                success: false,
                services: vec![],
                error: Some(err_msg),
            };
        }
    };

    // 提取服务信息
    let services = proto_parser::list_services(&pool);
    let message_definitions = Arc::new(proto_parser::get_message_definitions(&pool));
    let grpc_services: Vec<GrpcServiceInfo> = services
        .into_iter()
        .map(|s| GrpcServiceInfo {
            service_name: s.service_name,
            methods: s
                .methods
                .into_iter()
                .map(|m| GrpcMethodInfo {
                    method_name: m.method_name,
                    input_type: m.input_type,
                    output_type: m.output_type,
                    is_server_streaming: m.is_server_streaming,
                    is_client_streaming: m.is_client_streaming,
                })
                .collect(),
            message_definitions: message_definitions.clone(),
        })
        .collect();

    log::info!(
        "[gRPC] Descriptor Set parsed successfully: {} services found",
        grpc_services.len()
    );

    GrpcParseDescriptorSetOutput {
        success: true,
        services: grpc_services,
        error: None,
    }
}

// ============================================================================
// gRPC 健康检查命令
// ============================================================================
// gRPC Channelz 内省命令
// ============================================================================

/// gRPC Channelz 内省响应（EXPERIMENTAL）
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct GrpcChannelzOutput {
    /// 是否Success
    pub success: bool,
    /// Channelz 信息（Success时返回）
    pub info: Option<flow_engine::grpc_connection_manager::ChannelzInfo>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

/// 获取 gRPC 连接的 Channelz 内省信息（EXPERIMENTAL）
///
/// 运行时调用：获取活跃 gRPC 连接的轻量级诊断信息。
/// 包括连接状态、运行时长、活跃调用数等。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_channelz_info(
    connection_id: String,
    grpc_state: State<'_, GrpcCancelSessionManager>,
) -> Result<GrpcChannelzOutput, String> {
    log::info!(
        "[gRPC Channelz] Query connection state: connection_id={}",
        connection_id
    );

    // 从取消会话管理器获取 channelz_manager（克隆 Arc 以避免跨 await 持有锁）
    let channelz_manager = {
        let guard = grpc_state
            .channelz_manager
            .lock()
            .map_err(|e| e.to_string())?;
        guard.clone()
    };

    match channelz_manager {
        Some(m) => {
            // 先检查连接是否存在
            let grpc_manager = {
                let guard = grpc_state.grpc_manager.lock().map_err(|e| e.to_string())?;
                guard.clone()
            };
            if let Some(gm) = grpc_manager.as_ref() {
                let status = gm.get_status(&connection_id);
                if status.is_none() {
                    return Ok(GrpcChannelzOutput {
                        success: false,
                        info: None,
                        error: Some(format!(
                            "Connection '{}' does not exist (flow may not have been executed yet)",
                            connection_id
                        )),
                    });
                }
            }

            match m.get_channelz_info(&connection_id).await {
                Ok(info) => {
                    log::info!(
                        "[gRPC Channelz] Connection state: state={}, uptime={}s, active_calls={}",
                        info.state,
                        info.uptime_secs,
                        info.active_calls
                    );
                    Ok(GrpcChannelzOutput {
                        success: true,
                        info: Some(info),
                        error: None,
                    })
                }
                Err(e) => {
                    log::error!("[gRPC Channelz] Query failed: {}", e);
                    Ok(GrpcChannelzOutput {
                        success: false,
                        info: None,
                        error: Some(e),
                    })
                }
            }
        }
        None => Ok(GrpcChannelzOutput {
            success: false,
            info: None,
            error: Some("No active execution session".to_string()),
        }),
    }
}

// ============================================================================
// gRPC 请求骨架生成命令（原有）
// ============================================================================

/// gRPC 请求骨架生成请求参数
#[cfg(feature = "protocol-grpc")]
#[derive(serde::Deserialize)]
pub struct GrpcGenerateRequestSkeletonInput {
    /// Proto 文件列表 (path, content)
    pub proto_files: Vec<(String, String)>,
    /// 服务全名（如 "package.ServiceName"）
    pub service_name: String,
    /// 方法名
    pub method_name: String,
}

/// gRPC 请求骨架生成响应
#[cfg(feature = "protocol-grpc")]
#[derive(serde::Serialize)]
pub struct GrpcGenerateRequestSkeletonOutput {
    /// 是否Success
    pub success: bool,
    /// 生成的 JSON 骨架字符串
    pub skeleton: Option<String>,
    /// Error信息（Failed时）
    pub error: Option<String>,
}

// ============================================================================
// gRPC 请求 JSON 预验证命令
// ============================================================================

#[cfg(feature = "protocol-grpc")]
use flow_engine::proto_parser::ValidationResult;

/// gRPC 请求 JSON 验证响应
#[cfg(feature = "protocol-grpc")]
#[derive(Debug, serde::Serialize)]
pub struct GrpcValidateRequestOutput {
    /// 是否Success执行验证（不代表验证通过）
    pub success: bool,
    /// 验证结果（Success执行时返回）
    pub result: Option<ValidationResult>,
    /// Error信息（执行Failed时）
    pub error: Option<String>,
}

/// gRPC 请求 JSON 验证请求参数
#[cfg(feature = "protocol-grpc")]
#[derive(serde::Deserialize)]
pub struct GrpcValidateRequestInput {
    /// 待验证的 JSON 字符串
    pub json: String,
    /// 消息全名（如 "test.GetUserRequest"）
    pub message_name: String,
    /// Proto 文件列表 (path, content)，可选（用于从 proto 文件解析 schema）
    pub proto_files: Option<Vec<(String, String)>>,
}

/// 验证请求 JSON 是否符合 proto 消息定义
///
/// 解析 proto 文件获取消息 schema，检查 JSON 字段名和类型是否匹配。
/// 仅做建议性检查，不阻止发送。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_validate_request(input: GrpcValidateRequestInput) -> GrpcValidateRequestOutput {
    log::info!(
        "[gRPC] Validate request JSON: message_name={}, proto_files={}",
        input.message_name,
        input.proto_files.as_ref().map(|f| f.len()).unwrap_or(0)
    );

    let proto_files = match input.proto_files {
        Some(files) if !files.is_empty() => files,
        _ => {
            return GrpcValidateRequestOutput {
                success: false,
                result: None,
                error: Some("No proto files provided, cannot validate".to_string()),
            };
        }
    };

    // 解析 proto 文件
    let files: Vec<proto_parser::ProtoFile> = proto_files
        .iter()
        .map(|(path, content)| proto_parser::ProtoFile {
            path: path.clone(),
            content: content.clone(),
        })
        .collect();

    let pool = match proto_parser::parse_proto_files(&files) {
        Ok(pool) => pool,
        Err(e) => {
            let err_msg = format!("Failed to parse proto file: {}", e);
            log::error!("[gRPC] {}", err_msg);
            return GrpcValidateRequestOutput {
                success: false,
                result: None,
                error: Some(err_msg),
            };
        }
    };

    // 执行验证
    match proto_parser::validate_request_json(&pool, &input.message_name, &input.json) {
        Ok(result) => {
            log::info!(
                "[gRPC] Validation complete: valid={}, errors={}",
                result.valid,
                result.errors.len()
            );
            GrpcValidateRequestOutput {
                success: true,
                result: Some(result),
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] Validation failed: {}", e);
            GrpcValidateRequestOutput {
                success: false,
                result: None,
                error: Some(e),
            }
        }
    }
}

/// 生成 gRPC 请求消息的 JSON 骨架
///
/// 根据指定服务和方法的 Input Type，解析 proto 文件并生成占位 JSON。
/// 用于帮助用户快速构造请求消息。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_generate_request_skeleton(
    input: GrpcGenerateRequestSkeletonInput,
) -> GrpcGenerateRequestSkeletonOutput {
    log::info!(
        "[gRPC] Generate request skeleton: service={}, method={}, proto_files={}",
        input.service_name,
        input.method_name,
        input.proto_files.len()
    );

    // 将 (path, content) 元组转换为 ProtoFile
    let files: Vec<proto_parser::ProtoFile> = input
        .proto_files
        .iter()
        .map(|(path, content)| proto_parser::ProtoFile {
            path: path.clone(),
            content: content.clone(),
        })
        .collect();

    // 解析 proto 文件
    let pool = match proto_parser::parse_proto_files(&files) {
        Ok(pool) => pool,
        Err(e) => {
            let err_msg = format!("Failed to parse proto file: {}", e);
            log::error!("[gRPC] {}", err_msg);
            return GrpcGenerateRequestSkeletonOutput {
                success: false,
                skeleton: None,
                error: Some(err_msg),
            };
        }
    };

    // 查找指定服务和方法
    let service = match pool.get_service_by_name(&input.service_name) {
        Some(svc) => svc,
        None => {
            let err_msg = format!("Service not found: {}", input.service_name);
            log::error!("[gRPC] {}", err_msg);
            return GrpcGenerateRequestSkeletonOutput {
                success: false,
                skeleton: None,
                error: Some(err_msg),
            };
        }
    };

    let method = match service.methods().find(|m| m.name() == input.method_name) {
        Some(m) => m,
        None => {
            let err_msg = format!(
                "Method {} not found in service {}",
                input.method_name, input.service_name
            );
            log::error!("[gRPC] {}", err_msg);
            return GrpcGenerateRequestSkeletonOutput {
                success: false,
                skeleton: None,
                error: Some(err_msg),
            };
        }
    };

    // 获取输入消息描述符并生成骨架
    let input_msg = method.input();
    let skeleton = proto_parser::generate_json_skeleton(&input_msg);
    let skeleton_str = serde_json::to_string_pretty(&skeleton).unwrap_or_else(|_| "{}".to_string());

    log::info!(
        "[gRPC] Skeleton generated successfully: {}",
        skeleton_str.chars().take(100).collect::<String>()
    );

    GrpcGenerateRequestSkeletonOutput {
        success: true,
        skeleton: Some(skeleton_str),
        error: None,
    }
}

// ============================================================================
// gRPC 请求骨架生成命令（Reflection 模式）
// ============================================================================

/// gRPC Reflection 模式骨架生成请求参数
#[cfg(feature = "protocol-grpc")]
#[derive(serde::Deserialize)]
pub struct GrpcGenerateRequestSkeletonFromReflectionInput {
    /// gRPC 服务器地址
    pub url: String,
    /// 是否使用 TLS
    pub use_tls: Option<bool>,
    /// 是否跳过 TLS 证书验证
    pub tls_skip_verify: Option<bool>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
    /// TLS CA 证书文件路径
    pub tls_ca_cert: Option<String>,
    /// TLS 客户端证书文件路径
    pub tls_client_cert: Option<String>,
    /// TLS 客户端私钥文件路径
    pub tls_client_key: Option<String>,
    /// TLS SNI 覆盖
    pub tls_server_name_override: Option<String>,
    /// Reflection 请求附加 metadata
    pub reflection_metadata: Option<Vec<(String, String)>>,
    /// 服务全名（如 "package.ServiceName"）
    pub service_name: String,
    /// 方法名
    pub method_name: String,
}

/// 通过 Server Reflection 生成 gRPC 请求消息的 JSON 骨架
///
/// 创建临时连接，通过 Server Reflection 获取类型信息，
/// 生成指定服务和方法的输入消息骨架。
/// 适用于未提供 proto 文件的 Reflection 场景。
#[cfg(feature = "protocol-grpc")]
#[tauri::command]
pub async fn grpc_generate_request_skeleton_from_reflection(
    input: GrpcGenerateRequestSkeletonFromReflectionInput,
) -> GrpcGenerateRequestSkeletonOutput {
    let use_tls = input.use_tls.unwrap_or(false);
    let tls_skip_verify = input.tls_skip_verify.unwrap_or(false);
    let connect_timeout_ms = input.connect_timeout_ms.unwrap_or(30000);
    let reflection_metadata = input.reflection_metadata.unwrap_or_default();

    log::info!(
        "[gRPC] Generate request skeleton via Reflection: url={}, service={}, method={}",
        input.url,
        input.service_name,
        input.method_name
    );

    let result = flow_engine::reflection::generate_skeleton_via_reflection(
        &input.url,
        use_tls,
        tls_skip_verify,
        connect_timeout_ms,
        input.tls_ca_cert.as_deref(),
        input.tls_client_cert.as_deref(),
        input.tls_client_key.as_deref(),
        input.tls_server_name_override.as_deref(),
        reflection_metadata,
        &input.service_name,
        &input.method_name,
    )
    .await;

    match result {
        Ok(skeleton) => {
            log::info!(
                "[gRPC] Reflection skeleton generated successfully: {}",
                skeleton.chars().take(100).collect::<String>()
            );
            GrpcGenerateRequestSkeletonOutput {
                success: true,
                skeleton: Some(skeleton),
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] Reflection skeleton generation failed: {}", e);
            GrpcGenerateRequestSkeletonOutput {
                success: false,
                skeleton: None,
                error: Some(e),
            }
        }
    }
}

#[cfg(all(test, feature = "protocol-grpc"))]
mod grpc_skeleton_tests {
    use super::*;

    /// 测试用的简单 proto 文件
    const TEST_PROTO: &str = r#"
syntax = "proto3";

package test;

service TestService {
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
}

message GetUserRequest {
  string user_id = 1;
  int32 page = 2;
}

message GetUserResponse {
  string user_id = 1;
  string name = 2;
}
"#;

    /// 测试Success生成请求骨架
    #[tokio::test]
    async fn test_grpc_generate_skeleton_success() {
        let input = GrpcGenerateRequestSkeletonInput {
            proto_files: vec![("test.proto".to_string(), TEST_PROTO.to_string())],
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
        };

        let output = grpc_generate_request_skeleton(input).await;

        assert!(output.success, "should successfully generate skeleton");
        assert!(output.skeleton.is_some(), "should return skeleton content");
        assert!(output.error.is_none(), "should have no error");

        let skeleton = output.skeleton.expect("skeleton should exist");
        assert!(
            skeleton.contains("user_id"),
            "skeleton should contain user_id field"
        );
        assert!(
            skeleton.contains("page"),
            "skeleton should contain page field"
        );
    }

    /// 测试无效服务名
    #[tokio::test]
    async fn test_grpc_generate_skeleton_invalid_service() {
        let input = GrpcGenerateRequestSkeletonInput {
            proto_files: vec![("test.proto".to_string(), TEST_PROTO.to_string())],
            service_name: "test.NonExistent".to_string(),
            method_name: "GetUser".to_string(),
        };

        let output = grpc_generate_request_skeleton(input).await;

        assert!(!output.success, "should fail");
        assert!(output.error.is_some(), "should return error info");
        assert!(output.skeleton.is_none(), "should not return skeleton");
    }

    /// 测试无效方法名
    #[tokio::test]
    async fn test_grpc_generate_skeleton_invalid_method() {
        let input = GrpcGenerateRequestSkeletonInput {
            proto_files: vec![("test.proto".to_string(), TEST_PROTO.to_string())],
            service_name: "test.TestService".to_string(),
            method_name: "NonExistent".to_string(),
        };

        let output = grpc_generate_request_skeleton(input).await;

        assert!(!output.success, "should fail");
        assert!(output.error.is_some(), "should return error info");
    }

    /// 测试无效 proto 文件
    #[tokio::test]
    async fn test_grpc_generate_skeleton_invalid_proto() {
        let input = GrpcGenerateRequestSkeletonInput {
            proto_files: vec![("bad.proto".to_string(), "invalid proto !!!".to_string())],
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
        };

        let output = grpc_generate_request_skeleton(input).await;

        assert!(!output.success, "should fail");
        assert!(output.error.is_some(), "should return error info");
    }
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

/// GraphQL 内省响应
#[cfg(feature = "protocol-graphql")]
#[derive(Debug, serde::Serialize)]
pub struct GraphqlIntrospectOutput {
    /// 是否Success
    pub success: bool,
    /// Schema JSON 字符串
    pub schema: Option<serde_json::Value>,
    /// 生成的查询模板
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_templates: Option<Vec<flow_engine::traits::GraphqlQueryTemplate>>,
    /// Error信息
    pub error: Option<String>,
}

/// 从 Schema 类型定义中提取字段的嵌套子字段（递归，最大深度 2）
#[cfg(feature = "protocol-graphql")]
fn generate_nested_fields(
    type_name: &str,
    types_map: &std::collections::HashMap<&str, &serde_json::Value>,
    depth: usize,
    visited: &mut std::collections::HashSet<String>,
) -> String {
    if depth > 2 || visited.contains(type_name) {
        return String::new();
    }
    visited.insert(type_name.to_string());

    let type_def = match types_map.get(type_name) {
        Some(t) => t,
        None => return String::new(),
    };

    let fields = match type_def.get("fields").and_then(|f| f.as_array()) {
        Some(f) => f,
        None => return String::new(), // SCALAR/ENUM → 无子字段
    };

    let mut sub_fields = Vec::new();
    for field in fields {
        let name = match field.get("name").and_then(|n| n.as_str()) {
            Some(n) if !n.starts_with("__") => n,
            _ => continue,
        };

        // 提取字段的返回值类型名（剥掉 NON_NULL / LIST 包装）
        let return_type = resolve_type_name(field.get("type"));
        if let Some(rt) = return_type {
            if is_leaf_type(&rt) {
                sub_fields.push(name.to_string());
            } else {
                let nested = generate_nested_fields(&rt, types_map, depth + 1, visited);
                if nested.is_empty() {
                    sub_fields.push(name.to_string());
                } else {
                    sub_fields.push(format!("{} {{\n{}  }}", name, nested));
                }
            }
        } else {
            sub_fields.push(name.to_string());
        }
    }

    visited.remove(type_name);
    sub_fields.join("\n  ")
}

/// 判断类型是否为叶子类型（SCALAR / ENUM）
#[cfg(feature = "protocol-graphql")]
fn is_leaf_type(name: &str) -> bool {
    matches!(name, "String" | "Int" | "Float" | "Boolean" | "ID")
}

/// 从 type 引用结构中提取实际类型名（剥掉 `NON_NULL` / LIST 包装）
#[cfg(feature = "protocol-graphql")]
fn resolve_type_name(type_ref: Option<&serde_json::Value>) -> Option<String> {
    let mut current = type_ref?;
    loop {
        match current.get("kind").and_then(|k| k.as_str()) {
            Some("NON_NULL") | Some("LIST") => {
                current = current.get("ofType")?;
            }
            Some(_) => {
                return current
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
            }
            None => return None,
        }
    }
}

/// 从 Schema 数据生成查询模板（含展开的子字段）
#[cfg(feature = "protocol-graphql")]
fn generate_graphql_templates(
    schema: &serde_json::Value,
) -> Option<Vec<flow_engine::traits::GraphqlQueryTemplate>> {
    let schema_obj = schema.get("__schema")?;
    let types = schema_obj.get("types").and_then(|t| t.as_array())?;

    // 构建类型索引: type_name → type_def
    let types_map: std::collections::HashMap<&str, &serde_json::Value> = types
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|name| (name, t)))
        .collect();

    let mut templates = Vec::new();

    // 生成 Query 模板
    if let Some(qt_name) = schema_obj
        .get("queryType")
        .and_then(|qt| qt.get("name"))
        .and_then(|n| n.as_str())
    {
        if let Some(query_type_def) = types_map.get(qt_name) {
            if let Some(query_fields) = query_type_def.get("fields").and_then(|f| f.as_array()) {
                for field in query_fields {
                    let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.starts_with("__") {
                        continue;
                    }

                    let field_return_type = resolve_type_name(field.get("type"));
                    let field_nested = field_return_type
                        .as_ref()
                        .map(|rt| {
                            let mut v = std::collections::HashSet::new();
                            generate_nested_fields(rt, &types_map, 0, &mut v)
                        })
                        .unwrap_or_default();

                    let template = if field_nested.is_empty() {
                        format!("query {} {{\n  {}\n}}", pascal_case(name), name)
                    } else {
                        format!(
                            "query {} {{\n  {} {{\n{}  }}\n}}",
                            pascal_case(name),
                            name,
                            indent(&field_nested, 4)
                        )
                    };

                    templates.push(flow_engine::traits::GraphqlQueryTemplate {
                        type_name: qt_name.to_string(),
                        operation: "query".to_string(),
                        template,
                    });
                }
            }
        }
    }

    // 生成 Mutation 模板
    if let Some(mutation_type) = schema_obj.get("mutationType").and_then(|mt| mt.get("name")) {
        let type_name = mutation_type.as_str().unwrap_or("Mutation");
        if let Some(mutation_type_def) = types_map.get(type_name) {
            if let Some(mutation_fields) =
                mutation_type_def.get("fields").and_then(|f| f.as_array())
            {
                for field in mutation_fields {
                    let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");

                    let field_return_type = resolve_type_name(field.get("type"));
                    let field_nested = field_return_type
                        .as_ref()
                        .map(|rt| {
                            let mut v = std::collections::HashSet::new();
                            generate_nested_fields(rt, &types_map, 0, &mut v)
                        })
                        .unwrap_or_default();

                    let template = if field_nested.is_empty() {
                        format!("mutation {} {{\n  {}\n}}", pascal_case(name), name)
                    } else {
                        format!(
                            "mutation {} {{\n  {} {{\n{}  }}\n}}",
                            pascal_case(name),
                            name,
                            indent(&field_nested, 4)
                        )
                    };

                    templates.push(flow_engine::traits::GraphqlQueryTemplate {
                        type_name: type_name.to_string(),
                        operation: "mutation".to_string(),
                        template,
                    });
                }
            }
        }
    }

    if templates.is_empty() {
        None
    } else {
        Some(templates)
    }
}

/// 首字母大写
#[cfg(feature = "protocol-graphql")]
fn pascal_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

/// 缩进文本
#[cfg(feature = "protocol-graphql")]
fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// GraphQL Schema 内省
///
/// 设计时调用：向 GraphQL 端点发送内省查询，返回完整的 Schema 信息。
#[cfg(feature = "protocol-graphql")]
#[tauri::command]
pub async fn graphql_introspect_schema(input: GraphqlIntrospectInput) -> GraphqlIntrospectOutput {
    log::info!("[GraphQL] Introspect request: endpoint={}", input.endpoint);

    // URL 格式验证
    if input.endpoint.is_empty() {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("Endpoint URL must not be empty".to_string()),
        };
    }
    if !input.endpoint.starts_with("http://") && !input.endpoint.starts_with("https://") {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("Endpoint URL must start with http:// or https://".to_string()),
        };
    }

    // GraphQL 内省查询
    let introspection_query = serde_json::json!({
        "query": r#"query IntrospectionQuery {
          __schema {
            queryType { name }
            mutationType { name }
            subscriptionType { name }
            types {
              kind name description
              fields(includeDeprecated: true) {
                name description
                type { ...TypeRef }
                args {
                  name description
                  type { ...TypeRef }
                  defaultValue
                }
              }
              inputFields {
                name description
                type { ...TypeRef }
                defaultValue
              }
              enumValues(includeDeprecated: true) { name description }
            }
          }
        }
        fragment TypeRef on __Type {
          kind name
          ofType { kind name ofType { kind name ofType { kind name ofType { kind name } } } }
        }"#
    });

    // 构建 HTTP 客户端（不自动跟随重定向，手动处理 301→POST）
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e));

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return GraphqlIntrospectOutput {
                success: false,
                schema: None,
                query_templates: None,
                error: Some(e),
            }
        }
    };

    // 手动跟随重定向（保持 POST 方法）
    let max_redirects = 5u32;
    let mut current_url = input.endpoint.clone();
    let mut result = None;

    for _ in 0..=max_redirects {
        let mut req = client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&introspection_query);

        if let Some(ref headers) = input.headers {
            for (key, value) in headers {
                req = req.header(key, value);
            }
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_redirection() {
                    // 跟随重定向
                    if let Some(location) =
                        resp.headers().get("location").and_then(|v| v.to_str().ok())
                    {
                        let next_url = if location.starts_with("http://")
                            || location.starts_with("https://")
                        {
                            location.to_string()
                        } else if location.starts_with('/') {
                            // 以 / 开头的相对路径 → 相对 origin 解析
                            let origin = current_url
                                .split('/')
                                .take(3) // ["https:", "", "host"]
                                .collect::<Vec<_>>()
                                .join("/");
                            format!("{}{}", origin, location)
                        } else {
                            // 相对当前路径
                            let base = current_url.trim_end_matches('/');
                            format!("{}/{}", base, location.trim_start_matches('/'))
                        };
                        current_url = next_url;
                        continue;
                    }
                    result = Some(GraphqlIntrospectOutput {
                        success: false,
                        schema: None,
                        query_templates: None,
                        error: Some(format!("Redirect without Location header: {}", status)),
                    });
                    break;
                }

                // 非重定向 → 解析响应
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        if status.is_success() {
                            if let Some(errors) = body.get("errors") {
                                result = Some(GraphqlIntrospectOutput {
                                    success: false,
                                    schema: None,
                                    query_templates: None,
                                    error: Some(format!("GraphQL error: {}", errors)),
                                });
                            } else {
                                let schema = body.get("data").cloned().unwrap_or(body);
                                let query_templates = generate_graphql_templates(&schema);
                                result = Some(GraphqlIntrospectOutput {
                                    success: true,
                                    schema: Some(schema),
                                    query_templates,
                                    error: None,
                                });
                            }
                        } else {
                            result = Some(GraphqlIntrospectOutput {
                                success: false,
                                schema: None,
                                query_templates: None,
                                error: Some(format!("HTTP error: {}", status)),
                            });
                        }
                    }
                    Err(e) => {
                        result = Some(GraphqlIntrospectOutput {
                            success: false,
                            schema: None,
                            query_templates: None,
                            error: Some(format!("Failed to parse response: {}", e)),
                        });
                    }
                }
                break;
            }
            Err(e) => {
                result = Some(GraphqlIntrospectOutput {
                    success: false,
                    schema: None,
                    query_templates: None,
                    error: Some(format!("Request failed: {}", e)),
                });
                break;
            }
        }
    }

    result.unwrap_or_else(|| GraphqlIntrospectOutput {
        success: false,
        schema: None,
        query_templates: None,
        error: Some("Too many redirects".to_string()),
    })
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
/// 使用 `Arc<Mutex<>>` 保护，因为锁持有时间极短，且需要在 spawn 中共享。
pub struct DatasetExecutionSessionManager {
    /// 取消信号发送端（watch channel）
    cancel_sender: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl DatasetExecutionSessionManager {
    /// 创建新的数据驱动执行会话管理器
    pub fn new() -> Self {
        Self {
            cancel_sender: std::sync::Arc::new(std::sync::Mutex::new(None)),
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
    _app: tauri::AppHandle,
) -> Result<(), String> {
    // 检查是否已有调试会话
    let mut manager = state.lock().await;
    if manager.command_sender.is_some() {
        return Err("A debug session is already running".to_string());
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
pub fn delete_flow_file(uuid: String, flows_dir: String) -> Result<(), String> {
    flow_engine::storage::delete_flow_file(&uuid, std::path::Path::new(&flows_dir))
}

/// 复制流程文件
#[tauri::command]
pub fn copy_flow_file(uuid: String, flows_dir: String) -> Result<Flow, String> {
    flow_engine::storage::copy_flow_file(&uuid, std::path::Path::new(&flows_dir))
}

// ============================================================================
// 文件夹管理命令
// ============================================================================

/// 列出所有文件夹
#[tauri::command]
pub fn list_folders(flows_dir: String) -> Result<Vec<flow_engine::storage::FolderInfo>, String> {
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
pub fn delete_folder(flows_dir: String, folder_path: String) -> Result<(), String> {
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
    flow_engine::storage::move_flow_to_folder(
        &uuid,
        std::path::Path::new(&flows_dir),
        &target_folder_path,
    )
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

/// 获取所有已注册插件的元数据列表
///
/// 与 `get_registered_node_types` 相同，语义上更侧重"插件"概念。
/// 返回全部插件（含 sidecar 插件节点）的 `NodeMetadata` 列表。
#[tauri::command]
pub fn list_registered_plugins(node_registry: State<'_, NodeRegistryState>) -> Vec<NodeMetadata> {
    node_registry.snapshot().registry.all_metadata()
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
    let configured = app
        .store("store/settings.bin")
        .ok()
        .and_then(|store| store.get("pluginDir").and_then(|v| v.as_str().map(String::from)));
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
    // 读取宿主当前 UI 语言，注入新插件进程（与启动时行为一致）
    let host_locale = app
        .store("store/settings.bin")
        .ok()
        .and_then(|store| store.get("locale").and_then(|v| v.as_str().map(String::from)));

    // 新目录 → 新管理器 → describe → 新注册表
    let mut manager = flow_engine::PluginManager::new(PathBuf::from(&dir))
        .with_host_locale(host_locale);
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
        .any(|plugin| {
            !registered.contains(&plugin.name) && !quarantined.contains(&plugin.name)
        });
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
    let host_version = std::env::var("MPE_HOST_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let loaded: HashSet<&str> = snap
        .manager
        .registrations()
        .iter()
        .map(|registration| registration.name.as_str())
        .collect();
    flow_engine::PluginScan::scan_with_errors(&snap.plugins_dir, &host_version)
        .into_iter()
        .map(|entry| match entry {
            flow_engine::ScanEntry::Discovered(plugin) => InstalledPlugin {
                name: plugin.name.clone(),
                version: plugin.version.clone(),
                description: plugin.manifest.description.clone(),
                path: plugin.dir.display().to_string(),
                status: if loaded.contains(plugin.name.as_str()) {
                    "loaded".to_string()
                } else {
                    "error".to_string()
                },
                error: None,
            },
            flow_engine::ScanEntry::Skipped { dir, reason } => InstalledPlugin {
                name: dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                version: String::new(),
                description: None,
                path: dir.display().to_string(),
                status: "error".to_string(),
                error: Some(format!("{reason:?}")),
            },
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
    let install_dir = crate::resolve_plugin_dir(&app).0;
    let client = flow_engine::flow_engine_plugin_marketplace::MarketplaceClient::new(url);
    let plugins = client
        .list_plugins()
        .await
        .map_err(|e| e.to_string())?;
    let plugin = plugins
        .iter()
        .find(|plugin| plugin.name == name && version.as_deref().is_none_or(|v| plugin.version == v))
        .ok_or_else(|| {
            format!(
                "plugin {name}{} not found in marketplace",
                version.map(|v| format!("@{v}")).unwrap_or_default()
            )
        })?;
    flow_engine::flow_engine_plugin_marketplace::install_plugin(&client, plugin, &install_dir, force)
        .await
        .map_err(|e| e.to_string())
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
    // 获取应用数据目录（用于保存Failed行详情）
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel_tx);
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
    let state_cancel = state.cancel_sender.clone();
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
            *guard = None;
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
    let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
    match guard.take() {
        Some(tx) => {
            let _ = tx.send(true);
            Ok(())
        }
        None => Err("No active data-driven execution session".to_string()),
    }
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
    use serde_json::json;
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
    fn test_update_node_not_implemented() {
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

        let input = UpdateNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
            data: json!({ "updated": true }),
        };

        // 更新节点
        let output = update_node(input, flow);

        // 验证更新Failed（持久化未实现）
        assert!(!output.success, "update_node should return failure");
        assert!(output.error.is_some(), "should have error info");
        let error = output.error.unwrap();
        assert!(
            error.find("persistence not yet implemented").is_some(),
            "error should contain 'persistence not yet implemented', got: {}",
            error
        );
        assert!(
            error
                .find("update_node needs to load flow, update node, then save flow")
                .is_some(),
            "error should explain the reason"
        );
    }

    #[test]
    fn test_delete_start_node() {
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

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
        };

        // 尝试删除开始节点
        let output = delete_node(input, flow);

        // 验证删除Failed
        assert!(!output.success, "deleting start node should fail");
        assert!(output.error.is_some(), "should have error info");
        let error = output.error.unwrap();
        assert!(
            error.find("cannot delete start node").is_some(),
            "error should contain 'cannot delete start node', got: {}",
            error
        );
    }

    #[test]
    fn test_delete_node_not_implemented() {
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

        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node, http_node],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // 尝试删除非开始节点
        let output = delete_node(input, flow);

        // 验证删除Failed（持久化未实现）
        assert!(!output.success, "delete_node should return failure");
        assert!(output.error.is_some(), "should have error info");
        let error = output.error.unwrap();
        assert!(
            error.find("persistence not yet implemented").is_some(),
            "error should contain 'persistence not yet implemented', got: {}",
            error
        );
        assert!(
            error
                .find("delete_node needs to load flow, delete node and connections, then save flow")
                .is_some(),
            "error should explain the reason"
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
    fn test_delete_node_uses_node_uuid() {
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

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // verify delete_node uses node.uuid field to find node
        let output = delete_node(input, flow);

        // verify node was found (by uuid)
        assert!(
            !output.error.unwrap().contains("not found"),
            "should find node by uuid"
        );
    }

    #[test]
    fn test_delete_node_nonexistent_uuid() {
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

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: "non-existent-uuid".to_string(),
        };

        let output = delete_node(input, flow);

        // 验证找不到节点（通过 uuid）
        assert!(!output.success, "deleting non-existent node should fail");
        assert!(
            output.error.unwrap().contains("not found"),
            "error should contain 'not found'"
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
