//! 目录打开器抽象
//!
//! 用于在文件管理器中打开目录（Tauri 桌面应用）或空操作（CLI/测试）。

use async_trait::async_trait;

/// 目录打开器抽象
#[async_trait]
pub trait DirectoryOpener: Send + Sync {
    /// 打开目录
    async fn open(&self, path: &str) -> Result<(), String>;
}

/// 空实现 - 测试用
pub struct NoopDirectoryOpener;

#[async_trait]
impl DirectoryOpener for NoopDirectoryOpener {
    async fn open(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }
}
