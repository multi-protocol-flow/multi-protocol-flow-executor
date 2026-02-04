// 导入基础模块
mod command;
mod connection;
mod flow;
mod node;

// 公开类型导出
pub use command::*;
pub use connection::Connection;
pub use flow::Flow;
pub use node::{Node, NodePosition, NodeType, Port, PortType};

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
