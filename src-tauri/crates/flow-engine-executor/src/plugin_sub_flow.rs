// SubFlow 节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::SubFlowConfig;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;

/// 子流程节点插件
pub struct SubFlowPlugin;

#[async_trait::async_trait]
impl NodePlugin for SubFlowPlugin {
    fn type_id(&self) -> &str {
        "flow:sub"
    }

    fn display_name(&self) -> &str {
        "SubFlow"
    }

    fn category(&self) -> &str {
        "logic"
    }

    fn icon(&self) -> &str {
        "workflow"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "in".into(),
                label: "Input".into(),
                position: "left".into(),
            },
            PortDef {
                id: "out".into(),
                label: "Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "flow:sub",
            "target_flow_id": null,
            "target_flow_path": null,
            "inherit_all_variables": true,
            "variable_mappings": [],
            "export_all_variables": true,
            "output_mappings": []
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(false)) = map.get("success") {
                    "false".to_string()
                } else {
                    "out".to_string()
                }
            }
            Some(_) => "out".to_string(),
            None => "false".to_string(),
        }
    }

    fn is_port_routing(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        let parsed: SubFlowConfig = serde_json::from_value(config.clone()).ok()?;
        // 前端以空串表达"该引用未使用"（二选一），报告中按未设置呈现
        let target_flow_id = parsed.target_flow_id.filter(|s| !s.trim().is_empty());
        let target_flow_path = parsed.target_flow_path.filter(|s| !s.trim().is_empty());
        Some(serde_json::json!({
            "target_flow_id": target_flow_id,
            "target_flow_path": target_flow_path,
            "inherit_all_variables": parsed.inherit_all_variables,
            "variable_mappings": parsed.variable_mappings,
            "export_all_variables": parsed.export_all_variables,
            "output_mappings": parsed.output_mappings,
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match serde_json::from_value::<SubFlowConfig>(config.clone()) {
            Ok(cfg) => {
                let has_id = cfg
                    .target_flow_id
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty());
                let has_path = cfg
                    .target_flow_path
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty());
                if !has_id && !has_path {
                    errors.push(ValidationError {
                        field: "target_flow".to_string(),
                        message: "Either target_flow_id or target_flow_path must be specified"
                            .to_string(),
                    });
                }
            }
            Err(e) => {
                errors.push(ValidationError {
                    field: "config".to_string(),
                    message: format!("Invalid SubFlow config: {}", e),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::sub_flow_executor::SubFlowNodeExecutor::new();
        executor.execute(node, context, connections, node_map).await
    }
}
