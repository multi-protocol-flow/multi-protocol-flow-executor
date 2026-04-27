// 流程模块 - 完整结构体实现

use crate::connection::Connection;
use crate::hook::FlowHook;
use crate::node::Node;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 环境变量集 — 一组命名变量（如 "dev"、"staging"、"prod"）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSet {
    /// 环境名称（如 "dev"、"staging"、"prod"）
    pub name: String,
    /// 环境变量
    pub variables: HashMap<String, serde_json::Value>,
}

/// 流程数据结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    /// 流程唯一标识符（前端生成）
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 流程级别的初始变量
    #[serde(default)]
    pub initial_variables: Option<HashMap<String, serde_json::Value>>,
    /// 环境列表（多组命名变量集，如 dev/staging/prod）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<EnvironmentSet>,
    /// 当前激活的环境名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
    /// 流程中的节点列表
    pub nodes: Vec<Node>,
    /// 流程中的连接列表
    pub connections: Vec<Connection>,
    /// 流程钩子列表（流程完成时自动触发）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Vec<FlowHook>>,
}

impl Flow {
    /// 获取当前激活环境的变量
    ///
    /// 如果设置了 `active_environment` 且存在匹配的环境，返回该环境的变量；
    /// 否则返回 `initial_variables`。
    pub fn get_active_variables(&self) -> Option<HashMap<String, serde_json::Value>> {
        if let Some(ref env_name) = self.active_environment {
            if let Some(env) = self.environments.iter().find(|e| e.name == *env_name) {
                return Some(env.variables.clone());
            }
        }
        self.initial_variables.clone()
    }
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
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        assert_eq!(flow.uuid, "test-uuid-123");
        assert_eq!(flow.name, "Test Flow");
        assert!(flow.initial_variables.is_none());
        assert!(flow.environments.is_empty());
        assert!(flow.active_environment.is_none());
        assert!(flow.nodes.is_empty());
        assert!(flow.connections.is_empty());
        assert!(flow.hooks.is_none());
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
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
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
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
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
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
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
        assert!(flow.environments.is_empty());
        assert!(flow.active_environment.is_none());
        assert!(flow.hooks.is_none());
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
        assert!(flow.environments.is_empty());
        assert!(flow.active_environment.is_none());
        assert!(flow.hooks.is_none());
    }

    // ===== 环境变量集测试 =====

    #[test]
    fn test_environment_set_serialization() {
        let env = EnvironmentSet {
            name: "dev".to_string(),
            variables: {
                let mut vars = HashMap::new();
                vars.insert("api_url".to_string(), json!("http://localhost:8080"));
                vars.insert("debug".to_string(), json!(true));
                vars
            },
        };

        let json_str = serde_json::to_string(&env).unwrap();
        let deserialized: EnvironmentSet = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.name, "dev");
        assert_eq!(deserialized.variables.get("api_url"), Some(&json!("http://localhost:8080")));
        assert_eq!(deserialized.variables.get("debug"), Some(&json!(true)));
    }

    #[test]
    fn test_flow_get_active_variables_no_environment() {
        // 没有激活环境时，返回 initial_variables
        let flow = Flow {
            uuid: "test-1".to_string(),
            name: "Test".to_string(),
            initial_variables: Some({
                let mut vars = HashMap::new();
                vars.insert("key".to_string(), json!("fallback"));
                vars
            }),
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let vars = flow.get_active_variables().unwrap();
        assert_eq!(vars.get("key"), Some(&json!("fallback")));
    }

    #[test]
    fn test_flow_get_active_variables_with_environment() {
        // 有激活环境时，返回该环境的变量
        let flow = Flow {
            uuid: "test-2".to_string(),
            name: "Test".to_string(),
            initial_variables: Some({
                let mut vars = HashMap::new();
                vars.insert("key".to_string(), json!("fallback"));
                vars
            }),
            environments: vec![
                EnvironmentSet {
                    name: "dev".to_string(),
                    variables: {
                        let mut vars = HashMap::new();
                        vars.insert("key".to_string(), json!("dev-value"));
                        vars
                    },
                },
                EnvironmentSet {
                    name: "prod".to_string(),
                    variables: {
                        let mut vars = HashMap::new();
                        vars.insert("key".to_string(), json!("prod-value"));
                        vars
                    },
                },
            ],
            active_environment: Some("dev".to_string()),
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let vars = flow.get_active_variables().unwrap();
        assert_eq!(vars.get("key"), Some(&json!("dev-value")));
    }

    #[test]
    fn test_flow_get_active_variables_environment_not_found() {
        // 激活环境名称不存在时，回退到 initial_variables
        let flow = Flow {
            uuid: "test-3".to_string(),
            name: "Test".to_string(),
            initial_variables: Some({
                let mut vars = HashMap::new();
                vars.insert("key".to_string(), json!("fallback"));
                vars
            }),
            environments: vec![],
            active_environment: Some("nonexistent".to_string()),
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let vars = flow.get_active_variables().unwrap();
        assert_eq!(vars.get("key"), Some(&json!("fallback")));
    }

    #[test]
    fn test_flow_environments_skip_serializing_when_empty() {
        let flow = Flow {
            uuid: "test-4".to_string(),
            name: "Test".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let json_str = serde_json::to_string(&flow).unwrap();
        // environments 为空时不序列化
        assert!(!json_str.contains("environments"));
        assert!(!json_str.contains("active_environment"));
    }

    #[test]
    fn test_flow_environments_deserialization_backward_compat() {
        // 旧格式 JSON（无 environments/active_environment 字段）应能正常反序列化
        let json = r#"{
            "uuid": "compat-test",
            "name": "兼容性测试",
            "initial_variables": {"key": "value"},
            "nodes": [],
            "connections": []
        }"#;

        let flow: Flow = serde_json::from_str(json).unwrap();
        assert_eq!(flow.uuid, "compat-test");
        assert!(flow.environments.is_empty());
        assert!(flow.active_environment.is_none());
        assert_eq!(
            flow.initial_variables.as_ref().unwrap().get("key"),
            Some(&json!("value"))
        );
    }
}
