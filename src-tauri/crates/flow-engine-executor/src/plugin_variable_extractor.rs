// 变量提取节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::VariableExtractorConfig;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// 变量提取节点插件
pub struct VariableExtractorPlugin;

#[async_trait::async_trait]
impl NodePlugin for VariableExtractorPlugin {
    fn type_id(&self) -> &str {
        "variable:extractor"
    }

    fn display_name(&self) -> &str {
        "Variable Extraction"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "braces"
    }

    fn color(&self) -> &str {
        "#F59E0B"
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
                label: "Output".into(),
                position: "right".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "variable:extractor",
            "output_mappings": [],
            "source_node_id": null
        })
    }

    /// 变量提取节点固定返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    /// 不做端口路由（始终走 out）
    fn is_port_routing(&self) -> bool {
        false
    }

    /// 报告输入数据
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        let parsed: VariableExtractorConfig = serde_json::from_value(config.clone()).ok()?;
        Some(serde_json::json!({
            "mapped_count": parsed.output_mappings.len(),
            "source_node_id": parsed.source_node_id,
        }))
    }

    /// 委托给 `VariableExtractorExecutor` 执行
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::variable_extractor_executor::VariableExtractorExecutor;
        executor.execute(node, context, connections, node_map).await
    }
}
