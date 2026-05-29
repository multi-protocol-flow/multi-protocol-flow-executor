//! 正常执行模式事件类型定义
//!
//! 在正常（非调试）执行模式下，每完成一个节点就实时推送事件到前端

use crate::report::{FlowExecutionReport, NodeExecutionReport};
use serde::{Deserialize, Serialize};

/// 执行事件枚举（正常模式）
///
/// 四个变体覆盖整个流程执行生命周期：
/// - `FlowStarted`: 流程开始执行时
/// - `NodeStarted`: 每个节点开始执行时
/// - `NodeCompleted`: 每个节点执行完成时（成功或失败）
/// - `FlowCompleted`: 流程执行结束时
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionEvent {
    /// 流程开始执行
    FlowStarted {
        #[serde(rename = "flowUuid")]
        /// 流程唯一标识
        flow_uuid: String,
        #[serde(rename = "flowName")]
        /// 流程名称
        flow_name: String,
        #[serde(rename = "totalNodes")]
        /// 总节点数
        total_nodes: u32,
    },
    /// 节点开始执行
    NodeStarted {
        #[serde(rename = "nodeUuid")]
        /// 节点唯一标识
        node_uuid: String,
        #[serde(rename = "nodeName")]
        /// 节点名称
        node_name: String,
        #[serde(rename = "nodeType")]
        /// 节点类型
        node_type: String,
    },
    /// 节点执行完成
    NodeCompleted {
        /// 节点执行报告
        report: Box<NodeExecutionReport>,
    },
    /// 流程执行完成
    FlowCompleted {
        /// 流程执行报告
        report: Box<FlowExecutionReport>,
    },
    /// 流程被取消
    FlowCancelled {
        #[serde(rename = "flowUuid")]
        /// 流程唯一标识
        flow_uuid: String,
        /// 取消原因
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ExecutionStatus;
    use crate::statistics::FlowStatistics;

    /// 测试 FlowStarted 序列化使用 camelCase
    #[test]
    fn test_flow_started_camel_case() {
        let event = ExecutionEvent::FlowStarted {
            flow_uuid: "flow-001".to_string(),
            flow_name: "测试流程".to_string(),
            total_nodes: 5,
        };

        let json = serde_json::to_string(&event).expect("序列化失败");

        // 验证变体名使用 camelCase
        assert!(json.contains("\"flowStarted\""), "变体名应为 flowStarted");
        // 验证字段名使用 camelCase
        assert!(
            json.contains("\"flowUuid\""),
            "字段名应为 flowUuid，实际: {}",
            json
        );
        assert!(
            json.contains("\"flowName\""),
            "字段名应为 flowName，实际: {}",
            json
        );
        assert!(
            json.contains("\"totalNodes\""),
            "字段名应为 totalNodes，实际: {}",
            json
        );
    }

    /// 测试 NodeStarted 序列化使用 camelCase
    #[test]
    fn test_node_started_camel_case() {
        let event = ExecutionEvent::NodeStarted {
            node_uuid: "node-001".to_string(),
            node_name: "HTTP 请求".to_string(),
            node_type: "http".to_string(),
        };

        let json = serde_json::to_string(&event).expect("序列化失败");

        // 验证变体名使用 camelCase
        assert!(
            json.contains("\"nodeStarted\""),
            "变体名应为 nodeStarted，实际: {}",
            json
        );
        // 验证字段名使用 camelCase
        assert!(
            json.contains("\"nodeUuid\""),
            "字段名应为 nodeUuid，实际: {}",
            json
        );
        assert!(
            json.contains("\"nodeName\""),
            "字段名应为 nodeName，实际: {}",
            json
        );
        assert!(
            json.contains("\"nodeType\""),
            "字段名应为 nodeType，实际: {}",
            json
        );
    }

    /// 测试 NodeCompleted 序列化使用 camelCase 变体名
    #[test]
    fn test_node_completed_camel_case() {
        let report = NodeExecutionReport {
            node_uuid: "node-001".to_string(),
            node_type: "http".to_string(),
            node_name: "HTTP 请求".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: None,
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
        };

        let event = ExecutionEvent::NodeCompleted {
            report: Box::new(report),
        };

        let json = serde_json::to_string(&event).expect("序列化失败");

        // 验证变体名使用 camelCase（而非 snake_case）
        assert!(
            json.contains("\"nodeCompleted\""),
            "变体名应为 nodeCompleted，实际: {}",
            json
        );
        // 不应包含 snake_case 格式
        assert!(
            !json.contains("\"node_completed\""),
            "不应包含 node_completed"
        );
    }

    /// 测试 FlowCompleted 序列化使用 camelCase 变体名
    #[test]
    fn test_flow_completed_camel_case() {
        let report = FlowExecutionReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "测试流程".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: FlowStatistics::default(),
            flow_error: None,
            execution_mode: crate::report::ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };

        let event = ExecutionEvent::FlowCompleted {
            report: Box::new(report),
        };

        let json = serde_json::to_string(&event).expect("序列化失败");

        // 验证变体名使用 camelCase
        assert!(
            json.contains("\"flowCompleted\""),
            "变体名应为 flowCompleted，实际: {}",
            json
        );
    }

    /// 测试所有变体的序列化/反序列化往返
    #[test]
    fn test_round_trip_all_variants() {
        // FlowStarted
        let event = ExecutionEvent::FlowStarted {
            flow_uuid: "flow-001".to_string(),
            flow_name: "测试".to_string(),
            total_nodes: 3,
        };
        let json = serde_json::to_string(&event).expect("序列化失败");
        let deserialized: ExecutionEvent = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(json, serde_json::to_string(&deserialized).unwrap());

        // NodeStarted
        let event = ExecutionEvent::NodeStarted {
            node_uuid: "node-001".to_string(),
            node_name: "HTTP 请求".to_string(),
            node_type: "http".to_string(),
        };
        let json = serde_json::to_string(&event).expect("序列化失败");
        let deserialized: ExecutionEvent = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(json, serde_json::to_string(&deserialized).unwrap());

        // NodeCompleted
        let report = NodeExecutionReport {
            node_uuid: "node-001".to_string(),
            node_type: "entry".to_string(),
            node_name: "入口".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: None,
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
        };
        let event = ExecutionEvent::NodeCompleted {
            report: Box::new(report),
        };
        let json = serde_json::to_string(&event).expect("序列化失败");
        let deserialized: ExecutionEvent = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(json, serde_json::to_string(&deserialized).unwrap());

        // FlowCompleted
        let report = FlowExecutionReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "测试".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: FlowStatistics::default(),
            flow_error: None,
            execution_mode: crate::report::ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };
        let event = ExecutionEvent::FlowCompleted {
            report: Box::new(report),
        };
        let json = serde_json::to_string(&event).expect("序列化失败");
        let deserialized: ExecutionEvent = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(json, serde_json::to_string(&deserialized).unwrap());
    }
}
