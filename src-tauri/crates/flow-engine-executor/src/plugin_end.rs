// 结束节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// 结束节点插件
pub struct EndPlugin;

#[async_trait::async_trait]
impl NodePlugin for EndPlugin {
    fn type_id(&self) -> &str {
        "end"
    }

    fn display_name(&self) -> &str {
        "End"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "stop-circle"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "in".into(),
            label: "输入".into(),
            position: "left".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({"type": "end"})
    }

    /// 结束节点固定返回空字符串，表示无后续端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "".to_string()
    }

    /// `结束节点启用端口路由（resolve_port` 返回空字符串表示终止）
    fn is_port_routing(&self) -> bool {
        true
    }

    /// 结束节点无业务逻辑，直接返回Success
    async fn execute(
        &self,
        _node: &Node,
        _context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        Ok((vec![], None))
    }
}
