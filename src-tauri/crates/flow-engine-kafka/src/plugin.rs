// Kafka 节点插件 — NodePlugin trait 实现
//
// 提供 7 个 Kafka 节点的元数据、端口定义、配置验证和执行能力：
// - KafkaConnectPlugin: Kafka 连接
// - KafkaProducePlugin: Kafka 生产
// - KafkaConsumePlugin: Kafka 消费
// - KafkaConsumeBatchPlugin: Kafka 批量消费
// - KafkaAdminPlugin: Kafka 管理
// - KafkaSchemaRegistryPlugin: Kafka Schema Registry
// - KafkaClosePlugin: Kafka 关闭
// 以及 KafkaPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// KafkaConnectPlugin
// ---------------------------------------------------------------------------

/// Kafka 连接节点插件
pub struct KafkaConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaConnectPlugin {
    fn type_id(&self) -> &str {
        "kafka_connect"
    }

    fn display_name(&self) -> &str {
        "Kafka 连接"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "radio"
    }

    fn color(&self) -> &str {
        "#f43f5e"
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
            "bootstrap_servers": "localhost:9092",
            "security_protocol": "plaintext",
            "connect_timeout_ms": 5000,
        })
    }

    /// `KafkaConnect` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "bootstrap_servers": config.get("bootstrap_servers"),
            "client_id": config.get("client_id"),
            "security_protocol": config.get("security_protocol"),
            "sasl_mechanism": config.get("sasl_mechanism"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("bootstrap_servers").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "bootstrap_servers".into(),
                message: "bootstrap_servers 不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor = crate::kafka_connect_executor::KafkaConnectExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaProducePlugin
// ---------------------------------------------------------------------------

/// Kafka 生产节点插件
pub struct KafkaProducePlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaProducePlugin {
    fn type_id(&self) -> &str {
        "kafka_produce"
    }

    fn display_name(&self) -> &str {
        "Kafka 生产"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "send"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "生产成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "生产失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "value": null,
            "serialization_format": "json",
            "ack_timeout_ms": 5000,
            "compression": "none",
        })
    }

    /// `KafkaProduce` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "topic": config.get("topic"),
            "serialization_format": config.get("serialization_format"),
            "compression": config.get("compression"),
            "ack_timeout_ms": config.get("ack_timeout_ms"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
            }),
        }

        match config.get("topic").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "topic".into(),
                message: "主题不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor = crate::kafka_produce_executor::KafkaProduceExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaConsumePlugin
// ---------------------------------------------------------------------------

/// Kafka 消费节点插件
pub struct KafkaConsumePlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaConsumePlugin {
    fn type_id(&self) -> &str {
        "kafka_consume"
    }

    fn display_name(&self) -> &str {
        "Kafka 消费"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "download"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "消费成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "消费失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "offset_reset": "latest",
            "enable_auto_commit": false,
            "max_messages": null,
            "poll_timeout_ms": null,
            "deserialization_format": "json",
        })
    }

    /// `KafkaConsume` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "topic": config.get("topic"),
            "offset_reset": config.get("offset_reset"),
            "max_messages": config.get("max_messages"),
            "deserialization_format": config.get("deserialization_format"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
            }),
        }

        match config.get("topic").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "topic".into(),
                message: "主题不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor = crate::kafka_consume_executor::KafkaConsumeExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaConsumeBatchPlugin
// ---------------------------------------------------------------------------

/// Kafka 批量消费节点插件
pub struct KafkaConsumeBatchPlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaConsumeBatchPlugin {
    fn type_id(&self) -> &str {
        "kafka_consume_batch"
    }

    fn display_name(&self) -> &str {
        "Kafka 批量消费"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "layers"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "批量消费成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "批量消费失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "topic": "",
            "offset_reset": "latest",
            "enable_auto_commit": false,
            "max_messages": 10,
            "poll_timeout_ms": 10000,
            "batch_strategy": "count_timeout",
        })
    }

    /// `KafkaConsumeBatch` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "topic": config.get("topic"),
            "max_messages": config.get("max_messages"),
            "poll_timeout_ms": config.get("poll_timeout_ms"),
            "batch_strategy": config.get("batch_strategy"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
            }),
        }

        match config.get("topic").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "topic".into(),
                message: "主题不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor =
            crate::kafka_consume_batch_executor::KafkaConsumeBatchExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaAdminPlugin
