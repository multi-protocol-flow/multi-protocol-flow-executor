//! mpsc UnboundedSender adapter for DebugEventSender trait

use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::traits::DebugEventSender;
use tokio::sync::mpsc;

/// Wrapper around mpsc::UnboundedSender that implements DebugEventSender
pub struct MpscDebugEventSender {
    sender: mpsc::UnboundedSender<DebugEvent>,
}

impl MpscDebugEventSender {
    /// 创建新的 MpscDebugEventSender
    pub fn new(sender: mpsc::UnboundedSender<DebugEvent>) -> Self {
        Self { sender }
    }
}

impl DebugEventSender for MpscDebugEventSender {
    fn send_event(&self, event: DebugEvent) -> Result<(), String> {
        self.sender
            .send(event)
            .map_err(|e| format!("Failed to send debug event: {}", e))
    }
}
