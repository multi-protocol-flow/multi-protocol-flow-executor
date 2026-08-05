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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_directory_opener_returns_ok() {
        let opener = NoopDirectoryOpener;

        // 测试空路径
        let result = opener.open("").await;
        assert!(result.is_ok());

        // 测试有效路径
        let result = opener.open("/some/path").await;
        assert!(result.is_ok());

        // 测试带空格的路径
        let result = opener.open("/path with spaces").await;
        assert!(result.is_ok());

        // 测试 Windows 路径
        let result = opener.open("C:\\Users\\test").await;
        assert!(result.is_ok());
    }

    /// 可配置的 Mock 实现，用于测试
    struct MockDirectoryOpener {
        should_fail: bool,
        error_message: String,
    }

    impl MockDirectoryOpener {
        fn new() -> Self {
            Self {
                should_fail: false,
                error_message: String::new(),
            }
        }

        fn with_error(message: &str) -> Self {
            Self {
                should_fail: true,
                error_message: message.to_string(),
            }
        }
    }

    #[async_trait]
    impl DirectoryOpener for MockDirectoryOpener {
        async fn open(&self, _path: &str) -> Result<(), String> {
            if self.should_fail {
                Err(self.error_message.clone())
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_mock_directory_opener_success() {
        let opener = MockDirectoryOpener::new();
        let result = opener.open("/test/path").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_directory_opener_failure() {
        let opener = MockDirectoryOpener::with_error("目录不存在");
        let result = opener.open("/invalid/path").await;
        assert!(result.is_err());
        assert_eq!(
            result
                .err()
                .unwrap_or_else(|| panic!("expected Err, got Ok")),
            "目录不存在"
        );
    }

    /// 使用 trait object 测试动态分发
    #[tokio::test]
    async fn test_trait_object() {
        let opener: Box<dyn DirectoryOpener> = Box::new(NoopDirectoryOpener);
        let result = opener.open("/any/path").await;
        assert!(result.is_ok());
    }
}
