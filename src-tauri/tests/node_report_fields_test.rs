// NodeExecutionReport 新字段测试
#![allow(missing_docs)]

use flow_engine::report::{ExecutionStatus, NodeExecutionReport, VariableChange};
use serde_json::json;

/// 测试所有新字段存在并可正常使用
#[test]
fn test_node_report_fields_exist() {
    // 创建包含所有新字段的 NodeExecutionReport
    let report = NodeExecutionReport {
        node_uuid: "test-uuid".to_string(),
        node_type: "http".to_string(),
        node_name: "测试节点".to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: ExecutionStatus::Success,
        input_data: Some(json!({"key": "value"})),
        output_data: Some(json!({"result": "ok"})),
        used_port_id: Some("out".to_string()),
        error: None,
        // 新字段
        resolved_config: Some(json!({"url": "https://example.com"})),
        variable_changes: Some(vec![VariableChange {
            key: "response".to_string(),
            old_value: None,
            new_value: json!({"status": 200}),
        }]),
        skip_reason: None,
        wait_duration_ms: Some(500),
    };

    // 验证新字段值
    assert!(report.resolved_config.is_some());
    assert!(report.variable_changes.is_some());
    assert_eq!(report.variable_changes.as_ref().unwrap().len(), 1);
    assert!(report.skip_reason.is_none());
    assert_eq!(report.wait_duration_ms, Some(500));

    // 验证序列化包含新字段
    let json_str = serde_json::to_string(&report).expect("serialization failed");
    assert!(json_str.contains("resolved_config"));
    assert!(json_str.contains("variable_changes"));
    assert!(json_str.contains("wait_duration_ms"));
}

/// 测试向后兼容性：旧版 JSON（无新字段）可反序列化
#[test]
fn test_node_report_backward_compat() {
    // 模拟旧版 JSON（不包含新字段）
    let old_json = json!({
        "node_uuid": "old-uuid",
        "node_type": "entry",
        "node_name": "入口节点",
        "start_time": 1000,
        "end_time": 2000,
        "duration_ms": 1000,
        "status": "success",
        "input_data": null,
        "output_data": null,
        "used_port_id": "out",
        "error": null
    });

    // 反序列化应该成功，新字段使用默认值
    let report: NodeExecutionReport =
        serde_json::from_value(old_json).expect("legacy JSON deserialization failed");

    // 验证基本字段
    assert_eq!(report.node_uuid, "old-uuid");
    assert_eq!(report.node_type, "entry");
    assert_eq!(report.node_name, "入口节点");
    assert_eq!(report.status, ExecutionStatus::Success);

    // 验证新字段为默认值（None）
    assert!(report.resolved_config.is_none());
    assert!(report.variable_changes.is_none());
    assert!(report.skip_reason.is_none());
    assert!(report.wait_duration_ms.is_none());
}

/// 测试新字段的 skip_serializing_if 行为：值为 None 时不序列化
#[test]
fn test_node_report_skip_serializing_if() {
    // 创建所有新字段为 None 的报告
    let report = NodeExecutionReport {
        node_uuid: "test-uuid".to_string(),
        node_type: "http".to_string(),
        node_name: "测试".to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: ExecutionStatus::Success,
        input_data: None,
        output_data: None,
        used_port_id: None,
        error: None,
        // 新字段全部为 None
        resolved_config: None,
        variable_changes: None,
        skip_reason: None,
        wait_duration_ms: None,
    };

    // 序列化
    let json_str = serde_json::to_string(&report).expect("serialization failed");

    // 新字段不应出现在 JSON 中
    assert!(!json_str.contains("resolved_config"));
    assert!(!json_str.contains("variable_changes"));
    assert!(!json_str.contains("skip_reason"));
    assert!(!json_str.contains("wait_duration_ms"));
}

/// 测试 Skipped 状态节点的 skip_reason 字段
#[test]
fn test_node_report_skipped_with_reason() {
    let report = NodeExecutionReport {
        node_uuid: "skipped-uuid".to_string(),
        node_type: "http".to_string(),
        node_name: "被跳过的节点".to_string(),
        start_time: 1000,
        end_time: 1000,
        duration_ms: 0,
        status: ExecutionStatus::Skipped,
        input_data: None,
        output_data: None,
        used_port_id: None,
        error: None,
        resolved_config: None,
        variable_changes: None,
        skip_reason: Some("condition not met".to_string()),
        wait_duration_ms: None,
    };

    // 验证序列化包含 skip_reason
    let json_str = serde_json::to_string(&report).expect("serialization failed");
    assert!(json_str.contains("skip_reason"));
    assert!(json_str.contains("condition not met"));

    // 反序列化验证
    let deserialized: NodeExecutionReport = serde_json::from_str(&json_str).expect("deserialization failed");
    assert_eq!(deserialized.skip_reason, Some("condition not met".to_string()));
}
