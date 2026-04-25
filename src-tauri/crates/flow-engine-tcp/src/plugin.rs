// TCP 节点插件 — NodePlugin trait 实现
//
// 提供 TCP 节点的元数据、端口定义、配置验证和执行能力：
// - TcpConnectPlugin: TCP 连接
// - TcpSendPlugin: TCP 发送
// 以及 TcpPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

/// TCP 连接节点插件
pub struct TcpConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for TcpConnectPlugin {
    fn type_id(&self) -> &str {
        "tcp_connect"
    }

    fn display_name(&self) -> &str {
        "TCP 连接"
    }

    fn category(&self) -> &str {
        "tcp"
    }

    fn icon(&self) -> &str {
        "Cable"
    }

    fn color(&self) -> &str {
        "#10B981"
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
        use flow_engine_core::node_config::TcpConnectConfig;
        let config = TcpConnectConfig::default();
        serde_json::json!({
            "host": config.host,
            "port": config.port,
            "use_tls": config.use_tls,
            "tls_skip_verify": config.tls_skip_verify,
            "connect_timeout_ms": config.connect_timeout_ms,
            "framing_mode": config.framing_mode,
            "framing_config": config.framing_config,
        })
    }

    /// `TcpConnect` 检查 $.success → "true" / "false"
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
            "host": config.get("host"),
            "port": config.get("port"),
            "use_tls": config.get("use_tls"),
            "tls_skip_verify": config.get("tls_skip_verify"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
            "framing_mode": config.get("framing_mode"),
            "framing_config": config.get("framing_config"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let tcp_manager = context.tcp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::tcp_connection_manager::TcpConnectionManager::new())
        });
        let executor = crate::tcp_connect_executor::TcpConnectExecutor::new(tcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// TCP Send 节点插件
pub struct TcpSendPlugin;

#[async_trait::async_trait]
impl NodePlugin for TcpSendPlugin {
    fn type_id(&self) -> &str {
        "tcp_send"
    }

    fn display_name(&self) -> &str {
        "TCP 发送"
    }

    fn category(&self) -> &str {
        "tcp"
    }

    fn icon(&self) -> &str {
        "Send"
    }

    fn color(&self) -> &str {
        "#10B981"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "发送成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "发送失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        use flow_engine_core::node_config::TcpSendConfig;
        let config = TcpSendConfig::default();
        serde_json::json!({
            "connection_id": config.connection_id,
            "data": config.data,
            "data_encoding": config.data_encoding,
            "framing_mode": config.framing_mode,
            "framing_config": config.framing_config,
            "send_timeout_ms": config.send_timeout_ms,
        })
    }

    /// TcpSend 检查 $.success → "true" / "false"
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
            "data_encoding": config.get("data_encoding"),
            "framing_mode": config.get("framing_mode"),
            "send_timeout_ms": config.get("send_timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let tcp_manager = context.tcp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::tcp_connection_manager::TcpConnectionManager::new())
        });
        let executor = crate::tcp_send_executor::TcpSendExecutor::new(tcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// TCP Close 节点插件
pub struct TcpClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for TcpClosePlugin {
    fn type_id(&self) -> &str {
        "tcp_close"
    }

    fn display_name(&self) -> &str {
        "TCP 关闭"
    }

    fn category(&self) -> &str {
        "tcp"
    }

    fn icon(&self) -> &str {
        "Unplug"
    }

    fn color(&self) -> &str {
        "#10B981"
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
        use flow_engine_core::node_config::TcpCloseConfig;
        let config = TcpCloseConfig::default();
        serde_json::json!({
            "connection_id": config.connection_id,
            "close_timeout_ms": config.close_timeout_ms,
        })
    }

    /// TcpClose 检查 $.success → "true" / "false"
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
        let tcp_manager = context.tcp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::tcp_connection_manager::TcpConnectionManager::new())
        });
        let executor = crate::tcp_close_executor::TcpCloseExecutor::new(tcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// TCP 插件组 — 批量注册辅助
pub struct TcpPluginGroup;

impl TcpPluginGroup {
    /// 注册全部 TCP 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(TcpConnectPlugin));
        registry.register(Arc::new(TcpSendPlugin));
        registry.register(Arc::new(TcpClosePlugin));
    }
}

// ============================================================================
// UDP 节点插件
// ============================================================================

/// UDP Bind 节点插件
pub struct UdpBindPlugin;

#[async_trait::async_trait]
impl NodePlugin for UdpBindPlugin {
    fn type_id(&self) -> &str {
        "udp_bind"
    }

