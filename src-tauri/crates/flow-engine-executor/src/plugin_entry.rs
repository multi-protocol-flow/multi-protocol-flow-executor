// 入口节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// 入口节点插件
pub struct EntryPlugin;

#[async_trait::async_trait]
impl NodePlugin for EntryPlugin {
    fn type_id(&self) -> &str {
        "entry"
    }

    fn display_name(&self) -> &str {
        "Entry"
    }

    fn category(&self) -> &str {
        "logic"
    }

    fn icon(&self) -> &str {
        "play"
    }

    fn color(&self) -> &str {
        "#22C55E"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "输出".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({"initial_variables": []})
    }

    /// 入口节点固定返回 "out" 端口，不做端口路由
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    /// 入口节点不做端口路由（始终走 out）
    fn is_port_routing(&self) -> bool {
        false
    }

    /// 入口节点无报告输入
    fn build_report_input(&self, _config: &serde_json::Value) -> Option<serde_json::Value> {
        None
    }

    /// 委托给 `EntryNodeExecutor` 执行
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::entry_executor::EntryNodeExecutor::new();
        executor.execute(node, context, connections, node_map).await
    }
}
