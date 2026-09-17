//! HTTP Ingress 节点插件 — `NodePlugin` 实现
//! Mock 流程的服务端请求入口节点

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// HTTP Ingress 监听入口节点插件
pub struct HttpIngressPlugin;

#[async_trait::async_trait]
impl NodePlugin for HttpIngressPlugin {
    fn type_id(&self) -> &str {
        "http:ingress"
    }

    fn display_name(&self) -> &str {
        "HTTP Ingress"
    }

    fn category(&self) -> &str {
        "network"
    }

    fn icon(&self) -> &str {
        "log-in"
    }

    fn color(&self) -> &str {
        "#6366F1"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "Output".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "http:ingress",
            "port": 9527,
            "path_pattern": "/*",
            "methods": ["GET", "POST"]
        })
    }

    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_port_routing(&self) -> bool {
        false
    }

    fn build_report_input(&self, _config: &serde_json::Value) -> Option<serde_json::Value> {
        None
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        // 收集入参数据（由 IngressServer 注入到 context.variables）
        let request_body = context
            .variables
            .get_variable("request.body")
            .or_else(|| context.variables.get_variable("body"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let request_method = context
            .variables
            .get_variable("request.method")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("GET"));
        let request_path = context
            .variables
            .get_variable("request.path")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("/"));
        let request_query = context
            .variables
            .get_variable("request.query")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let request_headers = context
            .variables
            .get_variable("request.headers")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let output = serde_json::json!({
            "method": request_method,
            "path": request_path,
            "query": request_query,
            "headers": request_headers,
            "body": request_body,
        });

        // 沿 out / output 端口继续推进
        let next_nodes: Vec<String> = connections
            .iter()
            .filter(|conn| {
                conn.source_node_uuid == node.uuid
                    && (conn.source_port_id == "out" || conn.source_port_id == "output")
            })
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        Ok((next_nodes, Some(output)))
    }
}
