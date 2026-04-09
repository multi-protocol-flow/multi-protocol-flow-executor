//! 调试事件发送器抽象 trait

use crate::debug_types::DebugEvent;

/// 调试事件发送器 trait
///
/// 抽象调试事件的发送机制，支持不同的传输方式（如 Tauri channel、mpsc 等）。
/// 注意：此 trait 是同步的，因为 `mpsc::UnboundedSender::send()` 是同步操作。
pub trait DebugEventSender: Send + Sync {
    /// 发送调试事件
    fn send_event(&self, event: DebugEvent) -> Result<(), String>;
}
