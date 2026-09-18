//! Tauri 适配器模块
//!
//! 实现 flow-engine traits 的 Tauri 特定实现，用于连接 Tauri IPC 和业务逻辑层。

use async_trait::async_trait;
use flow_engine::debug::DebugEvent;
use flow_engine::execution_event::ExecutionEvent;
use flow_engine::log_event::LogEvent;
use flow_engine::traits::{
    DebugEventSender, DirectoryOpener, ExecutionEventSender, LogWorker as FlowLogWorker,
    QueuedLogSender, UnderlyingLogSender,
};
use tauri::ipc::Channel;
use tauri::Manager;

/// Tauri 底层日志发送器
///
/// 实现 `UnderlyingLogSender` trait，将日志同步发送到 Tauri IPC Channel。
pub struct TauriUnderlyingLogSender(Channel<LogEvent>);

impl TauriUnderlyingLogSender {
    /// 创建新的 Tauri 底层日志发送器
    pub fn new(channel: Channel<LogEvent>) -> Self {
        Self(channel)
    }
}

impl UnderlyingLogSender for TauriUnderlyingLogSender {
    /// 同步发送日志到 Tauri IPC Channel
    fn send_sync(&self, event: LogEvent) -> Result<(), String> {
        self.0.send(event).map_err(|e| e.to_string())
    }
}

/// 创建队列日志发送器
///
/// 根据编译模式自动选择：
/// - **调试模式**: 同步发送日志，确保日志顺序和完整性，便于调试
/// - **发布模式**: 队列异步发送日志，不阻塞执行流程，提升性能
///
/// # 参数
/// - `channel`: Tauri IPC Channel
///
/// # 返回
/// - `(sender, worker)`: sender 实现了 `LogSender` trait，worker 需要在 tokio 中 spawn
pub fn create_log_sender(
    channel: Channel<LogEvent>,
) -> (
    std::sync::Arc<dyn flow_engine::traits::LogSender>,
    FlowLogWorker,
) {
    let underlying = std::sync::Arc::new(TauriUnderlyingLogSender::new(channel));
    let (sender, worker) = QueuedLogSender::new(underlying);
    (std::sync::Arc::new(sender), worker)
}

/// 运行期节点完整执行详情缓存（冷通道内存存储）
#[derive(Default, Clone)]
pub struct ExecutionDetailCache {
    /// 结构: flow_uuid -> (node_uuid -> NodeExecutionReport)
    details: std::sync::Arc<
        std::sync::RwLock<
            std::collections::HashMap<
                String,
                std::collections::HashMap<String, flow_engine::report::NodeExecutionReport>,
            >,
        >,
    >,
    /// 流程执行淘汰队列（按插入顺序保留最近 N 个 flow_uuid，默认 10）
    history: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>,
}

impl ExecutionDetailCache {
    /// 创建执行详情缓存
    pub fn new() -> Self {
        Self::default()
    }

    /// 存储节点的完整执行报告
    pub fn store_node_detail(
        &self,
        flow_uuid: &str,
        node_uuid: &str,
        report: flow_engine::report::NodeExecutionReport,
    ) {
        let mut lock = self.details.write().unwrap_or_else(|e| e.into_inner());
        let is_new_flow = !lock.contains_key(flow_uuid);
        if is_new_flow {
            let mut hist = self.history.lock().unwrap_or_else(|e| e.into_inner());
            if hist.len() >= 10 {
                if let Some(old_flow) = hist.pop_front() {
                    lock.remove(&old_flow);
                }
            }
            hist.push_back(flow_uuid.to_string());
        }
        let node_map = lock.entry(flow_uuid.to_string()).or_default();
        node_map.insert(node_uuid.to_string(), report);
    }

    /// 查询节点的完整执行报告
    pub fn get_node_detail(
        &self,
        flow_uuid: &str,
        node_uuid: &str,
    ) -> Option<flow_engine::report::NodeExecutionReport> {
        let lock = self.details.read().unwrap_or_else(|e| e.into_inner());
        lock.get(flow_uuid)
            .and_then(|map| map.get(node_uuid).cloned())
    }

    /// 清理特定流程的缓存
    pub fn clear_flow(&self, flow_uuid: &str) {
        let mut lock = self.details.write().unwrap_or_else(|e| e.into_inner());
        lock.remove(flow_uuid);
        let mut hist = self.history.lock().unwrap_or_else(|e| e.into_inner());
        hist.retain(|id| id != flow_uuid);
    }
}

/// 队列化执行事件发送器（热通道轻量推送 + 冷通道落盘分离）
pub struct QueuedExecutionEventSender {
    tx: tokio::sync::mpsc::UnboundedSender<ExecutionEvent>,
    flow_uuid: String,
    detail_cache: ExecutionDetailCache,
}

