// VariableChange 结构体测试
#![allow(missing_docs)]

use serde_json::json;

/// 测试 VariableChange 序列化/反序列化
#[test]
fn test_variable_change_serde() {
    let change = flow_engine::report::VariableChange {
        key: "test_var".to_string(),
        old_value: Some(json!("old")),
        new_value: json!("new"),
    };

    // 序列化
    let json_str =
        serde_json::to_string(&change).expect("VariableChange serialization should succeed");

    // 反序列化
    let deserialized: flow_engine::report::VariableChange =
        serde_json::from_str(&json_str).expect("VariableChange deserialization should succeed");

    assert_eq!(change, deserialized);
}

/// 测试新创建的变量（old_value 为 None）
#[test]
fn test_variable_change_new_var() {
    let change = flow_engine::report::VariableChange {
        key: "new_var".to_string(),
        old_value: None,
        new_value: json!({"status": 200}),
    };

    // 序列化后检查 JSON 不包含 old_value 字段
    let json_str =
        serde_json::to_string(&change).expect("VariableChange serialization should succeed");
    assert!(!json_str.contains("\"old_value\":null"));

    // 反序列化
    let deserialized: flow_engine::report::VariableChange =
        serde_json::from_str(&json_str).expect("VariableChange deserialization should succeed");

    assert_eq!(change.key, deserialized.key);
    assert_eq!(change.old_value, None);
    assert_eq!(change.new_value, deserialized.new_value);
}

/// 测试有旧值的场景
#[test]
fn test_variable_change_with_old_value() {
    let change = flow_engine::report::VariableChange {
        key: "counter".to_string(),
        old_value: Some(json!(1)),
        new_value: json!(2),
    };

    // 序列化
    let json_str =
        serde_json::to_string(&change).expect("VariableChange serialization should succeed");

    // 反序列化
    let deserialized: flow_engine::report::VariableChange =
        serde_json::from_str(&json_str).expect("VariableChange deserialization should succeed");

    assert_eq!(change.key, deserialized.key);
    assert_eq!(change.old_value, Some(json!(1)));
    assert_eq!(change.new_value, json!(2));
}
