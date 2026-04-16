//! 多协议流程执行引擎库
//!
//! 提供 Tauri 应用入口点和 IPC 命令。
//! 业务逻辑委托给 flow-engine crate。

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

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
    test_db_connection,
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
    DebugSessionManager,
    ExecutionSessionManager,
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // 注册调试会话管理器状态
            app.manage(Arc::new(Mutex::new(DebugSessionManager::default())));
            
            // 注册压力测试会话管理器状态
            app.manage(Arc::new(flow_engine::stress_test::StressTestSessionManager::new()));
            
            // 注册执行会话管理器状态（用于取消流程执行）
            app.manage(ExecutionSessionManager::new());
            
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
    test_db_connection,
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
    move_flow_to_folder
])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
