// 压力测试类型定义

use flow_engine_core::dataset::DataPickStrategy;
use serde::{Deserialize, Serialize};

/// 压力测试状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum StressTestStatus {
    /// 空闲状态
    #[default]
    Idle,
    /// 爬坡阶段
    RampUp,
    /// 运行中
    Running,
    /// 已停止
    Stopped,
}

/// 压力测试配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressTestConfig {
    /// 并发用户数
    pub concurrent_users: u32,
    /// 爬坡时间（秒）
    pub ramp_up_seconds: u32,
    /// 持续时间（秒）
    pub duration_seconds: u32,
    /// 流程 UUID
    pub flow_uuid: String,
    /// CSV 数据源路径（可选，设置后启用数据驱动模式）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_path: Option<String>,
    /// CSV 编码（None = 自动检测）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_encoding: Option<String>,
    /// 数据取行策略
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_strategy: Option<DataPickStrategy>,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: String::new(),
            dataset_path: None,
            dataset_encoding: None,
            dataset_strategy: None,
        }
    }
}

/// 压力测试进度
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StressTestProgress {
    /// 当前状态
    pub status: StressTestStatus,
    /// 已运行时间（秒）
    pub elapsed_seconds: u64,
    /// 总迭代次数
    pub total_iterations: u64,
    /// Success迭代次数
    pub successful_iterations: u64,
    /// Failed迭代次数
    pub failed_iterations: u64,
    /// 当前 RPS（每秒请求数）
    pub current_rps: f64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 节点级实时指标
    #[serde(default)]
    pub node_metrics: Vec<NodeMetrics>,
}

/// 压力测试统计信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StressTestStatistics {
    /// 总迭代次数
    pub total_iterations: u64,
    /// Success次数
    pub successful: u64,
    /// Failed次数
    pub failed: u64,
    /// Success率 (0.0 - 1.0)
    pub success_rate: f64,
    /// RPS（每秒请求数）
    pub rps: f64,
}

/// 响应时间分布
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResponseTimeDistribution {
    /// 平均响应时间（毫秒）
    pub avg_ms: f64,
    /// 最小响应时间（毫秒）
    pub min_ms: u64,
    /// 最大响应时间（毫秒）
    pub max_ms: u64,
    /// P50 响应时间（毫秒）
    pub p50_ms: u64,
    /// P90 响应时间（毫秒）
    pub p90_ms: u64,
    /// P95 响应时间（毫秒）
    pub p95_ms: u64,
    /// P99 响应时间（毫秒）
    pub p99_ms: u64,
}

/// Error样本
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorSample {
    /// 迭代序号
    pub iteration: u64,
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// Error消息
    pub error_message: String,
}

/// Error统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ErrorStatistics {
    /// Error类型
    #[serde(default)]
    pub error_kind: String,
    /// 出现次数
    #[serde(default)]
    pub count: u64,
    /// Error样本（最多保留若干条）
    #[serde(default)]
    pub samples: Vec<ErrorSample>,
}

/// 节点级实时指标（压测过程中实时更新）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetrics {
    /// 节点 UUID
    #[serde(default)]
    pub node_uuid: String,
    /// 节点名称
    #[serde(default)]
    pub node_name: String,
    /// 节点类型 (`http`, `ws_connect`, `mqtt_connect` 等)
    #[serde(default)]
    pub node_type: String,
    /// 总执行次数
    #[serde(default)]
    pub total_executions: u64,
    /// Success执行次数
    #[serde(default)]
    pub successful_executions: u64,
    /// Failed执行次数
    #[serde(default)]
    pub failed_executions: u64,
    /// Success率 (0.0 - 1.0)
    #[serde(default)]
    pub success_rate: f64,
    /// 平均延迟（毫秒）
    #[serde(default)]
    pub avg_latency_ms: f64,
    /// 最小延迟（毫秒）
    #[serde(default)]
    pub min_latency_ms: u64,
    /// 最大延迟（毫秒）
    #[serde(default)]
    pub max_latency_ms: u64,
    /// P50 延迟（毫秒）
    #[serde(default)]
    pub p50_latency_ms: u64,
    /// P90 延迟（毫秒）
    #[serde(default)]
    pub p90_latency_ms: u64,
    /// P95 延迟（毫秒）
    #[serde(default)]
    pub p95_latency_ms: u64,
    /// P99 延迟（毫秒）
    #[serde(default)]
    pub p99_latency_ms: u64,
}

