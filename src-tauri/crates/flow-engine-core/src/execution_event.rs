//! 正常执行模式事件类型定义
//!
//! 在正常（非调试）执行模式下，每完成一个节点就实时推送事件到前端

use crate::report::{FlowExecutionReport, NodeExecutionReport};
use serde::{Deserialize, Serialize};

/// 执行事件枚举（正常模式）
///
/// 四个变体覆盖整个流程执行生命周期：
/// - `FlowStarted`: 流程start execution时
/// - `NodeStarted`: 每个节点start execution时
/// - `NodeCompleted`: 每个节点execution complete时（Success或Failed）
/// - `FlowCompleted`: 流程执行结束时
///
/// # 子流程嵌套
///
/// 子流程在宿主中被视为**单个节点**：其内部节点的执行事件同样流经父流程的
/// 事件通道，但会带上 `parentNodeUuid`（触发该子流程的 `SubFlow` 节点 UUID）。
/// 前端据此把嵌套事件归属到 `SubFlow` 节点下，而不计入顶层流程的节点计数、
/// 也不会用嵌套的 `FlowStarted`/`FlowCompleted` 重置顶层运行状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionEvent {
    /// 流程start execution
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
        /// 所属 `SubFlow` 调用节点 UUID；顶层流程为 `None`
        #[serde(
            rename = "parentNodeUuid",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        parent_node_uuid: Option<String>,
    },
    /// 节点start execution
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
        /// 所属 `SubFlow` 调用节点 UUID；顶层流程为 `None`
        #[serde(
            rename = "parentNodeUuid",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        parent_node_uuid: Option<String>,
    },
    /// 节点execution complete
    NodeCompleted {
        /// 节点执行报告
        report: Box<NodeExecutionReport>,
        /// 所属 `SubFlow` 调用节点 UUID；顶层流程为 `None`
        #[serde(
            rename = "parentNodeUuid",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        parent_node_uuid: Option<String>,
    },
    /// 流程execution complete
    FlowCompleted {
        /// 流程执行报告
        report: Box<FlowExecutionReport>,
        /// 所属 `SubFlow` 调用节点 UUID；顶层流程为 `None`
        #[serde(
            rename = "parentNodeUuid",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        parent_node_uuid: Option<String>,
    },
    /// 流程被取消
    FlowCancelled {
        #[serde(rename = "flowUuid")]
        /// 流程唯一标识
        flow_uuid: String,
        /// 取消原因
        reason: String,
        /// 所属 `SubFlow` 调用节点 UUID；顶层流程为 `None`
        #[serde(
            rename = "parentNodeUuid",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        parent_node_uuid: Option<String>,
    },
    /// 插件节点流式事件（`SidecarPlugin` 运行时推送）
    ///
    /// 插件在 execute 期间通过事件通道上报的实时事件（如日志、中间数据）。
    /// M3.1 仅落地变体；P1 阶段前端消费者在 M4 接入。
    PluginEvent {
        #[serde(rename = "flowUuid")]
        /// 流程唯一标识
        flow_uuid: String,
        #[serde(rename = "nodeUuid")]
        /// 节点唯一标识
        node_uuid: String,
        #[serde(rename = "nodeType")]
        /// 节点类型
        node_type: String,
        #[serde(rename = "eventType")]
        /// 事件类型
        event_type: String,
        /// 事件数据
        data: serde_json::Value,
        /// 事件时间戳
        timestamp: String,
    },
    /// 批量执行事件（用于高频执行合并推送，削减 IPC 调用频次）
    Batch(Vec<ExecutionEvent>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ExecutionStatus;
    use crate::statistics::FlowStatistics;

    /// 测试 `FlowStarted` 序列化使用 camelCase
    #[test]
    fn test_flow_started_camel_case() {
        let event = ExecutionEvent::FlowStarted {
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            total_nodes: 5,
            parent_node_uuid: None,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));

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

    /// 测试 `NodeStarted` 序列化使用 camelCase
    #[test]
    fn test_node_started_camel_case() {
        let event = ExecutionEvent::NodeStarted {
            node_uuid: "node-001".to_string(),
            node_name: "HTTP 请求".to_string(),
            node_type: "http:request".to_string(),
            parent_node_uuid: None,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));

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

    /// 测试 `NodeCompleted` 序列化使用 camelCase 变体名
    #[test]
    fn test_node_completed_camel_case() {
        let report = NodeExecutionReport {
            sub_flow_report: None,
            node_uuid: "node-001".to_string(),
            node_type: "http:request".to_string(),
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
            plugin_data: None,
            is_detail_stripped: None,
        };

        let event = ExecutionEvent::NodeCompleted {
            report: Box::new(report),
            parent_node_uuid: None,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));

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

    /// 测试 `FlowCompleted` 序列化使用 camelCase 变体名
    #[test]
    fn test_flow_completed_camel_case() {
        let report = FlowExecutionReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
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
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };

        let event = ExecutionEvent::FlowCompleted {
            report: Box::new(report),
            parent_node_uuid: None,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));

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
            parent_node_uuid: None,
        };
        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(
            json,
            serde_json::to_string(&deserialized).unwrap_or_else(|e| panic!("{e}"))
        );

        // NodeStarted
        let event = ExecutionEvent::NodeStarted {
            node_uuid: "node-001".to_string(),
            node_name: "HTTP 请求".to_string(),
            node_type: "http:request".to_string(),
            parent_node_uuid: None,
        };
        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(
            json,
            serde_json::to_string(&deserialized).unwrap_or_else(|e| panic!("{e}"))
        );

        // NodeCompleted
        let report = NodeExecutionReport {
            sub_flow_report: None,
            node_uuid: "node-001".to_string(),
            node_type: "entry".to_string(),
            node_name: "Entry".to_string(),
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
            plugin_data: None,
            is_detail_stripped: None,
        };
        let event = ExecutionEvent::NodeCompleted {
            report: Box::new(report),
            parent_node_uuid: None,
        };
        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(
            json,
            serde_json::to_string(&deserialized).unwrap_or_else(|e| panic!("{e}"))
        );

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
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };
        let event = ExecutionEvent::FlowCompleted {
            report: Box::new(report),
            parent_node_uuid: None,
        };
        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(
            json,
            serde_json::to_string(&deserialized).unwrap_or_else(|e| panic!("{e}"))
        );
    }

    /// `PluginEvent` 变体 serde 往返 + camelCase 字段名
    #[test]
    fn execution_event_plugin_event() {
        let event = ExecutionEvent::PluginEvent {
            flow_uuid: "flow-001".to_string(),
            node_uuid: "node-plugin-1".to_string(),
            node_type: "plugin:demo".to_string(),
            event_type: "log".to_string(),
            data: serde_json::json!({"level": "info", "msg": "hello"}),
            timestamp: "2026-08-04T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("\"pluginEvent\""), "变体名应为 pluginEvent");
        assert!(json.contains("\"flowUuid\""), "字段名应为 flowUuid");
        assert!(json.contains("\"nodeUuid\""), "字段名应为 nodeUuid");
        assert!(json.contains("\"nodeType\""), "字段名应为 nodeType");
        assert!(json.contains("\"eventType\""), "字段名应为 eventType");
        assert!(json.contains("plugin:demo"));
        assert!(json.contains("hello"));

        // 往返
        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert!(matches!(
            &deserialized,
            ExecutionEvent::PluginEvent { event_type, .. } if event_type == "log"
        ));
        assert_eq!(
            json,
            serde_json::to_string(&deserialized).unwrap_or_else(|e| panic!("序列化Failed: {e}"))
        );
    }

    /// 嵌套事件（子流程内部节点）携带 `parentNodeUuid`；顶层事件省略该字段
    #[test]
    fn test_nested_event_parent_node_uuid() {
        let nested = ExecutionEvent::NodeStarted {
            node_uuid: "child-node-1".to_string(),
            node_name: "子流程节点".to_string(),
            node_type: "http:request".to_string(),
            parent_node_uuid: Some("subflow-node-1".to_string()),
        };
        let json = serde_json::to_string(&nested).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            json.contains("\"parentNodeUuid\""),
            "字段名应为 parentNodeUuid，实际: {json}"
        );
        assert!(json.contains("\"subflow-node-1\""));

        let root = ExecutionEvent::NodeStarted {
            node_uuid: "node-1".to_string(),
            node_name: "顶层节点".to_string(),
            node_type: "http:request".to_string(),
            parent_node_uuid: None,
        };
        let json = serde_json::to_string(&root).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            !json.contains("parentNodeUuid"),
            "顶层事件不应包含 parentNodeUuid，实际: {json}"
        );

        // 旧版本事件（无该字段）必须仍可反序列化为顶层事件
        let legacy = r#"{"nodeStarted":{"nodeUuid":"n","nodeName":"N","nodeType":"http:request"}}"#;
        let parsed: ExecutionEvent =
            serde_json::from_str(legacy).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert!(matches!(
            parsed,
            ExecutionEvent::NodeStarted {
                ref parent_node_uuid,
                ..
            } if parent_node_uuid.is_none()
        ));
    }

    /// 测试 `Batch` 批量执行事件序列化与反序列化
    #[test]
    fn test_batch_execution_event() {
        let event1 = ExecutionEvent::NodeStarted {
            node_uuid: "node-1".to_string(),
            node_name: "节点1".to_string(),
            node_type: "http:request".to_string(),
            parent_node_uuid: None,
        };
        let event2 = ExecutionEvent::FlowCancelled {
            flow_uuid: "flow-1".to_string(),
            reason: "取消".to_string(),
            parent_node_uuid: None,
        };
        let batch = ExecutionEvent::Batch(vec![event1, event2]);
        let json = serde_json::to_string(&batch).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            json.starts_with(r#"{"batch":["#),
            "应当序列化为 camelCase batch 数组: {json}"
        );

        let deserialized: ExecutionEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        match deserialized {
            ExecutionEvent::Batch(events) => {
                assert_eq!(events.len(), 2);
            }
            _ => panic!("应当反序列化为 ExecutionEvent::Batch"),
        }
    }
}
