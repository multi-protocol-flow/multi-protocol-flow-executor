// MQTT 节点插件 — NodePlugin trait 实现
//
// 提供 7 个 MQTT 节点的元数据、端口定义、配置验证和执行能力：
// - MqttConnectPlugin: MQTT 连接
// - MqttPublishPlugin: MQTT 发布
// - MqttSubscribePlugin: MQTT 订阅
// - MqttUnsubscribePlugin: MQTT 取消订阅
// - MqttDisconnectPlugin: MQTT 断开连接
// - MqttWaitMessagePlugin: MQTT 等待消息
// - MqttRequestPlugin: MQTT 请求/响应
// 以及 MqttPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// MqttConnectPlugin
// ---------------------------------------------------------------------------

/// MQTT 连接节点插件
pub struct MqttConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttConnectPlugin {
    fn type_id(&self) -> &str {
        "mqtt_connect"
    }

    fn display_name(&self) -> &str {
        "MQTT 连接"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "radio"
    }

    fn color(&self) -> &str {
        "#06b6d4"
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
            "url": "mqtt://localhost:1883",
            "mqtt_version": "5.0",
            "clean_session": true,
            "keep_alive_secs": 60,
            "connect_timeout_ms": 5000,
        })
    }

    /// `MqttConnect` 检查 $.success → "true" / "false"
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
            "url": config.get("url"),
            "client_id": config.get("client_id"),
            "mqtt_version": config.get("mqtt_version"),
            "use_tls": config.get("use_tls"),
            "clean_session": config.get("clean_session"),
            "keep_alive_secs": config.get("keep_alive_secs"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_connect_executor::MqttConnectExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttPublishPlugin
// ---------------------------------------------------------------------------

/// MQTT 发布节点插件
pub struct MqttPublishPlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttPublishPlugin {
    fn type_id(&self) -> &str {
        "mqtt_publish"
    }

    fn display_name(&self) -> &str {
        "MQTT 发布"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "send"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "发布成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "发布失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "payload": null,
            "qos": 0,
        })
    }

    /// `MqttPublish` 检查 $.success → "true" / "false"
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
            "topic": config.get("topic"),
            "qos": config.get("qos"),
            "retain": config.get("retain"),
            "payload_format": config.get("payload_format"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_publish_executor::MqttPublishExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttSubscribePlugin
// ---------------------------------------------------------------------------

/// MQTT 订阅节点插件
pub struct MqttSubscribePlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttSubscribePlugin {
    fn type_id(&self) -> &str {
        "mqtt_subscribe"
    }

    fn display_name(&self) -> &str {
        "MQTT 订阅"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "arrow-down-to-line"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "订阅成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "订阅失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "qos": 0,
            "max_messages": null,
            "collect_timeout_ms": null,
            "output_mode": "first",
        })
    }

    /// `MqttSubscribe` 检查 $.success → "true" / "false"
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
            "topic": config.get("topic"),
            "qos": config.get("qos"),
            "output_mode": config.get("output_mode"),
            "max_messages": config.get("max_messages"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_subscribe_executor::MqttSubscribeExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttUnsubscribePlugin
// ---------------------------------------------------------------------------

/// MQTT 取消订阅节点插件
pub struct MqttUnsubscribePlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttUnsubscribePlugin {
    fn type_id(&self) -> &str {
        "mqtt_unsubscribe"
    }

    fn display_name(&self) -> &str {
        "MQTT 取消订阅"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "x-circle"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "取消成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "取消失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
        })
    }

    /// `MqttUnsubscribe` 检查 $.success → "true" / "false"
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
            "topic": config.get("topic"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_unsubscribe_executor::MqttUnsubscribeExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttDisconnectPlugin
// ---------------------------------------------------------------------------

/// MQTT 断开连接节点插件
pub struct MqttDisconnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttDisconnectPlugin {
    fn type_id(&self) -> &str {
        "mqtt_disconnect"
    }

    fn display_name(&self) -> &str {
        "MQTT 断开连接"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "unplug"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "输出".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
        })
    }

    /// `MqttDisconnect` 始终返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    fn supports_false_port(&self) -> bool {
        false
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_disconnect_executor::MqttDisconnectExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttWaitMessagePlugin
// ---------------------------------------------------------------------------

