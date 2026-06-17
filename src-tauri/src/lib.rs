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
mod tauri_adapters;
mod tauri_commands;

// 导出 flow-engine 的所有公开项
pub use flow_engine::*;

// 额外导出 CLI 需要的具体类型（flow-engine 未 re-export 的模块内容）
pub use flow_engine::executor::{ExecutionResult, FlowExecutor};
pub use flow_engine::report::{FlowExecutionReport, NodeExecutionReport};
pub use flow_engine::storage::{
    get_data_dir, load_flow, save_flow, save_report, ensure_flows_dir, get_flows_dir,
    get_reports_dir, load_report, list_reports, delete_report,
};

// 导出 Tauri 命令供 main.rs 使用
pub use tauri_commands::{
    test_redis_connection,
    create_flow,
    execute_flow,
    update_node,
    delete_node,
    save_flow_report,
    load_flow_report,
    list_flow_reports,
    delete_flow_report,
    get_app_data_dir,
    open_directory,
    save_csv_template,
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
    // 执行会话管理
    cancel_flow,
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
    // 数据集执行命令
    validate_dataset_csv,
    execute_flow_with_dataset,
    cancel_dataset_execution,
    save_dataset_report,
    list_dataset_reports,
    load_dataset_report,
    load_dataset_row_detail,
    delete_dataset_report,
    // 会话管理
    DebugSessionManager,
    ExecutionSessionManager,
    DatasetExecutionSessionManager,
    GrpcCancelSessionManager,
    validate_flow,
};

// 数据库连接命令
#[cfg(feature = "protocol-db")]
pub use tauri_commands::test_db_connection;

// gRPC 命令
#[cfg(feature = "protocol-grpc")]
pub use tauri_commands::{
    grpc_discover_services,
    grpc_generate_request_skeleton,
    grpc_generate_request_skeleton_from_reflection,
    grpc_validate_request,
    grpc_scan_proto_directory,
    grpc_read_proto_files,
    grpc_parse_descriptor_set,
    grpc_channelz_info,
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
    test_smtp_connection,
    email_oauth2_authorize,
    email_oauth2_refresh,
    email_oauth2_revoke,
};

// IMAP 连接测试命令
#[cfg(feature = "protocol-imap")]
pub use tauri_commands::test_imap_connection;

// TCP 协议模板命令
#[cfg(feature = "protocol-tcp")]
pub use tauri_commands::{
    tcp_list_templates,
    tcp_get_template,
};

// GraphQL 内省命令
#[cfg(feature = "protocol-graphql")]
pub use tauri_commands::graphql_introspect_schema;

// Kafka 命令
#[cfg(feature = "protocol-kafka")]
pub use tauri_commands::{
    kafka_test_connection,
    kafka_list_topics,
    kafka_list_consumer_groups,
    kafka_discover_schema,
};

// 导出适配器（供测试使用）
pub use tauri_adapters::{TauriUnderlyingLogSender, TauriDirectoryOpener, TauriAppDirProvider, TauriDebugEventSender, create_log_sender};
/// 获取默认日志目录
fn default_logs_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|_| PathBuf::from("logs"))
}

/// 启动 Tauri 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 安装 rustls CryptoProvider（HTTP/gRPC TLS 需要）
    // rustls 0.23+ 要求显式选择加密后端
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 注册调试会话管理器状态
            app.manage(Arc::new(Mutex::new(DebugSessionManager::default())));
            
            // 注册压力测试会话管理器状态
            app.manage(Arc::new(flow_engine::stress_test::StressTestSessionManager::new()));
            
            // 注册执行会话管理器状态（用于取消流程执行）
            app.manage(ExecutionSessionManager::new());
            
            // 注册数据驱动执行会话管理器状态（用于取消数据驱动执行）
            app.manage(DatasetExecutionSessionManager::new());
            
            // 注册 gRPC 取消会话管理器状态（用于取消流式调用和 Channelz 内省）
            app.manage(GrpcCancelSessionManager::new());
            
            // 从 store 读取日志目录配置
            let log_dir = match app.store("store/settings.bin") {
                Ok(store) => {
                    store.get("logDir")
                        .and_then(|v| v.as_str().map(PathBuf::from))
                        .unwrap_or_else(|| default_logs_dir(app.handle()))
                }
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
                            file_name: Some("app".into()) 
                        }
                    ))
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .build()
            );
            
            // 注册更新插件（仅桌面平台）
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let _ = app.handle().plugin(
                tauri_plugin_updater::Builder::new().build()
            );
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_flow,
            execute_flow,
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
        .expect("error while running tauri application");
}
