// 统计模块 - 流程执行统计信息

use crate::report::{ExecutionStatus, NodeExecutionReport};
use serde::{Deserialize, Serialize};

/// 流程执行统计信息
///
/// 提供流程执行的各项统计指标，包括成功/失败节点数量、执行时间、
/// 数据大小和执行效率等。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowStatistics {
    // ===== 基础统计 =====
    /// 成功的节点数量
    pub success_count: u32,

    /// 失败的节点数量
    pub failed_count: u32,

    // ===== 时间统计 =====
    /// 平均节点执行时间（毫秒）
    pub avg_node_duration_ms: Option<u64>,

    /// 最长节点执行时间（毫秒）
    pub max_node_duration_ms: Option<u64>,

    /// 最短节点执行时间（毫秒）
    pub min_node_duration_ms: Option<u64>,

    // ===== 数据统计 =====
    /// 总输入数据大小（字节，估算）
    pub total_input_size_bytes: Option<u64>,

    /// 总输出数据大小（字节，估算）
    pub total_output_size_bytes: Option<u64>,

    // ===== 执行效率 =====
    /// 错误率（0.0 - 1.0）
    pub error_rate: Option<f64>,

    /// 完成率（executed_count / total_nodes）
    pub completion_rate: Option<f64>,
}

impl FlowStatistics {
    /// 从节点报告列表计算统计信息
    ///
    /// # 参数
    ///
    /// * `reports` - 节点执行报告列表
    /// * `total_nodes` - 总节点数（包括未执行的节点）
    ///
    /// # 返回
    ///
    /// 计算后的统计信息
    pub fn from_reports(reports: &[NodeExecutionReport], total_nodes: u32) -> Self {
        if reports.is_empty() {
            return Self::default();
        }

        let mut stats = Self::default();

        // 计算成功/失败节点数量
        for report in reports {
            match report.status {
                ExecutionStatus::Success => {
                    stats.success_count += 1;
                }
                ExecutionStatus::Failed => {
                    stats.failed_count += 1;
                }
                ExecutionStatus::Skipped => {
                    // 跳过的节点不计入成功/失败计数
                }
            }
        }

        // 计算时间统计
        let durations: Vec<u64> = reports.iter().map(|r| r.duration_ms).collect();
        if !durations.is_empty() {
            let total_duration: u64 = durations.iter().sum();
            stats.avg_node_duration_ms = Some(total_duration / durations.len() as u64);
            stats.max_node_duration_ms = Some(*durations.iter().max().unwrap());
            stats.min_node_duration_ms = Some(*durations.iter().min().unwrap());
        }

        // 计算数据大小（估算）
        let mut total_input_size = 0u64;
        let mut total_output_size = 0u64;

        for report in reports {
            if let Some(input_data) = &report.input_data {
                total_input_size += Self::estimate_json_size(input_data);
            }
            if let Some(output_data) = &report.output_data {
                total_output_size += Self::estimate_json_size(output_data);
            }
        }

        if total_input_size > 0 {
            stats.total_input_size_bytes = Some(total_input_size);
        }
        if total_output_size > 0 {
            stats.total_output_size_bytes = Some(total_output_size);
        }

        // 计算错误率
        let total_executed = reports.len() as u32;
        if total_executed > 0 {
            stats.error_rate = Some(stats.failed_count as f64 / total_executed as f64);
        }

        // 计算完成率
        if total_nodes > 0 {
            stats.completion_rate = Some(total_executed as f64 / total_nodes as f64);
        }

        stats
    }

