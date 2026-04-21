// HTTP 请求节点插件 — NodePlugin trait 实现
//
// 提供 HTTP 节点的元数据、端口定义、配置验证和执行能力。
// 执行委托给 HttpNodeExecutor。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;

/// HTTP 请求节点插件
pub struct HttpPlugin;

#[async_trait::async_trait]
impl NodePlugin for HttpPlugin {
    fn type_id(&self) -> &str {
        "http"
    }

    fn display_name(&self) -> &str {
        "HTTP 请求"
    }

    fn category(&self) -> &str {
        "http"
    }

    fn icon(&self) -> &str {
        "globe"
    }

    fn color(&self) -> &str {
        "#3B82F6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "url": "",
            "method": "get",
            "headers": {},
            "body_type": "none",
            "content_type": "json",
        })
    }

    /// HTTP 检查 $.success → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "url": config.get("url"),
            "method": config.get("method"),
            "headers": config.get("headers"),
            "body": config.get("body"),
            "body_type": config.get("body_type"),
            "timeout_ms": config.get("timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let executor = crate::http_executor::HttpNodeExecutor::new();
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}
