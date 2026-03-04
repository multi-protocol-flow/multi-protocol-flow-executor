//! Tauri 适配器模块
//!
//! 实现 flow-engine traits 的 Tauri 特定实现，用于连接 Tauri IPC 和业务逻辑层。

use async_trait::async_trait;
use tauri::ipc::Channel;
use tauri::Manager;
use flow_engine::traits::{LogSender, DirectoryOpener};
use flow_engine::log_event::LogEvent;

/// Tauri 日志发送器
///
/// 包装 Tauri IPC Channel，将日志事件发送到前端。
pub struct TauriLogSender(pub Channel<LogEvent>);
impl TauriLogSender {
    /// 创建新的 Tauri 日志发送器
    pub fn new(channel: Channel<LogEvent>) -> Self {
        Self(channel)
    }
}

#[async_trait]
impl LogSender for TauriLogSender {
    async fn send(&self, event: LogEvent) -> Result<(), String> {
        self.0.send(event).map_err(|e| e.to_string())
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