impl ExecutionEventSender for QueuedExecutionEventSender {
    fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
        match event {
            ExecutionEvent::NodeCompleted {
                report,
                parent_node_uuid,
            } => {
                // 1. 完整数据写入冷通道内存缓存
                self.detail_cache.store_node_detail(
                    &self.flow_uuid,
                    &report.node_uuid,
                    (*report).clone(),
                );

                // 2. 构造轻量级热通道事件（脱敏大字段）
                let mut light_report = *report;
                let has_heavy_payload = light_report.input_data.is_some()
                    || light_report.output_data.is_some()
                    || light_report.resolved_config.is_some()
                    || light_report.plugin_data.is_some()
                    || light_report.sub_flow_report.is_some();

                if has_heavy_payload {
                    light_report.input_data = None;
                    light_report.output_data = None;
                    light_report.resolved_config = None;
                    light_report.plugin_data = None;
                    light_report.sub_flow_report = None;
                    light_report.is_detail_stripped = Some(true);
                }

                let stripped_event = ExecutionEvent::NodeCompleted {
                    report: Box::new(light_report),
                    parent_node_uuid,
                };

                self.tx
                    .send(stripped_event)
                    .map_err(|e| format!("Failed to queue node event: {}", e))
            }
            ExecutionEvent::FlowCompleted {
                report,
                parent_node_uuid,
            } => {
                // 构造轻量级完成事件：剥离各节点的重数据，大报文已由冷通道/落盘兜底
                let mut light_report = *report;
                for nr in &mut light_report.node_reports {
                    if nr.input_data.is_some()
                        || nr.output_data.is_some()
                        || nr.resolved_config.is_some()
                        || nr.plugin_data.is_some()
                        || nr.sub_flow_report.is_some()
                    {
                        nr.input_data = None;
                        nr.output_data = None;
                        nr.resolved_config = None;
                        nr.plugin_data = None;
                        nr.sub_flow_report = None;
                        nr.is_detail_stripped = Some(true);
                    }
                }
                light_report.flow_snapshot = None;
                let stripped_event = ExecutionEvent::FlowCompleted {
                    report: Box::new(light_report),
                    parent_node_uuid,
                };
                self.tx
                    .send(stripped_event)
                    .map_err(|e| format!("Failed to queue flow completed event: {}", e))
            }
            other => self
                .tx
                .send(other)
                .map_err(|e| format!("Failed to queue event: {}", e)),
        }
    }
}

/// 执行事件批量异步推送 Worker（20ms 双阈值聚合）
pub struct ExecutionEventWorker {
    receiver: tokio::sync::mpsc::UnboundedReceiver<ExecutionEvent>,
    channel: Channel<ExecutionEvent>,
}

impl ExecutionEventWorker {
    /// 启动 worker 循环消费
    pub async fn run(mut self) {
        use tokio::time::{interval, Duration};
        const FLUSH_INTERVAL_MS: u64 = 20; // 50 FPS
        const MAX_BATCH_SIZE: usize = 50;

        let mut buffer: Vec<ExecutionEvent> = Vec::with_capacity(MAX_BATCH_SIZE);
        let mut ticker = interval(Duration::from_millis(FLUSH_INTERVAL_MS));

        loop {
            tokio::select! {
                biased;

                Some(event) = self.receiver.recv() => {
                    let is_terminal = matches!(
                        &event,
                        ExecutionEvent::FlowCompleted { .. } | ExecutionEvent::FlowCancelled { .. }
                    );

                    buffer.push(event);

                    if is_terminal || buffer.len() >= MAX_BATCH_SIZE {
                        Self::flush(&mut buffer, &self.channel);
                    }
                }

                _ = ticker.tick() => {
                    if !buffer.is_empty() {
                        Self::flush(&mut buffer, &self.channel);
                    }
                }

                else => {
                    if !buffer.is_empty() {
                        Self::flush(&mut buffer, &self.channel);
                    }
                    break;
                }
            }
        }
    }

    fn flush(buffer: &mut Vec<ExecutionEvent>, channel: &Channel<ExecutionEvent>) {
        if buffer.is_empty() {
            return;
        }
        if buffer.len() == 1 {
            let event = buffer.remove(0);
            let _ = channel.send(event);
        } else {
            let events = std::mem::take(buffer);
            let _ = channel.send(ExecutionEvent::Batch(events));
        }
    }
}

