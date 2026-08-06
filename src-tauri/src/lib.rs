//! 多协议流程执行引擎库
//!
//! 提供 Tauri 应用入口点和 IPC 命令。
//! 业务逻辑委托给 flow-engine crate。

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

// CLI 模块（从 mpe-cli crate 导入，供统一入口 main.rs 调用）
pub use mpe_cli::cli;

// Tauri 特有模块
mod plugin_protocol;
mod tauri_adapters;
mod tauri_commands;

// 导出 flow-engine 的所有公开项
pub use flow_engine::*;

// 额外导出 CLI 需要的具体类型（flow-engine 未 re-export 的模块内容）
pub use flow_engine::executor::{ExecutionResult, FlowExecutor};
pub use flow_engine::report::{FlowExecutionReport, NodeExecutionReport};
pub use flow_engine::storage::{
    delete_report, ensure_flows_dir, get_data_dir, get_flows_dir, get_reports_dir, list_reports,
    load_flow, load_report, save_flow, save_report,
};

// 导出 Tauri 命令供 main.rs 使用
pub use tauri_commands::{
    cancel_dataset_execution,
    // 执行会话管理
    cancel_flow,
    copy_flow_file,
    create_flow,
    create_folder,
    delete_dataset_report,
    delete_flow_file,
    delete_flow_report,
    delete_folder,
    delete_node,
    delete_stress_test_report,
    end_debug,
    execute_flow,
    execute_flow_with_dataset,
    execute_single_node,
    force_end_debug,
    get_app_data_dir,
    get_debug_status,
    get_node_config_schema,
    // 节点注册表命令
    get_registered_node_types,
    get_stress_test_status,
    list_dataset_reports,
    // 流程文件存储命令
    list_flow_files,
    list_flow_reports,
    // 文件夹管理命令
    list_folders,
    list_registered_plugins,
    list_stress_test_reports,
    load_dataset_report,
    load_dataset_row_detail,
    load_flow_file,
    load_flow_report,
    load_stress_test_report,
    move_flow_to_folder,
    move_folder,
    open_directory,
    rename_folder,
    save_csv_template,
    save_dataset_report,
    save_flow_file,
    save_flow_report,
    save_stress_test_report,
    send_debug_command,
    start_debug,
    // 压力测试命令
    start_stress_test,
    stop_stress_test,
    test_redis_connection,
    update_node,
    // 数据集执行命令
    validate_dataset_csv,
    validate_flow,
    DatasetExecutionSessionManager,
    // 会话管理
    DebugSessionManager,
    ExecutionSessionManager,
    GrpcCancelSessionManager,
};

// 数据库连接命令
#[cfg(feature = "protocol-db")]
pub use tauri_commands::test_db_connection;

// gRPC 命令
#[cfg(feature = "protocol-grpc")]
pub use tauri_commands::{
    grpc_channelz_info, grpc_discover_services, grpc_generate_request_skeleton,
    grpc_generate_request_skeleton_from_reflection, grpc_parse_descriptor_set,
    grpc_read_proto_files, grpc_scan_proto_directory, grpc_validate_request,
};

// MCP 工具发现命令
#[cfg(feature = "protocol-mcp")]
pub use tauri_commands::mcp_discover_tools;

// MQTT 连接测试命令
#[cfg(feature = "protocol-mqtt")]
pub use tauri_commands::mqtt_test_connection;

// SSE 连接测试命令
#[cfg(feature = "protocol-sse")]
pub use tauri_commands::sse_test_connection;

// AMQP 连接测试命令
#[cfg(feature = "protocol-amqp")]
pub use tauri_commands::amqp_test_connection;

// SMTP 连接测试 + Email OAuth2 命令
#[cfg(feature = "protocol-smtp")]
pub use tauri_commands::{
    email_oauth2_authorize, email_oauth2_refresh, email_oauth2_revoke, test_smtp_connection,
};

// IMAP 连接测试命令
#[cfg(feature = "protocol-imap")]
pub use tauri_commands::test_imap_connection;

// TCP 协议模板命令
#[cfg(feature = "protocol-tcp")]
pub use tauri_commands::{tcp_get_template, tcp_list_templates};

// GraphQL 内省命令
#[cfg(feature = "protocol-graphql")]
pub use tauri_commands::graphql_introspect_schema;

// Kafka 命令
#[cfg(feature = "protocol-kafka")]
pub use tauri_commands::{
    kafka_discover_schema, kafka_list_consumer_groups, kafka_list_topics, kafka_test_connection,
};

// 导出适配器（供测试使用）
pub use tauri_adapters::{
    create_log_sender, TauriAppDirProvider, TauriDebugEventSender, TauriDirectoryOpener,
    TauriUnderlyingLogSender,
};
/// 获取默认日志目录
fn default_logs_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|_| PathBuf::from("logs"))
}

