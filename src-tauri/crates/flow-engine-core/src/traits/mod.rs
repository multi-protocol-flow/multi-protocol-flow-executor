//! 抽象接口模块
//!
//! 提供与 Tauri 解耦的 trait 定义，使得业务逻辑可以在无 Tauri 依赖的环境下运行。

mod app_dir;
mod directory_opener;
mod log_sender;

pub use app_dir::{AppDirProvider, DefaultAppDirProvider};
pub use directory_opener::{DirectoryOpener, NoopDirectoryOpener};
pub use log_sender::{ConsoleLogSender, LogSender, LogWorker, NoopLogSender, QueuedLogSender, UnderlyingLogSender};
