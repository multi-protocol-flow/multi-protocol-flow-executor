// 报告结构定义模块

/// 执行状态枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 执行跳过
    Skipped,
}

/// 节点执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionReport {
    /// 节点 ID
    pub node_id: String,
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
    /// 输入数据
    pub input_data: Option<serde_json::Value>,
    /// 输出数据
    pub output_data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

/// 流程执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 流程 ID
    pub flow_id: String,
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
    pub executed_nodes: u32,
    /// 整体执行状态
    pub status: ExecutionStatus,
    /// 节点执行报告列表
    pub node_reports: Vec<NodeExecutionReport>,
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
        assert_eq!(
            serde_json::to_string(&ExecutionStatus::Skipped).unwrap(),
            r#""skipped""#
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
        assert_eq!(
            serde_json::from_str::<ExecutionStatus>(r#""skipped""#).unwrap(),
            ExecutionStatus::Skipped
        );
    }

    #[test]
    fn test_node_execution_report_creation() {
        let report = NodeExecutionReport {
            node_id: "node-123".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP Request".to_string(),
            start_time: 1234567890,
            end_time: 1234567895,
            duration_ms: 5,
            status: ExecutionStatus::Success,
            input_data: Some(json!({"url": "https://example.com"})),
            output_data: Some(json!({"status": 200})),
            error: None,
        };

        assert_eq!(report.node_id, "node-123");
        assert_eq!(report.node_type, "http");
        assert_eq!(report.node_name, "HTTP Request");
        assert_eq!(report.start_time, 1234567890);
        assert_eq!(report.end_time, 1234567895);
        assert_eq!(report.duration_ms, 5);
        assert_eq!(report.status, ExecutionStatus::Success);
        assert!(report.input_data.is_some());
        assert!(report.output_data.is_some());
        assert!(report.error.is_none());
    }

    #[test]
    fn test_node_execution_report_serialization() {
        let report = NodeExecutionReport {
            node_id: "node-456".to_string(),
            node_type: "conditional".to_string(),
            node_name: "Conditional Node".to_string(),
            start_time: 1234567900,
            end_time: 1234567910,
            duration_ms: 10,
            status: ExecutionStatus::Failed,
            input_data: None,
            output_data: None,
            error: Some("Condition evaluation failed".to_string()),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("node-456"));
        assert!(json.contains("conditional"));
        assert!(json.contains("failed"));
        assert!(json.contains("Condition evaluation failed"));

        // 反序列化测试
        let deserialized: NodeExecutionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_id, "node-456");
        assert_eq!(deserialized.node_type, "conditional");
        assert_eq!(deserialized.status, ExecutionStatus::Failed);
        assert_eq!(
            deserialized.error,
            Some("Condition evaluation failed".to_string())
        );
    }

    #[test]
    fn test_flow_execution_report_creation() {
        let node_report = NodeExecutionReport {
            node_id: "node-1".to_string(),
            node_type: "start".to_string(),
            node_name: "Start Node".to_string(),
            start_time: 1234567890,
            end_time: 1234567891,
            duration_ms: 1,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(json!({"output": "success"})),
            error: None,
        };

        let flow_report = FlowExecutionReport {
            flow_id: "flow-123".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1234567890,
            end_time: 1234567900,
            duration_ms: 10,
            total_nodes: 1,
            executed_nodes: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![node_report],
        };

        assert_eq!(flow_report.flow_id, "flow-123");
        assert_eq!(flow_report.flow_name, "Test Flow");
        assert_eq!(flow_report.total_nodes, 1);
        assert_eq!(flow_report.executed_nodes, 1);
        assert_eq!(flow_report.node_reports.len(), 1);
        assert_eq!(flow_report.node_reports[0].node_id, "node-1");
    }

    #[test]
    fn test_flow_execution_report_serialization() {
        let node_report = NodeExecutionReport {
            node_id: "node-2".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP Node".to_string(),
            start_time: 1234567910,
            end_time: 1234567920,
            duration_ms: 10,
            status: ExecutionStatus::Skipped,
            input_data: None,
            output_data: None,
            error: None,
        };

        let flow_report = FlowExecutionReport {
            flow_id: "flow-456".to_string(),
            flow_name: "Another Flow".to_string(),
            start_time: 1234567910,
            end_time: 1234567930,
            duration_ms: 20,
            total_nodes: 5,
            executed_nodes: 1,
            status: ExecutionStatus::Skipped,
            node_reports: vec![node_report],
        };

        let json = serde_json::to_string(&flow_report).unwrap();
        assert!(json.contains("flow-456"));
        assert!(json.contains("Another Flow"));
        assert!(json.contains("skipped"));

        // 反序列化测试
        let deserialized: FlowExecutionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.flow_id, "flow-456");
        assert_eq!(deserialized.flow_name, "Another Flow");
        assert_eq!(deserialized.total_nodes, 5);
        assert_eq!(deserialized.executed_nodes, 1);
        assert_eq!(deserialized.status, ExecutionStatus::Skipped);
    }

    #[test]
    fn test_execution_status_equality() {
        assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
        assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);
        assert_eq!(ExecutionStatus::Skipped, ExecutionStatus::Skipped);
        assert_ne!(ExecutionStatus::Success, ExecutionStatus::Failed);
        assert_ne!(ExecutionStatus::Failed, ExecutionStatus::Skipped);
        assert_ne!(ExecutionStatus::Success, ExecutionStatus::Skipped);
    }

    #[test]
    fn test_node_execution_report_with_error() {
        let report = NodeExecutionReport {
            node_id: "node-error".to_string(),
            node_type: "http".to_string(),
            node_name: "Failed HTTP Request".to_string(),
            start_time: 1234568000,
            end_time: 1234568050,
            duration_ms: 50,
            status: ExecutionStatus::Failed,
            input_data: Some(json!({"url": "https://invalid.com"})),
            output_data: None,
            error: Some("Connection timeout".to_string()),
        };

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert_eq!(report.error, Some("Connection timeout".to_string()));
        assert!(report.output_data.is_none());
    }

    #[test]
    fn test_node_execution_report_cloning() {
        let report = NodeExecutionReport {
            node_id: "node-clone".to_string(),
            node_type: "start".to_string(),
            node_name: "Start Node".to_string(),
            start_time: 1234569000,
            end_time: 1234569001,
            duration_ms: 1,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: None,
            error: None,
        };

        let cloned = report.clone();
        assert_eq!(cloned.node_id, report.node_id);
        assert_eq!(cloned.status, report.status);
    }
}
