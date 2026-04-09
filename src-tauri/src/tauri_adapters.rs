//! Tauri 适配器模块
//!
//! 实现 flow-engine traits 的 Tauri 特定实现，用于连接 Tauri IPC 和业务逻辑层。

use async_trait::async_trait;
use tauri::ipc::Channel;
use tauri::Manager;
use flow_engine::traits::{UnderlyingLogSender, DirectoryOpener, QueuedLogSender, LogWorker as FlowLogWorker, ExecutionEventSender};
use flow_engine::log_event::LogEvent;
use flow_engine::execution_event::ExecutionEvent;

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
) -> (std::sync::Arc<dyn flow_engine::traits::LogSender>, FlowLogWorker) {
    let underlying = std::sync::Arc::new(TauriUnderlyingLogSender::new(channel));
    let (sender, worker) = QueuedLogSender::new(underlying);
    ( std::sync::Arc::new(sender), worker)
}

/// Tauri 执行事件发送器
///
/// 实现 `ExecutionEventSender` trait，将执行事件同步发送到 Tauri IPC Channel。
pub struct TauriExecutionEventSender(Channel<ExecutionEvent>);

impl TauriExecutionEventSender {
    /// 创建新的 Tauri 执行事件发送器
    pub fn new(channel: Channel<ExecutionEvent>) -> Self {
        Self(channel)
    }
}

impl ExecutionEventSender for TauriExecutionEventSender {
    /// 同步发送执行事件到 Tauri IPC Channel
    fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
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
        self.app.opener()
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
        self.app.path()
            .app_data_dir()
            .map_err(|e| format!("获取应用数据目录失败: {}", e))
    }
}