/// 创建冷热双通道执行事件发送器与批处理 Worker
pub fn create_execution_event_sender(
    channel: Channel<ExecutionEvent>,
    flow_uuid: String,
    detail_cache: ExecutionDetailCache,
) -> (
    std::sync::Arc<dyn ExecutionEventSender>,
    ExecutionEventWorker,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let sender = QueuedExecutionEventSender {
        tx,
        flow_uuid,
        detail_cache,
    };
    let worker = ExecutionEventWorker {
        receiver: rx,
        channel,
    };
    (std::sync::Arc::new(sender), worker)
}

/// Tauri 调试事件发送器（用于普通执行模式的 WS 流事件）
///
/// 实现 `DebugEventSender` trait，将调试事件同步发送到 Tauri IPC Channel。
pub struct TauriDebugEventSender(Channel<DebugEvent>);

impl TauriDebugEventSender {
    /// 创建新的 Tauri 调试事件发送器
    pub fn new(channel: Channel<DebugEvent>) -> Self {
        Self(channel)
    }
}

impl DebugEventSender for TauriDebugEventSender {
    /// 同步发送调试事件到 Tauri IPC Channel
    fn send_event(&self, event: DebugEvent) -> Result<(), String> {
        self.0.send(event).map_err(|e| format!("{:?}", e))
    }
}

/// Tauri 目录打开器
///
/// 使用 Tauri 的 opener 插件在系统文件管理器中打开目录。
pub struct TauriDirectoryOpener {
    app: tauri::AppHandle,
}

impl TauriDirectoryOpener {
    /// 创建新的 Tauri 目录打开器
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl DirectoryOpener for TauriDirectoryOpener {
    async fn open(&self, path: &str) -> Result<(), String> {
        use tauri_plugin_opener::OpenerExt;
        self.app
            .opener()
            .open_path(path, None::<String>)
            .map_err(|e| e.to_string())
    }
}

/// Tauri 应用目录提供者
///
/// 使用 Tauri 的 path API 获取应用数据目录。
pub struct TauriAppDirProvider {
    app: tauri::AppHandle,
}

impl TauriAppDirProvider {
    /// 创建新的 Tauri 应用目录提供者
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl flow_engine::AppDirProvider for TauriAppDirProvider {
    fn app_data_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine::execution_event::ExecutionEvent;
    use flow_engine::report::{ExecutionStatus, NodeExecutionReport};
    use serde_json::json;

    #[test]
    fn test_execution_detail_cache_store_and_evict() {
        let cache = ExecutionDetailCache::new();

        for i in 0..12 {
            let flow_id = format!("flow-{}", i);
            let mut report = NodeExecutionReport::default();
            report.node_uuid = format!("node-{}", i);
            report.output_data = Some(json!({"val": i}));
            let node_uuid = report.node_uuid.clone();
            cache.store_node_detail(&flow_id, &node_uuid, report);
        }

        // 最早的 flow-0 和 flow-1 应该被淘汰（最多保留 10 个）
        assert!(cache.get_node_detail("flow-0", "node-0").is_none());
        assert!(cache.get_node_detail("flow-1", "node-1").is_none());

        // 后续的应该存在
        let node11 = cache
            .get_node_detail("flow-11", "node-11")
            .expect("should exist");
        assert_eq!(node11.output_data, Some(json!({"val": 11})));
    }

    #[test]
    fn test_queued_event_sender_strips_payload() {
        let cache = ExecutionDetailCache::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = QueuedExecutionEventSender {
            tx,
            flow_uuid: "flow-test".to_string(),
            detail_cache: cache.clone(),
        };

        let mut full_report = NodeExecutionReport::default();
        full_report.node_uuid = "node-heavy".to_string();
        full_report.input_data = Some(json!({"huge": "input"}));
        full_report.output_data = Some(json!({"huge": "output"}));
        full_report.resolved_config = Some(json!({"url": "https://example.com"}));

        let event = ExecutionEvent::NodeCompleted {
            report: Box::new(full_report),
            parent_node_uuid: None,
        };

        sender.send_event(event).expect("send succeeded");

        // 1. 验证冷通道缓存了完整数据
        let cached = cache
            .get_node_detail("flow-test", "node-heavy")
            .expect("cache exists");
        assert!(cached.input_data.is_some());
        assert!(cached.output_data.is_some());
        assert!(cached.resolved_config.is_some());
        assert_eq!(cached.is_detail_stripped, None);

        // 2. 验证热通道推送到队列的是脱敏轻量数据
        let queued_event = rx.try_recv().expect("event received");
        match queued_event {
            ExecutionEvent::NodeCompleted { report, .. } => {
                assert_eq!(report.input_data, None);
                assert_eq!(report.output_data, None);
                assert_eq!(report.resolved_config, None);
                assert_eq!(report.is_detail_stripped, Some(true));
            }
            _ => panic!("Expected NodeCompleted"),
        }
    }
}
