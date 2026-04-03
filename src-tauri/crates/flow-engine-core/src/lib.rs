//! Flow Engine Core - 核心类型、trait 和共享工具
//!
//! 纯数据类型和抽象接口，无外部协议依赖（无 HTTP/WS/DB 客户端）。

pub mod connection;
pub mod debug_types;
pub mod error;
pub mod flow;
pub mod log_event;
pub mod node;
pub mod node_config;
pub mod report;
pub mod statistics;
pub mod traits;
pub mod utils;
pub mod variable;
pub mod ws_stream;

// Re-export key trait types
pub use traits::{
    AppDirProvider, ConsoleLogSender, DatabasePool, DbProvider, DebugEventSender,
    DefaultAppDirProvider, DirectoryOpener, IsolationLevel, LogSender, LogWorker,
    NoopDirectoryOpener, NoopLogSender, QueuedLogSender, QueryResult, TransactionState,
    UnderlyingLogSender, WsProvider,
};

// Re-export debug types
pub use debug_types::{
    DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason, ExecutionStackEntry,
    PauseReason,
};

// Re-export ws_stream types
pub use ws_stream::{
    MessageContent, MessageDirection, WsConnectionStream, WsMessage, WsStreamSegment,
};
