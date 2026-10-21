// HTTP 请求节点插件 — NodePlugin trait 实现
//
// 提供 HTTP 节点的元数据、端口定义、配置验证和执行能力。
// 执行委托给 HttpNodeExecutor。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;

/// HTTP 请求节点插件
pub struct HttpPlugin;

#[async_trait::async_trait]
impl NodePlugin for HttpPlugin {
    fn type_id(&self) -> &str {
        "http:request"
    }

    fn display_name(&self) -> &str {
        "HTTP Request"
    }

    fn category(&self) -> &str {
        "http:request"
    }

    fn icon(&self) -> &str {
        "globe"
    }

    fn color(&self) -> &str {
        "#3B82F6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Failure".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "url": "",
            "method": "get",
            "headers": {},
            "body_type": "none",
            "content_type": "json",
        })
    }

    /// HTTP 检查 $.success → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "url": config.get("url"),
            "method": config.get("method"),
            "headers": config.get("headers"),
            "body": config.get("body"),
            "body_type": config.get("body_type"),
            "timeout_ms": config.get("timeout_ms"),
        }))
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "required": ["url", "method"],
            "properties": {
                "url": {
                    "type": "string",
                    "title": "URL",
                    "description": "Request URL, supports variable references {{variable}}"
                },
                "method": {
                    "type": "string",
                    "title": "Request Method",
                    "enum": ["get", "post", "put", "delete", "patch", "head", "options"]
                },
                "headers": {
                    "type": "object",
                    "title": "Headers",
                    "additionalProperties": { "type": "string" }
                },
                "body_type": {
                    "type": "string",
                    "title": "Body Type",
                    "enum": ["none", "body", "form-data", "x-www-form-urlencoded", "binary"],
                    "default": "none"
                },
                "body": {
                    "type": "string",
                    "title": "Request Body"
                },
                "timeout_ms": {
                    "type": "integer",
                    "title": "Timeout (ms)",
                    "minimum": 1000,
                    "maximum": 300000,
                    "default": 30000
                }
            }
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 1. url non-empty
        match config.get("url").and_then(|v| v.as_str()) {
            Some(url) if !url.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "url".into(),
                message: "URL is required".into(),
            }),
        }

        // 2. method must be one of the valid HTTP methods
        let valid_methods = ["get", "post", "put", "delete", "patch", "head", "options"];
        match config.get("method").and_then(|v| v.as_str()) {
            Some(m) if valid_methods.contains(&m) => {}
            _ => errors.push(ValidationError {
                field: "method".into(),
                message: format!(
                    "Invalid request method, valid values are: {}",
                    valid_methods.join(", ")
                ),
            }),
        }

        // 3. timeout_ms range (if present)
        if let Some(timeout) = config.get("timeout_ms").and_then(|v| v.as_u64()) {
            if !(1000..=300000).contains(&timeout) {
                errors.push(ValidationError {
                    field: "timeout_ms".into(),
                    message: "Timeout must be between 1000ms and 300000ms".into(),
                });
            }
        }

        // 4. the payload source depends on body_type:
        //    - "body"     → raw `body` string
        //    - "form-data" / "x-www-form-urlencoded" → `form_fields`
        //    - "binary"   → `file_path`
        // Requiring `body` for form types rejected every valid multipart flow.
        match config.get("body_type").and_then(|v| v.as_str()) {
            None | Some("none") => {}
            Some("body") => {
                let body_missing = config
                    .get("body")
                    .and_then(|v| v.as_str())
                    .is_none_or(|s| s.is_empty());
                if body_missing {
                    errors.push(ValidationError {
                        field: "body".into(),
                        message: "Request body is required when body type is 'body'".into(),
                    });
                }
            }
            Some("form-data") | Some("x-www-form-urlencoded") => {
                let has_fields = config
                    .get("form_fields")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| !a.is_empty());
                if !has_fields {
                    errors.push(ValidationError {
                        field: "form_fields".into(),
                        message: "At least one form field is required for form body types".into(),
                    });
                }
            }
            Some("binary") => {
                let has_file = config
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty());
                if !has_file {
                    errors.push(ValidationError {
                        field: "file_path".into(),
                        message: "A file path is required for binary body type".into(),
                    });
                }
            }
            Some(_) => {}
        }

        errors
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::http_executor::HttpNodeExecutor::new();
        executor.execute(node, context, connections, node_map).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(config: serde_json::Value) -> Vec<String> {
        HttpPlugin
            .validate(&config)
            .into_iter()
            .map(|e| e.field)
            .collect()
    }

    #[test]
    fn body_type_none_accepts_empty_body() {
        let fields = validate(serde_json::json!({
            "url": "https://example.com", "method": "get", "body_type": "none"
        }));
        assert!(fields.is_empty(), "unexpected errors: {fields:?}");
    }

    #[test]
    fn body_type_body_requires_body() {
        let fields = validate(serde_json::json!({
            "url": "https://example.com", "method": "post", "body_type": "body"
        }));
        assert_eq!(fields, vec!["body".to_string()]);
    }

    #[test]
    fn body_type_body_accepts_non_empty_body() {
        let fields = validate(serde_json::json!({
            "url": "https://example.com", "method": "post",
            "body_type": "body", "body": "{\"a\":1}"
        }));
        assert!(fields.is_empty(), "unexpected errors: {fields:?}");
    }

    /// Regression: form body types were validated against `body`, so every
    /// multipart / urlencoded flow failed with "Request body is required".
    #[test]
    fn form_body_types_are_validated_against_form_fields() {
        for body_type in ["form-data", "x-www-form-urlencoded"] {
            let fields = validate(serde_json::json!({
                "url": "https://example.com", "method": "post",
                "body_type": body_type,
                "form_fields": [{ "key": "a", "value": "1", "type": "text" }]
            }));
            assert!(fields.is_empty(), "{body_type} rejected: {fields:?}");

            let missing = validate(serde_json::json!({
                "url": "https://example.com", "method": "post",
                "body_type": body_type, "form_fields": []
            }));
            assert_eq!(missing, vec!["form_fields".to_string()], "{body_type}");
        }
    }

    #[test]
    fn binary_body_type_requires_file_path() {
        let fields = validate(serde_json::json!({
            "url": "https://example.com", "method": "post", "body_type": "binary"
        }));
        assert_eq!(fields, vec!["file_path".to_string()]);

        let ok = validate(serde_json::json!({
            "url": "https://example.com", "method": "post",
            "body_type": "binary", "file_path": "a.bin"
        }));
        assert!(ok.is_empty(), "unexpected errors: {ok:?}");
    }
}
