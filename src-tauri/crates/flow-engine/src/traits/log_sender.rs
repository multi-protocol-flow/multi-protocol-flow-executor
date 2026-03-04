//! 日志发送器抽象
//!
//! 用于将执行日志发送到前端（Tauri）或控制台（CLI）。

use async_trait::async_trait;
use crate::log_event::LogEvent;

/// 日志发送器抽象
#[async_trait]
pub trait LogSender: Send + Sync {
    /// 发送日志事件
    async fn send(&self, event: LogEvent) -> Result<(), String>;
}

/// 空实现 - CLI/测试用
pub struct NoopLogSender;

#[async_trait]
impl LogSender for NoopLogSender {
    async fn send(&self, _event: LogEvent) -> Result<(), String> {
        Ok(())
    }
}

/// 控制台实现 - 调试用
pub struct ConsoleLogSender;

#[async_trait]
impl LogSender for ConsoleLogSender {
    async fn send(&self, event: LogEvent) -> Result<(), String> {
        println!("[{:?}] {}", event.level, event.message);
        Ok(())
    }
}