/// 节点级别统计（单个节点的详细统计，用于最终报告）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeLevelStatistics {
    /// 节点 UUID
    #[serde(default)]
    pub node_uuid: String,
    /// 节点名称
    #[serde(default)]
    pub node_name: String,
    /// 节点类型
    #[serde(default)]
    pub node_type: String,
    /// 总执行次数
    #[serde(default)]
    pub total_executions: u64,
    /// Success执行次数
    #[serde(default)]
    pub successful_executions: u64,
    /// Failed执行次数
    #[serde(default)]
    pub failed_executions: u64,
    /// Success率 (0.0 - 1.0)
    #[serde(default)]
    pub success_rate: f64,
    /// 响应时间分布
    #[serde(default)]
    pub response_time_distribution: ResponseTimeDistribution,
    /// 该节点的Error统计
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_statistics: Vec<ErrorStatistics>,
}

/// 节点类型聚合统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeTypeStatistics {
    /// 节点类型 (`http`, `ws_connect`, `mqtt_connect` 等)
    #[serde(default)]
    pub node_type: String,
    /// 该类型的节点数量
    #[serde(default)]
    pub node_count: u32,
    /// 总执行次数
    #[serde(default)]
    pub total_executions: u64,
    /// Success执行次数
    #[serde(default)]
    pub successful_executions: u64,
    /// Failed执行次数
    #[serde(default)]
    pub failed_executions: u64,
    /// Success率 (0.0 - 1.0)
    #[serde(default)]
    pub success_rate: f64,
    /// 响应时间分布（聚合后）
    #[serde(default)]
    pub response_time_distribution: ResponseTimeDistribution,
}

