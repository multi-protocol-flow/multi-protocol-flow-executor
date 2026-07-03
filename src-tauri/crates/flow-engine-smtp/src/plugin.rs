// SMTP 节点插件 — NodePlugin trait 实现
//
// 提供 3 个 SMTP 节点的元数据、端口定义、配置验证和执行能力：
// - SmtpConnectPlugin: SMTP 连接
// - SmtpSendPlugin: SMTP 发送邮件
// - SmtpDisconnectPlugin: SMTP 断开连接
// 以及 SmtpPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// SmtpConnectPlugin
// ---------------------------------------------------------------------------

/// SMTP 连接节点插件
pub struct SmtpConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for SmtpConnectPlugin {
    fn type_id(&self) -> &str {
        "smtp_connect"
    }

    fn display_name(&self) -> &str {
        "SMTP 连接"
    }

    fn category(&self) -> &str {
        "email"
    }

    fn icon(&self) -> &str {
        "mail"
    }

    fn color(&self) -> &str {
        "#ea580c"
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
            "host": "smtp.gmail.com",
            "port": 465,
            "tls_mode": "implicit",
        })
    }

    /// `SmtpConnect` 检查 $.success → "true" / "false"
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
            "host": config.get("host"),
            "port": config.get("port"),
            "tls_mode": config.get("tls_mode"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if !config.get("host").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
            errors.push(ValidationError {
                field: "host".to_string(),
                message: "host is required".to_string(),
            });
        }
        if !config.get("port").and_then(|v| v.as_u64()).map(|p| p > 0).unwrap_or(false) {
            errors.push(ValidationError {
                field: "port".to_string(),
                message: "port must be greater than 0".to_string(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let smtp_manager = context
            .get_provider::<crate::smtp_connection_manager::SmtpConnectionManager>("smtp")
            .unwrap_or_else(|| Arc::new(crate::smtp_connection_manager::SmtpConnectionManager::new()));
        let executor = crate::smtp_connect_executor::SmtpConnectExecutor::new(smtp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// SmtpSendPlugin
// ---------------------------------------------------------------------------

/// SMTP 发送邮件节点插件
pub struct SmtpSendPlugin;

#[async_trait::async_trait]
impl NodePlugin for SmtpSendPlugin {
    fn type_id(&self) -> &str {
        "smtp_send"
    }

    fn display_name(&self) -> &str {
        "SMTP 发送"
    }

    fn category(&self) -> &str {
        "email"
    }

    fn icon(&self) -> &str {
        "mail"
    }

    fn color(&self) -> &str {
        "#ea580c"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "发送Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "发送Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "from": null,
            "to": [],
            "cc": null,
            "bcc": null,
            "subject": null,
            "body_type": "plain_text",
            "body": null,
            "html_body": null,
            "attachments": null,
        })
    }

    /// `SmtpSend` 检查 $.success → "true" / "false"
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
            "from": config.get("from"),
            "to": config.get("to"),
            "subject": config.get("subject"),
            "body_type": config.get("body_type"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if !config.get("connection_id").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
            errors.push(ValidationError {
                field: "connection_id".to_string(),
                message: "connection_id is required".to_string(),
            });
        }
        if !config.get("from").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
            errors.push(ValidationError {
                field: "from".to_string(),
                message: "from is required".to_string(),
            });
        }
        if !config.get("to").and_then(|v| v.as_array()).map(|arr| !arr.is_empty()).unwrap_or(false) {
            errors.push(ValidationError {
                field: "to".to_string(),
                message: "to must be a non-empty array".to_string(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let smtp_manager = context
            .get_provider::<crate::smtp_connection_manager::SmtpConnectionManager>("smtp")
            .unwrap_or_else(|| Arc::new(crate::smtp_connection_manager::SmtpConnectionManager::new()));
        let executor = crate::smtp_send_executor::SmtpSendExecutor::new(smtp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// SmtpDisconnectPlugin
// ---------------------------------------------------------------------------

/// SMTP 断开连接节点插件
pub struct SmtpDisconnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for SmtpDisconnectPlugin {
    fn type_id(&self) -> &str {
        "smtp_disconnect"
    }

    fn display_name(&self) -> &str {
        "SMTP 断开"
    }

    fn category(&self) -> &str {
        "email"
    }

    fn icon(&self) -> &str {
        "mail"
    }

    fn color(&self) -> &str {
        "#ea580c"
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

    /// `SmtpDisconnect` 始终返回 "out" 端口（断开Failed不阻塞流程）
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
        if !config.get("connection_id").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
            errors.push(ValidationError {
                field: "connection_id".to_string(),
                message: "connection_id is required".to_string(),
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let smtp_manager = context
            .get_provider::<crate::smtp_connection_manager::SmtpConnectionManager>("smtp")
            .unwrap_or_else(|| Arc::new(crate::smtp_connection_manager::SmtpConnectionManager::new()));
        let executor = crate::smtp_disconnect_executor::SmtpDisconnectExecutor::new(smtp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// SmtpPluginGroup
// ---------------------------------------------------------------------------

/// SMTP 插件组 — 批量注册辅助
pub struct SmtpPluginGroup;

impl SmtpPluginGroup {
    /// 将所有 SMTP 插件注册到 `NodeRegistry`
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(SmtpConnectPlugin));
        registry.register(Arc::new(SmtpSendPlugin));
        registry.register(Arc::new(SmtpDisconnectPlugin));
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
    fn test_smtp_connect_plugin_metadata() {
        let plugin = SmtpConnectPlugin;
        assert_eq!(plugin.type_id(), "smtp_connect");
        assert_eq!(plugin.display_name(), "SMTP 连接");
        assert_eq!(plugin.category(), "email");
        assert_eq!(plugin.icon(), "mail");
        assert_eq!(plugin.color(), "#ea580c");
        assert!(plugin.is_protocol_node());
        assert!(plugin.ports().iter().any(|p| p.id == "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "Connect Success");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "connection failed");
    }

    #[test]
    fn test_smtp_send_plugin_metadata() {
        let plugin = SmtpSendPlugin;
        assert_eq!(plugin.type_id(), "smtp_send");
        assert_eq!(plugin.display_name(), "SMTP 发送");
        assert_eq!(plugin.category(), "email");
        assert_eq!(plugin.icon(), "mail");
        assert_eq!(plugin.color(), "#ea580c");
        assert!(plugin.is_protocol_node());
        assert!(plugin.ports().iter().any(|p| p.id == "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "发送Success");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "发送Failed");
    }

    #[test]
    fn test_smtp_disconnect_plugin_metadata() {
        let plugin = SmtpDisconnectPlugin;
        assert_eq!(plugin.type_id(), "smtp_disconnect");
        assert_eq!(plugin.display_name(), "SMTP 断开");
        assert_eq!(plugin.category(), "email");
        assert_eq!(plugin.icon(), "mail");
        assert_eq!(plugin.color(), "#ea580c");
        assert!(plugin.is_protocol_node());
        assert!(plugin.ports().iter().all(|p| p.id != "false"));

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
        assert_eq!(ports[0].label, "输出");
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_smtp_connect_resolve_port() {
        let plugin = SmtpConnectPlugin;
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
    fn test_smtp_send_resolve_port() {
        let plugin = SmtpSendPlugin;
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
    fn test_smtp_disconnect_resolve_port() {
        let plugin = SmtpDisconnectPlugin;
        // SmtpDisconnect 始终返回 "out"
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
        let plugin = SmtpConnectPlugin;

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
        let connect_plugin = SmtpConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("host").is_some());
        assert!(config.get("port").is_some());
        assert!(config.get("tls_mode").is_some());

        let send_plugin = SmtpSendPlugin;
        let config = send_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("from").is_some());
        assert!(config.get("to").is_some());

        let disconnect_plugin = SmtpDisconnectPlugin;
        let config = disconnect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = SmtpConnectPlugin;
        let config = serde_json::json!({
            "host": "smtp.gmail.com",
            "port": 465,
            "tls_mode": "implicit",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["host"], "smtp.gmail.com");
        assert_eq!(report["port"], 465);
        assert_eq!(report["tls_mode"], "implicit");
    }

    #[test]
    fn test_send_build_report_input() {
        let plugin = SmtpSendPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "from": "sender@test.com",
            "to": ["receiver@test.com"],
            "subject": "Test",
            "body_type": "plain_text",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["from"], "sender@test.com");
        assert_eq!(report["subject"], "Test");
    }

    #[test]
    fn test_disconnect_build_report_input() {
        let plugin = SmtpDisconnectPlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    // ---- SmtpPluginGroup.register_all 测试 ----

    #[test]
    fn test_smtp_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        SmtpPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 3);

        assert!(registry.get("smtp_connect").is_some());
        assert!(registry.get("smtp_send").is_some());
        assert!(registry.get("smtp_disconnect").is_some());
    }
}