    /// 估算 JSON 值的字节大小
    ///
    /// # 参数
    ///
    /// * `value` - JSON 值
    ///
    /// # 返回
    ///
    /// 估算的字节大小
    fn estimate_json_size(value: &serde_json::Value) -> u64 {
        match value {
            serde_json::Value::Null => 4,
            serde_json::Value::Bool(_) => 5,
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    8
                } else {
                    16
                }
            }
            serde_json::Value::String(s) => s.len() as u64,
            serde_json::Value::Array(arr) => {
                arr.iter().map(|v| Self::estimate_json_size(v)).sum::<u64>()
            }
            serde_json::Value::Object(obj) => obj
                .values()
                .map(|v| Self::estimate_json_size(v))
                .sum::<u64>(),
        }
    }

    /// 获取总执行节点数
    ///
    /// # 返回
    ///
    /// 成功 + 失败的节点数量
    pub fn total_executed_count(&self) -> u32 {
        self.success_count + self.failed_count
    }

    /// 判断是否所有节点都执行成功
    ///
    /// # 返回
    ///
    /// 如果没有失败节点则返回 true
    pub fn is_all_success(&self) -> bool {
        self.failed_count == 0 && self.success_count > 0
    }

    /// 判断是否有错误发生
    ///
    /// # 返回
    ///
    /// 如果有失败节点则返回 true
    pub fn has_errors(&self) -> bool {
        self.failed_count > 0
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_report(
        node_id: &str,
        status: ExecutionStatus,
        duration_ms: u64,
        input_size: Option<u64>,
        output_size: Option<u64>,
    ) -> NodeExecutionReport {
        NodeExecutionReport {
            node_uuid: node_id.to_string(),
            node_type: "test".to_string(),
            node_name: "Test Node".to_string(),
            start_time: 1000,
            end_time: 1000 + duration_ms,
            duration_ms,
            status,
            input_data: input_size.map(|s| serde_json::json!({"data": "x".repeat(s as usize)})),
            output_data: output_size.map(|s| serde_json::json!({"result": "y".repeat(s as usize)})),
            used_port_id: Some("out".to_string()),
            error: None,
        }
    }

    #[test]
    fn test_flow_statistics_default() {
        let stats = FlowStatistics::default();

        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.failed_count, 0);
        assert!(stats.avg_node_duration_ms.is_none());
        assert!(stats.max_node_duration_ms.is_none());
        assert!(stats.min_node_duration_ms.is_none());
        assert!(stats.total_input_size_bytes.is_none());
        assert!(stats.total_output_size_bytes.is_none());
        assert!(stats.error_rate.is_none());
        assert!(stats.completion_rate.is_none());
    }

    #[test]
    fn test_flow_statistics_from_empty_reports() {
        let reports: Vec<NodeExecutionReport> = vec![];
        let stats = FlowStatistics::from_reports(&reports, 5);

        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.failed_count, 0);
        assert!(stats.avg_node_duration_ms.is_none());
    }

    #[test]
    fn test_flow_statistics_success_and_failure() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, Some(10), Some(20)),
            create_test_report("node-2", ExecutionStatus::Success, 150, None, None),
            create_test_report("node-3", ExecutionStatus::Failed, 200, Some(5), None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 5);

        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.total_executed_count(), 3);
        assert!(stats.has_errors());
        assert!(!stats.is_all_success());
    }

    #[test]
    fn test_flow_statistics_all_success() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, Some(10), Some(20)),
            create_test_report("node-2", ExecutionStatus::Success, 150, None, None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 2);

        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failed_count, 0);
        assert!(stats.is_all_success());
        assert!(!stats.has_errors());
    }

    #[test]
    fn test_flow_statistics_skipped_nodes() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, None, None),
            create_test_report("node-2", ExecutionStatus::Success, 50, None, None),
            create_test_report("node-3", ExecutionStatus::Failed, 200, None, None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 3);

        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failed_count, 1);
        // Skipped 状态已移除，所有执行节点都计入统计
    }

    #[test]
    fn test_flow_statistics_time_metrics() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, None, None),
            create_test_report("node-2", ExecutionStatus::Success, 200, None, None),
            create_test_report("node-3", ExecutionStatus::Success, 300, None, None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 3);

        assert_eq!(stats.avg_node_duration_ms, Some(200));
        assert_eq!(stats.max_node_duration_ms, Some(300));
        assert_eq!(stats.min_node_duration_ms, Some(100));
    }

    #[test]
    fn test_flow_statistics_error_rate() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, None, None),
            create_test_report("node-2", ExecutionStatus::Success, 100, None, None),
            create_test_report("node-3", ExecutionStatus::Failed, 100, None, None),
            create_test_report("node-4", ExecutionStatus::Failed, 100, None, None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 4);

        // 2 失败 / 4 总数 = 0.5
        assert_eq!(stats.error_rate, Some(0.5));
    }

    #[test]
    fn test_flow_statistics_completion_rate() {
        let reports = vec![
            create_test_report("node-1", ExecutionStatus::Success, 100, None, None),
            create_test_report("node-2", ExecutionStatus::Failed, 100, None, None),
        ];

        let stats = FlowStatistics::from_reports(&reports, 5);

        // 2 执行 / 5 总数 = 0.4
        assert_eq!(stats.completion_rate, Some(0.4));
    }

    #[test]
    fn test_flow_statistics_serialization() {
        let reports = vec![create_test_report(
            "node-1",
            ExecutionStatus::Success,
            100,
            Some(10),
            Some(20),
        )];

        let stats = FlowStatistics::from_reports(&reports, 1);

        // 序列化
        let json = serde_json::to_string(&stats).expect("序列化失败");
        assert!(json.contains("success_count"));
        assert!(json.contains("1"));

        // 反序列化
        let deserialized: FlowStatistics = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized.success_count, 1);
    }

    #[test]
    fn test_flow_statistics_no_reports_completion_rate() {
        let reports: Vec<NodeExecutionReport> = vec![];
        let stats = FlowStatistics::from_reports(&reports, 5);

        // 没有报告时，所有统计字段都为默认值（None）
        assert_eq!(stats.completion_rate, None);
    }

    #[test]
    fn test_estimate_json_size() {
        // 测试不同类型的 JSON 值大小估算
        assert_eq!(
            FlowStatistics::estimate_json_size(&serde_json::Value::Null),
            4
        );
        assert_eq!(
            FlowStatistics::estimate_json_size(&serde_json::json!(true)),
            5
        );
        assert_eq!(
            FlowStatistics::estimate_json_size(&serde_json::json!(42)),
            8
        );
        assert_eq!(
            FlowStatistics::estimate_json_size(&serde_json::json!(3.14)),
            16
        );
        assert_eq!(
            FlowStatistics::estimate_json_size(&serde_json::json!("hello")),
            5
        );

        // 测试数组
        let arr = serde_json::json!([1, 2, 3]);
        let size = FlowStatistics::estimate_json_size(&arr);
        assert!(size > 0);

        // 测试对象
        let obj = serde_json::json!({"a": 1, "b": 2});
        let size = FlowStatistics::estimate_json_size(&obj);
        assert!(size > 0);
    }

    #[test]
    fn test_total_executed_count() {
        let mut stats = FlowStatistics::default();
        stats.success_count = 5;
        stats.failed_count = 2;

        assert_eq!(stats.total_executed_count(), 7);
    }
}
