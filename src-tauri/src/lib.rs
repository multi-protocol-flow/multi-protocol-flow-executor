//! 多协议流程执行引擎库
//!
//! 提供流程编排、节点执行、变量替换等核心功能。

use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::StoreExt;


// 导入基础模块
mod log_event;

mod command;
mod commands;
mod conditional_executor;
mod connection;
mod context;
mod entry_executor;
mod executor;
mod flow;
mod http_executor;
mod node;
mod node_config;
mod error;
/// 执行报告模块
pub mod report;
/// 流程统计模块
pub mod statistics;
mod storage;
mod validator;

// 公开类型导出
pub use command::*;
pub use commands::{create_flow, delete_node, execute_flow, update_node};
pub use conditional_executor::ConditionalNodeExecutor;
pub use connection::Connection;
pub use context::{ExecutionContext, VariableStorage};
pub use entry_executor::EntryNodeExecutor;
pub use executor::{ExecutionResult, FlowExecutor, NodeExecutor, replace_variables};
pub use flow::Flow;
pub use http_executor::HttpNodeExecutor;
pub use node::{Node, NodePosition, NodeType, Port, PortType};
pub use node_config::*;
pub use report::*;
pub use statistics::FlowStatistics;
pub use validator::{FlowValidator, ValidationResult};
pub use error::*;
pub use log_event::{LogEvent, LogLevel};


// 导出存储模块
pub use storage::{ensure_flows_dir, get_data_dir, get_flows_dir, get_reports_dir, load_flow, save_flow, save_report, load_report, list_reports, delete_report};

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
            crate::commands::save_flow_report,
            crate::commands::load_flow_report,
            crate::commands::list_flow_reports,
            crate::commands::delete_flow_report,
            crate::commands::get_app_data_dir,
            crate::commands::open_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
