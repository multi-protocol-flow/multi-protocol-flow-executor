//! ExecutionStackEntry 新字段测试
//!
//! 测试 Task 8: 验证 ExecutionStackEntry 新增的 3 个字段

use flow_engine::debug::ExecutionStackEntry;
use flow_engine::report::VariableChange;
use serde_json::json;

/// 测试所有新字段存在并可正常序列化/反序列化
#[test]
fn test_debug_stack_entry_fields_exist() {
    let entry = ExecutionStackEntry {
        node_uuid: "node-001".to_string(),
        node_name: "HTTP Request".to_string(),
        node_type: "http".to_string(),
        started_at: 1234567890,
        ended_at: Some(1234567895),
        input: Some(json!({"url": "https://example.com"})),
        output: Some(json!({"status": 200})),
        success: true,
        error: None,
        used_port_id: Some("out".to_string()),
        // 新字段
        resolved_config: Some(json!({"url": "https://api.example.com/users", "method": "get"})),
        variable_changes: Some(vec![
            VariableChange {
                key: "status".to_string(),
                old_value: None,
                new_value: json!(200),
            },
            VariableChange {
                key: "body".to_string(),
                old_value: Some(json!({"old": "data"})),
                new_value: json!({"new": "data"}),
            },
        ]),
        skip_reason: Some("condition not met".to_string()),
    };

    // serialize
    let json_str = serde_json::to_string(&entry).expect("serialization failed");

    // verify new fields exist in JSON
    assert!(json_str.contains("resolved_config"));
    assert!(json_str.contains("variable_changes"));
    assert!(json_str.contains("skip_reason"));
    assert!(json_str.contains("condition not met"));

    // deserialize
    let deserialized: ExecutionStackEntry =
        serde_json::from_str(&json_str).expect("deserialization failed");

    // verify new field values
    assert!(deserialized.resolved_config.is_some());
    assert!(deserialized.variable_changes.is_some());
    assert!(deserialized.skip_reason.is_some());

    let changes = deserialized
        .variable_changes
        .expect("variable_changes should be present after deserialization");
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].key, "status");
    assert_eq!(changes[1].key, "body");

    assert_eq!(
        deserialized.skip_reason,
        Some("condition not met".to_string())
    );
}

/// 测试向后兼容性：旧版 JSON（不包含新字段）可正常反序列化
#[test]
fn test_debug_stack_entry_backward_compat() {
    // 旧版 JSON（没有新字段）
    let old_json = r#"{
        "node_uuid": "node-001",
        "node_name": "HTTP Request",
        "node_type": "http",
        "started_at": 1234567890,
        "ended_at": 1234567895,
        "input": {"url": "https://example.com"},
        "output": {"status": 200},
        "success": true,
        "error": null,
        "used_port_id": "out"
    }"#;

    // 反序列化应该成功，新字段使用默认值
    let entry: ExecutionStackEntry =
        serde_json::from_str(old_json).expect("legacy JSON deserialization failed");

    // 验证基本字段
    assert_eq!(entry.node_uuid, "node-001");
    assert_eq!(entry.node_name, "HTTP Request");
    assert_eq!(entry.node_type, "http");
    assert!(entry.success);

    // 验证新字段为 None（默认值）
    assert!(entry.resolved_config.is_none());
    assert!(entry.variable_changes.is_none());
    assert!(entry.skip_reason.is_none());
}

/// 测试 skip_serializing_if：None 值不在 JSON 中出现
#[test]
fn test_debug_stack_entry_skip_serializing_if() {
    let entry = ExecutionStackEntry {
        node_uuid: "node-001".to_string(),
        node_name: "Test".to_string(),
        node_type: "entry".to_string(),
        started_at: 1000,
        ended_at: Some(1005),
        input: None,
        output: None,
        success: true,
        error: None,
        used_port_id: None,
        // 所有新字段为 None
        resolved_config: None,
        variable_changes: None,
        skip_reason: None,
    };

    let json_str = serde_json::to_string(&entry).expect("serialization failed");

    // None-valued new fields should not appear in JSON
    // Note: used_port_id uses #[serde(default)], not skip_serializing_if, so it appears in JSON
    assert!(!json_str.contains("resolved_config"));
    assert!(!json_str.contains("variable_changes"));
    assert!(!json_str.contains("skip_reason"));
}

/// 测试部分字段有值的情况
#[test]
fn test_debug_stack_entry_partial_fields() {
    let entry = ExecutionStackEntry {
        node_uuid: "node-002".to_string(),
        node_name: "Conditional".to_string(),
        node_type: "conditional".to_string(),
        started_at: 2000,
        ended_at: Some(2005),
        input: Some(json!({"condition": "{{status}} == 200"})),
        output: None,
        success: true,
        error: None,
        used_port_id: Some("true".to_string()),
        // 只有 resolved_config 有值
        resolved_config: Some(json!({"condition": "200 == 200", "result": true})),
        variable_changes: None,
        skip_reason: None,
    };

    let json_str = serde_json::to_string(&entry).expect("serialization failed");

    // resolved_config should appear
    assert!(json_str.contains("resolved_config"));
    // variable_changes and skip_reason should not appear (None)
    assert!(!json_str.contains("variable_changes"));
    assert!(!json_str.contains("skip_reason"));

    // verify deserialization
    let deserialized: ExecutionStackEntry =
        serde_json::from_str(&json_str).expect("deserialization failed");

    assert!(deserialized.resolved_config.is_some());
    assert!(deserialized.variable_changes.is_none());
    assert!(deserialized.skip_reason.is_none());
}

/// 测试完整场景：所有字段都有值
#[test]
fn test_debug_stack_entry_full_scenario() {
    let entry = ExecutionStackEntry {
        node_uuid: "node-003".to_string(),
        node_name: "Set Variable".to_string(),
        node_type: "set_variable".to_string(),
        started_at: 3000,
        ended_at: Some(3010),
        input: Some(json!({"key": "counter", "value": 42})),
        output: Some(json!({"counter": 42})),
        success: true,
        error: None,
        used_port_id: Some("out".to_string()),
        resolved_config: Some(json!({"key": "counter", "value": 42})),
        variable_changes: Some(vec![VariableChange {
            key: "counter".to_string(),
            old_value: None,
            new_value: json!(42),
        }]),
        skip_reason: None,
    };

    // 完整往返测试
    let json_str = serde_json::to_string(&entry).expect("serialization failed");
    let deserialized: ExecutionStackEntry =
        serde_json::from_str(&json_str).expect("deserialization failed");

    // verify all fields
    assert_eq!(deserialized.node_uuid, "node-003");
    assert_eq!(deserialized.node_name, "Set Variable");
    assert_eq!(deserialized.node_type, "set_variable");
    assert_eq!(deserialized.started_at, 3000);
    assert_eq!(deserialized.ended_at, Some(3010));
    assert!(deserialized.success);
    assert!(deserialized.resolved_config.is_some());
    assert!(deserialized.variable_changes.is_some());

    let changes = deserialized
        .variable_changes
        .expect("variable_changes should be present in full scenario");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].key, "counter");
    assert_eq!(changes[0].old_value, None);
    assert_eq!(changes[0].new_value, json!(42));
}
