// AMQP 0-9-1 节点插件 — NodePlugin trait 实现
//
// 提供 11 个 AMQP 0-9-1 节点的元数据、端口定义、配置验证和执行能力：
// - AmqpConnectPlugin: AMQP 连接
// - AmqpClosePlugin: AMQP 关闭连接
// - AmqpExchangeDeclarePlugin: 声明交换器
// - AmqpExchangeDeletePlugin: 删除交换器
// - AmqpQueueDeclarePlugin: 声明队列
// - AmqpQueueDeletePlugin: 删除队列
// - AmqpQueueBindPlugin: 队列绑定
// - AmqpPublishPlugin: 发布消息
// - AmqpConsumePlugin: 消费消息
// - AmqpGetPlugin: 获取消息
// - AmqpAckPlugin: 确认消息
// 以及 6 个 AMQP 1.0 节点：
// - Amqp10ConnectPlugin: AMQP 1.0 连接
// - Amqp10SendPlugin: AMQP 1.0 发送消息
// - Amqp10ReceivePlugin: AMQP 1.0 接收消息
// - Amqp10AcceptPlugin: AMQP 1.0 确认消息
// - Amqp10RejectPlugin: AMQP 1.0 拒绝消息
// - Amqp10DisconnectPlugin: AMQP 1.0 断开连接
// 以及 AmqpPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// AmqpConnectPlugin
// ---------------------------------------------------------------------------

/// AMQP 连接节点插件
pub struct AmqpConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpConnectPlugin {
    fn type_id(&self) -> &str {
        "amqp_connect"
    }

