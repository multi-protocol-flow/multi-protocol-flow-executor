//! 日志发送器抽象
//!
//! 用于将执行日志发送到前端（Tauri）或控制台（CLI）。
//!
//! # 实现策略
//!
//! - **调试模式 (debug)**: 同步发送日志，确保日志顺序和完整性，便于调试
//! - **发布模式 (release)**: 队列异步发送，不阻塞执行流程，提升性能

use crate::log_event::LogEvent;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

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

// ============================================================================
// 队列日志发送器 - 发布模式使用
// ============================================================================

/// 底层日志发送器（由 Tauri 适配器实现）
pub trait UnderlyingLogSender: Send + Sync {
    /// 同步发送日志到目标（Tauri Channel）
    fn send_sync(&self, event: LogEvent) -> Result<(), String>;
}

/// Worker Future 类型（用于统一调试/发布模式的返回类型）
pub enum LogWorker {
    /// 调试模式：空 worker
    Debug,
    /// 发布模式：队列消费 worker
    Release(Pin<Box<dyn Future<Output = ()> + Send>>),
}

impl Future for LogWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut *self {
            LogWorker::Debug => Poll::Ready(()),
            LogWorker::Release(fut) => fut.as_mut().poll(cx),
        }
    }
}

/// 队列日志发送器
///
/// 在发布模式下使用 mpsc 队列异步发送日志，不阻塞执行流程。
/// 在调试模式下直接同步发送，确保日志顺序和完整性。
///
/// # 架构
///
/// ```text
/// 发布模式:
/// ┌──────────────┐      ┌─────────────────┐      ┌──────────────┐
/// │   Executor   │ ───► │  mpsc::Sender   │ ───► │  Background  │ ───► Frontend
/// │  (生产者)     │ push │   (无界队列)     │ pull │   Task       │  (Tauri IPC)
/// └──────────────┘      └─────────────────┘      └──────────────┘
///       ↓                        ↓
///    emit_log()           tokio::spawn(循环消费)
///    立即返回 ~1µs
///
/// 调试模式:
/// ┌──────────────┐      ┌──────────────┐
/// │   Executor   │ ───► │   Frontend   │
/// │              │ sync │  (直接发送)   │
/// └──────────────┘      └──────────────┘
/// ```
pub struct QueuedLogSender {
    /// 队列发送端（发布模式使用）
    sender: Option<mpsc::UnboundedSender<LogEvent>>,
    /// 底层发送器（调试模式直接使用）
    underlying: Arc<dyn UnderlyingLogSender>,
    /// 是否为调试模式
    is_debug: bool,
}

impl QueuedLogSender {
    /// 创建队列日志发送器
    ///
    /// # 参数
    /// - `underlying`: 底层日志发送器（Tauri Channel 包装）
    ///
    /// # 返回
    /// - `(sender, worker)`: sender 实现了 `LogSender` trait，worker 需要在 tokio 中 spawn
    ///
    /// # 模式切换
    /// - 调试模式: 直接同步发送日志，worker 为空
    /// - 发布模式: 队列异步发送日志，需要 spawn worker
    pub fn new(underlying: Arc<dyn UnderlyingLogSender>) -> (Self, LogWorker) {
        let is_debug = cfg!(debug_assertions);

        if is_debug {
            // 调试模式：直接返回，不使用队列，同步发送确保顺序
            let sender = Self {
                sender: None,
                underlying,
                is_debug: true,
            };
            (sender, LogWorker::Debug)
        } else {
            // 发布模式：创建队列，异步发送提升性能
            let (tx, mut rx) = mpsc::unbounded_channel::<LogEvent>();
            let underlying_clone = underlying.clone();

            // 后台消费任务
            let worker = LogWorker::Release(Box::pin(async move {
                while let Some(event) = rx.recv().await {
                    // 按顺序发送，Failed不重试（避免阻塞）
                    let _ = underlying_clone.send_sync(event);
                }
            }));

            let sender = Self {
                sender: Some(tx),
                underlying,
                is_debug: false,
            };
            (sender, worker)
        }
    }
}

#[async_trait]
impl LogSender for QueuedLogSender {
    async fn send(&self, event: LogEvent) -> Result<(), String> {
        if self.is_debug {
            // 调试模式：直接同步发送，确保日志顺序和完整性
            self.underlying.send_sync(event)
        } else {
            // 发布模式：推入队列（几乎立即返回 ~1µs）
            self.sender
                .as_ref()
                .ok_or("queue sender not initialized")?
                .send(event)
                .map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// 测试用的底层发送器
    struct TestUnderlyingSender {
        count: AtomicUsize,
        events: Mutex<Vec<LogEvent>>,
    }

    impl TestUnderlyingSender {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
                events: Mutex::new(Vec::new()),
            }
        }

        fn get_count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }

        fn get_events(&self) -> Vec<LogEvent> {
            self.events.lock().unwrap_or_else(|e| panic!("{e}")).clone()
        }
    }

    impl UnderlyingLogSender for TestUnderlyingSender {
        fn send_sync(&self, event: LogEvent) -> Result<(), String> {
            self.count.fetch_add(1, Ordering::SeqCst);
            self.events
                .lock()
                .unwrap_or_else(|e| panic!("{e}"))
                .push(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_noop_log_sender() {
        let sender = NoopLogSender;
        let event = LogEvent::default();
        let result = sender.send(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_console_log_sender() {
        let sender = ConsoleLogSender;
        let event = LogEvent {
            level: crate::log_event::LogLevel::Info,
            message: "test message".to_string(),
            timestamp: 0,
            node_uuid: None,
            node_name: None,
            flow_uuid: "test".to_string(),
        };
        let result = sender.send(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queued_log_sender_debug_mode() {
        // 调试模式测试
        let test_sender = Arc::new(TestUnderlyingSender::new());
        let (sender, _worker) = QueuedLogSender::new(test_sender.clone());

        // 发送日志
        let event = LogEvent {
            level: crate::log_event::LogLevel::Info,
            message: "test".to_string(),
            timestamp: 0,
            node_uuid: None,
            node_name: None,
            flow_uuid: "test".to_string(),
        };
        sender.send(event).await.unwrap_or_else(|e| panic!("{e}"));

        // 验证日志已发送
        assert_eq!(test_sender.get_count(), 1);
    }

    #[tokio::test]
    async fn test_queued_log_sender_release_mode() {
        // 模拟发布模式行为（通过直接创建队列）
        let test_sender = Arc::new(TestUnderlyingSender::new());

        // 手动创建队列测试
        let (tx, mut rx) = mpsc::unbounded_channel::<LogEvent>();
        let test_clone = test_sender.clone();

        // 启动消费任务
        let handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(event) = rx.recv().await {
                let _ = test_clone.send_sync(event);
                count += 1;
                if count >= 3 {
                    break;
                }
            }
        });

        // 发送日志
        for i in 0..3 {
            let event = LogEvent {
                level: crate::log_event::LogLevel::Info,
                message: format!("message {}", i),
                timestamp: i,
                node_uuid: None,
                node_name: None,
                flow_uuid: "test".to_string(),
            };
            tx.send(event).unwrap_or_else(|e| panic!("{e}"));
        }

        // 等待消费完成
        handle.await.unwrap_or_else(|e| panic!("{e}"));

        // 验证所有日志都已发送
        assert_eq!(test_sender.get_count(), 3);

        // 验证顺序
        let events = test_sender.get_events();
        assert_eq!(events.len(), 3);
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.timestamp, i as u64);
        }
    }
}
