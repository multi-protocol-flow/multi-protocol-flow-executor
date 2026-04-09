//! 正常执行事件发送器抽象 trait

use crate::execution_event::ExecutionEvent;

/// 执行事件发送器 trait
///
/// 抽象执行事件的发送机制，支持不同的传输方式（如 Tauri channel、mpsc 等）。
/// 参考 `DebugEventSender` 的设计模式。
pub trait ExecutionEventSender: Send + Sync {
    /// 发送执行事件
    fn send_event(&self, event: ExecutionEvent) -> Result<(), String>;
}

/// Noop 实现，用于 CLI 模式（不需要发送事件）
pub struct NoopExecutionEventSender;

impl ExecutionEventSender for NoopExecutionEventSender {
    fn send_event(&self, _event: ExecutionEvent) -> Result<(), String> {
        Ok(())
    }
}
