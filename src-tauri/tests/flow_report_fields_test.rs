// FlowExecutionReport 新字段测试
#![allow(missing_docs)]

use flow_engine::report::{ExecutionMode, ExecutionStatus, FlowExecutionReport, SkippedNodeInfo};
use flow_engine::statistics::FlowStatistics;
use serde_json::json;

/// 测试所有新字段存在
#[test]
fn test_flow_report_fields_exist() {
    let report = FlowExecutionReport {
        uuid: "test-uuid".to_string(),
        flow_uuid: "flow-uuid".to_string(),
        flow_name: "Test Flow".to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        total_nodes: 2,
        executed_count: 2,
        status: ExecutionStatus::Success,
        node_reports: vec![],
        statistics: FlowStatistics::default(),
        flow_error: None,
        execution_mode: ExecutionMode::Normal,
        // 新字段
        initial_variables: Some(json!({"var1": "value1"})),
        final_variables: Some(json!({"var1": "value1", "var2": "value2"})),
        execution_path: vec!["node-1".to_string(), "node-2".to_string()],
        skipped_nodes: vec![],
    };

    // 验证新字段可以正常访问
    assert!(report.initial_variables.is_some());
    assert!(report.final_variables.is_some());
    assert_eq!(report.execution_path.len(), 2);
    assert_eq!(report.skipped_nodes.len(), 0);
}

/// 测试 SkippedNodeInfo 结构
#[test]
fn test_skipped_node_info() {
    let info = SkippedNodeInfo {
        node_uuid: "node-123".to_string(),
        node_name: "Test Node".to_string(),
        reason: "Condition not met".to_string(),
    };

    assert_eq!(info.node_uuid, "node-123");
    assert_eq!(info.node_name, "Test Node");
    assert_eq!(info.reason, "Condition not met");

    // 测试序列化
    let json = serde_json::to_string(&info).expect("Failed to serialize");
    assert!(json.contains("node-123"));
    assert!(json.contains("Test Node"));
    assert!(json.contains("Condition not met"));

    // 测试反序列化
    let deserialized: SkippedNodeInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.node_uuid, info.node_uuid);
    assert_eq!(deserialized.node_name, info.node_name);
    assert_eq!(deserialized.reason, info.reason);
}

/// 测试空执行路径
#[test]
fn test_flow_report_empty_path() {
    let report = FlowExecutionReport {
        uuid: "test-uuid".to_string(),
        flow_uuid: "flow-uuid".to_string(),
        flow_name: "Empty Flow".to_string(),
        start_time: 1000,
        end_time: 1000,
        duration_ms: 0,
        total_nodes: 0,
        executed_count: 0,
        status: ExecutionStatus::Success,
        node_reports: vec![],
        statistics: FlowStatistics::default(),
        flow_error: None,
        execution_mode: ExecutionMode::Normal,
        // 空路径和空跳过节点
        initial_variables: None,
        final_variables: None,
        execution_path: vec![],
        skipped_nodes: vec![],
    };

    // 验证空路径
    assert_eq!(report.execution_path.len(), 0);
    assert_eq!(report.skipped_nodes.len(), 0);

    // 测试序列化
    let json = serde_json::to_string(&report).expect("Failed to serialize");

    // 空的 Vec 应该序列化为 []
    assert!(json.contains("\"execution_path\":[]"));
    assert!(json.contains("\"skipped_nodes\":[]"));

    // None 的 Option 应该被跳过（skip_serializing_if）
    assert!(!json.contains("\"initial_variables\""));
    assert!(!json.contains("\"final_variables\""));
}

/// 测试向后兼容性（缺少新字段的 JSON 仍能反序列化）
#[test]
fn test_flow_report_backward_compat() {
    // 旧版 JSON（不包含新字段）
    let old_json = json!({
        "uuid": "test-uuid",
        "flow_uuid": "flow-uuid",
        "flow_name": "Old Flow",
        "start_time": 1000,
        "end_time": 2000,
        "duration_ms": 1000,
        "total_nodes": 1,
        "executed_count": 1,
        "status": "success",
        "node_reports": [],
        "statistics": {
            "success_count": 1,
            "failed_count": 0,
            "avg_node_duration_ms": 1000,
            "max_node_duration_ms": 1000,
            "min_node_duration_ms": 1000,
            "total_input_size_bytes": 0,
            "total_output_size_bytes": 0,
            "error_rate": 0.0,
            "completion_rate": 1.0
        },
        "flow_error": null,
        "execution_mode": "normal"
    });

    // 反序列化应该成功
    let report: FlowExecutionReport =
        serde_json::from_value(old_json).expect("Failed to deserialize old JSON");

    // 新字段应该有默认值
    assert_eq!(report.uuid, "test-uuid");
    assert_eq!(report.flow_name, "Old Flow");
    assert!(report.initial_variables.is_none());
    assert!(report.final_variables.is_none());
    assert_eq!(report.execution_path.len(), 0);
    assert_eq!(report.skipped_nodes.len(), 0);
}