/// 共享节点注册表状态（Tauri State）
///
/// 应用启动时构建一次，所有执行/验证/注册表查询命令共享同一个注册表，
/// 使插件节点（`SidecarPlugin`）的插件进程在多次执行之间复用。
pub struct NodeRegistryState {
    /// 共享节点注册表（内置节点 + 已登记的插件节点）
    pub registry: Arc<flow_engine::registry::NodeRegistry>,
    /// 插件管理器（持有插件进程生命周期；`plugin-manager` 特性关闭时不存在）
    #[cfg(feature = "plugin-manager")]
    pub manager: Arc<flow_engine::PluginManager>,
}

/// 解析插件目录：`MPE_PLUGIN_DIR` 环境变量优先，否则默认数据目录下 `plugins`
fn plugins_dir() -> PathBuf {
    std::env::var_os("MPE_PLUGIN_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            flow_engine::storage::get_data_dir()
                .ok()
                .map(|d| d.join("plugins"))
        })
        .unwrap_or_else(|| PathBuf::from("plugins"))
}

/// 构建共享节点注册表状态
///
/// 插件扫描与 describe 在启动时同步阻塞执行（describe 会拉起插件进程，
/// 但数量有限、耗时毫秒级，可接受）；扫描失败仅告警，不阻塞启动。
/// 关闭 `plugin-manager` 特性时仅构建内置节点注册表。
fn build_node_registry_state() -> NodeRegistryState {
    #[cfg(feature = "plugin-manager")]
    {
        let mut manager = flow_engine::PluginManager::new(plugins_dir());
        let _ = tauri::async_runtime::block_on(manager.scan_and_describe());
        let manager = Arc::new(manager);
        let registry = Arc::new(flow_engine::default_registry_with_plugins(&manager));
        NodeRegistryState { registry, manager }
    }
    #[cfg(not(feature = "plugin-manager"))]
    {
        NodeRegistryState {
            registry: Arc::new(flow_engine::default_registry::default_registry()),
        }
    }
}