// ---------------------------------------------------------------------------

/// Kafka 管理操作节点插件
pub struct KafkaAdminPlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaAdminPlugin {
    fn type_id(&self) -> &str {
        "kafka_admin"
    }

    fn display_name(&self) -> &str {
        "Kafka 管理"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "settings"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "操作成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "操作失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "operation": "",
            "topic_name": null,
            "partitions": null,
            "replication_factor": null,
            "dry_run": false,
        })
    }

    /// `KafkaAdmin` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "operation": config.get("operation"),
            "topic_name": config.get("topic_name"),
            "dry_run": config.get("dry_run"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
            }),
        }

        match config.get("operation").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "operation".into(),
                message: "操作类型不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor = crate::kafka_admin_executor::KafkaAdminExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaSchemaRegistryPlugin
// ---------------------------------------------------------------------------

/// Kafka Schema Registry 节点插件
pub struct KafkaSchemaRegistryPlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaSchemaRegistryPlugin {
    fn type_id(&self) -> &str {
        "kafka_schema_registry"
    }

    fn display_name(&self) -> &str {
        "Kafka Schema Registry"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "file-text"
    }

    fn color(&self) -> &str {
        "#f43f5e"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "操作成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "操作失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "operation": "",
            "subject": null,
            "version": null,
            "schema_definition": null,
            "schema_type": null,
            "compatibility_level": null,
        })
    }

    /// `KafkaSchemaRegistry` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "operation": config.get("operation"),
            "subject": config.get("subject"),
            "schema_type": config.get("schema_type"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
            }),
        }

        match config.get("operation").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "operation".into(),
                message: "操作类型不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor =
            crate::kafka_schema_registry_executor::KafkaSchemaRegistryExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaClosePlugin
// ---------------------------------------------------------------------------

/// Kafka 关闭连接节点插件
pub struct KafkaClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for KafkaClosePlugin {
    fn type_id(&self) -> &str {
        "kafka_close"
    }

    fn display_name(&self) -> &str {
        "Kafka 关闭"
    }

    fn category(&self) -> &str {
        "kafka"
    }

    fn icon(&self) -> &str {
        "x-circle"
    }