/// 压力测试报告
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StressTestReport {
    // ===== 元信息 =====
    /// 报告唯一 ID
    #[serde(default)]
    pub uuid: String,
    /// 关联的流程 UUID
    #[serde(default)]
    pub flow_uuid: String,
    /// 流程名称
    #[serde(default)]
    pub flow_name: String,
    /// 测试开始时间（毫秒时间戳）
    #[serde(default)]
    pub start_time: u64,
    /// 测试结束时间（毫秒时间戳）
    #[serde(default)]
    pub end_time: u64,
    /// 总执行时长（毫秒）
    #[serde(default)]
    pub duration_ms: u64,

    // ===== 配置快照 =====
    /// 测试配置
    #[serde(default)]
    pub config: StressTestConfig,

    // ===== 汇总统计 =====
    /// 统计信息
    #[serde(default)]
    pub statistics: StressTestStatistics,
    /// 响应时间分布
    #[serde(default)]
    pub response_time_distribution: ResponseTimeDistribution,

    // ===== Error统计 =====
    /// Error统计列表（按Error类型分组）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_statistics: Vec<ErrorStatistics>,

    // ===== 节点级统计 =====
    /// 节点级别统计（每个节点的详细统计）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_statistics: Vec<NodeLevelStatistics>,
    /// 节点类型聚合统计
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_type_statistics: Vec<NodeTypeStatistics>,

    // ===== 进度历史 =====
    /// 进度历史数据（用于折线图等可视化，每次进度事件记录一条）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub progress_history: Vec<StressTestProgress>,

    // ===== Flow 快照 =====
    /// Flow 快照（执行时的流程完整副本）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_snapshot: Option<flow_engine_core::flow::Flow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_test_status_default() {
        let status = StressTestStatus::default();
        assert_eq!(status, StressTestStatus::Idle);
    }

    #[test]
    fn test_stress_test_config_default() {
        let config = StressTestConfig::default();
        assert_eq!(config.concurrent_users, 10);
        assert_eq!(config.ramp_up_seconds, 10);
        assert_eq!(config.duration_seconds, 60);
        assert_eq!(config.flow_uuid, "");
    }

    #[test]
    fn test_stress_test_progress_default() {
        let progress = StressTestProgress::default();
        assert_eq!(progress.status, StressTestStatus::Idle);
        assert_eq!(progress.elapsed_seconds, 0);
        assert_eq!(progress.total_iterations, 0);
        assert_eq!(progress.successful_iterations, 0);
        assert_eq!(progress.failed_iterations, 0);
        assert_eq!(progress.current_rps, 0.0);
        assert_eq!(progress.avg_latency_ms, 0.0);
    }

    #[test]
    fn test_stress_test_statistics_default() {
        let stats = StressTestStatistics::default();
        assert_eq!(stats.total_iterations, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.success_rate, 0.0);
        assert_eq!(stats.rps, 0.0);
    }

    #[test]
    fn test_response_time_distribution_default() {
        let dist = ResponseTimeDistribution::default();
        assert_eq!(dist.avg_ms, 0.0);
        assert_eq!(dist.min_ms, 0);
        assert_eq!(dist.max_ms, 0);
        assert_eq!(dist.p50_ms, 0);
        assert_eq!(dist.p90_ms, 0);
        assert_eq!(dist.p95_ms, 0);
        assert_eq!(dist.p99_ms, 0);
    }

    #[test]
    fn test_error_sample_serialization() {
        let sample = ErrorSample {
            iteration: 1,
            node_uuid: "node-001".to_string(),
            node_name: "HTTP Request".to_string(),
            error_message: "Connection timeout".to_string(),
        };

        let json = serde_json::to_string(&sample).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("iteration"));
        assert!(json.contains("nodeUuid"));
        assert!(json.contains("nodeName"));
        assert!(json.contains("errorMessage"));
    }

    #[test]
    fn test_error_statistics_default() {
        let stats = ErrorStatistics::default();
        assert_eq!(stats.error_kind, "");
        assert_eq!(stats.count, 0);
        assert!(stats.samples.is_empty());
    }

    #[test]
    fn test_stress_test_report_default() {
        let report = StressTestReport::default();
        assert_eq!(report.uuid, "");
        assert_eq!(report.flow_uuid, "");
        assert_eq!(report.flow_name, "");
        assert_eq!(report.start_time, 0);
        assert_eq!(report.end_time, 0);
        assert_eq!(report.duration_ms, 0);
        assert!(report.error_statistics.is_empty());
    }

    #[test]
    fn test_stress_test_report_serialization() {
        let report = StressTestReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            config: StressTestConfig {
                concurrent_users: 5,
                ramp_up_seconds: 5,
                duration_seconds: 30,
                flow_uuid: "flow-001".to_string(),
                ..Default::default()
            },
            statistics: StressTestStatistics {
                total_iterations: 100,
                successful: 95,
                failed: 5,
                success_rate: 0.95,
                rps: 10.0,
            },
            response_time_distribution: ResponseTimeDistribution {
                avg_ms: 150.5,
                min_ms: 50,
                max_ms: 500,
                p50_ms: 140,
                p90_ms: 300,
                p95_ms: 400,
                p99_ms: 450,
            },
            error_statistics: vec![],
            node_statistics: vec![],
            node_type_statistics: vec![],
            progress_history: vec![],
            flow_snapshot: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("report-001"));
        assert!(json.contains("flow-001"));
        assert!(json.contains("Test Flow"));
        assert!(json.contains("concurrentUsers"));
        assert!(json.contains("rampUpSeconds"));
        assert!(json.contains("durationSeconds"));
        assert!(json.contains("totalIterations"));
        assert!(json.contains("successRate"));
        assert!(json.contains("avgMs"));
        assert!(json.contains("p50Ms"));

        // 空的 error_statistics 应该被跳过
        assert!(!json.contains("errorStatistics"));
    }

    #[test]
    fn test_stress_test_report_with_errors_serialization() {
        let report = StressTestReport {
            uuid: "report-002".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            config: StressTestConfig::default(),
            statistics: StressTestStatistics::default(),
            response_time_distribution: ResponseTimeDistribution::default(),
            error_statistics: vec![ErrorStatistics {
                error_kind: "Timeout".to_string(),
                count: 5,
                samples: vec![ErrorSample {
                    iteration: 10,
                    node_uuid: "node-001".to_string(),
                    node_name: "HTTP Request".to_string(),
                    error_message: "Request timeout".to_string(),
                }],
            }],
            node_statistics: vec![],
            node_type_statistics: vec![],
            progress_history: vec![],
            flow_snapshot: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("errorStatistics"));
        assert!(json.contains("Timeout"));
        assert!(json.contains("errorKind"));
        assert!(json.contains("samples"));
    }

    #[test]
    fn test_stress_test_status_serialization() {
        let status = StressTestStatus::Running;
        let json = serde_json::to_string(&status).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert_eq!(json, "\"running\"");

        let deserialized: StressTestStatus =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized, StressTestStatus::Running);
    }

    /// 测试 `flow_snapshot` 序列化往返（camelCase 键名）
    #[test]
    fn test_stress_test_report_snapshot_serialization() {
        let flow = flow_engine_core::flow::Flow {
            uuid: "flow-snap-001".to_string(),
            name: "Snapshot Flow".to_string(),
            description: None,
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let report = StressTestReport {
            uuid: "report-snap-001".to_string(),
            flow_uuid: "flow-snap-001".to_string(),
            flow_name: "Snapshot Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            config: StressTestConfig::default(),
            statistics: StressTestStatistics::default(),
            response_time_distribution: ResponseTimeDistribution::default(),
            error_statistics: vec![],
            node_statistics: vec![],
            node_type_statistics: vec![],
            progress_history: vec![],
            flow_snapshot: Some(flow.clone()),
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        // rename_all = "camelCase" → flowSnapshot
        assert!(json.contains("flowSnapshot"));
        assert!(json.contains("flow-snap-001"));

        // 反序列化往返
        let deserialized: StressTestReport =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized.flow_snapshot, Some(flow));
    }

    /// 测试 `flow_snapshot` 向后兼容（JSON 中缺少字段时反序列化为 None）
    #[test]
    fn test_stress_test_report_snapshot_backward_compat() {
        let json = r#"{
            "uuid": "report-bc-001",
            "flowUuid": "flow-bc-001",
            "flowName": "BackCompat Flow",
            "startTime": 1000,
            "endTime": 2000,
            "durationMs": 1000,
            "config": {
                "concurrentUsers": 10,
                "rampUpSeconds": 10,
                "durationSeconds": 60,
                "flowUuid": "flow-bc-001"
            },
            "statistics": {
                "totalIterations": 0,
                "successful": 0,
                "failed": 0,
                "successRate": 0.0,
                "rps": 0.0
            },
            "responseTimeDistribution": {
                "avgMs": 0.0,
                "minMs": 0,
                "maxMs": 0,
                "p50Ms": 0,
                "p90Ms": 0,
                "p95Ms": 0,
                "p99Ms": 0
            }
        }"#;

        let report: StressTestReport =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(report.uuid, "report-bc-001");
        assert!(report.flow_snapshot.is_none());
    }

    /// 测试 `flow_snapshot` 为 None 时序列化结果不包含该字段
    #[test]
    fn test_stress_test_report_snapshot_skip_when_none() {
        let report = StressTestReport {
            uuid: "report-skip-001".to_string(),
            flow_uuid: "flow-skip-001".to_string(),
            flow_name: "Skip Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            config: StressTestConfig::default(),
            statistics: StressTestStatistics::default(),
            response_time_distribution: ResponseTimeDistribution::default(),
            error_statistics: vec![],
            node_statistics: vec![],
            node_type_statistics: vec![],
            progress_history: vec![],
            flow_snapshot: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            !json.contains("flowSnapshot"),
            "flow_snapshot 为 None 时不应出现在 JSON 中"
        );
    }
}
