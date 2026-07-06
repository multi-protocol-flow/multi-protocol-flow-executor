// 数据集（Dataset）核心类型定义
//!
//! 定义 CSV 数据源配置、行取策略、行状态和数据驱动执行报告等类型。

use serde::{Deserialize, Serialize};

/// CSV 数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// CSV 文件绝对路径
    pub path: String,
    /// 编码（None 表示自动检测）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// 分隔符（None 表示逗号）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<char>,
    /// 是否有表头（默认 true）
    #[serde(default = "default_true")]
    pub has_headers: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DatasetConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            encoding: None,
            delimiter: None,
            has_headers: true,
        }
    }
}

/// 压测数据取行策略
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataPickStrategy {
    /// 轮询
    #[default]
    RoundRobin,
    /// 随机
    Random,
    /// 独占（每虚拟用户绑定固定行）
    Unique,
}

/// 行执行状态
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowStatus {
    /// Success
    #[default]
    Success,
    /// Failed（断言未通过）
    Fail,
    /// Error（execution error）
    Error,
}

/// 单行执行摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowSummary {
    /// 行索引
    pub row_index: u32,
    /// 输入数据
    pub input_data: serde_json::Value,
    /// 执行状态
    pub status: RowStatus,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// 是否有详细报告
    pub has_detail_report: bool,
}

