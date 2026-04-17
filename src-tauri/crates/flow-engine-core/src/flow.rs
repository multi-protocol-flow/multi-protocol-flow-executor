// 流程模块 - 完整结构体实现

use crate::connection::Connection;
use crate::node::Node;
use std::collections::HashMap;

/// 流程数据结构
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Flow {
    /// 流程唯一标识符（前端生成）
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 流程级别的初始变量
    #[serde(default)]
    pub initial_variables: Option<HashMap<String, serde_json::Value>>,
    /// 流程中的节点列表
    pub nodes: Vec<Node>,
    /// 流程中的连接列表
    pub connections: Vec<Connection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_flow_creation() {
        let flow = Flow {
            uuid: "test-uuid-123".to_string(),
            name: "Test Flow".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        };

        assert_eq!(flow.uuid, "test-uuid-123");
        assert_eq!(flow.name, "Test Flow");
        assert!(flow.initial_variables.is_none());
        assert!(flow.nodes.is_empty());
        assert!(flow.connections.is_empty());
    }

    #[test]
    fn test_flow_with_initial_variables() {
        let mut vars = HashMap::new();
        vars.insert("key1".to_string(), json!("value1"));
        vars.insert("key2".to_string(), json!(42));

        let flow = Flow {
            uuid: "test-uuid-456".to_string(),
            name: "Flow with Variables".to_string(),
            initial_variables: Some(vars),
            nodes: vec![],
            connections: vec![],
        };

        assert_eq!(flow.uuid, "test-uuid-456");
        assert!(flow.initial_variables.is_some());
        let vars_ref = flow.initial_variables.as_ref().unwrap();
        assert_eq!(vars_ref.len(), 2);
        assert_eq!(vars_ref.get("key1"), Some(&json!("value1")));
        assert_eq!(vars_ref.get("key2"), Some(&json!(42)));
    }

    #[test]
    fn test_flow_serialization() {
        let flow = Flow {
            uuid: "test-uuid-789".to_string(),
            name: "Serialization Test".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        };

        let json = serde_json::to_string(&flow).unwrap();
        assert!(json.contains("test-uuid-789"));
        assert!(json.contains("Serialization Test"));
    }

    #[test]
    fn test_flow_serialization_with_initial_variables() {
        let mut vars = HashMap::new();
        vars.insert("api_key".to_string(), json!("secret123"));

        let flow = Flow {
            uuid: "test-uuid-999".to_string(),
            name: "Flow with Initial Variables".to_string(),
            initial_variables: Some(vars),
            nodes: vec![],
            connections: vec![],
        };

        let json = serde_json::to_string(&flow).unwrap();
        assert!(json.contains("test-uuid-999"));
        assert!(json.contains("api_key"));
        assert!(json.contains("secret123"));

        // 反序列化测试
        let deserialized: Flow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uuid, "test-uuid-999");
        assert_eq!(deserialized.initial_variables.is_some(), true);
        let vars_ref = deserialized.initial_variables.as_ref().unwrap();
        assert_eq!(vars_ref.get("api_key"), Some(&json!("secret123")));
    }

    #[test]
    fn test_flow_deserialization_without_initial_variables() {
        let json = r#"{
            "uuid": "test-uuid-deser",
            "name": "Deserialization Test",
            "nodes": [],
            "connections": []
        }"#;

        let flow: Flow = serde_json::from_str(json).unwrap();
        assert_eq!(flow.uuid, "test-uuid-deser");
        assert_eq!(flow.name, "Deserialization Test");
        assert!(flow.initial_variables.is_none());
    }

    #[test]
    fn test_flow_default_initial_variables() {
        // 测试 initial_variables 使用 serde default 处理缺失值
        let json = r#"{
            "uuid": "test-uuid-default",
            "name": "Default Test",
            "nodes": [],
            "connections": []
        }"#;

        let flow: Flow = serde_json::from_str(json).unwrap();
        assert_eq!(flow.uuid, "test-uuid-default");
        // 如果 JSON 中没有 initial_variables 字段，serde default 应该使其为 None
        assert!(flow.initial_variables.is_none());
    }
}
