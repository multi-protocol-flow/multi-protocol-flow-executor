// 条件分支节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// 条件分支节点插件
pub struct ConditionalPlugin;

#[async_trait::async_trait]
impl NodePlugin for ConditionalPlugin {
    fn type_id(&self) -> &str {
        "conditional"
    }

    fn display_name(&self) -> &str {
        "条件分支"
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

    /// 委托给 `ConditionalNodeExecutor` 执行
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let executor = crate::conditional_executor::ConditionalNodeExecutor::new();
        executor.execute(node, context, connections, node_map).await
    }
}