/// 启动 Tauri 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 安装 rustls CryptoProvider（HTTP/gRPC TLS 需要）
    // rustls 0.23+ 要求显式选择加密后端
    let _ = rustls::crypto::ring::default_provider().install_default();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init());

    // 注册 `mpe-plugin` 自定义协议（PR-1）：为插件配置面板 iframe 提供前端 HTML。
    // 数据只来自内存 PluginManager 注册表缓存（D6），绝不触碰文件系统；
    // 每个响应带 D8 self-CSP 头。handler 在请求时经 state 取管理器，
    // 因此状态由 .setup 中的 `app.manage` 注入即可（setup 早于任何窗口导航）。
    // 注意：Tauri 2 的异步协议注册方法只在 `Builder` 上（App/AppHandle 没有），
    // 故在构建链上注册；handler 闭包按请求时态访问 state，无捕获、天然 'static。
    #[cfg(feature = "plugin-manager")]
    let builder = builder.register_asynchronous_uri_scheme_protocol(
        plugin_protocol::SCHEME,
        |ctx, request, responder| {
            let state = ctx.app_handle().state::<NodeRegistryState>();
            let manager = Arc::clone(&state.manager);
            let host = request.uri().host().unwrap_or_default().to_owned();
            let page = request.uri().path().trim_start_matches('/').to_owned();
            let response = plugin_protocol::plugin_iframe_response(
                |plugin_id| {
                    manager
                        .registrations()
                        .iter()
                        .find(|registration| registration.name == plugin_id)
                        .and_then(|registration| {
                            registration
                                .nodes
                                .iter()
                                .find_map(|node| node.frontend.as_ref())
                        })
                        .map(|frontend| (frontend.kind.as_str(), frontend.content.as_deref()))
                },
                &host,
                &page,
            );
            responder.respond(response);
        },
    );

    builder
        .setup(|app| {
            // 注册共享节点注册表状态（内置节点 + 插件节点，全局构建一次）
            let node_registry_state = build_node_registry_state();
            let stress_registry = Arc::clone(&node_registry_state.registry);
            app.manage(node_registry_state);

            // 注册调试会话管理器状态
            app.manage(Arc::new(Mutex::new(DebugSessionManager::default())));

            // 注册压力测试会话管理器状态（共享注册表，使插件进程跨虚拟用户复用）
            app.manage(Arc::new(
                flow_engine::stress_test::StressTestSessionManager::new()
                    .with_registry(stress_registry),
            ));

            // 注册执行会话管理器状态（用于取消流程执行）
            app.manage(ExecutionSessionManager::new());

            // 注册数据驱动执行会话管理器状态（用于取消数据驱动执行）
            app.manage(DatasetExecutionSessionManager::new());

            // 注册 gRPC 取消会话管理器状态（用于取消流式调用和 Channelz 内省）
            app.manage(GrpcCancelSessionManager::new());

            // 从 store 读取日志目录配置
            let log_dir = match app.store("store/settings.bin") {
                Ok(store) => store
                    .get("logDir")
                    .and_then(|v| v.as_str().map(PathBuf::from))
                    .unwrap_or_else(|| default_logs_dir(app.handle())),
                Err(_) => default_logs_dir(app.handle()),
            };

            // 确保目录存在
            let _ = std::fs::create_dir_all(&log_dir);

            // 设置日志级别（调试模式用 Debug，生产模式用 Info）
            let level = if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            };

            // 注册 log 插件（生产模式也启用）
            let _ = app.handle().plugin(
                tauri_plugin_log::Builder::new()
                    .level(level)
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Folder {
                            path: log_dir,
                            file_name: Some("app".into()),
                        },
                    ))
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .build(),
            );

            // 注册更新插件（仅桌面平台）
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let _ = app
                .handle()
                .plugin(tauri_plugin_updater::Builder::new().build());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_flow,
            execute_flow,
            execute_single_node,
            update_node,
            delete_node,
            save_flow_report,
            load_flow_report,
            list_flow_reports,
            delete_flow_report,
            get_app_data_dir,
            open_directory,
            start_debug,
            send_debug_command,
            end_debug,
            get_debug_status,
            force_end_debug,
            // 压力测试命令
            start_stress_test,
            stop_stress_test,
            get_stress_test_status,
            save_stress_test_report,
            load_stress_test_report,
            list_stress_test_reports,
            delete_stress_test_report,
            // 数据库连接命令
            #[cfg(feature = "protocol-db")]
            test_db_connection,
            // gRPC 服务发现命令
            #[cfg(feature = "protocol-grpc")]
            grpc_discover_services,
            // gRPC 请求骨架生成命令
            #[cfg(feature = "protocol-grpc")]
            grpc_generate_request_skeleton,
            // gRPC 请求骨架生成命令（Reflection 模式）
            #[cfg(feature = "protocol-grpc")]
            grpc_generate_request_skeleton_from_reflection,
            // gRPC 请求 JSON 预验证命令
            #[cfg(feature = "protocol-grpc")]
            grpc_validate_request,
            // gRPC Proto 目录扫描命令
            #[cfg(feature = "protocol-grpc")]
            grpc_scan_proto_directory,
            // gRPC 读取 Proto 文件命令
            #[cfg(feature = "protocol-grpc")]
            grpc_read_proto_files,
            // gRPC Descriptor Set 解析命令
            #[cfg(feature = "protocol-grpc")]
            grpc_parse_descriptor_set,
            // gRPC Channelz 内省命令（EXPERIMENTAL）
            #[cfg(feature = "protocol-grpc")]
            grpc_channelz_info,
            // MCP 工具发现命令
            #[cfg(feature = "protocol-mcp")]
            mcp_discover_tools,
            // MQTT 连接测试命令
            #[cfg(feature = "protocol-mqtt")]
            mqtt_test_connection,
            // SSE 连接测试命令
            #[cfg(feature = "protocol-sse")]
            sse_test_connection,
            // AMQP 连接测试命令
            #[cfg(feature = "protocol-amqp")]
            amqp_test_connection,
            // SMTP 连接测试命令
            #[cfg(feature = "protocol-smtp")]
            test_smtp_connection,
            // IMAP 连接测试命令
            #[cfg(feature = "protocol-imap")]
            test_imap_connection,
            // Email OAuth2 命令
            #[cfg(feature = "protocol-smtp")]
            email_oauth2_authorize,
            #[cfg(feature = "protocol-smtp")]
            email_oauth2_refresh,
            #[cfg(feature = "protocol-smtp")]
            email_oauth2_revoke,
            // TCP 协议模板命令
            #[cfg(feature = "protocol-tcp")]
            tcp_list_templates,
            #[cfg(feature = "protocol-tcp")]
            tcp_get_template,
            // GraphQL 内省命令
            #[cfg(feature = "protocol-graphql")]
            graphql_introspect_schema,
            // Kafka 命令
            #[cfg(feature = "protocol-kafka")]
            kafka_test_connection,
            #[cfg(feature = "protocol-kafka")]
            kafka_list_topics,
            #[cfg(feature = "protocol-kafka")]
            kafka_list_consumer_groups,
            #[cfg(feature = "protocol-kafka")]
            kafka_discover_schema,
            // 执行会话管理
            cancel_flow,
            test_redis_connection,
            // 流程验证命令
            validate_flow,
            // 流程文件存储命令
            list_flow_files,
            load_flow_file,
            save_flow_file,
            delete_flow_file,
            copy_flow_file,
            // 文件夹管理命令
            list_folders,
            create_folder,
            delete_folder,
            rename_folder,
            move_flow_to_folder,
            move_folder,
            // 节点注册表命令
            get_registered_node_types,
            get_node_config_schema,
            list_registered_plugins,
            // CSV 模板命令
            save_csv_template,
            // 数据集执行命令
            validate_dataset_csv,
            execute_flow_with_dataset,
            cancel_dataset_execution,
            save_dataset_report,
            list_dataset_reports,
            load_dataset_report,
            load_dataset_row_detail,
            delete_dataset_report,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| eprintln!("Tauri application error: {e}"));
}
