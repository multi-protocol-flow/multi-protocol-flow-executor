// 导入基础模块
mod command;
mod commands;
mod connection;
mod executor;
mod flow;
mod node;
mod node_config;
mod start_executor;
mod storage;
mod validator;

// 公开类型导出
pub use command::*;
pub use commands::{create_flow, delete_node, execute_flow, update_node};
pub use connection::Connection;
pub use executor::{ExecutionContext, ExecutionResult, FlowExecutor, NodeExecutor};
pub use flow::Flow;
pub use node::{Node, NodePosition, NodeType, Port, PortType};
pub use node_config::*;
pub use start_executor::StartNodeExecutor;
pub use validator::{FlowValidator, ValidationResult};

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
