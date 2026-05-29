//! Tauri 命令模块
//!
//! 提供 #[`tauri::command`] 包装器，调用 flow-engine 的业务逻辑。

use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::Emitter;
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

/// gRPC 取消会话管理器
///
/// 管理当前流程执行中的 gRPC 连接管理器引用，
/// 允许前端通过 `grpc_cancel_stream` 命令取消进行中的流式调用，
/// 并通过 `grpc_channelz_info` 命令查询连接状态。
pub struct GrpcCancelSessionManager {
    grpc_manager: std::sync::Mutex<Option<Arc<dyn GrpcProvider>>>,
    /// 具体类型的 GrpcConnectionManager（用于 Channelz 内省）
    channelz_manager: std::sync::Mutex<Option<Arc<flow_engine::grpc_connection_manager::GrpcConnectionManager>>>,
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
    pub fn set_manager(&self, manager: Arc<dyn GrpcProvider>) {
        let mut guard = self.grpc_manager.lock().expect("锁不应失败");
        *guard = Some(manager);
    }

    /// 设置当前活跃的具体类型 GrpcConnectionManager（用于 Channelz）
    pub fn set_channelz_manager(&self, manager: Arc<flow_engine::grpc_connection_manager::GrpcConnectionManager>) {
        let mut guard = self.channelz_manager.lock().expect("锁不应失败");
        *guard = Some(manager);
    }

    /// 清除当前活跃的 gRPC manager（执行结束时清理）
    pub fn clear(&self) {
        {
            let mut guard = self.grpc_manager.lock().expect("锁不应失败");
            *guard = None;
        }
        {
            let mut guard = self.channelz_manager.lock().expect("锁不应失败");
            *guard = None;
        }
    }

    /// 取消指定调用
    pub fn cancel_stream(&self, connection_id: &str, call_id: &str) -> Result<(), String> {
        let guard = self.grpc_manager.lock().map_err(|e| e.to_string())?;
        match guard.as_ref() {
            Some(manager) => manager.cancel_stream(connection_id, call_id),
            None => Err("没有活跃的执行会话".to_string()),
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
/// - 执行完成后自动清理 sender
#[tauri::command]
pub async fn execute_flow(
    input: ExecuteFlowCommandInput,
    flow: Flow,
    on_event: Channel<LogEvent>,
    on_execution_event: Channel<ExecutionEvent>,
    on_ws_event: Channel<DebugEvent>,
    state: State<'_, ExecutionSessionManager>,
    grpc_state: State<'_, GrpcCancelSessionManager>,
) -> Result<ExecuteFlowCommandOutput, String> {
    // 创建取消信号通道
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 存储 sender 到会话管理器
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel_tx);
    }

    // 创建共享的 gRPC manager 并注册到取消会话管理器
    let concrete_grpc_manager = Arc::new(
        flow_engine::grpc_connection_manager::GrpcConnectionManager::new()
    );
    let grpc_manager: Arc<dyn GrpcProvider> = concrete_grpc_manager.clone();
    grpc_state.set_manager(grpc_manager.clone());
    // 注册具体类型引用（用于 Channelz 内省，共享同一个连接池）
    grpc_state.set_channelz_manager(concrete_grpc_manager);

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

    // 执行流程（传入取消信号 receiver + 共享的 grpc_manager）
    let flow_uuid = flow.uuid.clone();
    let flow_name = flow.name.clone();
    let input_variables = input.variables.clone();

    let mut executor = flow_engine::executor::FlowExecutor::new()
        .with_grpc_manager(grpc_manager);
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
        flow_engine::hook_runner::DefaultHookExecutor::new()
    ));

    let result = executor
        .execute_flow(&flow, input_variables, None, Some(log_sender))
        .await;

    // 发送流程结束日志
    let _ = (flow_uuid, flow_name);

    // 先计算 execution_time，避免 move 后借用
    let execution_time = result.flow_report.as_ref().map(|r| r.duration_ms).unwrap_or(0);
    let output = flow_engine::command::ExecuteFlowCommandOutput {
        success: result.success,
        data: result.data,
        flow_report: result.flow_report,
        error: result.error,
        execution_time,
    };

    // 执行完成后清理会话（take sender 使其 drop，后续 cancel_flow 调用将返回"没有活跃的执行会话"）
    {
        let mut guard = state.cancel_sender.lock().map_err(|e| e.to_string())?;
        *guard = None;
    }
    // 清理 gRPC 取消会话
    grpc_state.clear();

    Ok(output)
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

