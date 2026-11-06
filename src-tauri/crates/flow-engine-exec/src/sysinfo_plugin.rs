//! `local:sysinfo` 节点插件

use crate::sysinfo::{self, SUPPORTED_CHECKS};
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{LocalSysinfoConfig, NodeData};
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use serde_json::json;
use std::collections::HashMap;

/// 本机信息节点插件
///
/// 只读信息节点：不执行任意命令、不接受程序名，因此**不需要审批**。
/// 单出口 `out`。
pub struct LocalSysinfoPlugin;

#[async_trait::async_trait]
impl NodePlugin for LocalSysinfoPlugin {
    fn type_id(&self) -> &str {
        flow_engine_core::node_types::LOCAL_SYSINFO
    }

    fn display_name(&self) -> &str {
        "Local System Info"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "cpu"
    }

    fn color(&self) -> &str {
        "#0EA5E9"
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
            "type": "local:sysinfo",
            "checks": ["os", "cpu", "mem", "ffmpeg"],
            "disk_path": ""
        })
    }

    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_port_routing(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(json!({
            "checks": config.get("checks").cloned().unwrap_or(json!([])),
            "disk_path": config.get("disk_path").cloned().unwrap_or(json!(null)),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let cfg: LocalSysinfoConfig = match serde_json::from_value(config.clone()) {
            Ok(cfg) => cfg,
            Err(e) => {
                return vec![ValidationError {
                    field: "config".to_string(),
                    message: e.to_string(),
                }];
            }
        };
        cfg.checks
            .iter()
            .filter(|check| {
                let normalized = check.trim().to_ascii_lowercase();
                !SUPPORTED_CHECKS.contains(&normalized.as_str())
            })
            .map(|check| ValidationError {
                field: "checks".to_string(),
                message: format!(
                    "unknown check `{check}` (expected os | cpu | mem | disk | ffmpeg)"
                ),
            })
            .collect()
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let cfg = match &node.data {
            NodeData::LocalSysinfo(cfg) => cfg.clone(),
            _ => LocalSysinfoConfig::default(),
        };
        let output = sysinfo::collect(&cfg, context).await?;
        let next: Vec<String> = connections
            .iter()
            .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == "out")
            .map(|c| c.target_node_uuid.clone())
            .collect();
        Ok((next, Some(output)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_config_is_valid() {
        assert!(LocalSysinfoPlugin
            .validate(&LocalSysinfoPlugin.default_config())
            .is_empty());
    }

    #[test]
    fn unknown_check_is_rejected() {
        let errors = LocalSysinfoPlugin.validate(&json!({
            "type": "local:sysinfo",
            "checks": ["os", "gpu"]
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "checks");
        assert!(errors[0].message.contains("gpu"));
    }

    #[tokio::test]
    async fn executes_and_routes_out_port() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let node = Node {
            uuid: "sysinfo-1".to_string(),
            node_type: "local:sysinfo".to_string(),
            name: "sysinfo".to_string(),
            data: NodeData::LocalSysinfo(LocalSysinfoConfig {
                checks: vec!["os".to_string()],
                disk_path: None,
            }),
            ..Default::default()
        };
        let connections = vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "sysinfo-1".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "next".to_string(),
            target_port_id: "in".to_string(),
        }];
        let (next, output) = LocalSysinfoPlugin
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(next, vec!["next".to_string()]);
        let output = output.expect("output");
        assert_eq!(output["success"], json!(true));
        assert!(output.get("os").is_some());
        assert!(output.get("ffmpeg_version").is_none());
    }

    #[test]
    fn resolves_out_port_always() {
        assert_eq!(LocalSysinfoPlugin.resolve_port(&None), "out");
        assert_eq!(
            LocalSysinfoPlugin.resolve_port(&Some(json!({"success": false}))),
            "out"
        );
    }
}
