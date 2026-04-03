//! 抽象接口模块
//!
//! 提供与 Tauri 解耦的 trait 定义，使得业务逻辑可以在无 Tauri 依赖的环境下运行。

mod app_dir;
mod db_provider;
mod debug_event_sender;
mod directory_opener;
mod log_sender;
mod ws_provider;

pub use app_dir::{AppDirProvider, DefaultAppDirProvider};
pub use db_provider::{
    DatabasePool, DbProvider, IsolationLevel, QueryResult, TransactionState,
};
pub use debug_event_sender::DebugEventSender;
pub use directory_opener::{DirectoryOpener, NoopDirectoryOpener};
pub use log_sender::{
    ConsoleLogSender, LogSender, LogWorker, NoopLogSender, QueuedLogSender, UnderlyingLogSender,
};
pub use ws_provider::WsProvider;
