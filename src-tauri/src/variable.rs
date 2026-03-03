// 变量系统核心类型定义
//!
//! 定义变量池、输出映射、初始变量等核心类型。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 变量名（全局唯一）
    pub name: String,
    /// 变量值
    pub value: Value,
    /// 来源节点 UUID
    pub source_node_uuid: String,
    /// 变量说明（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 变量池
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariablePool {
    /// 变量列表
    pub variables: Vec<Variable>,
}

/// 输出映射配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMapping {
    /// 源字段路径（如 "status", "body.data.id"）
    pub source: String,
    /// 目标变量名
    pub target: String,
    /// 映射说明（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// 默认启用状态
fn default_enabled() -> bool {
    true
}

/// 初始变量配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialVariable {
    /// 变量名
    pub name: String,
    /// 变量值
    pub value: Value,
    /// 变量说明（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_variable_creation() {
        let variable = Variable {
            name: "user_id".to_string(),
            value: json!(12345),
            source_node_uuid: "node-001".to_string(),
            description: Some("用户ID".to_string()),
        };

        assert_eq!(variable.name, "user_id");
        assert_eq!(variable.value, json!(12345));
        assert_eq!(variable.source_node_uuid, "node-001");
        assert_eq!(variable.description, Some("用户ID".to_string()));
    }

    #[test]
    fn test_variable_pool_operations() {
        let mut pool = VariablePool::default();

        // 测试初始状态
        assert!(pool.variables.is_empty());

        // 添加变量
        let var1 = Variable {
            name: "token".to_string(),
            value: json!("abc123"),
            source_node_uuid: "auth-node".to_string(),
            description: None,
        };
        pool.variables.push(var1);

        let var2 = Variable {
            name: "status".to_string(),
            value: json!(200),
            source_node_uuid: "http-node".to_string(),
            description: Some("HTTP状态码".to_string()),
        };
        pool.variables.push(var2);

        // 测试查找
        let found = pool.variables.iter().find(|v| v.name == "token");
        assert!(found.is_some());
        assert_eq!(found.unwrap().value, json!("abc123"));

        let not_found = pool.variables.iter().find(|v| v.name == "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_variable_serialization() {
        // 测试 Variable 序列化/反序列化
        let variable = Variable {
            name: "response_data".to_string(),
            value: json!({"id": 1, "name": "test"}),
            source_node_uuid: "node-002".to_string(),
            description: Some("API响应数据".to_string()),
        };

        let json_str = serde_json::to_string(&variable).expect("序列化失败");
        let deserialized: Variable = serde_json::from_str(&json_str).expect("反序列化失败");

        assert_eq!(deserialized.name, variable.name);
        assert_eq!(deserialized.value, variable.value);
        assert_eq!(deserialized.source_node_uuid, variable.source_node_uuid);
        assert_eq!(deserialized.description, variable.description);
    }

    #[test]
    fn test_variable_serialization_without_description() {
        // 测试 description 为 None 时不序列化该字段
        let variable = Variable {
            name: "simple_var".to_string(),
            value: json!("value"),
            source_node_uuid: "node-003".to_string(),
            description: None,
        };

        let json_str = serde_json::to_string(&variable).expect("序列化失败");
        assert!(!json_str.contains("description"));
    }

    #[test]
    fn test_output_mapping_default_enabled() {
        // 测试 OutputMapping 的 enabled 默认值
        let json_str = r#"{"source": "body.id", "target": "user_id"}"#;
        let mapping: OutputMapping = serde_json::from_str(json_str).expect("反序列化失败");

        assert_eq!(mapping.source, "body.id");
        assert_eq!(mapping.target, "user_id");
        assert!(mapping.enabled); // 默认为 true
        assert!(mapping.description.is_none());
    }

    #[test]
    fn test_output_mapping_with_all_fields() {
        let mapping = OutputMapping {
            source: "status".to_string(),
            target: "http_status".to_string(),
            description: Some("HTTP状态码映射".to_string()),
            enabled: false,
        };

        let json_str = serde_json::to_string(&mapping).expect("序列化失败");
        let deserialized: OutputMapping = serde_json::from_str(&json_str).expect("反序列化失败");

        assert_eq!(deserialized.source, "status");
        assert_eq!(deserialized.target, "http_status");
        assert_eq!(deserialized.description, Some("HTTP状态码映射".to_string()));
        assert!(!deserialized.enabled);
    }

    #[test]
    fn test_initial_variable() {
        let initial_var = InitialVariable {
            name: "api_base".to_string(),
            value: json!("https://api.example.com"),
            description: Some("API基础URL".to_string()),
        };

        let json_str = serde_json::to_string(&initial_var).expect("序列化失败");
        let deserialized: InitialVariable = serde_json::from_str(&json_str).expect("反序列化失败");

        assert_eq!(deserialized.name, "api_base");
        assert_eq!(deserialized.value, json!("https://api.example.com"));
        assert_eq!(deserialized.description, Some("API基础URL".to_string()));
    }

    #[test]
    fn test_variable_pool_default() {
        let pool = VariablePool::default();
        assert!(pool.variables.is_empty());
    }

    #[test]
    fn test_variable_pool_serialization() {
        let pool = VariablePool {
            variables: vec![
                Variable {
                    name: "var1".to_string(),
                    value: json!(1),
                    source_node_uuid: "node1".to_string(),
                    description: None,
                },
                Variable {
                    name: "var2".to_string(),
                    value: json!("two"),
                    source_node_uuid: "node2".to_string(),
                    description: Some("第二个变量".to_string()),
                },
            ],
        };

        let json_str = serde_json::to_string(&pool).expect("序列化失败");
        let deserialized: VariablePool = serde_json::from_str(&json_str).expect("反序列化失败");

        assert_eq!(deserialized.variables.len(), 2);
        assert_eq!(deserialized.variables[0].name, "var1");
        assert_eq!(deserialized.variables[1].name, "var2");
    }
}
