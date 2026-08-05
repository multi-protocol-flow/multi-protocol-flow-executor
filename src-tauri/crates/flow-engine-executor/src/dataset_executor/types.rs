// 数据驱动执行类型定义

use serde::{Deserialize, Serialize};

/// Failed策略
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStrategy {
    /// 遇到Failed继续执行后续行
    #[default]
    Continue,
    /// 遇到第一个Failed即停止
    StopOnFirst,
}

/// 数据驱动执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetExecutionConfig {
    /// 流程 UUID
    pub flow_uuid: String,
    /// CSV 文件路径
    pub dataset_path: String,
    /// CSV 编码（None 表示自动检测）
    pub dataset_encoding: Option<String>,
    /// CSV 分隔符（None 表示逗号）
    pub delimiter: Option<char>,
    /// 并发数（默认 1，即顺序执行）
    pub concurrency: u32,
    /// Failed策略
    pub failure_strategy: FailureStrategy,
}

impl Default for DatasetExecutionConfig {
    fn default() -> Self {
        Self {
            flow_uuid: String::new(),
            dataset_path: String::new(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_strategy_default() {
        assert_eq!(FailureStrategy::default(), FailureStrategy::Continue);
    }

    #[test]
    fn test_failure_strategy_serde_roundtrip() {
        let strategies = vec![FailureStrategy::Continue, FailureStrategy::StopOnFirst];
        for s in strategies {
            let json = serde_json::to_string(&s).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
            let de: FailureStrategy =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
            assert_eq!(s, de);
        }
    }

    #[test]
    fn test_failure_strategy_snake_case() {
        let json = r#""stop_on_first""#;
        let s: FailureStrategy =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(s, FailureStrategy::StopOnFirst);
    }

    #[test]
    fn test_dataset_execution_config_default() {
        let config = DatasetExecutionConfig::default();
        assert!(config.flow_uuid.is_empty());
        assert!(config.dataset_path.is_empty());
        assert!(config.dataset_encoding.is_none());
        assert!(config.delimiter.is_none());
        assert_eq!(config.concurrency, 1);
        assert_eq!(config.failure_strategy, FailureStrategy::Continue);
    }
}