/// 数据驱动执行报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetExecutionReport {
    /// 报告 UUID
    pub uuid: String,
    /// 流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 数据集文件路径
    pub dataset_path: String,
    /// 数据集名称
    pub dataset_name: String,
    /// 编码
    pub encoding: String,
    /// 执行时间戳（毫秒）
    pub executed_at: u64,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// 总行数
    pub total_rows: u32,
    /// Success数
    pub success_count: u32,
    /// Failed数
    pub fail_count: u32,
    /// Error数
    pub error_count: u32,
    /// 平均耗时（毫秒）
    pub avg_duration_ms: u64,
    /// P50 耗时（毫秒）
    pub p50_duration_ms: u64,
    /// P95 耗时（毫秒）
    pub p95_duration_ms: u64,
    /// P99 耗时（毫秒）
    pub p99_duration_ms: u64,
    /// 行级执行摘要
    #[serde(default)]
    pub row_summaries: Vec<RowSummary>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dataset_config_roundtrip() {
        let config = DatasetConfig {
            path: "/data/test.csv".to_string(),
            encoding: Some("utf-8".to_string()),
            delimiter: Some(','),
            has_headers: true,
        };
        let json = serde_json::to_string(&config).expect("序列化Failed");
        let deserialized: DatasetConfig = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(config.path, deserialized.path);
        assert_eq!(config.encoding, deserialized.encoding);
        assert_eq!(config.delimiter, deserialized.delimiter);
        assert_eq!(config.has_headers, deserialized.has_headers);
    }

    #[test]
    fn test_dataset_config_default() {
        let config = DatasetConfig::default();
        assert!(config.path.is_empty());
        assert!(config.encoding.is_none());
        assert!(config.delimiter.is_none());
        assert!(config.has_headers);
    }

    #[test]
    fn test_dataset_config_minimal_json() {
        // 只提供必填字段，可选字段应有默认值
        let json = r#"{"path":"/data/test.csv"}"#;
        let config: DatasetConfig = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(config.path, "/data/test.csv");
        assert!(config.encoding.is_none());
        assert!(config.delimiter.is_none());
        assert!(config.has_headers);
    }

    #[test]
    fn test_data_pick_strategy_roundtrip() {
        let strategies = vec![
            DataPickStrategy::RoundRobin,
            DataPickStrategy::Random,
            DataPickStrategy::Unique,
        ];
        for strategy in strategies {
            let json = serde_json::to_string(&strategy).expect("序列化Failed");
            let deserialized: DataPickStrategy =
                serde_json::from_str(&json).expect("反序列化Failed");
            assert_eq!(strategy, deserialized);
        }
    }

    #[test]
    fn test_data_pick_strategy_snake_case() {
        let json = r#""round_robin""#;
        let strategy: DataPickStrategy = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(strategy, DataPickStrategy::RoundRobin);

        let json = r#""random""#;
        let strategy: DataPickStrategy = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(strategy, DataPickStrategy::Random);

        let json = r#""unique""#;
        let strategy: DataPickStrategy = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(strategy, DataPickStrategy::Unique);
    }

    #[test]
    fn test_row_status_roundtrip() {
        let statuses = vec![RowStatus::Success, RowStatus::Fail, RowStatus::Error];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("序列化Failed");
            let deserialized: RowStatus = serde_json::from_str(&json).expect("反序列化Failed");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_row_status_snake_case() {
        let json = r#""success""#;
        let status: RowStatus = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(status, RowStatus::Success);

        let json = r#""fail""#;
        let status: RowStatus = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(status, RowStatus::Fail);

        let json = r#""error""#;
        let status: RowStatus = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(status, RowStatus::Error);
    }

    #[test]
    fn test_row_summary_roundtrip() {
        let summary = RowSummary {
            row_index: 0,
            input_data: json!({"name": "test", "value": 42}),
            status: RowStatus::Success,
            duration_ms: 150,
            error_message: None,
            has_detail_report: false,
        };
        let json_str = serde_json::to_string(&summary).expect("序列化Failed");
        let deserialized: RowSummary = serde_json::from_str(&json_str).expect("反序列化Failed");
        assert_eq!(summary.row_index, deserialized.row_index);
        assert_eq!(summary.status, deserialized.status);
        assert_eq!(summary.duration_ms, deserialized.duration_ms);
        assert_eq!(summary.error_message, deserialized.error_message);
        assert_eq!(summary.has_detail_report, deserialized.has_detail_report);
    }

    #[test]
    fn test_row_summary_with_error() {
        let summary = RowSummary {
            row_index: 3,
            input_data: json!({"id": 1}),
            status: RowStatus::Error,
            duration_ms: 2000,
            error_message: Some("连接超时".to_string()),
            has_detail_report: true,
        };
        let json_str = serde_json::to_string(&summary).expect("序列化Failed");
        let deserialized: RowSummary = serde_json::from_str(&json_str).expect("反序列化Failed");
        assert_eq!(deserialized.error_message, Some("连接超时".to_string()));
    }

    #[test]
    fn test_dataset_execution_report_roundtrip() {
        let report = DatasetExecutionReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            dataset_path: "/data/test.csv".to_string(),
            dataset_name: "test.csv".to_string(),
            encoding: "utf-8".to_string(),
            executed_at: 1700000000000,
            total_duration_ms: 5000,
            total_rows: 10,
            success_count: 8,
            fail_count: 1,
            error_count: 1,
            avg_duration_ms: 500,
            p50_duration_ms: 450,
            p95_duration_ms: 800,
            p99_duration_ms: 900,
            row_summaries: vec![RowSummary {
                row_index: 0,
                input_data: json!({"name": "a"}),
                status: RowStatus::Success,
                duration_ms: 100,
                error_message: None,
                has_detail_report: false,
            }],
        };
        let json_str = serde_json::to_string(&report).expect("序列化Failed");
        let deserialized: DatasetExecutionReport =
            serde_json::from_str(&json_str).expect("反序列化Failed");
        assert_eq!(report.uuid, deserialized.uuid);
        assert_eq!(report.total_rows, deserialized.total_rows);
        assert_eq!(report.success_count, deserialized.success_count);
        assert_eq!(report.row_summaries.len(), deserialized.row_summaries.len());
    }

    #[test]
    fn test_dataset_execution_report_empty_summaries() {
        let json = r#"{
            "uuid": "r1",
            "flow_uuid": "f1",
            "flow_name": "test",
            "dataset_path": "/d.csv",
            "dataset_name": "d.csv",
            "encoding": "utf-8",
            "executed_at": 0,
            "total_duration_ms": 0,
            "total_rows": 0,
            "success_count": 0,
            "fail_count": 0,
            "error_count": 0,
            "avg_duration_ms": 0,
            "p50_duration_ms": 0,
            "p95_duration_ms": 0,
            "p99_duration_ms": 0
        }"#;
        let report: DatasetExecutionReport = serde_json::from_str(json).expect("反序列化Failed");
        assert!(report.row_summaries.is_empty());
    }
}
