//! 调试模式模块
//!
//! 提供流程调试功能，包括断点管理、单步执行、变量查看等

mod executor;
mod types;

// Re-export 主要类型
pub use types::{
    DebugCallFrame, DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason,
    ExecutionStackEntry, PauseReason,
};

// Re-export 执行器
pub use executor::DebuggableFlowExecutor;