    fn color(&self) -> &str {
        "#f43f5e"
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

    /// `KafkaClose` 始终返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn is_protocol_node(&self) -> bool {
        true
    }

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        false
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "连接 ID 不能为空".into(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let kafka_manager = context
            .get_provider::<crate::kafka_connection_manager::KafkaConnectionManager>("kafka")
            .unwrap_or_else(|| Arc::new(crate::kafka_connection_manager::KafkaConnectionManager::new()));
        let executor = crate::kafka_close_executor::KafkaCloseExecutor::new(kafka_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// KafkaPluginGroup
// ---------------------------------------------------------------------------

/// Kafka 插件组 — 批量注册辅助
pub struct KafkaPluginGroup;

impl KafkaPluginGroup {
    /// 注册全部 Kafka 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(KafkaConnectPlugin));
        registry.register(Arc::new(KafkaProducePlugin));
        registry.register(Arc::new(KafkaConsumePlugin));
        registry.register(Arc::new(KafkaConsumeBatchPlugin));
        registry.register(Arc::new(KafkaAdminPlugin));
        registry.register(Arc::new(KafkaSchemaRegistryPlugin));
        registry.register(Arc::new(KafkaClosePlugin));
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
    fn test_kafka_connect_plugin_metadata() {
        let plugin = KafkaConnectPlugin;
        assert_eq!(plugin.type_id(), "kafka_connect");
        assert_eq!(plugin.display_name(), "Kafka 连接");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "radio");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_produce_plugin_metadata() {
        let plugin = KafkaProducePlugin;
        assert_eq!(plugin.type_id(), "kafka_produce");
        assert_eq!(plugin.display_name(), "Kafka 生产");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "send");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_consume_plugin_metadata() {
        let plugin = KafkaConsumePlugin;
        assert_eq!(plugin.type_id(), "kafka_consume");
        assert_eq!(plugin.display_name(), "Kafka 消费");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "download");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_consume_batch_plugin_metadata() {
        let plugin = KafkaConsumeBatchPlugin;
        assert_eq!(plugin.type_id(), "kafka_consume_batch");
        assert_eq!(plugin.display_name(), "Kafka 批量消费");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "layers");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_admin_plugin_metadata() {
        let plugin = KafkaAdminPlugin;
        assert_eq!(plugin.type_id(), "kafka_admin");
        assert_eq!(plugin.display_name(), "Kafka 管理");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "settings");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_schema_registry_plugin_metadata() {
        let plugin = KafkaSchemaRegistryPlugin;
        assert_eq!(plugin.type_id(), "kafka_schema_registry");
        assert_eq!(plugin.display_name(), "Kafka Schema Registry");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "file-text");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_kafka_close_plugin_metadata() {
        let plugin = KafkaClosePlugin;
        assert_eq!(plugin.type_id(), "kafka_close");
        assert_eq!(plugin.display_name(), "Kafka 关闭");
        assert_eq!(plugin.category(), "kafka");
        assert_eq!(plugin.icon(), "x-circle");
        assert_eq!(plugin.color(), "#f43f5e");
        assert!(plugin.is_protocol_node());
        assert!(!plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_kafka_connect_resolve_port() {
        let plugin = KafkaConnectPlugin;
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
    fn test_kafka_produce_resolve_port() {
        let plugin = KafkaProducePlugin;
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
    fn test_kafka_consume_resolve_port() {
        let plugin = KafkaConsumePlugin;
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
    fn test_kafka_close_resolve_port() {
        let plugin = KafkaClosePlugin;
        // KafkaClose 始终返回 "out"
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
    fn test_resolve_port_edge_cases() {
        let plugin = KafkaConnectPlugin;

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
        let connect_plugin = KafkaConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("bootstrap_servers").is_some());
        assert!(config.get("connect_timeout_ms").is_some());

        let produce_plugin = KafkaProducePlugin;
        let config = produce_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("topic").is_some());

        let consume_plugin = KafkaConsumePlugin;
        let config = consume_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("topic").is_some());

        let consume_batch_plugin = KafkaConsumeBatchPlugin;
        let config = consume_batch_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("topic").is_some());
        assert!(config.get("max_messages").is_some());
        assert!(config.get("batch_strategy").is_some());

        let admin_plugin = KafkaAdminPlugin;
        let config = admin_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("operation").is_some());

        let schema_plugin = KafkaSchemaRegistryPlugin;
        let config = schema_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("operation").is_some());

        let close_plugin = KafkaClosePlugin;
        let config = close_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = KafkaConnectPlugin;
        let config = serde_json::json!({
            "bootstrap_servers": "broker1:9092,broker2:9092",
            "client_id": "test-client",
            "security_protocol": "sasl_ssl",
            "sasl_mechanism": "PLAIN",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["bootstrap_servers"], "broker1:9092,broker2:9092");
        assert_eq!(report["sasl_mechanism"], "PLAIN");
    }

    #[test]
    fn test_produce_build_report_input() {
        let plugin = KafkaProducePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "topic": "test-topic",
            "compression": "gzip",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["topic"], "test-topic");
        assert_eq!(report["compression"], "gzip");
    }

    #[test]
    fn test_consume_build_report_input() {
        let plugin = KafkaConsumePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "topic": "test-topic",
            "offset_reset": "earliest",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["topic"], "test-topic");
        assert_eq!(report["offset_reset"], "earliest");
    }

    #[test]
    fn test_close_build_report_input() {
        let plugin = KafkaClosePlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    #[test]
    fn test_admin_build_report_input() {
        let plugin = KafkaAdminPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "operation": "topic_list",
            "dry_run": false,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["operation"], "topic_list");
    }

    #[test]
    fn test_schema_registry_build_report_input() {
        let plugin = KafkaSchemaRegistryPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "operation": "list_subjects",
            "subject": "test-value",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["operation"], "list_subjects");
    }

    // ---- KafkaPluginGroup.register_all 测试 ----

    #[test]
    fn test_kafka_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        KafkaPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 7);

        assert!(registry.get("kafka_connect").is_some());
        assert!(registry.get("kafka_produce").is_some());
        assert!(registry.get("kafka_consume").is_some());
        assert!(registry.get("kafka_consume_batch").is_some());
        assert!(registry.get("kafka_admin").is_some());
        assert!(registry.get("kafka_schema_registry").is_some());
        assert!(registry.get("kafka_close").is_some());
    }
}
