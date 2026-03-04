//! 应用目录提供者抽象
//!
//! **使用场景**: 仅用于 Tauri 应用需要获取 Tauri 特有的应用目录时。
//! **不使用场景**: CLI/测试应直接使用 storage::get_data_dir() 函数。

use std::path::PathBuf;

/// 应用目录提供者抽象
pub trait AppDirProvider: Send + Sync {
    /// 获取应用数据目录
    fn app_data_dir(&self) -> Result<PathBuf, String>;
}

/// 默认实现 - 使用环境变量
pub struct DefaultAppDirProvider;

impl AppDirProvider for DefaultAppDirProvider {
    fn app_data_dir(&self) -> Result<PathBuf, String> {
        // 后续将从 storage 模块导入
        // crate::storage::get_data_dir()

        // 临时实现：使用环境变量
        if cfg!(windows) {
            std::env::var("APPDATA")
                .map(|p| PathBuf::from(p).join("flow-executor"))
                .map_err(|e| format!("获取 APPDATA 失败: {}", e))
        } else {
            std::env::var("HOME")
                .map(|p| PathBuf::from(p).join(".flow-executor"))
                .map_err(|e| format!("获取 HOME 失败: {}", e))
        }
    }
}
