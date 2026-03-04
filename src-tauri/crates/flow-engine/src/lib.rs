//! Flow Engine - 流程执行引擎核心库
//!
//! 独立于 Tauri 的业务逻辑层，可在 CLI 和测试环境中直接使用。

pub mod commands;
pub mod traits;

// Wave 2: 无依赖的基础模块
pub mod command;
pub mod connection;
pub mod error;
pub mod flow;
pub mod log_event;
pub mod node;
pub mod node_config;
pub mod report;
pub mod statistics;
pub mod storage;
pub mod variable;

// Wave 3: 执行器模块（需要修改 Tauri 依赖）
pub mod context;
pub mod entry_executor;
pub mod executor;
pub mod http_executor;
pub mod conditional_executor;
pub mod validator;

// Re-exports
pub use traits::{
    AppDirProvider, ConsoleLogSender, DefaultAppDirProvider, DirectoryOpener, LogSender,
    NoopDirectoryOpener, NoopLogSender,
};
