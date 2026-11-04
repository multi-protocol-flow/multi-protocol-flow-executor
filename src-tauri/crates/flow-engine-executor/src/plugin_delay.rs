//! 延时节点插件 — `NodePlugin` 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::DelayConfig;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use serde_json::json;
use std::collections::HashMap;

use crate::delay_executor::DelayNodeExecutor;

/// 延时节点插件
///
/// 配置 `duration_ms`（毫秒，支持变量模板）与可选 `jitter_ms` 抖动。
/// 等待期间可被取消信号中断；单次上限 1 小时（见 [`crate::delay_executor::MAX_DELAY_MS`]）。
pub struct DelayPlugin;

#[async_trait::async_trait]
impl NodePlugin for DelayPlugin {
    fn type_id(&self) -> &str {
        "flow:delay"
    }

    fn display_name(&self) -> &str {
        "Delay"
    }

    fn category(&self) -> &str {
        "logic"
    }

    fn icon(&self) -> &str {
        "timer"
    }

    fn color(&self) -> &str {
        "#64748B"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".to_string(),
            label: "Output".to_string(),
            position: "right".to_string(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "type": "flow:delay",
            "duration_ms": "1000",
            "jitter_ms": 0
        })
    }

    /// 延时节点固定走 out 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_port_routing(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(json!({
            "duration_ms": config.get("duration_ms").cloned().unwrap_or(json!(null)),
            "jitter_ms": config.get("jitter_ms").cloned().unwrap_or(json!(0)),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let cfg: DelayConfig = match serde_json::from_value(config.clone()) {
            Ok(cfg) => cfg,
            Err(e) => {
                return vec![ValidationError {
                    field: "config".to_string(),
                    message: format!("Invalid Delay config: {e}"),
                }]
            }
        };
        let mut errors = Vec::new();
        if cfg.duration_ms.trim().is_empty() {
            errors.push(ValidationError {
                field: "duration_ms".to_string(),
                message: "Duration is required".to_string(),
            });
        }
        if cfg.jitter_ms > 60_000 {
            errors.push(ValidationError {
                field: "jitter_ms".to_string(),
                message: format!("jitter_ms = {} exceeds 60000", cfg.jitter_ms),
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
        DelayNodeExecutor::new()
            .execute(node, context, connections, node_map)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_plugin_validate_requires_duration() {
        let errors = DelayPlugin.validate(&json!({"type": "flow:delay", "duration_ms": " "}));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "duration_ms");
    }

    #[test]
    fn delay_plugin_validate_rejects_huge_jitter() {
        let errors = DelayPlugin.validate(&json!({
            "type": "flow:delay",
            "duration_ms": "10",
            "jitter_ms": 60001
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "jitter_ms");
    }

    #[test]
    fn delay_plugin_validate_accepts_default_config() {
        assert!(DelayPlugin
            .validate(&DelayPlugin.default_config())
            .is_empty());
    }

    #[test]
    fn delay_plugin_resolves_out_port() {
        assert_eq!(DelayPlugin.resolve_port(&None), "out");
        assert_eq!(
            DelayPlugin.resolve_port(&Some(json!({"requested_ms": 5}))),
            "out"
        );
        assert!(DelayPlugin.is_port_routing());
    }

    #[tokio::test]
    async fn delay_plugin_executes_via_executor() {
        let mut ctx = ExecutionContext::new("flow-1".to_string(), 0);
        let node = Node {
            uuid: "d1".to_string(),
            node_type: "flow:delay".to_string(),
            name: "delay".to_string(),
            data: flow_engine_core::node_config::NodeData::delay("5"),
            ..Default::default()
        };
        let (next, output) = DelayPlugin
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(next.is_empty());
        assert_eq!(
            output.unwrap_or_else(|| panic!("expected output"))["requested_ms"],
            json!(5)
        );
    }
}
