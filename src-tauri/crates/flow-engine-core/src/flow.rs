// 流程模块 - 完整结构体实现

use crate::connection::Connection;
use crate::node::Node;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 流程子图（用于 Setup 准备阶段与 Teardown 收尾阶段）
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FlowSubGraph {
    /// 子图中的节点列表
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// 子图中的连接列表
    #[serde(default)]
    pub connections: Vec<Connection>,
}

impl FlowSubGraph {
    /// 创建新的流程子图
    pub fn new(nodes: Vec<Node>, connections: Vec<Connection>) -> Self {
        Self { nodes, connections }
    }

    /// 检查子图是否为空
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// 环境变量集 — 一组命名变量（如 "dev"、"staging"、"prod"）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSet {
    /// 环境名称（如 "dev"、"staging"、"prod"）
    pub name: String,
    /// 环境描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 环境变量
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

/// 流程数据结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    /// 流程唯一标识符（前端生成）
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 流程描述（Markdown 格式）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 流程级别的初始变量（作为环境未定义时的兜底基底）
    #[serde(default)]
    pub initial_variables: Option<HashMap<String, serde_json::Value>>,
    /// Setup 准备阶段子图（可选，测试前单次执行）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<FlowSubGraph>,
    /// 流程中的节点列表（Main 核心阶段）
    pub nodes: Vec<Node>,
    /// 流程中的连接列表（Main 核心阶段）
    pub connections: Vec<Connection>,
    /// Teardown 收尾阶段子图（可选，测试后单次执行）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teardown: Option<FlowSubGraph>,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            name: String::new(),
            description: None,
            initial_variables: None,
            setup: None,
            nodes: Vec::new(),
            connections: Vec::new(),
            teardown: None,
        }
    }
}

impl Flow {
    /// 获取流程定义的初始变量（兜底基底）
    pub fn get_active_variables(&self) -> Option<HashMap<String, serde_json::Value>> {
        self.initial_variables.clone()
    }
}

