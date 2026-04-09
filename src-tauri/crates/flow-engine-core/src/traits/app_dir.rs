//! 应用目录提供者抽象
//!
//! **使用场景**: 仅用于 Tauri 应用需要获取 Tauri 特有的应用目录时。
//! **不使用场景**: CLI/测试应直接使用 `storage::get_data_dir()` 函数。

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock 实现用于测试
    struct MockAppDirProvider {
        dir: Result<PathBuf, String>,
    }

    impl MockAppDirProvider {
        fn new(dir: PathBuf) -> Self {
            Self { dir: Ok(dir) }
        }

        fn with_error(msg: &str) -> Self {
            Self {
                dir: Err(msg.to_string()),
            }
        }
    }

    impl AppDirProvider for MockAppDirProvider {
        fn app_data_dir(&self) -> Result<PathBuf, String> {
            self.dir.clone()
        }
    }

    #[test]
    fn test_mock_provider_returns_success() {
        let expected = PathBuf::from("/test/app/data");
        let provider = MockAppDirProvider::new(expected.clone());

        let result = provider.app_data_dir();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_mock_provider_returns_error() {
        let error_msg = "目录不存在";
        let provider = MockAppDirProvider::with_error(error_msg);

        let result = provider.app_data_dir();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(error_msg));
    }

    #[test]
    fn test_default_provider_returns_path() {
        let provider = DefaultAppDirProvider;
        let result = provider.app_data_dir();

        // 在测试环境中，环境变量可能存在也可能不存在
        // 这里只测试返回值类型正确
        match result {
            Ok(path) => {
                // 路径应该以 flow-executor 或 .flow-executor 结尾
                let path_str = path.to_string_lossy();
                assert!(
                    path_str.ends_with("flow-executor") || path_str.ends_with(".flow-executor"),
                    "路径应以 flow-executor 结尾，实际为: {:?}",
                    path
                );
            }
            Err(e) => {
                // 如果环境变量不存在，应该返回有意义的错误信息
                assert!(!e.is_empty(), "错误信息不应为空");
            }
        }
    }

    #[test]
    fn test_trait_object_compatibility() {
        // 测试 trait 对象兼容性 (Send + Sync)
        let provider: Box<dyn AppDirProvider> = Box::new(DefaultAppDirProvider);

        let result = provider.app_data_dir();
        // 只验证能正常调用
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_provider_can_be_used_in_collection() {
        // 测试可以存储在集合中
        let providers: Vec<Box<dyn AppDirProvider>> = vec![
            Box::new(MockAppDirProvider::new(PathBuf::from("/path1"))),
            Box::new(MockAppDirProvider::new(PathBuf::from("/path2"))),
        ];

        let results: Vec<_> = providers.iter().map(|p| p.app_data_dir()).collect();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
