//! 断言节点插件
//!
//! 实现 `NodePlugin` trait，提供断言节点的元数据、端口路由和执行能力。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;

/// 断言节点插件
pub struct AssertionPlugin;

#[async_trait::async_trait]
impl NodePlugin for AssertionPlugin {
    fn type_id(&self) -> &str {
        "assertion"
    }

    fn display_name(&self) -> &str {
        "断言"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "check-circle"
    }

    fn color(&self) -> &str {
        "#10B981"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "通过".into(),
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
            "mode": "all",
            "on_failure": "route_false",
            "continue_on_failure": false,
            "assertions": [],
            "source_node_id": null,
        })
    }

    /// 断言检查 $.passed → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("passed") {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => "false".to_string(),
        }
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "preset": config.get("preset"),
            "mode": config.get("mode"),
            "on_failure": config.get("on_failure"),
            "continue_on_failure": config.get("continue_on_failure"),
            "assertions": config.get("assertions"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check if preset is set; if not, assertions must be a non-empty array
        let has_preset = config
            .get("preset")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        if !has_preset {
            match config.get("assertions") {
                Some(serde_json::Value::Array(arr)) => {
                    if arr.is_empty() {
                        errors.push(ValidationError {
                            field: "assertions".to_string(),
                            message: "Assertion list is empty".to_string(),
                        });
                    } else {
                        for (i, assertion) in arr.iter().enumerate() {
                            // target must be non-empty
                            let target = assertion
                                .get("target")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if target.is_empty() {
                                errors.push(ValidationError {
                                    field: format!("assertions[{}].target", i),
                                    message: "Assertion target is empty".to_string(),
                                });
                            }

                            // operator must be non-empty
                            let operator = assertion
                                .get("operator")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if operator.is_empty() {
                                errors.push(ValidationError {
                                    field: format!("assertions[{}].operator", i),
                                    message: "Assertion operator is empty".to_string(),
                                });
                            }
                        }
                    }
                }
                _ => {
                    errors.push(ValidationError {
                        field: "assertions".to_string(),
                        message: "Assertion list is empty".to_string(),
                    });
                }
            }
        }

        // Validate mode: must be "all" or "any" if present
        if let Some(mode) = config.get("mode").and_then(|v| v.as_str()) {
            if mode != "all" && mode != "any" {
                errors.push(ValidationError {
                    field: "mode".to_string(),
                    message: "Assertion mode must be 'all' or 'any'".to_string(),
                });
            }
        }

        // Validate on_failure: must be "route_false" or "throw" if present
        if let Some(on_failure) = config.get("on_failure").and_then(|v| v.as_str()) {
            if on_failure != "route_false" && on_failure != "throw" {
                errors.push(ValidationError {
                    field: "on_failure".to_string(),
                    message: "Failure mode must be 'route_false' or 'throw'".to_string(),
                });
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
        let executor = crate::assertion_executor::AssertionNodeExecutor::new();
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}
