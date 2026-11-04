//! 循环节点插件 — `NodePlugin` 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{LoopConfig, LoopSource, NodeData};
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use serde_json::json;
use std::collections::HashMap;

use crate::loop_executor::LoopNodeExecutor;
use crate::loop_scope::MAX_LOOP_NESTING;

/// 循环节点插件
///
/// 三种来源（`count` / `items` / `until`），body 为内联子图。
/// 端口：`done`（次数/集合耗尽或 `until` 命中）与 `false`（预算耗尽、超时、
/// 嵌套超限、body 失败、取消）。
pub struct LoopPlugin;

impl LoopPlugin {
    /// 表达式语法检查
    ///
    /// evalexpr 11 的解析器会接受「悬空运算符」这类残缺表达式（错误延后到求值期），
    /// 因此这里把表达式中出现的裸标识符（排除 `true`/`false`、函数调用 `f(` 与
    /// 模块路径 `m::f`）先注入占位值再求值，把结构性错误判为非法。
    fn expression_error(expr: &str) -> Option<String> {
        let processed = match crate::expression::preprocess_expression(expr) {
            Ok(processed) => processed,
            Err(e) => return Some(e),
        };
        // `regex` crate 不支持 lookahead，函数调用/模块路径的排除在下方手工判定
        let identifier_re = match regex::Regex::new(r"[A-Za-z_]\w*") {
            Ok(re) => re,
            Err(e) => return Some(format!("Invalid regex: {e}")),
        };
        use evalexpr::ContextWithMutableVariables;
        let mut probe = evalexpr::HashMapContext::new();
        for cap in identifier_re.find_iter(&processed) {
            let name = cap.as_str();
            if name == "true" || name == "false" {
                continue;
            }
            // 跳过函数调用 `f(` / `m::f(` 与模块路径 `m::`
            let rest = processed[cap.end()..].trim_start();
            if rest.starts_with('(') || rest.starts_with("::") {
                continue;
            }
            if let Err(e) = probe.set_value(name.to_string(), evalexpr::Value::Int(1)) {
                return Some(format!("Setting probe variable '{name}' failed: {e}"));
            }
        }
        match evalexpr::eval_with_context(&processed, &probe) {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        }
    }

