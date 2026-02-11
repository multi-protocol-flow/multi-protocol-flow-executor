// 节点配置模块 - 强类型节点配置定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Entry 节点配置
// ============================================================================

/// 入口节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryConfig {}

// ============================================================================
// Http 节点配置
// ============================================================================

/// HTTP 方法枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Http 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// 请求 URL
    pub url: String,
    /// HTTP 方法
    #[serde(default)]
    pub method: HttpMethod,
    /// 请求头
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// 请求体
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::Get,
            headers: None,
            body: None,
        }
    }
}

// ============================================================================
// Conditional 节点配置
// ============================================================================

/// 条件类型枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionType {
    #[default]
    Simple,
    Advanced,
}

/// 条件操作符枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    #[default]
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Exists,
}

/// Conditional 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalConfig {
    /// 条件类型
    #[serde(default)]
    pub condition_type: ConditionType,
    /// 字段路径（用于简单条件）
    #[serde(default)]
    pub field_path: Option<String>,
    /// 操作符
    #[serde(default)]
    pub operator: Option<ConditionOperator>,
    /// 期望值
    #[serde(default)]
    pub expected_value: Option<serde_json::Value>,
    /// 高级表达式（用于复杂条件）
    #[serde(default)]
    pub advanced_expression: Option<String>,
}

// ============================================================================
// 节点数据配置统一枚举
// ============================================================================

/// 节点数据配置（强类型枚举）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NodeData {
    Entry(EntryConfig),
    Http(HttpConfig),
    Conditional(ConditionalConfig),
}

impl NodeData {
    /// 获取节点类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            NodeData::Entry(_) => "entry",
            NodeData::Http(_) => "http",
            NodeData::Conditional(_) => "conditional",
        }
    }

    /// 创建空的 Entry 节点配置
    pub fn entry() -> Self {
        Self::Entry(EntryConfig::default())
    }

    /// 创建 Http 节点配置
    pub fn http(url: impl Into<String>) -> Self {
        Self::Http(HttpConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 Conditional 节点配置（简单条件）
    pub fn conditional_simple(
        field_path: impl Into<String>,
        operator: ConditionOperator,
        expected_value: serde_json::Value,
    ) -> Self {
        Self::Conditional(ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some(field_path.into()),
            operator: Some(operator),
            expected_value: Some(expected_value),
            advanced_expression: None,
        })
    }

    /// 创建 Conditional 节点配置（高级条件）
    pub fn conditional_advanced(expression: impl Into<String>) -> Self {
        Self::Conditional(ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some(expression.into()),
        })
    }
}

// ============================================================================
// 辅助宏和函数
// ============================================================================

/// 为 HTTP 配置添加请求头的便捷宏
#[macro_export]
macro_rules! http_config {
    ($url:expr) => {
        NodeData::http($url)
    };
    ($url:expr, headers: { $($key:expr => $val:expr),* $(,)? }) => {{
        let mut headers_map = HashMap::new();
        $(
            headers_map.insert($key.to_string(), $val.to_string());
        )*
        NodeData::Http(HttpConfig {
            url: $url.to_string(),
            headers: if headers_map.is_empty() { None } else { Some(headers_map) },
            ..Default::default()
        })
    }};
    ($url:expr, method: $method:expr) => {
        NodeData::Http(HttpConfig {
            url: $url.to_string(),
            method: $method,
            ..Default::default()
        })
    };
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_entry_config_serialization() {
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_entry_config_empty_structure() {
        // 测试 EntryConfig 是空结构
        let config = EntryConfig {};
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_entry_config_default_impl() {
        // 测试 Default impl
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_http_config_serialization() {
        let config = HttpConfig {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Get,
            headers: None,
            body: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("https://api.example.com/users"));
        assert!(json.contains("get"));

        // 反序列化测试
        let deserialized: HttpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, config.url);
        assert_eq!(deserialized.method, HttpMethod::Get);
    }

    #[test]
    fn test_conditional_config_simple() {
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("http_response.status".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(json!(200)),
            advanced_expression: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("simple"));
        assert!(json.contains("http_response.status"));
        assert!(json.contains("equal"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_conditional_config_advanced() {
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some(
                "http_response.status >= 200 && http_response.status < 300".to_string(),
            ),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("advanced"));
        assert!(json.contains("http_response.status >= 200"));
    }

    #[test]
    fn test_node_data_type_name() {
        assert_eq!(NodeData::entry().type_name(), "entry");
        assert_eq!(NodeData::http("https://example.com").type_name(), "http");
        assert_eq!(
            NodeData::conditional_simple("field", ConditionOperator::Equal, json!(1)).type_name(),
            "conditional"
        );
    }

    #[test]
    fn test_node_data_serialization_with_tag() {
        let data = NodeData::http("https://api.example.com");
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"http""#));
        assert!(json.contains("https://api.example.com"));

        // 反序列化测试
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::Http(config) => {
                assert_eq!(config.url, "https://api.example.com");
                assert_eq!(config.method, HttpMethod::Get);
            }
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    fn test_http_config_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer xxx".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let config = HttpConfig {
            url: "https://api.example.com".to_string(),
            method: HttpMethod::Post,
            headers: Some(headers),
            body: Some(json!({"key": "value"})),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("Authorization"));
        assert!(json.contains("Bearer xxx"));
        assert!(json.contains("Content-Type"));
        assert!(json.contains("application/json"));
        assert!(json.contains("post"));
        assert!(json.contains(r#""key""#));
        assert!(json.contains(r#""value""#));
    }
}
