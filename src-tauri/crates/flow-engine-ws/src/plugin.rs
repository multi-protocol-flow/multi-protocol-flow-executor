// WebSocket 节点插件 — NodePlugin trait 实现
//
// 提供 3 个 WebSocket 节点的元数据、端口定义、配置验证和执行能力：
// - WsConnectPlugin: WebSocket 连接
// - WsSendCollectPlugin: WebSocket 发送采集
// - WsClosePlugin: WebSocket 关闭
// 以及 WsPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

/// WebSocket 连接节点插件
pub struct WsConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for WsConnectPlugin {
    fn type_id(&self) -> &str {
        "ws_connect"
    }

    fn display_name(&self) -> &str {
        "WebSocket 连接"
    }

    fn category(&self) -> &str {
        "websocket"
    }

    fn icon(&self) -> &str {
        "plug"
    }

    fn color(&self) -> &str {
        "#06B6D4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "连接成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "连接失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "url": "",
            "headers": [],
            "subprotocols": [],
            "connect_timeout_ms": 30000,
            "use_tls": false,
            "tls_skip_verify": false,
            "ping_interval_ms": null,
            "ping_timeout_ms": null,
            "collect_message": false,
            "collect_mode": "FirstMatch",
            "collect_timeout_ms": 10000,
        })
    }

    /// `WsConnect` 检查 $.connected → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("connected") {
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

    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "url": config.get("url"),
            "headers": config.get("headers"),
            "subprotocols": config.get("subprotocols"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
            "collect_message": config.get("collect_message"),
            "collect_mode": config.get("collect_mode"),
            "collect_timeout_ms": config.get("collect_timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let ws_manager = context
            .get_provider::<crate::websocket_manager::WebSocketConnectionManager>("ws")
            .unwrap_or_else(|| Arc::new(crate::websocket_manager::WebSocketConnectionManager::new()));
        let executor = crate::ws_connect_executor::WsConnectExecutor::new(ws_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// WebSocket 发送采集节点插件
pub struct WsSendCollectPlugin;

#[async_trait::async_trait]
impl NodePlugin for WsSendCollectPlugin {
    fn type_id(&self) -> &str {
        "ws_send_collect"
    }

    fn display_name(&self) -> &str {
        "WebSocket 发送采集"
    }

    fn category(&self) -> &str {
        "websocket"
    }

    fn icon(&self) -> &str {
        "send"
    }

    fn color(&self) -> &str {
        "#06B6D4"
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
            "connection_id": "",
            "message": "",
            "message_type": "text",
            "send_timeout_ms": 10000,
            "collect_response": true,
            "collect_mode": "FirstMatch",
            "collect_timeout_ms": 10000,
        })
    }

    /// `WsSendCollect` 检查 $.success → "true" / "false"
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

    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "message": config.get("message"),
            "message_type": config.get("message_type"),
            "send_timeout_ms": config.get("send_timeout_ms"),
            "collect_response": config.get("collect_response"),
            "collect_mode": config.get("collect_mode"),
            "collect_timeout_ms": config.get("collect_timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let ws_manager = context
            .get_provider::<crate::websocket_manager::WebSocketConnectionManager>("ws")
            .unwrap_or_else(|| Arc::new(crate::websocket_manager::WebSocketConnectionManager::new()));
        let executor = crate::ws_send_collect_executor::WsSendCollectExecutor::new(ws_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// WebSocket 关闭节点插件
pub struct WsClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for WsClosePlugin {
    fn type_id(&self) -> &str {
        "ws_close"
    }

    fn display_name(&self) -> &str {
        "WebSocket 关闭"
    }

    fn category(&self) -> &str {
        "websocket"
    }

    fn icon(&self) -> &str {
        "x-circle"
    }

    fn color(&self) -> &str {
        "#06B6D4"
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
            "connection_id": "",
            "close_code": 1000,
            "close_reason": null,
            "close_timeout_ms": 5000,
        })
    }

    /// `WsClose` 检查 $.success → "true" / "false"
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

    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "close_code": config.get("close_code"),
            "close_reason": config.get("close_reason"),
            "close_timeout_ms": config.get("close_timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let ws_manager = context
            .get_provider::<crate::websocket_manager::WebSocketConnectionManager>("ws")
            .unwrap_or_else(|| Arc::new(crate::websocket_manager::WebSocketConnectionManager::new()));
        let executor = crate::ws_close_executor::WsCloseExecutor::new(ws_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// WebSocket 插件组 — 批量注册辅助
pub struct WsPluginGroup;

impl WsPluginGroup {
    /// 注册全部 WebSocket 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(WsConnectPlugin));
        registry.register(Arc::new(WsSendCollectPlugin));
        registry.register(Arc::new(WsClosePlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::plugin::NodePlugin;

    // --- WsSendCollectPlugin 测试 ---

    #[test]
    fn test_ws_send_collect_plugin_ports() {
        let plugin = WsSendCollectPlugin;
        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_ws_send_collect_plugin_resolve_port_true() {
        let plugin = WsSendCollectPlugin;
        let output = Some(serde_json::json!({"success": true}));
        assert_eq!(plugin.resolve_port(&output), "true");
    }

    #[test]
    fn test_ws_send_collect_plugin_resolve_port_false() {
        let plugin = WsSendCollectPlugin;
        let output = Some(serde_json::json!({"success": false}));
        assert_eq!(plugin.resolve_port(&output), "false");
    }

    #[test]
    fn test_ws_send_collect_plugin_resolve_port_none() {
        let plugin = WsSendCollectPlugin;
        assert_eq!(plugin.resolve_port(&None), "false");
    }

    // --- WsClosePlugin 测试 ---

    #[test]
    fn test_ws_close_plugin_ports() {
        let plugin = WsClosePlugin;
        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_ws_close_plugin_resolve_port_true() {
        let plugin = WsClosePlugin;
        let output = Some(serde_json::json!({"success": true}));
        assert_eq!(plugin.resolve_port(&output), "true");
    }

    #[test]
    fn test_ws_close_plugin_resolve_port_false() {
        let plugin = WsClosePlugin;
        let output = Some(serde_json::json!({"success": false}));
        assert_eq!(plugin.resolve_port(&output), "false");
    }

    #[test]
    fn test_ws_close_plugin_resolve_port_none() {
        let plugin = WsClosePlugin;
        assert_eq!(plugin.resolve_port(&None), "false");
    }
}
