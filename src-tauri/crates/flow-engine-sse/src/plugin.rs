// SSE 节点插件 — NodePlugin trait 实现
//
// 提供 3 个 SSE 节点的元数据、端口定义、配置验证和执行能力：
// - SseConnectPlugin: SSE 连接
// - SseListenPlugin: SSE 监听
// - SseDisconnectPlugin: SSE 断开连接
// 以及 SsePluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// SseConnectPlugin
// ---------------------------------------------------------------------------

/// SSE 连接节点插件
pub struct SseConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for SseConnectPlugin {
    fn type_id(&self) -> &str {
        "sse_connect"
    }

    fn display_name(&self) -> &str {
        "SSE Connect"
    }

    fn category(&self) -> &str {
        "sse"
    }

    fn icon(&self) -> &str {
        "radio-tower"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "Connect Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "connection failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "url": "https://example.com/sse",
            "timeout_ms": 30000,
            "auto_reconnect": false,
        })
    }

    /// `SseConnect` 检查 $.success → "true" / "false"
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
            "last_event_id": config.get("last_event_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match config.get("url").and_then(|v| v.as_str()) {
            Some(url) if !url.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "url".into(),
                message: "URL is required".into(),
            }),
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
        let sse_manager = context
            .get_provider::<crate::sse_connection_manager::SseConnectionManager>("sse")
            .unwrap_or_else(
                || Arc::new(crate::sse_connection_manager::SseConnectionManager::new()),
            );
        let executor = crate::sse_connect_executor::SseConnectExecutor::new(sse_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// SseListenPlugin
// ---------------------------------------------------------------------------

/// SSE 监听节点插件
pub struct SseListenPlugin;

#[async_trait::async_trait]
impl NodePlugin for SseListenPlugin {
    fn type_id(&self) -> &str {
        "sse_listen"
    }

    fn display_name(&self) -> &str {
        "SSE Listen"
    }

    fn category(&self) -> &str {
        "sse"
    }

    fn icon(&self) -> &str {
        "ear"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "收到事件".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "超时/无事件".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "max_events": 10,
            "timeout_ms": 5000,
        })
    }

    /// `SseListen` 检查 $.success → "true" / "false"
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
            "connection_id": config.get("connection_id"),
            "event_filter": config.get("event_filter"),
            "max_events": config.get("max_events"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "Connection ID is required".into(),
            }),
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
        let sse_manager = context
            .get_provider::<crate::sse_connection_manager::SseConnectionManager>("sse")
            .unwrap_or_else(
                || Arc::new(crate::sse_connection_manager::SseConnectionManager::new()),
            );
        let executor = crate::sse_listen_executor::SseListenExecutor::new(sse_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// SseDisconnectPlugin
// ---------------------------------------------------------------------------

/// SSE 断开连接节点插件
pub struct SseDisconnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for SseDisconnectPlugin {
    fn type_id(&self) -> &str {
        "sse_disconnect"
    }

    fn display_name(&self) -> &str {
        "SSE Disconnect"
    }

    fn category(&self) -> &str {
        "sse"
    }

    fn icon(&self) -> &str {
        "unplug"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "完成".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
        })
    }

    /// `SseDisconnect` 始终返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "Connection ID is required".into(),
            }),
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
        let sse_manager = context
            .get_provider::<crate::sse_connection_manager::SseConnectionManager>("sse")
            .unwrap_or_else(
                || Arc::new(crate::sse_connection_manager::SseConnectionManager::new()),
            );
        let executor = crate::sse_disconnect_executor::SseDisconnectExecutor::new(sse_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// SsePluginGroup
// ---------------------------------------------------------------------------

/// SSE 插件组 — 批量注册辅助
pub struct SsePluginGroup;

impl SsePluginGroup {
    /// 注册全部 SSE 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(SseConnectPlugin));
        registry.register(Arc::new(SseListenPlugin));
        registry.register(Arc::new(SseDisconnectPlugin));
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 元数据测试 ----

    #[test]
    fn test_sse_connect_plugin_metadata() {
        let plugin = SseConnectPlugin;
        assert_eq!(plugin.type_id(), "sse_connect");
        assert_eq!(plugin.display_name(), "SSE Connect");
        assert_eq!(plugin.category(), "sse");
        assert_eq!(plugin.icon(), "radio-tower");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.ports().iter().any(|p| p.id == "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_sse_listen_plugin_metadata() {
        let plugin = SseListenPlugin;
        assert_eq!(plugin.type_id(), "sse_listen");
        assert_eq!(plugin.display_name(), "SSE Listen");
        assert_eq!(plugin.category(), "sse");
        assert_eq!(plugin.icon(), "ear");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.ports().iter().any(|p| p.id == "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_sse_disconnect_plugin_metadata() {
        let plugin = SseDisconnectPlugin;
        assert_eq!(plugin.type_id(), "sse_disconnect");
        assert_eq!(plugin.display_name(), "SSE Disconnect");
        assert_eq!(plugin.category(), "sse");
        assert_eq!(plugin.icon(), "unplug");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(!plugin.ports().iter().any(|p| p.id == "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_sse_connect_resolve_port() {
        let plugin = SseConnectPlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
        assert_eq!(plugin.resolve_port(&None), "false");
    }

    #[test]
    fn test_sse_listen_resolve_port() {
        let plugin = SseListenPlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
        assert_eq!(plugin.resolve_port(&None), "false");
    }

    #[test]
    fn test_sse_disconnect_resolve_port() {
        let plugin = SseDisconnectPlugin;
        // SseDisconnect 始终返回 "out"
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "out"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "out"
        );
        assert_eq!(plugin.resolve_port(&None), "out");
    }

    // ---- default_config 测试 ----

    #[test]
    fn test_sse_connect_default_config() {
        let plugin = SseConnectPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("url").is_some());
        assert!(config.get("timeout_ms").is_some());
        assert!(config.get("auto_reconnect").is_some());
    }

    #[test]
    fn test_default_config_not_empty() {
        let connect_plugin = SseConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("url").is_some());
        assert!(config.get("timeout_ms").is_some());

        let listen_plugin = SseListenPlugin;
        let config = listen_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("max_events").is_some());
        assert!(config.get("timeout_ms").is_some());

        let disconnect_plugin = SseDisconnectPlugin;
        let config = disconnect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = SseConnectPlugin;
        let config = serde_json::json!({
            "url": "https://example.com/events",
            "last_event_id": "evt-123",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["url"], "https://example.com/events");
        assert_eq!(report["last_event_id"], "evt-123");
    }

    #[test]
    fn test_listen_build_report_input() {
        let plugin = SseListenPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "event_filter": "message",
            "max_events": 5,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["event_filter"], "message");
        assert_eq!(report["max_events"], 5);
    }

    #[test]
    fn test_disconnect_build_report_input() {
        let plugin = SseDisconnectPlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    // ---- SsePluginGroup.register_all 测试 ----

    #[test]
    fn test_sse_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        SsePluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 3);

        assert!(registry.get("sse_connect").is_some());
        assert!(registry.get("sse_listen").is_some());
        assert!(registry.get("sse_disconnect").is_some());
    }
}