/// MQTT 等待消息节点插件
pub struct MqttWaitMessagePlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttWaitMessagePlugin {
    fn type_id(&self) -> &str {
        "mqtt_wait_message"
    }

    fn display_name(&self) -> &str {
        "MQTT 等待消息"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "timer"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "收到消息".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "等待超时".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "timeout_ms": 30000,
            "max_messages": null,
        })
    }

    /// `MqttWaitMessage` 检查 $.success → "true" / "false"
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
            "topic": config.get("topic"),
            "timeout_ms": config.get("timeout_ms"),
            "max_messages": config.get("max_messages"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_wait_message_executor::MqttWaitMessageExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttRequestPlugin
// ---------------------------------------------------------------------------

/// MQTT 请求/响应节点插件
pub struct MqttRequestPlugin;

#[async_trait::async_trait]
impl NodePlugin for MqttRequestPlugin {
    fn type_id(&self) -> &str {
        "mqtt_request"
    }

    fn display_name(&self) -> &str {
        "MQTT 请求/响应"
    }

    fn category(&self) -> &str {
        "mqtt"
    }

    fn icon(&self) -> &str {
        "message-square"
    }

    fn color(&self) -> &str {
        "#06b6d4"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "请求成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "请求失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "request_topic": "",
            "response_topic": "",
            "payload": null,
            "qos": null,
            "timeout_ms": 30000,
        })
    }

    /// `MqttRequest` 检查 $.success → "true" / "false"
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
            "request_topic": config.get("request_topic"),
            "response_topic": config.get("response_topic"),
            "qos": config.get("qos"),
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
        let mqtt_manager = context.mqtt_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::mqtt_connection_manager::MqttConnectionManager::new())
        });
        let executor = crate::mqtt_request_executor::MqttRequestExecutor::new(mqtt_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// MqttPluginGroup
// ---------------------------------------------------------------------------

/// MQTT 插件组 — 批量注册辅助
pub struct MqttPluginGroup;

impl MqttPluginGroup {
    /// 注册全部 MQTT 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(MqttConnectPlugin));
        registry.register(Arc::new(MqttPublishPlugin));
        registry.register(Arc::new(MqttSubscribePlugin));
        registry.register(Arc::new(MqttUnsubscribePlugin));
        registry.register(Arc::new(MqttDisconnectPlugin));
        registry.register(Arc::new(MqttWaitMessagePlugin));
        registry.register(Arc::new(MqttRequestPlugin));
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
    fn test_mqtt_connect_plugin_metadata() {
        let plugin = MqttConnectPlugin;
        assert_eq!(plugin.type_id(), "mqtt_connect");
        assert_eq!(plugin.display_name(), "MQTT 连接");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "radio");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mqtt_publish_plugin_metadata() {
        let plugin = MqttPublishPlugin;
        assert_eq!(plugin.type_id(), "mqtt_publish");
        assert_eq!(plugin.display_name(), "MQTT 发布");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "send");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mqtt_subscribe_plugin_metadata() {
        let plugin = MqttSubscribePlugin;
        assert_eq!(plugin.type_id(), "mqtt_subscribe");
        assert_eq!(plugin.display_name(), "MQTT 订阅");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "arrow-down-to-line");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mqtt_disconnect_plugin_metadata() {
        let plugin = MqttDisconnectPlugin;
        assert_eq!(plugin.type_id(), "mqtt_disconnect");
        assert_eq!(plugin.display_name(), "MQTT 断开连接");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "unplug");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(!plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    #[test]
    fn test_mqtt_wait_message_plugin_metadata() {
        let plugin = MqttWaitMessagePlugin;
        assert_eq!(plugin.type_id(), "mqtt_wait_message");
        assert_eq!(plugin.display_name(), "MQTT 等待消息");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "timer");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mqtt_request_plugin_metadata() {
        let plugin = MqttRequestPlugin;
        assert_eq!(plugin.type_id(), "mqtt_request");
        assert_eq!(plugin.display_name(), "MQTT 请求/响应");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "message-square");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_mqtt_connect_resolve_port() {
        let plugin = MqttConnectPlugin;
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
    fn test_mqtt_disconnect_resolve_port() {
        let plugin = MqttDisconnectPlugin;
        // MqttDisconnect 始终返回 "out"
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

    #[test]
    fn test_mqtt_publish_resolve_port() {
        let plugin = MqttPublishPlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
    }

    #[test]
    fn test_mqtt_unsubscribe_resolve_port() {
        let plugin = MqttUnsubscribePlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
    }

    #[test]
    fn test_resolve_port_edge_cases() {
        let plugin = MqttConnectPlugin;

        // success 为 null → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": null}))),
            "false"
        );

        // success 为数字 → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": 1}))),
            "false"
        );

        // 空 object → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({}))),
            "false"
        );

        // 非 object → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!("string"))),
            "false"
        );

        // array → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!([1, 2, 3]))),
            "false"
        );
    }

    // ---- default_config 测试 ----

    #[test]
    fn test_default_config_not_empty() {
        let connect_plugin = MqttConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("url").is_some());
        assert!(config.get("mqtt_version").is_some());

        let publish_plugin = MqttPublishPlugin;
        let config = publish_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("topic").is_some());

        let subscribe_plugin = MqttSubscribePlugin;
        let config = subscribe_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("topic").is_some());

        let disconnect_plugin = MqttDisconnectPlugin;
        let config = disconnect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());

        let wait_plugin = MqttWaitMessagePlugin;
        let config = wait_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("timeout_ms").is_some());

        let request_plugin = MqttRequestPlugin;
        let config = request_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("request_topic").is_some());
        assert!(config.get("response_topic").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = MqttConnectPlugin;
        let config = serde_json::json!({
            "url": "mqtt://broker.example.com:1883",
            "mqtt_version": "5.0",
            "client_id": "test-client",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["url"], "mqtt://broker.example.com:1883");
        assert_eq!(report["mqtt_version"], "5.0");
    }

    #[test]
    fn test_publish_build_report_input() {
        let plugin = MqttPublishPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "topic": "test/topic",
            "qos": 1,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["topic"], "test/topic");
        assert_eq!(report["qos"], 1);
    }

    #[test]
    fn test_subscribe_build_report_input() {
        let plugin = MqttSubscribePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "topic": "sensor/#",
            "qos": 0,
            "output_mode": "first",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["topic"], "sensor/#");
    }

    #[test]
    fn test_disconnect_build_report_input() {
        let plugin = MqttDisconnectPlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    #[test]
    fn test_request_build_report_input() {
        let plugin = MqttRequestPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "request_topic": "req/topic",
            "response_topic": "resp/topic",
            "timeout_ms": 5000,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["request_topic"], "req/topic");
        assert_eq!(report["response_topic"], "resp/topic");
    }

    // ---- MqttPluginGroup.register_all 测试 ----

    #[test]
    fn test_mqtt_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        MqttPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 7);

        assert!(registry.get("mqtt_connect").is_some());
        assert!(registry.get("mqtt_publish").is_some());
        assert!(registry.get("mqtt_subscribe").is_some());
        assert!(registry.get("mqtt_unsubscribe").is_some());
        assert!(registry.get("mqtt_disconnect").is_some());
        assert!(registry.get("mqtt_wait_message").is_some());
        assert!(registry.get("mqtt_request").is_some());
    }

    #[test]
    fn test_mqtt_unsubscribe_plugin_metadata() {
        let plugin = MqttUnsubscribePlugin;
        assert_eq!(plugin.type_id(), "mqtt_unsubscribe");
        assert_eq!(plugin.display_name(), "MQTT 取消订阅");
        assert_eq!(plugin.category(), "mqtt");
        assert_eq!(plugin.icon(), "x-circle");
        assert_eq!(plugin.color(), "#06b6d4");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_wait_message_resolve_port() {
        let plugin = MqttWaitMessagePlugin;
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
    fn test_request_resolve_port() {
        let plugin = MqttRequestPlugin;
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
}