    fn display_name(&self) -> &str {
        "UDP 绑定"
    }

    fn category(&self) -> &str {
        "udp"
    }

    fn icon(&self) -> &str {
        "Radio"
    }

    fn color(&self) -> &str {
        "#F97316"
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
        use flow_engine_core::node_config::UdpBindConfig;
        let config = UdpBindConfig::default();
        serde_json::json!({
            "local_addr": config.local_addr,
            "local_port": config.local_port,
            "reuse_addr": config.reuse_addr,
        })
    }

    /// UdpBind 检查 $.success → "true" / "false"
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
            "local_addr": config.get("local_addr"),
            "local_port": config.get("local_port"),
            "reuse_addr": config.get("reuse_addr"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let udp_manager = context.udp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::udp_socket_manager::UdpSocketManager::new())
        });
        let executor = crate::udp_bind_executor::UdpBindExecutor::new(udp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// UDP SendTo 节点插件
pub struct UdpSendToPlugin;

#[async_trait::async_trait]
impl NodePlugin for UdpSendToPlugin {
    fn type_id(&self) -> &str {
        "udp_send_to"
    }

    fn display_name(&self) -> &str {
        "UDP 发送"
    }

    fn category(&self) -> &str {
        "udp"
    }

    fn icon(&self) -> &str {
        "ArrowUpRight"
    }

    fn color(&self) -> &str {
        "#F97316"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "发送成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "发送失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        use flow_engine_core::node_config::UdpSendToConfig;
        let config = UdpSendToConfig::default();
        serde_json::json!({
            "connection_id": config.connection_id,
            "data": config.data,
            "data_encoding": config.data_encoding,
            "target_addr": config.target_addr,
            "target_port": config.target_port,
            "send_timeout_ms": config.send_timeout_ms,
        })
    }

    /// UdpSendTo 检查 $.success → "true" / "false"
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
            "data_encoding": config.get("data_encoding"),
            "target_addr": config.get("target_addr"),
            "target_port": config.get("target_port"),
            "send_timeout_ms": config.get("send_timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let udp_manager = context.udp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::udp_socket_manager::UdpSocketManager::new())
        });
        let executor = crate::udp_send_to_executor::UdpSendToExecutor::new(udp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// UDP RecvFrom 节点插件
pub struct UdpRecvFromPlugin;

#[async_trait::async_trait]
impl NodePlugin for UdpRecvFromPlugin {
    fn type_id(&self) -> &str {
        "udp_recv_from"
    }

    fn display_name(&self) -> &str {
        "UDP 接收"
    }

    fn category(&self) -> &str {
        "udp"
    }

    fn icon(&self) -> &str {
        "ArrowDownLeft"
    }

    fn color(&self) -> &str {
        "#F97316"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "接收成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "接收失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        use flow_engine_core::node_config::UdpRecvFromConfig;
        let config = UdpRecvFromConfig::default();
        serde_json::json!({
            "connection_id": config.connection_id,
            "receive_timeout_ms": config.receive_timeout_ms,
            "max_size": config.max_size,
        })
    }

    /// UdpRecvFrom 检查 $.success → "true" / "false"
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
            "receive_timeout_ms": config.get("receive_timeout_ms"),
            "max_size": config.get("max_size"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let udp_manager = context.udp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::udp_socket_manager::UdpSocketManager::new())
        });
        let executor = crate::udp_recv_from_executor::UdpRecvFromExecutor::new(udp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// UDP Close 节点插件
pub struct UdpClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for UdpClosePlugin {
    fn type_id(&self) -> &str {
        "udp_close"
    }

    fn display_name(&self) -> &str {
        "UDP 关闭"
    }

    fn category(&self) -> &str {
        "udp"
    }

    fn icon(&self) -> &str {
        "Unplug"
    }

    fn color(&self) -> &str {
        "#F97316"
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
        use flow_engine_core::node_config::UdpCloseConfig;
        let config = UdpCloseConfig::default();
        serde_json::json!({
            "connection_id": config.connection_id,
        })
    }

    /// UdpClose 检查 $.success → "true" / "false"
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
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let udp_manager = context.udp_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::udp_socket_manager::UdpSocketManager::new())
        });
        let executor = crate::udp_close_executor::UdpCloseExecutor::new(udp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// UDP 插件组 — 批量注册辅助
pub struct UdpPluginGroup;

impl UdpPluginGroup {
    /// 注册全部 UDP 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(UdpBindPlugin));
        registry.register(Arc::new(UdpSendToPlugin));
        registry.register(Arc::new(UdpRecvFromPlugin));
        registry.register(Arc::new(UdpClosePlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 元数据测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_tcp_connect_plugin_metadata() {
        let plugin = TcpConnectPlugin;
        assert_eq!(plugin.type_id(), "tcp_connect");
        assert_eq!(plugin.display_name(), "TCP 连接");
        assert_eq!(plugin.category(), "tcp");
        assert_eq!(plugin.icon(), "Cable");
        assert_eq!(plugin.color(), "#10B981");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "失败");
    }

    #[test]
    fn test_tcp_connect_resolve_port() {
        let plugin = TcpConnectPlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
        assert_eq!(plugin.resolve_port(&None), "false");
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!("string"))),
            "false"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({}))),
            "false"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": null}))),
            "false"
        );
    }

    #[test]
    fn test_default_config_not_empty() {
        let plugin = TcpConnectPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("host").is_some());
        assert!(config.get("port").is_some());
        assert!(config.get("use_tls").is_some());
        assert!(config.get("connect_timeout_ms").is_some());
        assert!(config.get("framing_mode").is_some());
        assert!(config.get("framing_config").is_some());
    }

    #[test]
    fn test_build_report_input() {
        let plugin = TcpConnectPlugin;
        let config = serde_json::json!({
            "host": "127.0.0.1",
            "port": 8080,
            "use_tls": false,
            "tls_skip_verify": false,
            "connect_timeout_ms": 30000,
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["host"], "127.0.0.1");
        assert_eq!(report["port"], 8080);
        assert_eq!(report["use_tls"], false);
    }

    #[test]
    fn test_tcp_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        TcpPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 3);
        assert!(registry.get("tcp_connect").is_some());
        assert!(registry.get("tcp_send").is_some());
        assert!(registry.get("tcp_close").is_some());
    }

    // -----------------------------------------------------------------------
    // TcpSendPlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_tcp_send_plugin_metadata() {
        let plugin = TcpSendPlugin;
        assert_eq!(plugin.type_id(), "tcp_send");
        assert_eq!(plugin.display_name(), "TCP 发送");
        assert_eq!(plugin.category(), "tcp");
        assert_eq!(plugin.icon(), "Send");
        assert_eq!(plugin.color(), "#10B981");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "发送成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "发送失败");
    }

    #[test]
    fn test_tcp_send_resolve_port() {
        let plugin = TcpSendPlugin;
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
    fn test_tcp_send_default_config() {
        let plugin = TcpSendPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("data").is_some());
        assert!(config.get("data_encoding").is_some());
        assert!(config.get("framing_mode").is_some());
        assert!(config.get("send_timeout_ms").is_some());
    }

    #[test]
    fn test_tcp_send_build_report_input() {
        let plugin = TcpSendPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "data": "Hello",
            "data_encoding": "text",
            "framing_mode": { "LengthPrefix": { "prefix_bytes": 4 } },
            "send_timeout_ms": 30000,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["data_encoding"], "text");
        assert_eq!(report["send_timeout_ms"], 30000);
    }

    #[test]
    fn test_tcp_send_resolve_port_edge_cases() {
        let plugin = TcpSendPlugin;

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

        // array → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!([1, 2, 3]))),
            "false"
        );
    }

    // -----------------------------------------------------------------------
    // TcpClosePlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_tcp_close_plugin_metadata() {
        let plugin = TcpClosePlugin;
        assert_eq!(plugin.type_id(), "tcp_close");
        assert_eq!(plugin.display_name(), "TCP 关闭");
        assert_eq!(plugin.category(), "tcp");
        assert_eq!(plugin.icon(), "Unplug");
        assert_eq!(plugin.color(), "#10B981");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "失败");
    }

    #[test]
    fn test_tcp_close_resolve_port() {
        let plugin = TcpClosePlugin;
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );
        assert_eq!(plugin.resolve_port(&None), "false");
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!("string"))),
            "false"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({}))),
            "false"
        );
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": null}))),
            "false"
        );
    }

    #[test]
    fn test_tcp_close_default_config() {
        let plugin = TcpClosePlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("close_timeout_ms").is_some());
    }

    #[test]
    fn test_tcp_close_build_report_input() {
        let plugin = TcpClosePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "close_timeout_ms": 5000,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["close_timeout_ms"], 5000);
    }

    // -----------------------------------------------------------------------
    // UdpBindPlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_udp_bind_plugin_metadata() {
        let plugin = UdpBindPlugin;
        assert_eq!(plugin.type_id(), "udp_bind");
        assert_eq!(plugin.display_name(), "UDP 绑定");
        assert_eq!(plugin.category(), "udp");
        assert_eq!(plugin.icon(), "Radio");
        assert_eq!(plugin.color(), "#F97316");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "失败");
    }

    #[test]
    fn test_udp_bind_resolve_port() {
        let plugin = UdpBindPlugin;
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
    fn test_udp_bind_default_config() {
        let plugin = UdpBindPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("local_addr").is_some());
        assert!(config.get("local_port").is_some());
        assert!(config.get("reuse_addr").is_some());
    }

    #[test]
    fn test_udp_bind_build_report_input() {
        let plugin = UdpBindPlugin;
        let config = serde_json::json!({
            "local_addr": "0.0.0.0",
            "local_port": 8080,
            "reuse_addr": true,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["local_addr"], "0.0.0.0");
        assert_eq!(report["local_port"], 8080);
    }

    // -----------------------------------------------------------------------
    // UdpSendToPlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_udp_send_to_plugin_metadata() {
        let plugin = UdpSendToPlugin;
        assert_eq!(plugin.type_id(), "udp_send_to");
        assert_eq!(plugin.display_name(), "UDP 发送");
        assert_eq!(plugin.category(), "udp");
        assert_eq!(plugin.icon(), "ArrowUpRight");
        assert_eq!(plugin.color(), "#F97316");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "发送成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "发送失败");
    }

    #[test]
    fn test_udp_send_to_resolve_port() {
        let plugin = UdpSendToPlugin;
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
    fn test_udp_send_to_default_config() {
        let plugin = UdpSendToPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("data").is_some());
        assert!(config.get("data_encoding").is_some());
        assert!(config.get("target_addr").is_some());
        assert!(config.get("target_port").is_some());
        assert!(config.get("send_timeout_ms").is_some());
    }

    #[test]
    fn test_udp_send_to_build_report_input() {
        let plugin = UdpSendToPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "data_encoding": "text",
            "target_addr": "127.0.0.1",
            "target_port": 9090,
            "send_timeout_ms": 30000,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["target_addr"], "127.0.0.1");
        assert_eq!(report["send_timeout_ms"], 30000);
    }

    // -----------------------------------------------------------------------
    // UdpRecvFromPlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_udp_recv_from_plugin_metadata() {
        let plugin = UdpRecvFromPlugin;
        assert_eq!(plugin.type_id(), "udp_recv_from");
        assert_eq!(plugin.display_name(), "UDP 接收");
        assert_eq!(plugin.category(), "udp");
        assert_eq!(plugin.icon(), "ArrowDownLeft");
        assert_eq!(plugin.color(), "#F97316");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "接收成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "接收失败");
    }

    #[test]
    fn test_udp_recv_from_resolve_port() {
        let plugin = UdpRecvFromPlugin;
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
    fn test_udp_recv_from_default_config() {
        let plugin = UdpRecvFromPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("receive_timeout_ms").is_some());
        assert!(config.get("max_size").is_some());
    }

    #[test]
    fn test_udp_recv_from_build_report_input() {
        let plugin = UdpRecvFromPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "receive_timeout_ms": 5000,
            "max_size": 65536,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["receive_timeout_ms"], 5000);
    }

    // -----------------------------------------------------------------------
    // UdpClosePlugin 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_udp_close_plugin_metadata() {
        let plugin = UdpClosePlugin;
        assert_eq!(plugin.type_id(), "udp_close");
        assert_eq!(plugin.display_name(), "UDP 关闭");
        assert_eq!(plugin.category(), "udp");
        assert_eq!(plugin.icon(), "Unplug");
        assert_eq!(plugin.color(), "#F97316");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[0].label, "成功");
        assert_eq!(ports[1].id, "false");
        assert_eq!(ports[1].label, "失败");
    }

    #[test]
    fn test_udp_close_resolve_port() {
        let plugin = UdpClosePlugin;
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
    fn test_udp_close_default_config() {
        let plugin = UdpClosePlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    #[test]
    fn test_udp_close_build_report_input() {
        let plugin = UdpClosePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
    }

    // -----------------------------------------------------------------------
    // UdpPluginGroup 测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_udp_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        UdpPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 4);
        assert!(registry.get("udp_bind").is_some());
        assert!(registry.get("udp_send_to").is_some());
        assert!(registry.get("udp_recv_from").is_some());
        assert!(registry.get("udp_close").is_some());
    }

    #[test]
    fn test_tcp_and_udp_plugin_groups_combined() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        TcpPluginGroup::register_all(&mut registry);
        UdpPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 7);
    }
}