/// 验证流程
///
/// 检查流程结构是否正确，返回验证结果（valid/errors/warnings）。
#[tauri::command]
pub fn validate_flow(flow: Flow) -> serde_json::Value {
    let validator = flow_engine::validator::FlowValidator::new();
    let result = validator.validate(&flow);
    serde_json::json!({
        "valid": result.valid,
        "errors": result.errors,
        "warnings": result.warnings,
    })
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

use flow_engine::db::{
    DbConnection, DatabasePool, DbDriver, PostgresConnection, MySqlConnection, SqliteConnection,
    RedisConnection, TestDbConnectionResult,
};

/// 测试数据库连接
///
/// 接收前端传来的数据库连接配置，根据驱动类型创建真实连接池并测试连接。
#[tauri::command]
pub async fn test_db_connection(connection: DbConnection) -> TestDbConnectionResult {
    log::info!(
        "[数据库] 测试连接: id={}, name={}, driver={}",
        connection.id,
        connection.name,
        connection.driver,
    );

    // 根据驱动类型创建真实连接池
    let pool: Arc<dyn DatabasePool> = match connection.driver {
        DbDriver::Postgres => {
            match PostgresConnection::from_config(&connection).await {
                Ok(p) => {
                    log::info!("[数据库] PostgreSQL 连接池创建成功: id={}", connection.id);
                    Arc::new(p) as Arc<dyn DatabasePool>
                }
                Err(e) => {
                    log::error!("[数据库] PostgreSQL 连接池创建失败: id={}, error={}", connection.id, e);
                    return TestDbConnectionResult::failure(format!("PostgreSQL 连接失败: {}", e));
                }
            }
        }
        DbDriver::Mysql => {
            match MySqlConnection::from_config(&connection).await {
                Ok(p) => {
                    log::info!("[数据库] MySQL 连接池创建成功: id={}", connection.id);
                    Arc::new(p) as Arc<dyn DatabasePool>
                }
                Err(e) => {
                    log::error!("[数据库] MySQL 连接池创建失败: id={}, error={}", connection.id, e);
                    return TestDbConnectionResult::failure(format!("MySQL 连接失败: {}", e));
                }
            }
        }
        DbDriver::Sqlite => {
            match SqliteConnection::from_config(&connection).await {
                Ok(p) => {
                    log::info!("[数据库] SQLite 连接池创建成功: id={}", connection.id);
                    Arc::new(p) as Arc<dyn DatabasePool>
                }
                Err(e) => {
                    log::error!("[数据库] SQLite 连接池创建失败: id={}, error={}", connection.id, e);
                    return TestDbConnectionResult::failure(format!("SQLite 连接失败: {}", e));
                }
            }
        }
        DbDriver::Redis => {
            match RedisConnection::from_config(&connection).await {
                Ok(p) => {
                    log::info!("[数据库] Redis 连接池创建成功: id={}", connection.id);
                    Arc::new(p) as Arc<dyn DatabasePool>
                }
                Err(e) => {
                    log::error!("[数据库] Redis 连接池创建失败: id={}, error={}", connection.id, e);
                    return TestDbConnectionResult::failure(format!("Redis 连接失败: {}", e));
                }
            }
        }
    };

    // 执行连接测试
    match pool.test_connection().await {
        Ok(()) => {
            log::info!("[数据库] 连接测试成功: id={}", connection.id);
            pool.close().await;
            TestDbConnectionResult::success()
        }
        Err(e) => {
            log::error!("[数据库] 连接测试失败: id={}, error={}", connection.id, e);
            pool.close().await;
            TestDbConnectionResult::failure(e)
        }
    }
}

// ============================================================================
// gRPC 服务发现命令
// ============================================================================

use flow_engine::grpc_connection_manager;
use flow_engine::traits::{GrpcMethodInfo, GrpcProvider, GrpcServiceInfo};

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
    let tls_ca_cert = input.tls_ca_cert.as_deref();
    let tls_client_cert = input.tls_client_cert.as_deref();
    let tls_client_key = input.tls_client_key.as_deref();
    let tls_server_name_override = input.tls_server_name_override.as_deref();
    let reflection_metadata = input.reflection_metadata.unwrap_or_default();

    log::info!(
        "[gRPC] 发现服务: url={}, reflection={}, proto_files={}, metadata_count={}",
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
// gRPC 请求骨架生成命令
// ============================================================================

use flow_engine::proto_parser;

// ============================================================================
// gRPC Proto 目录扫描和 Descriptor Set 命令
// ============================================================================

/// Proto 文件信息（IPC 输出）
#[derive(Debug, serde::Serialize)]
pub struct ProtoFileOutput {
    /// 文件名（相对路径）
    pub path: String,
    /// 文件内容
    pub content: String,
}

/// 扫描 Proto 目录响应
#[derive(Debug, serde::Serialize)]
pub struct GrpcScanProtoDirectoryOutput {
    /// 是否成功
    pub success: bool,
    /// 发现的 proto 文件列表
    pub files: Vec<ProtoFileOutput>,
    /// 目录根路径（用作 proto 编译的 include 路径）
    pub import_path: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 扫描目录，发现所有 .proto 文件
///
/// 递归遍历指定目录及其子目录，收集所有 `.proto` 文件。
/// 跳过以 `.` 开头的隐藏目录。
#[tauri::command]
pub async fn grpc_scan_proto_directory(
    dir_path: String,
) -> GrpcScanProtoDirectoryOutput {
    log::info!("[gRPC] 扫描 Proto 目录: {}", dir_path);

    match proto_parser::discover_proto_files_in_dir(&dir_path) {
        Ok((files, import_path)) => {
            log::info!("[gRPC] 扫描完成: 发现 {} 个 proto 文件", files.len());
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
            log::error!("[gRPC] 扫描目录失败: {}", e);
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
#[derive(Debug, serde::Serialize)]
pub struct GrpcReadProtoFilesOutput {
    /// 是否成功
    pub success: bool,
    /// 文件列表 (filename, content)
    pub files: Vec<ProtoFileOutput>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 读取多个 .proto 文件的内容
///
/// 接收文件路径列表，读取每个文件的内容并返回。
/// 用于文件选择器选择 .proto 文件后批量读取内容。
#[tauri::command]
pub async fn grpc_read_proto_files(
    file_paths: Vec<String>,
) -> GrpcReadProtoFilesOutput {
    log::info!("[gRPC] 读取 {} 个 Proto 文件", file_paths.len());

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
                log::error!("[gRPC] 读取文件失败: {} - {}", file_path, e);
                return GrpcReadProtoFilesOutput {
                    success: false,
                    files: vec![],
                    error: Some(format!("读取文件 {} 失败: {}", file_path, e)),
                };
            }
        }
    }

    log::info!("[gRPC] 成功读取 {} 个 Proto 文件", files.len());
    GrpcReadProtoFilesOutput {
        success: true,
        files,
        error: None,
    }
}

/// 解析 Descriptor Set 响应
#[derive(Debug, serde::Serialize)]
pub struct GrpcParseDescriptorSetOutput {
    /// 是否成功
    pub success: bool,
    /// 发现的服务列表
    pub services: Vec<GrpcServiceInfo>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 解析 `FileDescriptorSet` 二进制文件
///
/// 读取 `.pb` / `.fdset` / `.desc` 格式的 `FileDescriptorSet` 文件，
/// 解析并返回其中的服务和方法信息。
#[tauri::command]
pub async fn grpc_parse_descriptor_set(
    file_path: String,
) -> GrpcParseDescriptorSetOutput {
    log::info!("[gRPC] 解析 Descriptor Set 文件: {}", file_path);

    // 读取文件内容
    let data = match std::fs::read(&file_path) {
        Ok(data) => data,
        Err(e) => {
            let err_msg = format!("读取文件失败: {}", e);
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
    let grpc_services: Vec<GrpcServiceInfo> = services
        .iter()
        .map(|s| GrpcServiceInfo {
            service_name: s.service_name.clone(),
            methods: s
                .methods
                .iter()
                .map(|m| GrpcMethodInfo {
                    method_name: m.method_name.clone(),
                    input_type: m.input_type.clone(),
                    output_type: m.output_type.clone(),
                    is_server_streaming: m.is_server_streaming,
                    is_client_streaming: m.is_client_streaming,
                })
                .collect(),
        })
        .collect();

    log::info!(
        "[gRPC] Descriptor Set 解析成功: 发现 {} 个服务",
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
#[derive(Debug, serde::Serialize)]
pub struct GrpcChannelzOutput {
    /// 是否成功
    pub success: bool,
    /// Channelz 信息（成功时返回）
    pub info: Option<flow_engine::grpc_connection_manager::ChannelzInfo>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 获取 gRPC 连接的 Channelz 内省信息（EXPERIMENTAL）
///
/// 运行时调用：获取活跃 gRPC 连接的轻量级诊断信息。
/// 包括连接状态、运行时长、活跃调用数等。
#[tauri::command]
pub async fn grpc_channelz_info(
    connection_id: String,
    grpc_state: State<'_, GrpcCancelSessionManager>,
) -> Result<GrpcChannelzOutput, String> {
    log::info!("[gRPC Channelz] 查询连接状态: connection_id={}", connection_id);

    // 从取消会话管理器获取 channelz_manager（克隆 Arc 以避免跨 await 持有锁）
    let channelz_manager = {
        let guard = grpc_state.channelz_manager.lock().map_err(|e| e.to_string())?;
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
                        error: Some(format!("连接 '{}' 不存在（可能尚未执行流程）", connection_id)),
                    });
                }
            }

            match m.get_channelz_info(&connection_id).await {
                Ok(info) => {
                    log::info!(
                        "[gRPC Channelz] 连接状态: state={}, uptime={}s, active_calls={}",
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
                    log::error!("[gRPC Channelz] 查询失败: {}", e);
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
            error: Some("没有活跃的执行会话".to_string()),
        }),
    }
}

// ============================================================================
// gRPC 请求骨架生成命令（原有）
// ============================================================================

/// gRPC 请求骨架生成请求参数
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
#[derive(serde::Serialize)]
pub struct GrpcGenerateRequestSkeletonOutput {
    /// 是否成功
    pub success: bool,
    /// 生成的 JSON 骨架字符串
    pub skeleton: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

// ============================================================================
// gRPC 请求 JSON 预验证命令
// ============================================================================

use flow_engine::proto_parser::ValidationResult;

/// gRPC 请求 JSON 验证响应
#[derive(Debug, serde::Serialize)]
pub struct GrpcValidateRequestOutput {
    /// 是否成功执行验证（不代表验证通过）
    pub success: bool,
    /// 验证结果（成功执行时返回）
    pub result: Option<ValidationResult>,
    /// 错误信息（执行失败时）
    pub error: Option<String>,
}

/// gRPC 请求 JSON 验证请求参数
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
#[tauri::command]
pub async fn grpc_validate_request(
    input: GrpcValidateRequestInput,
) -> GrpcValidateRequestOutput {
    log::info!(
        "[gRPC] 验证请求 JSON: message_name={}, proto_files={}",
        input.message_name,
        input.proto_files.as_ref().map(|f| f.len()).unwrap_or(0)
    );

    let proto_files = match input.proto_files {
        Some(files) if !files.is_empty() => files,
        _ => {
            return GrpcValidateRequestOutput {
                success: false,
                result: None,
                error: Some("未提供 proto 文件，无法验证".to_string()),
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
            let err_msg = format!("解析 proto 文件失败: {}", e);
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
                "[gRPC] 验证完成: valid={}, errors={}",
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
            log::error!("[gRPC] 验证失败: {}", e);
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
#[tauri::command]
pub async fn grpc_generate_request_skeleton(
    input: GrpcGenerateRequestSkeletonInput,
) -> GrpcGenerateRequestSkeletonOutput {    log::info!(
        "[gRPC] 生成请求骨架: service={}, method={}, proto_files={}",
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
            let err_msg = format!("解析 proto 文件失败: {}", e);
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
            let err_msg = format!("未找到服务: {}", input.service_name);
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
                "服务 {} 中未找到方法: {}",
                input.service_name, input.method_name
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
        "[gRPC] 骨架生成成功: {}",
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
#[tauri::command]
pub async fn grpc_generate_request_skeleton_from_reflection(
    input: GrpcGenerateRequestSkeletonFromReflectionInput,
) -> GrpcGenerateRequestSkeletonOutput {
    let use_tls = input.use_tls.unwrap_or(false);
    let tls_skip_verify = input.tls_skip_verify.unwrap_or(false);
    let connect_timeout_ms = input.connect_timeout_ms.unwrap_or(30000);
    let reflection_metadata = input.reflection_metadata.unwrap_or_default();

    log::info!(
        "[gRPC] 通过 Reflection 生成请求骨架: url={}, service={}, method={}",
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
                "[gRPC] Reflection 骨架生成成功: {}",
                skeleton.chars().take(100).collect::<String>()
            );
            GrpcGenerateRequestSkeletonOutput {
                success: true,
                skeleton: Some(skeleton),
                error: None,
            }
        }
        Err(e) => {
            log::error!("[gRPC] Reflection 骨架生成失败: {}", e);
            GrpcGenerateRequestSkeletonOutput {
                success: false,
                skeleton: None,
                error: Some(e),
            }
        }
    }
}

#[cfg(test)]
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

    /// 测试成功生成请求骨架
    #[tokio::test]
    async fn test_grpc_generate_skeleton_success() {
        let input = GrpcGenerateRequestSkeletonInput {
            proto_files: vec![("test.proto".to_string(), TEST_PROTO.to_string())],
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
        };

        let output = grpc_generate_request_skeleton(input).await;

        assert!(output.success, "应成功生成骨架");
        assert!(output.skeleton.is_some(), "应返回骨架内容");
        assert!(output.error.is_none(), "不应有错误");

        let skeleton = output.skeleton.expect("骨架应存在");
        assert!(skeleton.contains("user_id"), "骨架应包含 user_id 字段");
        assert!(skeleton.contains("page"), "骨架应包含 page 字段");
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

        assert!(!output.success, "应失败");
        assert!(output.error.is_some(), "应返回错误信息");
        assert!(output.skeleton.is_none(), "不应返回骨架");
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

        assert!(!output.success, "应失败");
        assert!(output.error.is_some(), "应返回错误信息");
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

        assert!(!output.success, "应失败");
        assert!(output.error.is_some(), "应返回错误信息");
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
    /// 传输模式: "`streamable_http`" | "stdio"
    pub transport: Option<String>,
    /// Stdio 模式：可执行文件路径
    pub command: Option<String>,
    /// Stdio 模式：命令参数
    pub args: Option<Vec<String>>,
    /// Stdio 模式：环境变量
    pub env: Option<std::collections::HashMap<String, String>>,
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
/// 支持 Streamable HTTP 和 Stdio 两种传输模式。
#[tauri::command]
pub async fn mcp_discover_tools(
    input: McpDiscoverToolsInput,
) -> McpDiscoverToolsOutput {
    use flow_engine::node_config::{McpConnectConfig, McpTransportMode};

    let transport = match input.transport.as_deref() {
        Some("stdio") => McpTransportMode::Stdio,
        _ => McpTransportMode::StreamableHttp,
    };

    log::info!(
        "[MCP] 发现工具: url={}, protocol_version={}, transport={}",
        input.url,
        input.protocol_version.as_deref().unwrap_or("default"),
        match transport {
            McpTransportMode::StreamableHttp => "streamable_http",
            McpTransportMode::Stdio => "stdio",
        }
    );

    // 创建临时连接管理器
    let manager = flow_engine::flow_engine_mcp::McpConnectionManager::new();

    let config = McpConnectConfig {
        url: input.url,
        headers: input.headers,
        protocol_version: input.protocol_version.unwrap_or_else(|| "2025-11-25".to_string()),
        custom_headers: None,
        discovered_tools: Vec::new(),
        discovered_resources: Vec::new(),
        discovered_prompts: Vec::new(),
        transport,
        command: input.command,
        args: input.args,
        env: input.env,
    };

    match manager.connect(
        "temp-discover".to_string(),
        config,
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
// MQTT 连接测试命令
// ============================================================================

use flow_engine::mqtt_connection_manager::MqttConnectionManager;
use flow_engine::traits::SmtpProvider;
use flow_engine::traits::ImapProvider;
use flow_engine::traits::MqttProvider;
use flow_engine::node_config::MqttConnectConfig;

/// MQTT 连接测试请求参数
#[derive(Debug, serde::Deserialize)]
pub struct MqttTestConnectionInput {
    /// MQTT Broker 地址
    pub url: String,
    /// 客户端 ID
    pub client_id: Option<String>,
    /// 用户名
    pub username: Option<String>,
    /// 密码
    pub password: Option<String>,
    /// 是否使用 TLS
    pub use_tls: Option<bool>,
    /// 是否跳过 TLS 证书验证
    pub tls_skip_verify: Option<bool>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
}

/// MQTT 连接测试响应
#[derive(Debug, serde::Serialize)]
pub struct MqttTestConnectionOutput {
    /// 是否成功
    pub success: bool,
    /// Broker 信息
    pub broker_info: Option<flow_engine::traits::MqttBrokerInfo>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 测试 MQTT 连接
///
/// 设计时调用：创建临时 MQTT 连接验证 Broker 可达性，测试后立即断开。
#[tauri::command]
pub async fn mqtt_test_connection(
    input: MqttTestConnectionInput,
) -> MqttTestConnectionOutput {
    log::info!(
        "[MQTT] 测试连接: url={}, client_id={}",
        input.url,
        input.client_id.as_deref().unwrap_or("auto-generated")
    );

    // 构造连接配置
    let config = MqttConnectConfig {
        url: input.url,
        client_id: input.client_id.or_else(|| {
            Some(format!("mpe-test-{}", uuid::Uuid::new_v4()))
        }),
        username: input.username,
        password: input.password,
        use_tls: input.use_tls,
        tls_skip_verify: input.tls_skip_verify,
        connect_timeout_ms: input.connect_timeout_ms,
        ..Default::default()
    };

    // 创建临时连接管理器
    let manager = MqttConnectionManager::new();

    // 尝试连接
    let result = manager.connect("test", &config).await;

    match result {
        Ok(output) => {
            // 测试完成后立即断开
            let disconnect_config = flow_engine::node_config::MqttDisconnectConfig {
                connection_id: "test".to_string(),
                ..Default::default()
            };
            let _ = manager.disconnect("test", &disconnect_config).await;

            log::info!(
                "[MQTT] 连接测试{}",
                if output.success { "成功" } else { "失败" }
            );

            MqttTestConnectionOutput {
                success: output.success,
                broker_info: output.broker_info,
                error: output.error,
            }
        }
        Err(e) => {
            log::error!("[MQTT] 连接测试失败: {}", e);
            MqttTestConnectionOutput {
                success: false,
                broker_info: None,
                error: Some(e),
            }
        }
    }
}

// ============================================================================
// SSE 连接测试命令
// ============================================================================

/// 测试 SSE 连接
///
/// 设计时调用：验证 SSE 端点 URL 格式有效性。
#[tauri::command]
pub async fn sse_test_connection(url: String) -> Result<bool, String> {
    // URL 格式验证
    if url.is_empty() {
        return Err("URL 不能为空".to_string());
    }
    // 基本 URL 格式验证
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL 必须以 http:// 或 https:// 开头".to_string());
    }
    Ok(true)
}

// ============================================================================
// AMQP 连接测试命令
// ============================================================================

/// 测试 AMQP 连接
///
/// 设计时调用：验证 AMQP Broker URL 格式有效性。
#[tauri::command]
pub async fn amqp_test_connection(url: String) -> Result<bool, String> {
    // URL 格式验证
    if url.is_empty() {
        return Err("URL 不能为空".to_string());
    }
    // 基本 URL 格式验证（支持 amqp:// 和 amqps://）
    if !url.starts_with("amqp://") && !url.starts_with("amqps://") {
        return Err("URL 必须以 amqp:// 或 amqps:// 开头".to_string());
    }
    Ok(true)
}

// ============================================================================
// Email 连接测试命令
// ============================================================================

/// SMTP 连接测试输入
#[derive(Debug, serde::Deserialize)]
pub struct SmtpTestConnectionInput {
    /// SMTP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式: "none", "starttls", "implicit"
    pub tls_mode: String,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 密码（明文，会被自动加密）
    pub password: Option<String>,
}

/// SMTP 连接测试输出
#[derive(Debug, serde::Serialize)]
pub struct SmtpTestConnectionOutput {
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 测试 SMTP 连接
///
/// 设计时调用：创建临时 SMTP 连接验证服务器可达性，测试后不保留连接。
#[tauri::command]
pub async fn test_smtp_connection(
    input: SmtpTestConnectionInput,
) -> SmtpTestConnectionOutput {
    use flow_engine::email_types::{SmtpConfig, TlsMode, EmailAuthConfig};

    log::info!("[SMTP] 测试连接: {}:{}", input.host, input.port);

    let tls_mode = match input.tls_mode.as_str() {
        "starttls" => TlsMode::StartTls,
        "implicit" => TlsMode::Implicit,
        _ => TlsMode::None,
    };

    let auth = match (input.username, input.password) {
        (Some(u), Some(p)) => {
            match flow_engine::encryption::encrypt(&p) {
                Ok(encrypted) => Some(EmailAuthConfig::Plain {
                    username: u,
                    encrypted_password: encrypted,
                }),
                Err(e) => {
                    return SmtpTestConnectionOutput {
                        success: false,
                        error: Some(format!("加密密码失败: {}", e)),
                    };
                }
            }
        }
        _ => None,
    };

    let config = SmtpConfig {
        host: input.host,
        port: input.port,
        tls_mode,
        auth,
    };

    let manager = flow_engine::smtp_connection_manager::SmtpConnectionManager::new();
    match manager.test_connection(&config).await {
        Ok(true) => {
            log::info!("[SMTP] 连接测试成功");
            SmtpTestConnectionOutput {
                success: true,
                error: None,
            }
        }
        Ok(false) => {
            log::warn!("[SMTP] 连接测试失败");
            SmtpTestConnectionOutput {
                success: false,
                error: Some("连接测试失败".to_string()),
            }
        }
        Err(e) => {
            log::error!("[SMTP] 连接测试失败: {}", e);
            SmtpTestConnectionOutput {
                success: false,
                error: Some(e),
            }
        }
    }
}

/// IMAP 连接测试输入
#[derive(Debug, serde::Deserialize)]
pub struct ImapTestConnectionInput {
    /// IMAP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式: "none", "starttls", "implicit"
    pub tls_mode: String,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 密码（明文，会被自动加密）
    pub password: Option<String>,
}

/// IMAP 连接测试输出
#[derive(Debug, serde::Serialize)]
pub struct ImapTestConnectionOutput {
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 测试 IMAP 连接
///
/// 设计时调用：创建临时 IMAP 连接验证服务器可达性，测试后立即断开。
#[tauri::command]
pub async fn test_imap_connection(
    input: ImapTestConnectionInput,
) -> ImapTestConnectionOutput {
    use flow_engine::email_types::{ImapConfig, TlsMode, EmailAuthConfig};

    log::info!("[IMAP] 测试连接: {}:{}", input.host, input.port);

    let tls_mode = match input.tls_mode.as_str() {
        "starttls" => TlsMode::StartTls,
        "implicit" => TlsMode::Implicit,
        _ => TlsMode::None,
    };

    let auth = match (input.username, input.password) {
        (Some(u), Some(p)) => {
            match flow_engine::encryption::encrypt(&p) {
                Ok(encrypted) => Some(EmailAuthConfig::Plain {
                    username: u,
                    encrypted_password: encrypted,
                }),
                Err(e) => {
                    return ImapTestConnectionOutput {
                        success: false,
                        error: Some(format!("加密密码失败: {}", e)),
                    };
                }
            }
        }
        _ => None,
    };

    let config = ImapConfig {
        host: input.host,
        port: input.port,
        tls_mode,
        auth,
    };

    let manager = flow_engine::imap_connection_manager::ImapConnectionManager::new();
    match manager.test_connection(&config).await {
        Ok(true) => {
            log::info!("[IMAP] 连接测试成功");
            ImapTestConnectionOutput {
                success: true,
                error: None,
            }
        }
        Ok(false) => {
            log::warn!("[IMAP] 连接测试失败");
            ImapTestConnectionOutput {
                success: false,
                error: Some("连接测试失败".to_string()),
            }
        }
        Err(e) => {
            log::error!("[IMAP] 连接测试失败: {}", e);
            ImapTestConnectionOutput {
                success: false,
                error: Some(e),
            }
        }
    }
}

// ============================================================================
// Email OAuth2 命令
// ============================================================================

/// `OAuth2` 授权输入
#[derive(serde::Deserialize)]
pub struct EmailOAuth2AuthorizeInput {
    /// 邮件服务商: "gmail" | "outlook" | "yahoo" | "custom"
    pub provider: String,
    /// `OAuth2` 客户端 ID
    pub client_id: String,
    /// `OAuth2` 客户端密钥（可选，PKCE 流程不需要）
    pub client_secret: Option<String>,
    /// 本地回调监听端口
    pub redirect_port: Option<u16>,
}

/// `OAuth2` 授权输出
#[derive(serde::Serialize)]
pub struct EmailOAuth2AuthorizeOutput {
    /// 是否成功
    pub success: bool,
    /// 加密后的 Token（格式: `base64(nonce):base64(ciphertext)`）
    pub encrypted_tokens: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 触发 `OAuth2` 授权流程
///
/// 在浏览器中打开 `OAuth2` 授权页面，等待用户授权后获取 Token。
/// 使用 PKCE 流程，无需 `client_secret`。
/// Token 获取后自动加密存储并返回密文。
#[tauri::command]
pub async fn email_oauth2_authorize(
    input: EmailOAuth2AuthorizeInput,
) -> EmailOAuth2AuthorizeOutput {
    log::info!(
        "[OAuth2] 开始授权流程: provider={}, client_id={}",
        input.provider,
        input.client_id
    );

    // 解析 provider
    let provider = match input.provider.as_str() {
        "gmail" => flow_engine::email_types::EmailProvider::Gmail,
        "outlook" => flow_engine::email_types::EmailProvider::Outlook,
        "yahoo" => flow_engine::email_types::EmailProvider::Yahoo,
        _ => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(format!("不支持的服务商: {}", input.provider)),
            };
        }
    };

    let redirect_port = input.redirect_port.unwrap_or(8080);

    // 1. 创建 OAuth2 配置
    let config = match flow_engine::oauth2::oauth2_config_from_provider(
        &provider,
        &input.client_id,
        input.client_secret.as_deref(),
        redirect_port,
    ) {
        Ok(c) => c,
        Err(e) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 2. 构建 PKCE 授权 URL
    let (auth_url, pkce_verifier, _csrf_state) = match flow_engine::oauth2::build_pkce_auth_url(&config) {
        Ok(result) => result,
        Err(e) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 3. 在后台启动回调监听器，同时打开浏览器
    let listener_handle = tokio::spawn(async move {
        flow_engine::oauth2::start_callback_listener(redirect_port).await
    });

    // 使用 tauri-plugin-opener 打开浏览器
    if let Err(e) = tauri_plugin_opener::open_url(&auth_url, None::<&str>) {
        log::warn!("[OAuth2] 打开浏览器失败: {}，请手动打开以下链接:\n{}", e, auth_url);
    }

    // 4. 等待回调
    let (code, _state) = match listener_handle.await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
        Err(e) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(format!("监听任务失败: {}", e)),
            };
        }
    };

    // 5. 用授权码换取 Token
    let tokens = match flow_engine::oauth2::exchange_code(&config, &code, &pkce_verifier).await {
        Ok(t) => t,
        Err(e) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 6. 加密存储 Token
    let encrypted = match flow_engine::oauth2::store_tokens_encrypted(&tokens) {
        Ok(e) => e,
        Err(e) => {
            return EmailOAuth2AuthorizeOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    EmailOAuth2AuthorizeOutput {
        success: true,
        encrypted_tokens: Some(encrypted),
        error: None,
    }
}

/// `OAuth2` 刷新输入
#[derive(serde::Deserialize)]
pub struct EmailOAuth2RefreshInput {
    /// 邮件服务商: "gmail" | "outlook" | "yahoo" | "custom"
    pub provider: String,
    /// `OAuth2` 客户端 ID
    pub client_id: String,
    /// `OAuth2` 客户端密钥（可选）
    pub client_secret: Option<String>,
    /// 加密后的 Token 密文
    pub encrypted_tokens: String,
}

/// `OAuth2` 刷新输出
#[derive(serde::Serialize)]
pub struct EmailOAuth2RefreshOutput {
    /// 是否成功
    pub success: bool,
    /// 加密后的新 Token
    pub encrypted_tokens: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 刷新 `OAuth2` Token
///
/// 解密现有 Token，使用 `refresh_token` 获取新的 `access_token`，
/// 加密后返回。
#[tauri::command]
pub async fn email_oauth2_refresh(
    input: EmailOAuth2RefreshInput,
) -> EmailOAuth2RefreshOutput {
    log::info!("[OAuth2] 刷新 token: provider={}", input.provider);

    let provider = match input.provider.as_str() {
        "gmail" => flow_engine::email_types::EmailProvider::Gmail,
        "outlook" => flow_engine::email_types::EmailProvider::Outlook,
        "yahoo" => flow_engine::email_types::EmailProvider::Yahoo,
        _ => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(format!("不支持的服务商: {}", input.provider)),
            };
        }
    };

    // 1. 解密现有 Token
    let mut tokens = match flow_engine::oauth2::load_tokens_decrypted(&input.encrypted_tokens) {
        Ok(t) => t,
        Err(e) => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 2. 检查是否有 refresh_token
    let refresh_token = match tokens.refresh_token {
        Some(ref t) if !t.is_empty() => t.clone(),
        _ => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some("没有 refresh_token，无法刷新。请重新授权。".to_string()),
            };
        }
    };

    // 3. 创建 OAuth2 配置
    let config = match flow_engine::oauth2::oauth2_config_from_provider(
        &provider,
        &input.client_id,
        input.client_secret.as_deref(),
        0,
    ) {
        Ok(c) => c,
        Err(e) => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 4. 刷新 Token
    let new_tokens = match flow_engine::oauth2::refresh_token(&config, &refresh_token).await {
        Ok(t) => t,
        Err(e) => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    // 如果响应中没有 refresh_token，保留原有的
    let has_refresh = new_tokens.refresh_token.is_some();
    let tokens_to_store = if has_refresh {
        new_tokens
    } else {
        tokens.access_token = new_tokens.access_token;
        tokens.expires_at = new_tokens.expires_at;
        tokens.scope = new_tokens.scope;
        tokens
    };

    // 5. 加密返回
    let encrypted = match flow_engine::oauth2::store_tokens_encrypted(&tokens_to_store) {
        Ok(e) => e,
        Err(e) => {
            return EmailOAuth2RefreshOutput {
                success: false,
                encrypted_tokens: None,
                error: Some(e),
            };
        }
    };

    EmailOAuth2RefreshOutput {
        success: true,
        encrypted_tokens: Some(encrypted),
        error: None,
    }
}

/// `OAuth2` 撤销输入
#[derive(serde::Deserialize)]
pub struct EmailOAuth2RevokeInput {
    /// 邮件服务商: "gmail" | "outlook" | "yahoo" | "custom"
    pub provider: String,
    /// 加密后的 Token 密文
    pub encrypted_tokens: String,
}

/// `OAuth2` 撤销输出
#[derive(serde::Serialize)]
pub struct EmailOAuth2RevokeOutput {
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 撤销 `OAuth2` 授权
///
/// 解密 Token 并向服务商发送撤销请求，使 Token 失效。
#[tauri::command]
pub async fn email_oauth2_revoke(
    input: EmailOAuth2RevokeInput,
) -> EmailOAuth2RevokeOutput {
    log::info!("[OAuth2] 撤销授权: provider={}", input.provider);

    // 解密 Token 获取 access_token
    let tokens = match flow_engine::oauth2::load_tokens_decrypted(&input.encrypted_tokens) {
        Ok(t) => t,
        Err(e) => {
            return EmailOAuth2RevokeOutput {
                success: false,
                error: Some(e),
            };
        }
    };

    // 确定撤销端点
    let revoke_url = match input.provider.as_str() {
        "gmail" => Some("https://oauth2.googleapis.com/revoke"),
        "yahoo" => Some("https://api.login.yahoo.com/oauth2/revoke"),
        _ => None, // Outlook/Microsoft 无标准 OAuth2 撤销端点
    };

    if let Some(url) = revoke_url {
        let client = reqwest::Client::new();
        match client.post(url).form(&[("token", tokens.access_token.as_str())]).send().await {
            Ok(resp) if resp.status().is_success() => {
                log::info!("[OAuth2] Token 撤销成功");
            }
            Ok(resp) => {
                log::warn!("[OAuth2] 撤销请求返回非成功状态: {}", resp.status());
                // 不阻塞——即使撤销请求失败，Token 也会从本地移除
            }
            Err(e) => {
                log::warn!("[OAuth2] 撤销请求失败: {}", e);
            }
        }
    } else {
        log::info!("[OAuth2] {} 无标准撤销端点，仅清除本地 Token", input.provider);
    }

    EmailOAuth2RevokeOutput {
        success: true,
        error: None,
    }
}

// ============================================================================
// TCP 协议模板命令
// ============================================================================

/// 获取所有 TCP 协议内置模板
///
/// 返回所有预置的协议模板（Redis RESP、TLV、固定头部等）。
#[tauri::command]
pub fn tcp_list_templates() -> Vec<flow_engine::templates::ProtocolTemplate> {
    flow_engine::templates::builtin_templates()
}

/// 根据 ID 获取指定 TCP 协议模板
///
/// 返回与 ID 匹配的模板，不存在则返回 null。
#[tauri::command]
pub fn tcp_get_template(id: String) -> Option<flow_engine::templates::ProtocolTemplate> {
    flow_engine::templates::get_template(&id)
}

// ============================================================================
// GraphQL IPC 命令
// ============================================================================

/// GraphQL 内省请求参数
#[derive(Debug, serde::Deserialize)]
pub struct GraphqlIntrospectInput {
    /// GraphQL 端点 URL
    pub endpoint: String,
    /// 请求头
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// GraphQL 内省响应
#[derive(Debug, serde::Serialize)]
pub struct GraphqlIntrospectOutput {
    /// 是否成功
    pub success: bool,
    /// Schema JSON 字符串
    pub schema: Option<serde_json::Value>,
    /// 生成的查询模板
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_templates: Option<Vec<flow_engine::traits::GraphqlQueryTemplate>>,
    /// 错误信息
    pub error: Option<String>,
}

/// 从 Schema 类型定义中提取字段的嵌套子字段（递归，最大深度 2）
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
fn is_leaf_type(name: &str) -> bool {
    matches!(name, "String" | "Int" | "Float" | "Boolean" | "ID")
}

/// 从 type 引用结构中提取实际类型名（剥掉 `NON_NULL` / LIST 包装）
fn resolve_type_name(type_ref: Option<&serde_json::Value>) -> Option<String> {
    let mut current = type_ref?;
    loop {
        match current.get("kind").and_then(|k| k.as_str()) {
            Some("NON_NULL") | Some("LIST") => {
                current = current.get("ofType")?;
            }
            Some(_) => {
                return current.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
            }
            None => return None,
        }
    }
}

/// 从 Schema 数据生成查询模板（含展开的子字段）
fn generate_graphql_templates(schema: &serde_json::Value) -> Option<Vec<flow_engine::traits::GraphqlQueryTemplate>> {
    let schema_obj = schema.get("__schema")?;
    let types = schema_obj.get("types").and_then(|t| t.as_array())?;

    // 构建类型索引: type_name → type_def
    let types_map: std::collections::HashMap<&str, &serde_json::Value> = types
        .iter()
        .filter_map(|t| {
            t.get("name")
                .and_then(|n| n.as_str())
                .map(|name| (name, t))
        })
        .collect();

    let mut templates = Vec::new();

    // 生成 Query 模板
    if let Some(qt_name) = schema_obj.get("queryType").and_then(|qt| qt.get("name")).and_then(|n| n.as_str()) {
        if let Some(query_type_def) = types_map.get(qt_name) {
            if let Some(query_fields) = query_type_def.get("fields").and_then(|f| f.as_array()) {
                for field in query_fields {
                    let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.starts_with("__") { continue; }

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
                        format!("query {} {{\n  {} {{\n{}  }}\n}}", pascal_case(name), name, indent(&field_nested, 4))
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
            if let Some(mutation_fields) = mutation_type_def.get("fields").and_then(|f| f.as_array()) {
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
                        format!("mutation {} {{\n  {} {{\n{}  }}\n}}", pascal_case(name), name, indent(&field_nested, 4))
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

    if templates.is_empty() { None } else { Some(templates) }
}

/// 首字母大写
fn pascal_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

/// 缩进文本
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
#[tauri::command]
pub async fn graphql_introspect_schema(
    input: GraphqlIntrospectInput,
) -> GraphqlIntrospectOutput {
    log::info!("[GraphQL] 内省请求: endpoint={}", input.endpoint);

    // URL 格式验证
    if input.endpoint.is_empty() {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("端点 URL 不能为空".to_string()),
        };
    }
    if !input.endpoint.starts_with("http://") && !input.endpoint.starts_with("https://") {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("端点 URL 必须以 http:// 或 https:// 开头".to_string()),
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
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e));

    let client = match client {
        Ok(c) => c,
        Err(e) => return GraphqlIntrospectOutput {
            success: false, schema: None, query_templates: None, error: Some(e),
        },
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
                    if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
                        let next_url = if location.starts_with("http://") || location.starts_with("https://") {
                            location.to_string()
                        } else if location.starts_with('/') {
                            // 以 / 开头的相对路径 → 相对 origin 解析
                            let origin = current_url.split('/')
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
                        success: false, schema: None, query_templates: None,
                        error: Some(format!("重定向无 Location 头: {}", status)),
                    });
                    break;
                }

                // 非重定向 → 解析响应
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        if status.is_success() {
                            if let Some(errors) = body.get("errors") {
                                result = Some(GraphqlIntrospectOutput {
                                    success: false, schema: None, query_templates: None,
                                    error: Some(format!("GraphQL 错误: {}", errors)),
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
                                success: false, schema: None, query_templates: None,
                                error: Some(format!("HTTP 错误: {}", status)),
                            });
                        }
                    }
                    Err(e) => {
                        result = Some(GraphqlIntrospectOutput {
                            success: false, schema: None, query_templates: None,
                            error: Some(format!("解析响应失败: {}", e)),
                        });
                    }
                }
                break;
            }
            Err(e) => {
                result = Some(GraphqlIntrospectOutput {
                    success: false, schema: None, query_templates: None,
                    error: Some(format!("请求失败: {}", e)),
                });
                break;
            }
        }
    }

    result.unwrap_or_else(|| GraphqlIntrospectOutput {
        success: false, schema: None, query_templates: None,
        error: Some("重定向次数过多".to_string()),
    })
}

// ============================================================================
// Kafka IPC 命令
// ============================================================================

use flow_engine::kafka_connection_manager::KafkaConnectionManager;
use flow_engine::traits::KafkaProvider;
use flow_engine::node_config::KafkaConnectConfig;

/// Kafka 连接测试请求
#[derive(Debug, serde::Deserialize)]
pub struct KafkaTestConnectionInput {
    /// Kafka Bootstrap Servers（逗号分隔）
    pub bootstrap_servers: String,
    /// 客户端 ID
    pub client_id: Option<String>,
    /// `安全协议："plaintext"/"ssl"/"sasl_plaintext"/"sasl_ssl`"
    pub security_protocol: Option<String>,
    /// SASL 机制："PLAIN"/"SCRAM-SHA-256"/"SCRAM-SHA-512"/"OAUTHBEARER"
    pub sasl_mechanism: Option<String>,
    /// SASL 用户名
    pub sasl_username: Option<String>,
    /// SASL 密码
    pub sasl_password: Option<String>,
    /// SASL OAuth Token
    pub sasl_oauth_token: Option<String>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
}

/// Kafka 连接测试响应
#[derive(Debug, serde::Serialize)]
pub struct KafkaTestConnectionOutput {
    /// 是否成功
    pub success: bool,
    /// Broker 信息列表
    pub broker_info: Option<Vec<flow_engine::traits::KafkaBrokerInfo>>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// Kafka 列出 Topic 请求
#[derive(Debug, serde::Deserialize)]
pub struct KafkaListTopicsInput {
    /// Kafka Bootstrap Servers
    pub bootstrap_servers: String,
    /// 安全协议
    pub security_protocol: Option<String>,
    /// SASL 机制
    pub sasl_mechanism: Option<String>,
    /// SASL 用户名
    pub sasl_username: Option<String>,
    /// SASL 密码
    pub sasl_password: Option<String>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
}

/// Kafka 列出 Topic 响应
#[derive(Debug, serde::Serialize)]
pub struct KafkaListTopicsOutput {
    /// 是否成功
    pub success: bool,
    /// Topic 列表
    pub topics: Option<Vec<flow_engine::traits::KafkaTopicInfo>>,
    /// 错误信息
    pub error: Option<String>,
}

/// Kafka 列出 Consumer Group 请求
#[derive(Debug, serde::Deserialize)]
pub struct KafkaListConsumerGroupsInput {
    /// Kafka Bootstrap Servers
    pub bootstrap_servers: String,
    /// 安全协议
    pub security_protocol: Option<String>,
    /// SASL 机制
    pub sasl_mechanism: Option<String>,
    /// SASL 用户名
    pub sasl_username: Option<String>,
    /// SASL 密码
    pub sasl_password: Option<String>,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: Option<u64>,
}

/// Kafka 列出 Consumer Group 响应
#[derive(Debug, serde::Serialize)]
pub struct KafkaListConsumerGroupsOutput {
    /// 是否成功
    pub success: bool,
    /// Consumer Group 列表
    pub consumer_groups: Option<Vec<flow_engine::traits::KafkaConsumerGroupInfo>>,
    /// 错误信息
    pub error: Option<String>,
}

/// Kafka Schema 发现请求
#[derive(Debug, serde::Deserialize)]
pub struct KafkaDiscoverSchemaInput {
    /// Schema Registry URL
    pub schema_registry_url: String,
    /// Kafka Bootstrap Servers（用于建立连接）
    pub bootstrap_servers: Option<String>,
    /// 用户名（Basic Auth）
    pub username: Option<String>,
    /// 密码（Basic Auth）
    pub password: Option<String>,
}

/// Kafka Schema 发现响应
#[derive(Debug, serde::Serialize)]
pub struct KafkaDiscoverSchemaOutput {
    /// 是否成功
    pub success: bool,
    /// Schema Subject 列表
    pub subjects: Option<Vec<String>>,
    /// 错误信息
    pub error: Option<String>,
}

/// 测试 Kafka 连接
///
/// 设计时调用：创建临时 Kafka 连接，获取 Broker 信息后立即断开。
#[tauri::command]
pub async fn kafka_test_connection(
    input: KafkaTestConnectionInput,
) -> KafkaTestConnectionOutput {
    log::info!(
        "[Kafka] 测试连接: bootstrap_servers={}",
        input.bootstrap_servers
    );

    // 构造连接配置
    let config = KafkaConnectConfig {
        bootstrap_servers: input.bootstrap_servers,
        client_id: input
            .client_id
            .or_else(|| Some(format!("mpe-test-{}", uuid::Uuid::new_v4()))),
        security_protocol: input.security_protocol,
        sasl_mechanism: input.sasl_mechanism,
        sasl_username: input.sasl_username,
        sasl_password: input.sasl_password,
        sasl_oauth_token: input.sasl_oauth_token,
        connect_timeout_ms: input.connect_timeout_ms,
        ..Default::default()
    };

    let manager = KafkaConnectionManager::new();

    // 尝试连接
    let result = manager.connect("test", &config).await;

    match result {
        Ok(_output) => {
            // 连接成功后获取 Broker 信息
            let admin_config = flow_engine::node_config::KafkaAdminConfig {
                operation: "broker_list".to_string(),
                ..Default::default()
            };
            let admin_result = manager.admin("test", &admin_config).await;

            // 关闭连接
            let close_config = flow_engine::node_config::KafkaCloseConfig {
                ..Default::default()
            };
            let _ = manager.close("test", &close_config).await;

            match admin_result {
                Ok(admin_output) => {
                    log::info!(
                        "[Kafka] 连接测试成功，获取到 {} 个 Broker",
                        admin_output.brokers.len()
                    );
                    KafkaTestConnectionOutput {
                        success: true,
                        broker_info: Some(admin_output.brokers),
                        error: None,
                    }
                }
                Err(e) => {
                    log::warn!("[Kafka] 连接成功但获取 Broker 信息失败: {}", e);
                    KafkaTestConnectionOutput {
                        success: true,
                        broker_info: None,
                        error: Some(format!("连接成功但获取 Broker 信息失败: {}", e)),
                    }
                }
            }
        }
        Err(e) => {
            log::error!("[Kafka] 连接测试失败: {}", e);
            KafkaTestConnectionOutput {
                success: false,
                broker_info: None,
                error: Some(e),
            }
        }
    }
}

/// 列出 Kafka Topic
///
/// 设计时调用：创建临时连接，列出所有 Topic 后立即断开。
#[tauri::command]
pub async fn kafka_list_topics(
    input: KafkaListTopicsInput,
) -> KafkaListTopicsOutput {
    log::info!(
        "[Kafka] 列出 Topic: bootstrap_servers={}",
        input.bootstrap_servers
    );

    // 构造连接配置
    let config = KafkaConnectConfig {
        bootstrap_servers: input.bootstrap_servers,
        client_id: Some(format!("mpe-topics-{}", uuid::Uuid::new_v4())),
        security_protocol: input.security_protocol,
        sasl_mechanism: input.sasl_mechanism,
        sasl_username: input.sasl_username,
        sasl_password: input.sasl_password,
        connect_timeout_ms: input.connect_timeout_ms,
        ..Default::default()
    };

    let manager = KafkaConnectionManager::new();

    // 连接
    let result = manager.connect("test", &config).await;

    match result {
        Ok(_output) => {
            // 列出 Topic
            let admin_config = flow_engine::node_config::KafkaAdminConfig {
                operation: "topic_list".to_string(),
                ..Default::default()
            };
            let admin_result = manager.admin("test", &admin_config).await;

            // 关闭连接
            let close_config = flow_engine::node_config::KafkaCloseConfig {
                ..Default::default()
            };
            let _ = manager.close("test", &close_config).await;

            match admin_result {
                Ok(admin_output) => {
                    log::info!(
                        "[Kafka] 列出 Topic 成功，共 {} 个",
                        admin_output.topics.len()
                    );
                    KafkaListTopicsOutput {
                        success: true,
                        topics: Some(admin_output.topics),
                        error: None,
                    }
                }
                Err(e) => {
                    log::error!("[Kafka] 列出 Topic 失败: {}", e);
                    KafkaListTopicsOutput {
                        success: false,
                        topics: None,
                        error: Some(e),
                    }
                }
            }
        }
        Err(e) => {
            log::error!("[Kafka] 连接失败: {}", e);
            KafkaListTopicsOutput {
                success: false,
                topics: None,
                error: Some(e),
            }
        }
    }
}

/// 列出 Kafka Consumer Group
///
/// 设计时调用：创建临时连接，列出所有 Consumer Group 后立即断开。
#[tauri::command]
pub async fn kafka_list_consumer_groups(
    input: KafkaListConsumerGroupsInput,
) -> KafkaListConsumerGroupsOutput {
    log::info!(
        "[Kafka] 列出 Consumer Group: bootstrap_servers={}",
        input.bootstrap_servers
    );

    // 构造连接配置
    let config = KafkaConnectConfig {
        bootstrap_servers: input.bootstrap_servers,
        client_id: Some(format!("mpe-groups-{}", uuid::Uuid::new_v4())),
        security_protocol: input.security_protocol,
        sasl_mechanism: input.sasl_mechanism,
        sasl_username: input.sasl_username,
        sasl_password: input.sasl_password,
        connect_timeout_ms: input.connect_timeout_ms,
        ..Default::default()
    };

    let manager = KafkaConnectionManager::new();

    // 连接
    let result = manager.connect("test", &config).await;

    match result {
        Ok(_output) => {
            // 列出 Consumer Group
            let admin_config = flow_engine::node_config::KafkaAdminConfig {
                operation: "consumer_group_list".to_string(),
                ..Default::default()
            };
            let admin_result = manager.admin("test", &admin_config).await;

            // 关闭连接
            let close_config = flow_engine::node_config::KafkaCloseConfig {
                ..Default::default()
            };
            let _ = manager.close("test", &close_config).await;

            match admin_result {
                Ok(admin_output) => {
                    log::info!(
                        "[Kafka] 列出 Consumer Group 成功，共 {} 个",
                        admin_output.consumer_groups.len()
                    );
                    KafkaListConsumerGroupsOutput {
                        success: true,
                        consumer_groups: Some(admin_output.consumer_groups),
                        error: None,
                    }
                }
                Err(e) => {
                    log::error!("[Kafka] 列出 Consumer Group 失败: {}", e);
                    KafkaListConsumerGroupsOutput {
                        success: false,
                        consumer_groups: None,
                        error: Some(e),
                    }
                }
            }
        }
        Err(e) => {
            log::error!("[Kafka] 连接失败: {}", e);
            KafkaListConsumerGroupsOutput {
                success: false,
                consumer_groups: None,
                error: Some(e),
            }
        }
    }
}

/// 发现 Kafka Schema Registry 中的 Subject
///
/// 设计时调用：创建临时连接，列出所有 Schema Subject 后立即断开。
#[tauri::command]
pub async fn kafka_discover_schema(
    input: KafkaDiscoverSchemaInput,
) -> KafkaDiscoverSchemaOutput {
    log::info!(
        "[Kafka] 发现 Schema: schema_registry_url={}",
        input.schema_registry_url
    );

    // 构造连接配置（包含 Schema Registry 信息）
    let config = KafkaConnectConfig {
        bootstrap_servers: input
            .bootstrap_servers
            .unwrap_or_else(|| "localhost:9092".to_string()),
        client_id: Some(format!("mpe-schema-{}", uuid::Uuid::new_v4())),
        schema_registry_url: Some(input.schema_registry_url),
        schema_registry_username: input.username,
        schema_registry_password: input.password,
        ..Default::default()
    };

    let manager = KafkaConnectionManager::new();

    // 连接
    let result = manager.connect("test", &config).await;

    match result {
        Ok(_output) => {
            // 列出 Subject
            let sr_config = flow_engine::node_config::KafkaSchemaRegistryConfig {
                operation: "list_subjects".to_string(),
                ..Default::default()
            };
            let sr_result = manager.schema_registry("test", &sr_config).await;

            // 关闭连接
            let close_config = flow_engine::node_config::KafkaCloseConfig {
                ..Default::default()
            };
            let _ = manager.close("test", &close_config).await;

            match sr_result {
                Ok(sr_output) => {
                    log::info!(
                        "[Kafka] 发现 Schema 成功，共 {} 个 Subject",
                        sr_output.subjects.len()
                    );
                    KafkaDiscoverSchemaOutput {
                        success: true,
                        subjects: Some(sr_output.subjects),
                        error: None,
                    }
                }
                Err(e) => {
                    log::error!("[Kafka] 发现 Schema 失败: {}", e);
                    KafkaDiscoverSchemaOutput {
                        success: false,
                        subjects: None,
                        error: Some(e),
                    }
                }
            }
        }
        Err(e) => {
            log::error!("[Kafka] 连接失败: {}", e);
            KafkaDiscoverSchemaOutput {
                success: false,
                subjects: None,
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
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    let templates_dir = app_data_dir.join("templates");
    std::fs::create_dir_all(&templates_dir)
        .map_err(|e| format!("创建模板目录失败: {}", e))?;
    let file_path = templates_dir.join(&filename);
    // 写入 UTF-8 BOM + 内容，确保 Excel 正确识别中文编码
    let bom = "\u{FEFF}";
    let bytes = format!("{}{}", bom, content);
    std::fs::write(&file_path, bytes.as_bytes())
        .map_err(|e| format!("写入模板文件失败: {}", e))?;
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
    GetStressTestStatusOutput, StartStressTestInput, StressTestEvent,
    StressTestSessionManager,
};

// ============================================================================
// 数据集执行命令
// ============================================================================

use flow_engine::dataset_executor::{
    DatasetEvent, DatasetExecutionConfig,
};
use flow_engine::csv_parser;

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
/// 通过 `type_id` 查找已注册的节点插件，返回其配置 schema。
/// 若 `type_id` 不存在或插件未提供 schema，返回 None。
#[tauri::command]
pub fn get_node_config_schema(type_id: String) -> Option<serde_json::Value> {
    let registry = flow_engine::default_registry::default_registry();
    registry.get(&type_id).and_then(|plugin| plugin.config_schema())
}

/// 获取所有已注册插件的元数据列表
///
/// 与 `get_registered_node_types` 相同，语义上更侧重"插件"概念。
/// 返回全部插件的 `NodeMetadata` 列表。
#[tauri::command]
pub fn list_registered_plugins() -> Vec<NodeMetadata> {
    let registry = flow_engine::default_registry::default_registry();
    registry.all_metadata()
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
) -> Result<(), String> {
    // 获取应用数据目录（用于保存失败行详情）
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;

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

    // 在后台启动执行
    tokio::spawn(async move {
        let registry = std::sync::Arc::new(flow_engine::default_registry::default_registry());
        let executor = flow_engine::dataset_executor::DatasetExecutor::new(registry);
        let result = executor
            .execute(&config, &flow, Some(cancel_rx), Some(event_sender))
            .await;

        // 保存所有行详情到磁盘
        if let Ok((_summaries, row_details, report_uuid)) = result {
            if !row_details.is_empty() {
                // 使用 executor 返回的 report UUID 直接定位目录
                let report_dir = app_data_dir.join("reports").join(format!("dataset_{}", report_uuid));
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

        // 执行完成后清理 cancel sender
        if let Ok(mut guard) = state_cancel.lock() {
            *guard = None;
        }
    });

    Ok(())
}

/// 取消数据驱动执行
///
/// 向当前正在运行的数据驱动执行发送取消信号。
/// 幂等操作：第二次调用返回"没有活跃的执行会话"错误。
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
        None => Err("没有活跃的数据驱动执行会话".to_string()),
    }
}

/// 列出数据集报告
///
/// 扫描报告目录中所有 dataset_* 子目录，读取 summary.json 返回摘要列表。
#[tauri::command]
pub fn list_dataset_reports(
    reports_dir: String,
) -> Result<Vec<serde_json::Value>, String> {
    let reports_path = std::path::Path::new(&reports_dir);
    if !reports_path.exists() {
        return Ok(vec![]);
    }

    let mut summaries: Vec<serde_json::Value> = Vec::new();

    let entries = std::fs::read_dir(reports_path)
        .map_err(|e| format!("读取报告目录失败: {}", e))?;

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
        return Err(format!("报告不存在: {}", report_id));
    }

    let content = std::fs::read_to_string(&summary_path)
        .map_err(|e| format!("读取报告失败: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("解析报告失败: {}", e))
}

/// 加载失败行详情
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
        .map_err(|e| format!("读取行详情失败: {}", e))?;

    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析行详情失败: {}", e))?;

    Ok(Some(value))
}

/// 删除数据集报告
///
/// 删除指定报告 ID 的整个目录。
#[tauri::command]
pub fn delete_dataset_report(
    report_id: String,
    reports_dir: String,
) -> Result<(), String> {
    let report_dir = std::path::Path::new(&reports_dir)
        .join(format!("dataset_{}", report_id));

    if !report_dir.exists() {
        return Err(format!("报告不存在: {}", report_id));
    }

    std::fs::remove_dir_all(&report_dir)
        .map_err(|e| format!("删除报告失败: {}", e))
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
    use std::collections::HashMap;
    use flow_engine::report::FlowExecutionReport;

    let parsed_report: DatasetExecutionReport =
        serde_json::from_value(report).map_err(|e| format!("解析报告失败: {}", e))?;

    // save_report expects app_data_dir (parent of reports/)，所以去掉末尾的 "reports" 组件
    let reports_path = std::path::Path::new(&reports_dir);
    let app_data_dir = reports_path
        .parent()
        .ok_or_else(|| "无法获取报告目录的父目录".to_string())?;

    let failed_details: HashMap<u32, FlowExecutionReport> = HashMap::new();
    flow_engine::dataset_executor::save_report(&parsed_report, app_data_dir, &failed_details)
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
        let result = executor.execute_flow(&flow, None, None, None).await;

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
        let result = executor.execute_flow(&flow, None, None, None).await;

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
        let result = executor.execute_flow(&flow, None, None, None).await;

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
        let result = executor.execute_flow(&flow, None, None, None).await;

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