    /// 统计一组节点里 loop 的最大嵌套深度
    fn max_nesting_depth(nodes: &[Node]) -> u32 {
        nodes
            .iter()
            .map(|node| match &node.data {
                NodeData::Loop(inner) => 1 + Self::max_nesting_depth(&inner.body.nodes),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl NodePlugin for LoopPlugin {
    fn type_id(&self) -> &str {
        "flow:loop"
    }

    fn display_name(&self) -> &str {
        "Loop"
    }

    fn category(&self) -> &str {
        "logic"
    }

    fn icon(&self) -> &str {
        "rotate-cw"
    }

    fn color(&self) -> &str {
        "#0EA5E9"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "done".to_string(),
                label: "Done".to_string(),
                position: "right".to_string(),
            },
            PortDef {
                id: "false".to_string(),
                label: "Failed".to_string(),
                position: "right".to_string(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "type": "flow:loop",
            "source": "count",
            "count": 1,
            "interval_ms": 0,
            "interval_max_ms": 10000,
            "initial_delay_ms": 0,
            "max_iterations": 100,
            "concurrency": 1,
            "break_on_error": true,
            "output_mappings": [],
            "report": {
                "detail_iterations": 20,
                "always_keep_failed": true
            },
            "body": {
                "nodes": [],
                "connections": []
            }
        })
    }

    /// 成功终止（`exhausted`/`until`）走 `done`，其余走 `false`
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output.as_ref().and_then(|v| v.get("break_reason")) {
            Some(serde_json::Value::String(reason))
                if reason == "exhausted" || reason == "until" =>
            {
                "done".to_string()
            }
            _ => "false".to_string(),
        }
    }

    fn is_port_routing(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        let source = config
            .get("source")
            .cloned()
            .unwrap_or_else(|| json!("count"));
        let body_nodes = config
            .get("body")
            .and_then(|b| b.get("nodes"))
            .and_then(|n| n.as_array())
            .map(|n| n.len())
            .unwrap_or(0);
        Some(json!({
            "source": source,
            "items": config.get("items").cloned().unwrap_or(json!(null)),
            "count": config.get("count").cloned().unwrap_or(json!(null)),
            "until": config.get("until").cloned().unwrap_or(json!(null)),
            "max_iterations": config.get("max_iterations").cloned().unwrap_or(json!(null)),
            "timeout_ms": config.get("timeout_ms").cloned().unwrap_or(json!(null)),
            "concurrency": config.get("concurrency").cloned().unwrap_or(json!(null)),
            "break_on_error": config.get("break_on_error").cloned().unwrap_or(json!(null)),
            "output_mappings": config.get("output_mappings").cloned().unwrap_or(json!([])),
            "body_nodes": body_nodes,
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let cfg: LoopConfig = match serde_json::from_value(config.clone()) {
            Ok(cfg) => cfg,
            Err(e) => {
                return vec![ValidationError {
                    field: "config".to_string(),
                    message: format!("Invalid Loop config: {e}"),
                }]
            }
        };

        let mut errors = Vec::new();

        let items_empty = cfg
            .items
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);
        let until_empty = cfg
            .until
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);

        match cfg.source {
            LoopSource::Items if items_empty => errors.push(ValidationError {
                field: "items".to_string(),
                message: "Loop items expression is required".to_string(),
            }),
            LoopSource::Count if cfg.count.unwrap_or(0) == 0 => errors.push(ValidationError {
                field: "count".to_string(),
                message: "Loop count must be at least 1".to_string(),
            }),
            LoopSource::Until if until_empty => errors.push(ValidationError {
                field: "until".to_string(),
                message: "Loop until expression is required".to_string(),
            }),
            _ => {}
        }

        if !until_empty {
            let expr = cfg.until.as_deref().unwrap_or_default();
            if let Some(e) = Self::expression_error(expr) {
                errors.push(ValidationError {
                    field: "until".to_string(),
                    message: format!("Invalid until expression: {e}"),
                });
            }
        }

        if cfg.concurrency != 1 {
            errors.push(ValidationError {
                field: "concurrency".to_string(),
                message: format!(
                    "concurrency = {} is not supported yet (only 1)",
                    cfg.concurrency
                ),
            });
        }

        if cfg.max_iterations == 0 {
            errors.push(ValidationError {
                field: "max_iterations".to_string(),
                message: "max_iterations must be at least 1".to_string(),
            });
        }

        // 本节点自身算一层，body 内每多一层 loop 加一
        let depth = 1 + Self::max_nesting_depth(&cfg.body.nodes);
        if depth > MAX_LOOP_NESTING {
            errors.push(ValidationError {
                field: "body".to_string(),
                message: format!("Loop nesting depth exceeds {} levels", MAX_LOOP_NESTING),
            });
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
        LoopNodeExecutor::new()
            .execute(node, context, connections, node_map)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_with_loop(depth: u32) -> serde_json::Value {
        if depth == 0 {
            return json!({"nodes": [], "connections": []});
        }
        json!({
            "nodes": [{
                "uuid": format!("inner-{}", depth),
                "type": "flow:loop",
                "name": "inner",
                "position": {"x": 0.0, "y": 0.0},
                "data": {
                    "type": "flow:loop",
                    "source": "count",
                    "count": 1,
                    "max_iterations": 10,
                    "concurrency": 1,
                    "break_on_error": true,
                    "report": {"detail_iterations": 20, "always_keep_failed": true},
                    "body": body_with_loop(depth - 1)
                }
            }],
            "connections": []
        })
    }

    fn config_with(body: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "flow:loop",
            "source": "count",
            "count": 2,
            "max_iterations": 10,
            "concurrency": 1,
            "break_on_error": true,
            "report": {"detail_iterations": 20, "always_keep_failed": true},
            "body": body
        })
    }

    #[test]
    fn validate_accepts_default_config() {
        assert!(LoopPlugin.validate(&LoopPlugin.default_config()).is_empty());
    }

    #[test]
    fn validate_requires_items_for_items_source() {
        let errors = LoopPlugin.validate(&json!({
            "type": "flow:loop",
            "source": "items",
            "max_iterations": 10,
            "concurrency": 1,
            "body": {"nodes": [], "connections": []}
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "items");
    }

    #[test]
    fn validate_rejects_zero_count() {
        let errors = LoopPlugin.validate(&config_with(json!({"nodes": [], "connections": []})));
        assert!(errors.is_empty());

        let errors = LoopPlugin.validate(&json!({
            "type": "flow:loop",
            "source": "count",
            "count": 0,
            "max_iterations": 10,
            "concurrency": 1,
            "body": {"nodes": [], "connections": []}
        }));
        assert_eq!(errors[0].field, "count");
    }

    #[test]
    fn validate_rejects_concurrency_above_one() {
        let errors = LoopPlugin.validate(&json!({
            "type": "flow:loop",
            "source": "count",
            "count": 2,
            "max_iterations": 10,
            "concurrency": 4,
            "body": {"nodes": [], "connections": []}
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "concurrency");
        assert!(errors[0].message.contains("concurrency"));
    }

    #[test]
    fn validate_rejects_invalid_until_expression() {
        let errors = LoopPlugin.validate(&json!({
            "type": "flow:loop",
            "source": "until",
            "until": "last.status ==",
            "max_iterations": 10,
            "concurrency": 1,
            "body": {"nodes": [], "connections": []}
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "until");
    }

    #[test]
    fn expression_error_detects_dangling_operator() {
        // evalexpr 的解析器接受残缺表达式，求值期才会暴露
        assert!(LoopPlugin::expression_error("last.status ==").is_some());
        assert!(LoopPlugin::expression_error("1 +").is_some());
        assert!(LoopPlugin::expression_error("last.status == \"unterminated").is_some());
        assert!(LoopPlugin::expression_error("last.status == 200").is_none());
        assert!(LoopPlugin::expression_error("loop.index >= 2 && last.success == true").is_none());
        assert!(LoopPlugin::expression_error("math::sqrt(4) > 1").is_none());
    }

    #[test]
    fn validate_rejects_too_deep_nesting() {
        // 校验器统计的是「body 内的 loop 链深度」，绝对层数由执行期作用域把关
        let errors = LoopPlugin.validate(&config_with(body_with_loop(MAX_LOOP_NESTING + 1)));
        assert!(
            errors
                .iter()
                .any(|e| e.field == "body" && e.message.contains("nesting")),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn resolve_port_maps_break_reason() {
        assert_eq!(
            LoopPlugin.resolve_port(&Some(json!({"break_reason": "exhausted"}))),
            "done"
        );
        assert_eq!(
            LoopPlugin.resolve_port(&Some(json!({"break_reason": "until"}))),
            "done"
        );
        assert_eq!(
            LoopPlugin.resolve_port(&Some(json!({"break_reason": "budget"}))),
            "false"
        );
        assert_eq!(LoopPlugin.resolve_port(&None), "false");
    }

    #[test]
    fn report_input_omits_body_contents() {
        let input = LoopPlugin
            .build_report_input(&config_with(json!({
                "nodes": [{
                    "uuid": "n1",
                    "type": "script",
                    "name": "s",
                    "position": {"x": 0.0, "y": 0.0},
                    "data": {"type": "script", "script": "1"}
                }],
                "connections": []
            })))
            .unwrap_or_else(|| panic!("expected report input"));
        assert_eq!(input["body_nodes"], json!(1));
        assert!(input.get("body").is_none());
    }
}
