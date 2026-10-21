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
                label: "Condition is True".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Condition is False".into(),
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

                // Derive the valid set from `ConditionOperator` itself so the
                // whitelist can never drift from what the executor accepts.
                // The previous hand-written list used the pre-rename spellings
                // (`gt`/`gte`/`lt`/`lte`) and rejected every operator the
                // frontend can actually emit (`greater`, `less_or_equal`, …).
                if let Some(op) = config.get("operator").and_then(|v| v.as_str()) {
                    let parsed: Result<
                        flow_engine_core::node_config::ConditionOperator,
                        _,
                    > = serde_json::from_value(serde_json::Value::String(op.to_string()));
                    if parsed.is_err() {
                        errors.push(ValidationError {
                            field: "operator".into(),
                            message: format!("Unknown operator: {}", op),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn operator_errors(operator: &str) -> Vec<String> {
        ConditionalPlugin
            .validate(&serde_json::json!({
                "condition_type": "simple",
                "field_path": "http_response.status",
                "operator": operator,
                "expected_value": 200
            }))
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// Regression: the whitelist used the pre-rename spellings (`gt`/`lt`), so
    /// every operator the frontend can emit was rejected by `mpe validate`.
    #[test]
    fn accepts_every_operator_the_ui_can_emit() {
        for op in [
            "equal",
            "not_equal",
            "greater",
            "greater_or_equal",
            "less",
            "less_or_equal",
            "contains",
            "starts_with",
            "ends_with",
            "exists",
        ] {
            let errors = operator_errors(op);
            assert!(errors.is_empty(), "operator {op} rejected: {errors:?}");
        }
    }

    #[test]
    fn rejects_unknown_operator() {
        let errors = operator_errors("gt_but_not_really");
        assert_eq!(errors.len(), 1, "expected one error, got {errors:?}");
        assert!(
            errors[0].contains("Unknown operator"),
            "unexpected message: {errors:?}"
        );
    }
}
