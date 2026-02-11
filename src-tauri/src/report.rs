// 报告结构定义模块

/// 执行状态枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 跳过执行
    Skipped,
}

/// 节点执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionReport {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点类型
    pub node_type: String,
    /// 节点名称
    pub node_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 输入数据（完整存储，不截断）
    pub input_data: Option<serde_json::Value>,
    /// 输出数据（完整存储，不截断）
    pub output_data: Option<serde_json::Value>,
    /// 实际使用的输出端口 ID（仅已执行节点有值）
    /// - Entry 节点：固定为 "out"
    /// - Http 节点：固定为 "out"
    /// - Conditional 节点：根据条件评估结果为 "true" 或 "false"
    pub used_port_id: Option<String>,
    /// 错误详情
    pub error: Option<crate::error::ErrorDetail>,
}

/// 流程执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 总执行时长（毫秒）
    pub duration_ms: u64,
    /// 总节点数
    pub total_nodes: u32,
    /// 已执行节点数
    pub executed_count: u32,
    /// 整体执行状态
    pub status: ExecutionStatus,
    /// 节点执行报告列表（扁平结构）
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程统计信息
    pub statistics: crate::statistics::FlowStatistics,
    /// 流程级错误详情（可选）
    pub flow_error: Option<crate::error::ErrorDetail>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_execution_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ExecutionStatus::Success).unwrap(),
            r#""success""#
        );
        assert_eq!(
            serde_json::to_string(&ExecutionStatus::Failed).unwrap(),
            r#""failed""#
        );
    }

    #[test]
    fn test_execution_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<ExecutionStatus>(r#""success""#).unwrap(),
            ExecutionStatus::Success
        );
        assert_eq!(
            serde_json::from_str::<ExecutionStatus>(r#""failed""#).unwrap(),
            ExecutionStatus::Failed
        );
    }

    #[test]
    fn test_node_execution_report_creation() {
        let report = NodeExecutionReport {
            node_uuid: "node-123".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP Request".to_string(),
            start_time: 1234567890,
            end_time: 1234567895,
            duration_ms: 5,
            status: ExecutionStatus::Success,
            input_data: Some(json!({"url": "https://example.com"})),
            output_data: Some(json!({"status": 200})),
            used_port_id: Some("out".to_string()),
            error: None,
        };

        assert_eq!(report.node_uuid, "node-123");
        assert_eq!(report.node_type, "http");
        assert_eq!(report.node_name, "HTTP Request");
        assert_eq!(report.start_time, 1234567890);
        assert_eq!(report.end_time, 1234567895);
        assert_eq!(report.duration_ms, 5);
        assert_eq!(report.status, ExecutionStatus::Success);
        assert!(report.input_data.is_some());
        assert!(report.output_data.is_some());
        assert!(report.error.is_none());
        assert_eq!(report.used_port_id, Some("out".to_string()));
    }

    #[test]
    fn test_node_execution_report_serialization() {
        let report = NodeExecutionReport {
            node_uuid: "node-456".to_string(),
            node_type: "conditional".to_string(),
            node_name: "Conditional Node".to_string(),
            start_time: 1234567900,
            end_time: 1234567910,
            duration_ms: 10,
            status: ExecutionStatus::Failed,
            input_data: None,
            output_data: None,
            used_port_id: None,
            error: Some(crate::error::ErrorDetail::new(
                crate::error::ErrorKind::ConditionEvaluation,
                "Condition evaluation failed".to_string(),
            )),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("node-456"));
        assert!(json.contains("conditional"));
        assert!(json.contains("failed"));

        // 反序列化测试
        let deserialized: NodeExecutionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_uuid, "node-456");
        assert_eq!(deserialized.node_type, "conditional");
        assert_eq!(deserialized.status, ExecutionStatus::Failed);
    }

    #[test]
    fn test_flow_execution_report_creation() {
        let node_report = NodeExecutionReport {
            node_uuid: "node-1".to_string(),
            node_type: "start".to_string(),
            node_name: "Start Node".to_string(),
            start_time: 1234567890,
            end_time: 1234567891,
            duration_ms: 1,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(json!({"output": "success"})),
            used_port_id: Some("out".to_string()),
            error: None,
        };

        let flow_report = FlowExecutionReport {
            flow_uuid: "flow-123".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1234567890,
            end_time: 1234567900,
            duration_ms: 10,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![node_report],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
        };

        assert_eq!(flow_report.flow_uuid, "flow-123");
        assert_eq!(flow_report.flow_name, "Test Flow");
        assert_eq!(flow_report.total_nodes, 1);
        assert_eq!(flow_report.executed_count, 1);
        assert_eq!(flow_report.node_reports.len(), 1);
        assert_eq!(flow_report.node_reports[0].node_uuid, "node-1");
    }

    #[test]
    fn test_flow_execution_report_serialization_2() {
        let node_report = NodeExecutionReport {
            node_uuid: "node-2".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP Node".to_string(),
            start_time: 1234567910,
            end_time: 1234567920,
            duration_ms: 10,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(json!({"status": 200})),
            used_port_id: Some("out".to_string()),
            error: None,
        };

        let flow_report = FlowExecutionReport {
            flow_uuid: "flow-456".to_string(),
            flow_name: "Another Flow".to_string(),
            start_time: 1234567910,
            end_time: 1234567930,
            duration_ms: 20,
            total_nodes: 5,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![node_report],
            statistics: crate::statistics::FlowStatistics {
                success_count: 1,
                failed_count: 0,
                avg_node_duration_ms: Some(10),
                max_node_duration_ms: Some(10),
                min_node_duration_ms: Some(10),
                total_input_size_bytes: None,
                total_output_size_bytes: None,
                error_rate: Some(0.0),
                completion_rate: Some(0.2),
            },
            flow_error: None,
        };

        let json = serde_json::to_string(&flow_report).unwrap();
        assert!(json.contains("flow-456"));
        assert!(json.contains("Another Flow"));

        // 反序列化测试
        let deserialized: FlowExecutionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.flow_uuid, "flow-456");
        assert_eq!(deserialized.flow_name, "Another Flow");
        assert_eq!(deserialized.total_nodes, 5);
        assert_eq!(deserialized.executed_count, 1);
        assert_eq!(deserialized.status, ExecutionStatus::Success);
    }

    #[test]
    fn test_execution_status_equality() {
        assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
        assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);
        assert_ne!(ExecutionStatus::Success, ExecutionStatus::Failed);
    }

    #[test]
    fn test_node_execution_report_with_error() {
        let error_detail = crate::error::ErrorDetail::new(
            crate::error::ErrorKind::Connection,
            "Connection timeout".to_string(),
        );

        let report = NodeExecutionReport {
            node_uuid: "node-error".to_string(),
            node_type: "http".to_string(),
            node_name: "Failed HTTP Request".to_string(),
            start_time: 1234568000,
            end_time: 1234568050,
            duration_ms: 50,
            status: ExecutionStatus::Failed,
            input_data: Some(json!({"url": "https://invalid.com"})),
            output_data: None,
            used_port_id: Some("out".to_string()),
            error: Some(error_detail),
        };

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert!(report.error.is_some());
        assert!(report.output_data.is_none());
    }

    #[test]
    fn test_flow_execution_report_serialization_with_default_stats() {
        let node_report = NodeExecutionReport {
            node_uuid: "node-2".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP Node".to_string(),
            start_time: 1234567910,
            end_time: 1234567920,
            duration_ms: 10,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: None,
            used_port_id: Some("out".to_string()),
            error: None,
        };

        let flow_report = FlowExecutionReport {
            flow_uuid: "flow-456".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1234567910,
            end_time: 1234567930,
            duration_ms: 20,
            total_nodes: 5,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![node_report],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
        };

        let json = serde_json::to_string(&flow_report).unwrap();
        assert!(json.contains("flow-456"));
        assert!(json.contains("Test Flow"));
        assert!(json.contains("success"));

        // 反序列化测试
        let deserialized: FlowExecutionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.flow_uuid, "flow-456");
        assert_eq!(deserialized.flow_name, "Test Flow");
        assert_eq!(deserialized.total_nodes, 5);
        assert_eq!(deserialized.executed_count, 1);
    }
}
