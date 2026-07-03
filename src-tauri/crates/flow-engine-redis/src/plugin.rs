//! Redis 节点插件注册

use async_trait::async_trait;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::PortDef;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::connection::Connection;
use flow_engine_core::plugin::{NodePlugin, ValidationError};
use flow_engine_core::registry::NodeRegistry;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// RedisConnectPlugin
// ---------------------------------------------------------------------------

/// Redis Connect节点插件
pub struct RedisConnectPlugin;

#[async_trait]
impl NodePlugin for RedisConnectPlugin {
    fn type_id(&self) -> &str {
        "redis_connect"
    }

    fn display_name(&self) -> &str {
        "Redis Connect"
    }

    fn category(&self) -> &str {
        "redis"
    }

    fn icon(&self) -> &str {
        "radio"
    }

    fn color(&self) -> &str {
        "#dc2626"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Failure".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "host": "127.0.0.1",
            "port": 6379,
            "password": null,
            "db_index": 0,
            "connect_timeout_ms": 10000,
        })
    }

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
        Some(json!({
            "host": config.get("host"),
            "port": config.get("port"),
            "db_index": config.get("db_index"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match config.get("host").and_then(|v| v.as_str()) {
            Some(host) if !host.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "host".into(),
                message: "Host is required".into(),
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
        let redis_manager = context
            .get_provider::<crate::redis_connection_manager::RedisConnectionManager>("redis")
            .unwrap_or_else(|| Arc::new(crate::redis_connection_manager::RedisConnectionManager::new()));
        let executor = crate::redis_connect_executor::RedisConnectExecutor::new(redis_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// RedisCommandPlugin
// ---------------------------------------------------------------------------

/// Redis Command节点插件
pub struct RedisCommandPlugin;

#[async_trait]
impl NodePlugin for RedisCommandPlugin {
    fn type_id(&self) -> &str {
        "redis_command"
    }

    fn display_name(&self) -> &str {
        "Redis Command"
    }

    fn category(&self) -> &str {
        "redis"
    }

    fn icon(&self) -> &str {
        "terminal"
    }

    fn color(&self) -> &str {
        "#dc2626"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Failure".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "connection_id": "",
            "command": "GET",
            "args": [],
        })
    }

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
        Some(json!({
            "connection_id": config.get("connection_id"),
            "command": config.get("command"),
            "args": config.get("args"),
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
        match config.get("command").and_then(|v| v.as_str()) {
            Some(cmd) if !cmd.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "command".into(),
                message: "Command is required".into(),
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
        let redis_manager = context
            .get_provider::<crate::redis_connection_manager::RedisConnectionManager>("redis")
            .unwrap_or_else(|| Arc::new(crate::redis_connection_manager::RedisConnectionManager::new()));
        let executor = crate::redis_command_executor::RedisCommandExecutor::new(redis_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// RedisClosePlugin
// ---------------------------------------------------------------------------

/// Redis Close连接节点插件
pub struct RedisClosePlugin;

#[async_trait]
impl NodePlugin for RedisClosePlugin {
    fn type_id(&self) -> &str {
        "redis_close"
    }

    fn display_name(&self) -> &str {
        "Redis Close"
    }

    fn category(&self) -> &str {
        "redis"
    }

    fn icon(&self) -> &str {
        "log-out"
    }

    fn color(&self) -> &str {
        "#dc2626"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "Output".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "connection_id": "",
        })
    }

    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(json!({
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
        let redis_manager = context
            .get_provider::<crate::redis_connection_manager::RedisConnectionManager>("redis")
            .unwrap_or_else(|| Arc::new(crate::redis_connection_manager::RedisConnectionManager::new()));
        let executor = crate::redis_close_executor::RedisCloseExecutor::new(redis_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// RedisPluginGroup
// ---------------------------------------------------------------------------

/// Redis 插件组，统一注册所有 Redis 节点
pub struct RedisPluginGroup;

impl RedisPluginGroup {
    /// Register all Redis node plugins into the given registry.
    ///
    /// # Arguments
    ///
    /// * `registry` - The node registry to register Redis plugins into.
    pub fn register_all(registry: &mut NodeRegistry) {
        registry.register(Arc::new(RedisConnectPlugin));
        registry.register(Arc::new(RedisCommandPlugin));
        registry.register(Arc::new(RedisClosePlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_connect_plugin_metadata() {
        let plugin = RedisConnectPlugin;
        assert_eq!(plugin.type_id(), "redis_connect");
        assert_eq!(plugin.display_name(), "Redis Connect");
        assert_eq!(plugin.category(), "redis");
        assert!(plugin.is_protocol_node());
        assert_eq!(plugin.ports().len(), 2);
    }

    #[test]
    fn test_redis_command_plugin_metadata() {
        let plugin = RedisCommandPlugin;
        assert_eq!(plugin.type_id(), "redis_command");
        assert_eq!(plugin.display_name(), "Redis Command");
        assert_eq!(plugin.category(), "redis");
        assert!(plugin.is_protocol_node());
        assert_eq!(plugin.ports().len(), 2);
    }

    #[test]
    fn test_redis_close_plugin_metadata() {
        let plugin = RedisClosePlugin;
        assert_eq!(plugin.type_id(), "redis_close");
        assert_eq!(plugin.display_name(), "Redis Close");
        assert_eq!(plugin.category(), "redis");
        assert!(plugin.is_protocol_node());
        assert_eq!(plugin.ports().len(), 1);
    }

    #[test]
    fn test_default_config_not_empty() {
        let connect_plugin = RedisConnectPlugin;
        let command_plugin = RedisCommandPlugin;
        let close_plugin = RedisClosePlugin;

        let connect_config = connect_plugin.default_config();
        assert!(connect_config.get("host").is_some());
        assert!(connect_config.get("port").is_some());

        let command_config = command_plugin.default_config();
        assert!(command_config.get("command").is_some());

        let close_config = close_plugin.default_config();
        assert!(close_config.get("connection_id").is_some());
    }

    #[test]
    fn test_plugin_group_registration() {
        let mut registry = NodeRegistry::new();
        RedisPluginGroup::register_all(&mut registry);

        assert!(registry.get("redis_connect").is_some());
        assert!(registry.get("redis_command").is_some());
        assert!(registry.get("redis_close").is_some());
    }
}
