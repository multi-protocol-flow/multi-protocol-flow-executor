// HTTP 请求节点插件 — NodePlugin trait 实现
//
// 提供 HTTP 节点的元数据、端口定义、配置验证和执行能力。
// 执行委托给 HttpNodeExecutor。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;

/// HTTP 请求节点插件
pub struct HttpPlugin;

#[async_trait::async_trait]
impl NodePlugin for HttpPlugin {
    fn type_id(&self) -> &str {
        "http"
    }

    fn display_name(&self) -> &str {
        "HTTP 请求"
    }

    fn category(&self) -> &str {
        "http"
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
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
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
                    "description": "请求 URL，支持变量引用 {{variable}}"
                },
                "method": {
                    "type": "string",
                    "title": "请求方法",
                    "enum": ["get", "post", "put", "delete", "patch", "head", "options"]
                },
                "headers": {
                    "type": "object",
                    "title": "请求头",
                    "additionalProperties": { "type": "string" }
                },
                "body_type": {
                    "type": "string",
                    "title": "请求体类型",
                    "enum": ["none", "body", "form-data", "x-www-form-urlencoded", "binary"],
                    "default": "none"
                },
                "body": {
                    "type": "string",
                    "title": "请求体"
                },
                "timeout_ms": {
                    "type": "integer",
                    "title": "超时时间（毫秒）",
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
            if timeout < 1000 || timeout > 300000 {
                errors.push(ValidationError {
                    field: "timeout_ms".into(),
                    message: "Timeout must be between 1000ms and 300000ms".into(),
                });
            }
        }

        // 4. body must be non-empty when body_type != "none" and no file_path
        let body_type = config.get("body_type").and_then(|v| v.as_str());
        if body_type.is_some_and(|bt| bt != "none") {
            let has_file = config
                .get("file_path")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty());

            if !has_file {
                let body_missing = config
                    .get("body")
                    .and_then(|v| v.as_str())
                    .map_or(true, |s| s.is_empty());

                if body_missing {
                    errors.push(ValidationError {
                        field: "body".into(),
                        message: "Request body is required when body type is not 'none'".into(),
                    });
                }
            }
        }

        errors
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let executor = crate::http_executor::HttpNodeExecutor::new();
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}
