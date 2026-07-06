// 条件分支节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;

/// 条件分支节点插件
pub struct ConditionalPlugin;

#[async_trait::async_trait]
impl NodePlugin for ConditionalPlugin {
    fn type_id(&self) -> &str {
        "conditional"
    }

    fn display_name(&self) -> &str {
        "Conditional Branch"
    }

    fn category(&self) -> &str {
        "logic"
    }

    fn icon(&self) -> &str {
        "git-branch"
    }

    fn color(&self) -> &str {
        "#F59E0B"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "条件为真".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "条件为假".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "condition_type": "simple",
            "field_path": "",
            "operator": "equal",
            "expected_value": null,
        })
    }

    /// 条件节点检查 $.result → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("result") {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => "false".to_string(),
        }
    }

    /// 条件节点的报告输入：提取条件配置关键字段
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "condition_type": config.get("condition_type"),
            "field_path": config.get("field_path"),
            "operator": config.get("operator"),
            "expected_value": config.get("expected_value"),
            "advanced_expression": config.get("advanced_expression"),
            "expected_value_type": config.get("expected_value_type"),
        }))
    }

    /// 配置验证
    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let condition_type = config
            .get("condition_type")
            .and_then(|v| v.as_str())
            .unwrap_or("simple");

        match condition_type {
            "simple" => {
                // field_path must be non-empty if present
                if let Some(fp) = config.get("field_path").and_then(|v| v.as_str()) {
                    if fp.is_empty() {
                        errors.push(ValidationError {
                            field: "field_path".into(),
                            message: "Field path is required".into(),
                        });
                    }
                }

                // operator must be a known value if present
                const VALID_OPERATORS: &[&str] = &[
                    "equal",
                    "not_equal",
                    "gt",
                    "gte",
                    "lt",
                    "lte",
                    "contains",
                    "matches",
                    "exists",
                    "not_exists",
                    "true",
                    "false",
                    "in",
                    "not_in",
                    "type_is",
                    "between",
                ];
                if let Some(op) = config.get("operator").and_then(|v| v.as_str()) {
                    if !VALID_OPERATORS.contains(&op) {
                        errors.push(ValidationError {
                            field: "operator".into(),
                            message: format!("未知的操作符: {}", op),
                        });
                    }
                }
            }
            "advanced" => {
                // advanced_expression must be non-empty
                let expr = config.get("advanced_expression");
                let is_empty = expr
                    .and_then(|v| v.as_str())
                    .map(|s| s.is_empty())
                    .unwrap_or(true); // absent/null → treat as empty
                if is_empty {
                    errors.push(ValidationError {
                        field: "advanced_expression".into(),
                        message: "Advanced expression is required".into(),
                    });
                }
            }
            _ => {}
        }

        errors
    }

    /// 委托给 `ConditionalNodeExecutor` 执行
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::conditional_executor::ConditionalNodeExecutor::new();
        executor.execute(node, context, connections, node_map).await
    }
}