    fn display_name(&self) -> &str {
        "AMQP 连接"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "radio-tower"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "url": "amqp://guest:guest@localhost:5672/%2f",
            "connection_timeout_ms": 30000,
            "tls_mode": "disabled",
        })
    }

    /// `AmqpConnect` 检查 $.success → "true" / "false"
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
            "url": config.get("url"),
            "connection_id": config.get("connection_id"),
            "tls_mode": config.get("tls_mode"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("url").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "url".into(),
                message: "url must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_connect_executor::AmqpConnectExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpClosePlugin
// ---------------------------------------------------------------------------

/// AMQP 关闭连接节点插件
pub struct AmqpClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpClosePlugin {
    fn type_id(&self) -> &str {
        "amqp_close"
    }

    fn display_name(&self) -> &str {
        "AMQP 关闭连接"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "unplug"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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

    /// `AmqpClose` 始终返回 "out" 端口
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
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_close_executor::AmqpCloseExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpExchangeDeclarePlugin
// ---------------------------------------------------------------------------

/// AMQP 声明交换器节点插件
pub struct AmqpExchangeDeclarePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpExchangeDeclarePlugin {
    fn type_id(&self) -> &str {
        "amqp_exchange_declare"
    }

    fn display_name(&self) -> &str {
        "AMQP 声明交换器"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "shuffle"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "exchange_name": "",
            "exchange_type": "direct",
            "durable": true,
            "auto_delete": false,
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "exchange_name": config.get("exchange_name"),
            "exchange_type": config.get("exchange_type"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("exchange_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "exchange_name".into(),
                message: "exchange_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_exchange_declare_executor::AmqpExchangeDeclareExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpExchangeDeletePlugin
// ---------------------------------------------------------------------------

/// AMQP 删除交换器节点插件
pub struct AmqpExchangeDeletePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpExchangeDeletePlugin {
    fn type_id(&self) -> &str {
        "amqp_exchange_delete"
    }

    fn display_name(&self) -> &str {
        "AMQP 删除交换器"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "trash-2"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "exchange_name": "",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "exchange_name": config.get("exchange_name"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("exchange_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "exchange_name".into(),
                message: "exchange_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_exchange_delete_executor::AmqpExchangeDeleteExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpQueueDeclarePlugin
// ---------------------------------------------------------------------------

/// AMQP 声明队列节点插件
pub struct AmqpQueueDeclarePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpQueueDeclarePlugin {
    fn type_id(&self) -> &str {
        "amqp_queue_declare"
    }

    fn display_name(&self) -> &str {
        "AMQP 声明队列"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "list"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "queue_name": "",
            "durable": true,
            "exclusive": false,
            "auto_delete": false,
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "queue_name": config.get("queue_name"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_queue_declare_executor::AmqpQueueDeclareExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpQueueDeletePlugin
// ---------------------------------------------------------------------------

/// AMQP 删除队列节点插件
pub struct AmqpQueueDeletePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpQueueDeletePlugin {
    fn type_id(&self) -> &str {
        "amqp_queue_delete"
    }

    fn display_name(&self) -> &str {
        "AMQP 删除队列"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "trash"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "queue_name": "",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "queue_name": config.get("queue_name"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("queue_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "queue_name".into(),
                message: "queue_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_queue_delete_executor::AmqpQueueDeleteExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpQueueBindPlugin
// ---------------------------------------------------------------------------

/// AMQP 队列绑定节点插件
pub struct AmqpQueueBindPlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpQueueBindPlugin {
    fn type_id(&self) -> &str {
        "amqp_queue_bind"
    }

    fn display_name(&self) -> &str {
        "AMQP 队列绑定"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "link"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "queue_name": "",
            "exchange_name": "",
            "routing_key": "",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "queue_name": config.get("queue_name"),
            "exchange_name": config.get("exchange_name"),
            "routing_key": config.get("routing_key"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("queue_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "queue_name".into(),
                message: "queue_name must not be empty".into(),
            });
        }
        if config.get("exchange_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "exchange_name".into(),
                message: "exchange_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_queue_bind_executor::AmqpQueueBindExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpPublishPlugin
// ---------------------------------------------------------------------------

/// AMQP 发布消息节点插件
pub struct AmqpPublishPlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpPublishPlugin {
    fn type_id(&self) -> &str {
        "amqp_publish"
    }

    fn display_name(&self) -> &str {
        "AMQP 发布消息"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "send"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "exchange": "",
            "routing_key": "",
            "body": "",
            "body_type": "text",
            "confirm_mode": "none",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "exchange": config.get("exchange"),
            "routing_key": config.get("routing_key"),
            "body": config.get("body"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_publish_executor::AmqpPublishExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpConsumePlugin
// ---------------------------------------------------------------------------

/// AMQP 消费消息节点插件
pub struct AmqpConsumePlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpConsumePlugin {
    fn type_id(&self) -> &str {
        "amqp_consume"
    }

    fn display_name(&self) -> &str {
        "AMQP 消费消息"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "inbox"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "queue_name": "",
            "consumer_tag": "",
            "max_messages": 1,
            "timeout_ms": 5000,
            "auto_ack": false,
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "queue_name": config.get("queue_name"),
            "max_messages": config.get("max_messages"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("queue_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "queue_name".into(),
                message: "queue_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_consume_executor::AmqpConsumeExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpGetPlugin
// ---------------------------------------------------------------------------

/// AMQP 获取消息节点插件
pub struct AmqpGetPlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpGetPlugin {
    fn type_id(&self) -> &str {
        "amqp_get"
    }

    fn display_name(&self) -> &str {
        "AMQP 获取消息"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "download"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "queue_name": "",
            "auto_ack": false,
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "queue_name": config.get("queue_name"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("queue_name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "queue_name".into(),
                message: "queue_name must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_get_executor::AmqpGetExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpAckPlugin
// ---------------------------------------------------------------------------

/// AMQP 确认消息节点插件
pub struct AmqpAckPlugin;

#[async_trait::async_trait]
impl NodePlugin for AmqpAckPlugin {
    fn type_id(&self) -> &str {
        "amqp_ack"
    }

    fn display_name(&self) -> &str {
        "AMQP 确认消息"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "check-circle"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "delivery_tag": 0,
            "ack_mode": "ack",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "delivery_tag": config.get("delivery_tag"),
            "ack_mode": config.get("ack_mode"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_ack_executor::AmqpAckExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AmqpManagementPlugin（可选 feature: management）
// ---------------------------------------------------------------------------

/// AMQP Management 节点插件（需要 `management` feature）
#[cfg(feature = "management")]
pub struct AmqpManagementPlugin;

#[cfg(feature = "management")]
#[async_trait::async_trait]
impl NodePlugin for AmqpManagementPlugin {
    fn type_id(&self) -> &str {
        "amqp_management"
    }

    fn display_name(&self) -> &str {
        "AMQP 管理"
    }

    fn category(&self) -> &str {
        "amqp"
    }

    fn icon(&self) -> &str {
        "settings"
    }

    fn color(&self) -> &str {
        "#7c3aed"
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
            "management_url": "http://localhost:15672",
            "username": "guest",
            "password": "guest",
            "action": "overview",
            "vhost": "/",
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
        true
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "management_url": config.get("management_url"),
            "action": config.get("action"),
            "vhost": config.get("vhost"),
            "queue_name": config.get("queue_name"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("management_url").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "management_url".into(),
                message: "management_url must not be empty".into(),
            });
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
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp091::amqp_management_executor::AmqpManagementExecutor::new(amqp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// AMQP 1.0 插件
// ---------------------------------------------------------------------------

/// AMQP 1.0 连接节点插件
pub struct Amqp10ConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10ConnectPlugin {
    fn type_id(&self) -> &str { "amqp10_connect" }
    fn display_name(&self) -> &str { "AMQP 1.0 连接" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "radio-tower" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef { id: "true".into(), label: "连接成功".into(), position: "right".into() },
            PortDef { id: "false".into(), label: "连接失败".into(), position: "bottom".into() },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "url": "amqp://localhost:5672",
            "tls_mode": "disabled",
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") { "true".to_string() } else { "false".to_string() }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "url": config.get("url"),
            "connection_id": config.get("connection_id"),
            "tls_mode": config.get("tls_mode"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("url").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "url".into(),
                message: "url must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_connect_executor::Amqp10ConnectExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

/// AMQP 1.0 发送消息节点插件
pub struct Amqp10SendPlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10SendPlugin {
    fn type_id(&self) -> &str { "amqp10_send" }
    fn display_name(&self) -> &str { "AMQP 1.0 发送消息" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "send" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef { id: "true".into(), label: "成功".into(), position: "right".into() },
            PortDef { id: "false".into(), label: "失败".into(), position: "bottom".into() },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "address": "",
            "message_body": "",
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") { "true".to_string() } else { "false".to_string() }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "address": config.get("address"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("address").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "address".into(),
                message: "address must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_send_executor::Amqp10SendExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

/// AMQP 1.0 接收消息节点插件
pub struct Amqp10ReceivePlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10ReceivePlugin {
    fn type_id(&self) -> &str { "amqp10_receive" }
    fn display_name(&self) -> &str { "AMQP 1.0 接收消息" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "inbox" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef { id: "true".into(), label: "成功".into(), position: "right".into() },
            PortDef { id: "false".into(), label: "失败".into(), position: "bottom".into() },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "address": "",
            "max_messages": 1,
            "timeout_ms": 5000,
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") { "true".to_string() } else { "false".to_string() }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "address": config.get("address"),
            "max_messages": config.get("max_messages"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        if config.get("address").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "address".into(),
                message: "address must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_receive_executor::Amqp10ReceiveExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

/// AMQP 1.0 确认消息节点插件
pub struct Amqp10AcceptPlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10AcceptPlugin {
    fn type_id(&self) -> &str { "amqp10_accept" }
    fn display_name(&self) -> &str { "AMQP 1.0 确认消息" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "check-circle" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef { id: "true".into(), label: "成功".into(), position: "right".into() },
            PortDef { id: "false".into(), label: "失败".into(), position: "bottom".into() },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "delivery_id": 0,
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") { "true".to_string() } else { "false".to_string() }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "delivery_id": config.get("delivery_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_accept_executor::Amqp10AcceptExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

/// AMQP 1.0 拒绝消息节点插件
pub struct Amqp10RejectPlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10RejectPlugin {
    fn type_id(&self) -> &str { "amqp10_reject" }
    fn display_name(&self) -> &str { "AMQP 1.0 拒绝消息" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "x-circle" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef { id: "true".into(), label: "成功".into(), position: "right".into() },
            PortDef { id: "false".into(), label: "失败".into(), position: "bottom".into() },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "delivery_id": 0,
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") { "true".to_string() } else { "false".to_string() }
            }
            _ => "false".to_string(),
        }
    }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
            "delivery_id": config.get("delivery_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_reject_executor::Amqp10RejectExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

/// AMQP 1.0 断开连接节点插件
pub struct Amqp10DisconnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for Amqp10DisconnectPlugin {
    fn type_id(&self) -> &str { "amqp10_disconnect" }
    fn display_name(&self) -> &str { "AMQP 1.0 断开连接" }
    fn category(&self) -> &str { "amqp" }
    fn icon(&self) -> &str { "unplug" }
    fn color(&self) -> &str { "#7c3aed" }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef { id: "out".into(), label: "完成".into(), position: "right".into() }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
        })
    }

    /// `Amqp10Disconnect` 始终返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String { "out".to_string() }

    fn is_protocol_node(&self) -> bool { true }
    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool { false }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_id": config.get("connection_id"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if config.get("connection_id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
            errors.push(ValidationError {
                field: "connection_id".into(),
                message: "connection_id must not be empty".into(),
            });
        }
        errors
    }

    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection], node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let amqp_manager = context
            .get_provider::<crate::amqp091::amqp_connection_manager::AmqpConnectionManager>("amqp")
            .unwrap_or_else(|| Arc::new(crate::amqp091::amqp_connection_manager::AmqpConnectionManager::new()));
        let executor = crate::amqp10::amqp10_disconnect_executor::Amqp10DisconnectExecutor::new(amqp_manager);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// AmqpPluginGroup
// ---------------------------------------------------------------------------

/// AMQP 插件组 — 批量注册辅助
pub struct AmqpPluginGroup;

impl AmqpPluginGroup {
    /// 注册全部 AMQP 节点插件到注册表（11 个 AMQP 0-9-1 + 6 个 AMQP 1.0）
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        // AMQP 0-9-1 (11 个)
        registry.register(Arc::new(AmqpConnectPlugin));
        registry.register(Arc::new(AmqpClosePlugin));
        registry.register(Arc::new(AmqpExchangeDeclarePlugin));
        registry.register(Arc::new(AmqpExchangeDeletePlugin));
        registry.register(Arc::new(AmqpQueueDeclarePlugin));
        registry.register(Arc::new(AmqpQueueDeletePlugin));
        registry.register(Arc::new(AmqpQueueBindPlugin));
        registry.register(Arc::new(AmqpPublishPlugin));
        registry.register(Arc::new(AmqpConsumePlugin));
        registry.register(Arc::new(AmqpGetPlugin));
        registry.register(Arc::new(AmqpAckPlugin));
        // AMQP 1.0 (6 个)
        registry.register(Arc::new(Amqp10ConnectPlugin));
        registry.register(Arc::new(Amqp10SendPlugin));
        registry.register(Arc::new(Amqp10ReceivePlugin));
        registry.register(Arc::new(Amqp10AcceptPlugin));
        registry.register(Arc::new(Amqp10RejectPlugin));
        registry.register(Arc::new(Amqp10DisconnectPlugin));
        #[cfg(feature = "management")]
        registry.register(Arc::new(AmqpManagementPlugin));
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
    fn test_amqp_connect_plugin_metadata() {
        let plugin = AmqpConnectPlugin;
        assert_eq!(plugin.type_id(), "amqp_connect");
        assert_eq!(plugin.display_name(), "AMQP 连接");
        assert_eq!(plugin.category(), "amqp");
        assert_eq!(plugin.icon(), "radio-tower");
        assert_eq!(plugin.color(), "#7c3aed");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_amqp_close_plugin_metadata() {
        let plugin = AmqpClosePlugin;
        assert_eq!(plugin.type_id(), "amqp_close");
        assert_eq!(plugin.display_name(), "AMQP 关闭连接");
        assert_eq!(plugin.category(), "amqp");
        assert_eq!(plugin.icon(), "unplug");
        assert_eq!(plugin.color(), "#7c3aed");
        assert!(plugin.is_protocol_node());
        assert!(!plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    #[test]
    fn test_amqp_exchange_declare_plugin_metadata() {
        let plugin = AmqpExchangeDeclarePlugin;
        assert_eq!(plugin.type_id(), "amqp_exchange_declare");
        assert_eq!(plugin.display_name(), "AMQP 声明交换器");
        assert_eq!(plugin.category(), "amqp");
        assert_eq!(plugin.icon(), "shuffle");
        assert_eq!(plugin.color(), "#7c3aed");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_amqp_exchange_delete_plugin_metadata() {
        let plugin = AmqpExchangeDeletePlugin;
        assert_eq!(plugin.type_id(), "amqp_exchange_delete");
        assert_eq!(plugin.display_name(), "AMQP 删除交换器");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_queue_declare_plugin_metadata() {
        let plugin = AmqpQueueDeclarePlugin;
        assert_eq!(plugin.type_id(), "amqp_queue_declare");
        assert_eq!(plugin.display_name(), "AMQP 声明队列");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_queue_delete_plugin_metadata() {
        let plugin = AmqpQueueDeletePlugin;
        assert_eq!(plugin.type_id(), "amqp_queue_delete");
        assert_eq!(plugin.display_name(), "AMQP 删除队列");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_queue_bind_plugin_metadata() {
        let plugin = AmqpQueueBindPlugin;
        assert_eq!(plugin.type_id(), "amqp_queue_bind");
        assert_eq!(plugin.display_name(), "AMQP 队列绑定");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_publish_plugin_metadata() {
        let plugin = AmqpPublishPlugin;
        assert_eq!(plugin.type_id(), "amqp_publish");
        assert_eq!(plugin.display_name(), "AMQP 发布消息");
        assert_eq!(plugin.category(), "amqp");
        assert_eq!(plugin.icon(), "send");
        assert_eq!(plugin.color(), "#7c3aed");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_amqp_consume_plugin_metadata() {
        let plugin = AmqpConsumePlugin;
        assert_eq!(plugin.type_id(), "amqp_consume");
        assert_eq!(plugin.display_name(), "AMQP 消费消息");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_get_plugin_metadata() {
        let plugin = AmqpGetPlugin;
        assert_eq!(plugin.type_id(), "amqp_get");
        assert_eq!(plugin.display_name(), "AMQP 获取消息");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    #[test]
    fn test_amqp_ack_plugin_metadata() {
        let plugin = AmqpAckPlugin;
        assert_eq!(plugin.type_id(), "amqp_ack");
        assert_eq!(plugin.display_name(), "AMQP 确认消息");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_amqp_connect_resolve_port() {
        let plugin = AmqpConnectPlugin;
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
    fn test_amqp_close_resolve_port() {
        let plugin = AmqpClosePlugin;
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
    fn test_amqp_publish_resolve_port() {
        let plugin = AmqpPublishPlugin;
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
    fn test_amqp_consume_resolve_port() {
        let plugin = AmqpConsumePlugin;
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
    fn test_amqp_ack_resolve_port() {
        let plugin = AmqpAckPlugin;
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

    // ---- default_config 测试 ----

    #[test]
    fn test_all_default_configs_not_empty() {
        let plugins: Vec<Box<dyn NodePlugin>> = vec![
            Box::new(AmqpConnectPlugin),
            Box::new(AmqpClosePlugin),
            Box::new(AmqpExchangeDeclarePlugin),
            Box::new(AmqpExchangeDeletePlugin),
            Box::new(AmqpQueueDeclarePlugin),
            Box::new(AmqpQueueDeletePlugin),
            Box::new(AmqpQueueBindPlugin),
            Box::new(AmqpPublishPlugin),
            Box::new(AmqpConsumePlugin),
            Box::new(AmqpGetPlugin),
            Box::new(AmqpAckPlugin),
            Box::new(Amqp10ConnectPlugin),
            Box::new(Amqp10SendPlugin),
            Box::new(Amqp10ReceivePlugin),
            Box::new(Amqp10AcceptPlugin),
            Box::new(Amqp10RejectPlugin),
            Box::new(Amqp10DisconnectPlugin),
        ];

        for plugin in &plugins {
            let config = plugin.default_config();
            assert!(config.is_object(), "{} default_config 应为对象", plugin.type_id());
        }
    }

    #[test]
    fn test_connect_default_config_fields() {
        let plugin = AmqpConnectPlugin;
        let config = plugin.default_config();
        assert!(config.get("url").is_some());
        assert!(config.get("connection_timeout_ms").is_some());
        assert!(config.get("tls_mode").is_some());
    }

    #[test]
    fn test_publish_default_config_fields() {
        let plugin = AmqpPublishPlugin;
        let config = plugin.default_config();
        assert!(config.get("connection_id").is_some());
        assert!(config.get("exchange").is_some());
        assert!(config.get("routing_key").is_some());
        assert!(config.get("body").is_some());
        assert!(config.get("confirm_mode").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = AmqpConnectPlugin;
        let config = serde_json::json!({
            "url": "amqp://localhost:5672",
            "connection_id": "conn-1",
            "tls_mode": "disabled",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["url"], "amqp://localhost:5672");
        assert_eq!(report["connection_id"], "conn-1");
    }

    #[test]
    fn test_publish_build_report_input() {
        let plugin = AmqpPublishPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "exchange": "test-exchange",
            "routing_key": "test.key",
            "body": "hello",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["exchange"], "test-exchange");
        assert_eq!(report["routing_key"], "test.key");
    }

    #[test]
    fn test_consume_build_report_input() {
        let plugin = AmqpConsumePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "queue_name": "test-queue",
            "max_messages": 5,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["queue_name"], "test-queue");
        assert_eq!(report["max_messages"], 5);
    }

    #[test]
    fn test_close_build_report_input() {
        let plugin = AmqpClosePlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    // ---- AmqpPluginGroup.register_all 测试 ----

    #[test]
    fn test_amqp_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        AmqpPluginGroup::register_all(&mut registry);
        #[cfg(feature = "management")]
        assert_eq!(registry.len(), 18);
        #[cfg(not(feature = "management"))]
        assert_eq!(registry.len(), 17);

        assert!(registry.get("amqp_connect").is_some());
        assert!(registry.get("amqp_close").is_some());
        assert!(registry.get("amqp_exchange_declare").is_some());
        assert!(registry.get("amqp_exchange_delete").is_some());
        assert!(registry.get("amqp_queue_declare").is_some());
        assert!(registry.get("amqp_queue_delete").is_some());
        assert!(registry.get("amqp_queue_bind").is_some());
        assert!(registry.get("amqp_publish").is_some());
        assert!(registry.get("amqp_consume").is_some());
        assert!(registry.get("amqp_get").is_some());
        assert!(registry.get("amqp_ack").is_some());
        // AMQP 1.0
        assert!(registry.get("amqp10_connect").is_some());
        assert!(registry.get("amqp10_send").is_some());
        assert!(registry.get("amqp10_receive").is_some());
        assert!(registry.get("amqp10_accept").is_some());
        assert!(registry.get("amqp10_reject").is_some());
        assert!(registry.get("amqp10_disconnect").is_some());
        #[cfg(feature = "management")]
        assert!(registry.get("amqp_management").is_some());
    }

    #[test]
    fn test_amqp_plugin_group_all_metadata_complete() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        AmqpPluginGroup::register_all(&mut registry);

        let metadata = registry.all_metadata();
        #[cfg(feature = "management")]
        assert_eq!(metadata.len(), 18);
        #[cfg(not(feature = "management"))]
        assert_eq!(metadata.len(), 17);

        for meta in &metadata {
            assert!(!meta.type_id.is_empty(), "type_id 不能为空");
            assert!(!meta.display_name.is_empty(), "display_name 不能为空");
            assert!(!meta.category.is_empty(), "category 不能为空");
            assert_eq!(meta.category, "amqp", "category 应为 amqp");
            assert!(!meta.icon.is_empty(), "icon 不能为空");
            assert!(!meta.color.is_empty(), "color 不能为空");
            assert_eq!(meta.color, "#7c3aed", "color 应为 #7c3aed");
            assert!(!meta.ports.is_empty(), "ports 不能为空");
        }
    }

    // ---- Management 插件测试（仅在 management feature 下） ----

    #[cfg(feature = "management")]
    #[test]
    fn test_amqp_management_plugin_metadata() {
        let plugin = AmqpManagementPlugin;
        assert_eq!(plugin.type_id(), "amqp_management");
        assert_eq!(plugin.display_name(), "AMQP 管理");
        assert_eq!(plugin.category(), "amqp");
        assert_eq!(plugin.icon(), "settings");
        assert_eq!(plugin.color(), "#7c3aed");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[cfg(feature = "management")]
    #[test]
    fn test_amqp_management_resolve_port() {
        let plugin = AmqpManagementPlugin;
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

    #[cfg(feature = "management")]
    #[test]
    fn test_amqp_management_default_config() {
        let plugin = AmqpManagementPlugin;
        let config = plugin.default_config();
        assert!(config.get("management_url").is_some());
        assert!(config.get("username").is_some());
        assert!(config.get("password").is_some());
        assert!(config.get("action").is_some());
        assert!(config.get("vhost").is_some());
    }

    #[cfg(feature = "management")]
    #[test]
    fn test_amqp_management_build_report_input() {
        let plugin = AmqpManagementPlugin;
        let config = serde_json::json!({
            "management_url": "http://localhost:15672",
            "action": "list_queues",
            "vhost": "/",
            "queue_name": "test-queue",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["management_url"], "http://localhost:15672");
        assert_eq!(report["action"], "list_queues");
        assert_eq!(report["vhost"], "/");
        assert_eq!(report["queue_name"], "test-queue");
    }
}
