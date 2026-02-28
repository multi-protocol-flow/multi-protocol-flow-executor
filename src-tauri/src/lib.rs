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
pub mod report;
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
pub use storage::{ensure_flows_dir, get_data_dir, get_flows_dir, load_flow, save_flow};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_flow,
            execute_flow,
            update_node,
            delete_node
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
