//! HTTP Respond 响应节点插件 — `NodePlugin` 实现
//! Mock 流程的服务端响应输出节点

use crate::ingress::server::{IngressResponseChannel, IngressResponsePayload};
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// HTTP Respond 响应节点插件
pub struct HttpRespondPlugin;

#[async_trait::async_trait]
impl NodePlugin for HttpRespondPlugin {
    fn type_id(&self) -> &str {
        "http:respond"
    }

    fn display_name(&self) -> &str {
        "HTTP Respond"
    }

    fn category(&self) -> &str {
        "network"
    }

    fn icon(&self) -> &str {
        "log-out"
    }

    fn color(&self) -> &str {
        "#14B8A6"
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
            "type": "http:respond",
            "status_code": 200,
            "headers": {},
            "body_type": "json",
            "body": "{\"success\": true}"
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
        let (status_code, raw_headers, raw_body, body_type) = match &node.data {
            NodeData::HttpRespond(cfg) => (
                cfg.status_code,
                cfg.headers.clone(),
                cfg.body.clone(),
                cfg.body_type.clone(),
            ),
            _ => (
                200,
                HashMap::new(),
                r#"{"success": true}"#.to_string(),
                "json".to_string(),
            ),
        };

        // 解析 headers 中的变量 {{var}}
        let mut evaluated_headers = HashMap::new();
        for (k, v) in raw_headers {
            let eval_v = flow_engine_core::executor::replace_variables(&v, context).unwrap_or(v);
            evaluated_headers.insert(k, eval_v);
        }

        // 默认加上 Content-Type
        if !evaluated_headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("content-type"))
        {
            if body_type == "json" {
                evaluated_headers.insert(
                    "Content-Type".to_string(),
                    "application/json; charset=utf-8".to_string(),
                );
            } else if body_type == "text" {
                evaluated_headers.insert(
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                );
            }
        }

        // 解析 body 中的变量 {{var}}
        let evaluated_body =
            flow_engine_core::executor::replace_variables(&raw_body, context).unwrap_or(raw_body);

        // 如果上下文中注册了 IngressResponseChannel，将响应送回等待中的 HTTP 连接
        if let Some(channel) = context
            .get_provider::<IngressResponseChannel>(crate::ingress::INGRESS_RESPONSE_PROVIDER)
        {
            let mut guard = channel.tx.lock().await;
            if let Some(sender) = guard.take() {
                let _ = sender.send(IngressResponsePayload {
                    status_code,
                    headers: evaluated_headers.clone(),
                    body: evaluated_body.clone().into_bytes(),
                });
            }
        }

        let output = serde_json::json!({
            "status_code": status_code,
            "headers": evaluated_headers,
            "body": evaluated_body,
        });

        // 沿 out 端口连接（如果有后续节点如通知或日志）
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