/// 合并多层变量集
///
/// 优先级由低到高（后序覆盖前序）：
/// 1. `fallback`: 流程内置的初始变量（`flow.initial_variables`）
/// 2. `env`: 环境定义文件中的变量（如 `<name>.yaml`）
/// 3. `local_override`: 本地私有覆盖文件中的变量（如 `<name>.local.yaml`）
/// 4. `runtime_override`: 运行时显式传入的变量覆盖（如 CLI `--var` 或调试单步注入）
///
/// 每层只克隆键值本身，不构造中间 `HashMap`（执行热路径，避免多余堆分配）。
pub fn merge_variable_layers(
    fallback: Option<&HashMap<String, serde_json::Value>>,
    env: Option<&EnvironmentSet>,
    local_override: Option<&EnvironmentSet>,
    runtime_override: Option<&HashMap<String, serde_json::Value>>,
) -> HashMap<String, serde_json::Value> {
    fn extend_from(
        target: &mut HashMap<String, serde_json::Value>,
        source: &HashMap<String, serde_json::Value>,
    ) {
        target.extend(source.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    let mut merged = fallback.cloned().unwrap_or_default();
    if let Some(env) = env {
        extend_from(&mut merged, &env.variables);
    }
    if let Some(local) = local_override {
        extend_from(&mut merged, &local.variables);
    }
    if let Some(runtime) = runtime_override {
        extend_from(&mut merged, runtime);
    }
    merged
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
            description: None,
            initial_variables: None,
            setup: None,
            nodes: vec![],
            connections: vec![],
            teardown: None,
        };

        assert_eq!(flow.uuid, "test-uuid-123");
        assert_eq!(flow.name, "Test Flow");
        assert!(flow.initial_variables.is_none());
        assert!(flow.nodes.is_empty());
        assert!(flow.connections.is_empty());
        assert!(flow.setup.is_none());
        assert!(flow.teardown.is_none());
    }

    #[test]
    fn test_flow_with_initial_variables() {
        let mut vars = HashMap::new();
        vars.insert("key1".to_string(), json!("value1"));
        vars.insert("key2".to_string(), json!(42));

        let flow = Flow {
            uuid: "test-uuid-456".to_string(),
            name: "Flow with Variables".to_string(),
            description: None,
            initial_variables: Some(vars),
            setup: None,
            nodes: vec![],
            connections: vec![],
            teardown: None,
        };
        assert_eq!(flow.uuid, "test-uuid-456");
        assert!(flow.initial_variables.is_some());
        let vars_ref = flow
            .initial_variables
            .as_ref()
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(vars_ref.len(), 2);
        assert_eq!(vars_ref.get("key1"), Some(&json!("value1")));
        assert_eq!(vars_ref.get("key2"), Some(&json!(42)));
    }

    #[test]
    fn test_flow_serialization() {
        let flow = Flow {
            uuid: "test-uuid-789".to_string(),
            name: "Serialization Test".to_string(),
            description: None,
            initial_variables: None,
            setup: None,
            nodes: vec![],
            connections: vec![],
            teardown: None,
        };
        let json = serde_json::to_string(&flow).unwrap_or_else(|e| panic!("{e}"));
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
            description: None,
            initial_variables: Some(vars),
            setup: None,
            nodes: vec![],
            connections: vec![],
            teardown: None,
        };
        let json = serde_json::to_string(&flow).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("test-uuid-999"));
        assert!(json.contains("api_key"));
        assert!(json.contains("secret123"));

        // 反序列化测试
        let deserialized: Flow = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.uuid, "test-uuid-999");
        assert!(deserialized.initial_variables.is_some());
        let vars_ref = deserialized
            .initial_variables
            .as_ref()
            .unwrap_or_else(|| panic!("expected Some"));
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

        let flow: Flow = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(flow.uuid, "test-uuid-deser");
        assert_eq!(flow.name, "Deserialization Test");
        assert!(flow.initial_variables.is_none());
        assert!(flow.setup.is_none());
        assert!(flow.teardown.is_none());
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

        let flow: Flow = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(flow.uuid, "test-uuid-default");
        // 如果 JSON 中没有 initial_variables 字段，serde default 应该使其为 None
        assert!(flow.initial_variables.is_none());
        assert!(flow.setup.is_none());
        assert!(flow.teardown.is_none());
    }
    #[test]
    fn test_flow_with_setup_and_teardown() {
        let flow = Flow {
            uuid: "test-lifecycle".to_string(),
            name: "Lifecycle Flow".to_string(),
            setup: Some(FlowSubGraph::new(vec![], vec![])),
            teardown: Some(FlowSubGraph::new(vec![], vec![])),
            ..Default::default()
        };
        let json = serde_json::to_string(&flow).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("setup"));
        assert!(json.contains("teardown"));

        let deser: Flow = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert!(deser.setup.is_some());
        assert!(deser.teardown.is_some());
        assert!(deser
            .setup
            .unwrap_or_else(|| panic!("expected Some"))
            .is_empty());
    }

    // ===== 环境变量集测试 =====

    #[test]
    fn test_environment_set_serialization() {
        let env = EnvironmentSet {
            name: "dev".to_string(),
            description: Some("开发环境".to_string()),
            variables: {
                let mut vars = HashMap::new();
                vars.insert("api_url".to_string(), json!("http://localhost:8080"));
                vars.insert("debug".to_string(), json!(true));
                vars
            },
        };

        let json_str = serde_json::to_string(&env).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: EnvironmentSet =
            serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.name, "dev");
        assert_eq!(deserialized.description, Some("开发环境".to_string()));
        assert_eq!(
            deserialized.variables.get("api_url"),
            Some(&json!("http://localhost:8080"))
        );
        assert_eq!(deserialized.variables.get("debug"), Some(&json!(true)));
    }

    #[test]
    fn test_flow_get_active_variables_fallback() {
        let flow = Flow {
            uuid: "test-1".to_string(),
            name: "Test".to_string(),
            description: None,
            initial_variables: Some({
                let mut vars = HashMap::new();
                vars.insert("key".to_string(), json!("fallback"));
                vars
            }),
            setup: None,
            nodes: vec![],
            connections: vec![],
            teardown: None,
        };

        let vars = flow
            .get_active_variables()
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(vars.get("key"), Some(&json!("fallback")));
    }

    #[test]
    fn test_merge_variable_layers_priority() {
        // Layer 1: fallback
        let mut fallback = HashMap::new();
        fallback.insert("host".to_string(), json!("default.com"));
        fallback.insert("port".to_string(), json!(80));
        fallback.insert("token".to_string(), json!("default-tok"));
        fallback.insert("region".to_string(), json!("fallback-region"));

        // Layer 2: env
        let mut env_vars = HashMap::new();
        env_vars.insert("host".to_string(), json!("dev.com"));
        env_vars.insert("env_only".to_string(), json!("env-val"));
        env_vars.insert("region".to_string(), json!("env-region"));
        let env = EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: env_vars,
        };

        // Layer 3: local_override
        let mut local_vars = HashMap::new();
        local_vars.insert("host".to_string(), json!("localhost"));
        local_vars.insert("local_only".to_string(), json!("local-val"));
        let local_env = EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: local_vars,
        };

        // Layer 4: runtime_override
        let mut runtime_vars = HashMap::new();
        runtime_vars.insert("port".to_string(), json!(9090));
        runtime_vars.insert("runtime_only".to_string(), json!("run-val"));

        let merged = merge_variable_layers(
            Some(&fallback),
            Some(&env),
            Some(&local_env),
            Some(&runtime_vars),
        );

        // host 被 local_override 覆盖
        assert_eq!(merged.get("host"), Some(&json!("localhost")));
        // region 仅存在于 fallback 与 env：env 必须覆盖 fallback（Layer 2 > Layer 1）
        assert_eq!(merged.get("region"), Some(&json!("env-region")));
        // port 被 runtime_override 覆盖
        assert_eq!(merged.get("port"), Some(&json!(9090)));
        // token 沿用 fallback
        assert_eq!(merged.get("token"), Some(&json!("default-tok")));
        // env_only 沿用 env
        assert_eq!(merged.get("env_only"), Some(&json!("env-val")));
        // local_only 沿用 local
        assert_eq!(merged.get("local_only"), Some(&json!("local-val")));
        // runtime_only 沿用 runtime
        assert_eq!(merged.get("runtime_only"), Some(&json!("run-val")));
    }
}
