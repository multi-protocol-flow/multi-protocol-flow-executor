// MCP 节点插件 — NodePlugin trait 实现
//
// 提供 5 个 MCP 节点的元数据、端口定义、配置验证和执行能力：
// - McpConnectPlugin: MCP 连接
// - McpCallToolPlugin: MCP 调用工具
// - McpReadResourcePlugin: MCP 读取资源
// - McpGetPromptPlugin: MCP 获取提示词
// - McpClosePlugin: MCP 关闭
// 以及 McpPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

/// MCP 连接节点插件
pub struct McpConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for McpConnectPlugin {
    fn type_id(&self) -> &str {
        "mcp_connect"
    }

    fn display_name(&self) -> &str {
        "MCP 连接"
    }

    fn category(&self) -> &str {
        "mcp"
    }

    fn icon(&self) -> &str {
        "puzzle"
    }

    fn color(&self) -> &str {
        "#F59E0B"
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
            "url": "",
            "headers": null,
            "protocol_version": "2025-11-25",
            "custom_headers": null,
            "transport": "streamable_http",
            "command": null,
            "args": null,
            "env": null,
        })
    }

    /// `McpConnect` 检查 $.success → "true" / "false"
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
            "protocol_version": config.get("protocol_version"),
            "headers": config.get("headers"),
            "custom_headers": config.get("custom_headers"),
            "transport": config.get("transport"),
            "command": config.get("command"),
            "args": config.get("args"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 根据传输模式选择验证字段
        let transport = config
            .get("transport")
            .and_then(|v| v.as_str())
            .unwrap_or("streamable_http");

        match transport {
            "stdio" => {
                // Stdio 模式：command 必填，url 可选
                match config.get("command").and_then(|v| v.as_str()) {
                    Some(cmd) if !cmd.is_empty() => {}
                    _ => errors.push(ValidationError {
                        field: "command".into(),
                        message: "Command is required in Stdio mode".into(),
                    }),
                }
            }
            _ => {
                // Streamable HTTP 模式（默认）：url 必填
                match config.get("url").and_then(|v| v.as_str()) {
                    Some(url) if !url.is_empty() => {}
                    _ => errors.push(ValidationError {
                        field: "url".into(),
                        message: "URL is required".into(),
                    }),
                }
            }
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
        let mcp_manager = context
            .get_provider::<crate::mcp_connection_manager::McpConnectionManager>("mcp")
            .unwrap_or_else(|| Arc::new(crate::mcp_connection_manager::McpConnectionManager::new()));
        let executor = crate::mcp_connect_executor::McpConnectExecutor::new(mcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// MCP 调用工具节点插件
pub struct McpCallToolPlugin;

#[async_trait::async_trait]
impl NodePlugin for McpCallToolPlugin {
    fn type_id(&self) -> &str {
        "mcp_call_tool"
    }

    fn display_name(&self) -> &str {
        "MCP 调用工具"
    }

    fn category(&self) -> &str {
        "mcp"
    }

    fn icon(&self) -> &str {
        "wrench"
    }

    fn color(&self) -> &str {
        "#F59E0B"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "调用Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "调用Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "tool_name": "",
            "arguments": null,
        })
    }

    /// `McpCallTool` 检查 $.success → "true" / "false"
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
            "tool_name": config.get("tool_name"),
            "arguments": config.get("arguments"),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        match config.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "connection_id".into(),
                message: "Connection ID is required".into(),
            })
        }
        match config.get("tool_name").and_then(|v| v.as_str()) {
            Some(name) if !name.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "tool_name".into(),
                message: "Tool name is required".into(),
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
        let mcp_manager = context
            .get_provider::<crate::mcp_connection_manager::McpConnectionManager>("mcp")
            .unwrap_or_else(|| Arc::new(crate::mcp_connection_manager::McpConnectionManager::new()));
        let executor = crate::mcp_call_tool_executor::McpCallToolExecutor::new(mcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// MCP 读取资源节点插件
pub struct McpReadResourcePlugin;

#[async_trait::async_trait]
impl NodePlugin for McpReadResourcePlugin {
    fn type_id(&self) -> &str {
        "mcp_read_resource"
    }

    fn display_name(&self) -> &str {
        "MCP 读取资源"
    }

    fn category(&self) -> &str {
        "mcp"
    }

    fn icon(&self) -> &str {
        "file-search"
    }

    fn color(&self) -> &str {
        "#F59E0B"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "读取Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "读取Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "resource_uri": "",
        })
    }

    /// `McpReadResource` 检查 $.success → "true" / "false"
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
            "resource_uri": config.get("resource_uri"),
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
        match config.get("resource_uri").and_then(|v| v.as_str()) {
            Some(uri) if !uri.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "resource_uri".into(),
                message: "Resource URI is required".into(),
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
        let mcp_manager = context
            .get_provider::<crate::mcp_connection_manager::McpConnectionManager>("mcp")
            .unwrap_or_else(|| Arc::new(crate::mcp_connection_manager::McpConnectionManager::new()));
        let executor = crate::mcp_read_resource_executor::McpReadResourceExecutor::new(mcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// MCP 关闭节点插件
pub struct McpClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for McpClosePlugin {
    fn type_id(&self) -> &str {
        "mcp_close"
    }

    fn display_name(&self) -> &str {
        "MCP 关闭"
    }

    fn category(&self) -> &str {
        "mcp"
    }

    fn icon(&self) -> &str {
        "unplug"
    }

    fn color(&self) -> &str {
        "#F59E0B"
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

    /// `McpClose` 始终返回 "out" 端口
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
        let mcp_manager = context
            .get_provider::<crate::mcp_connection_manager::McpConnectionManager>("mcp")
            .unwrap_or_else(|| Arc::new(crate::mcp_connection_manager::McpConnectionManager::new()));
        let executor = crate::mcp_close_executor::McpCloseExecutor::new(mcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// MCP 获取提示词节点插件
pub struct McpGetPromptPlugin;

#[async_trait::async_trait]
impl NodePlugin for McpGetPromptPlugin {
    fn type_id(&self) -> &str {
        "mcp_get_prompt"
    }

    fn display_name(&self) -> &str {
        "MCP 获取提示词"
    }

    fn category(&self) -> &str {
        "mcp"
    }

    fn icon(&self) -> &str {
        "message-square"
    }

    fn color(&self) -> &str {
        "#F59E0B"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "获取Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "获取Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
            "prompt_name": "",
            "arguments": {},
        })
    }

    /// `McpGetPrompt` 检查 $.success → "true" / "false"
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
            "prompt_name": config.get("prompt_name"),
            "arguments": config.get("arguments"),
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
        match config.get("prompt_name").and_then(|v| v.as_str()) {
            Some(name) if !name.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "prompt_name".into(),
                message: "Prompt name is required".into(),
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
        let mcp_manager = context
            .get_provider::<crate::mcp_connection_manager::McpConnectionManager>("mcp")
            .unwrap_or_else(|| Arc::new(crate::mcp_connection_manager::McpConnectionManager::new()));
        let executor = crate::mcp_get_prompt_executor::McpGetPromptExecutor::new(mcp_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// MCP 插件组 — 批量注册辅助
pub struct McpPluginGroup;

impl McpPluginGroup {
    /// 注册全部 MCP 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(McpConnectPlugin));
        registry.register(Arc::new(McpCallToolPlugin));
        registry.register(Arc::new(McpReadResourcePlugin));
        registry.register(Arc::new(McpGetPromptPlugin));
        registry.register(Arc::new(McpClosePlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_connect_plugin_metadata() {
        let plugin = McpConnectPlugin;
        assert_eq!(plugin.type_id(), "mcp_connect");
        assert_eq!(plugin.display_name(), "MCP 连接");
        assert_eq!(plugin.category(), "mcp");
        assert_eq!(plugin.icon(), "puzzle");
        assert_eq!(plugin.color(), "#F59E0B");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mcp_call_tool_plugin_metadata() {
        let plugin = McpCallToolPlugin;
        assert_eq!(plugin.type_id(), "mcp_call_tool");
        assert_eq!(plugin.display_name(), "MCP 调用工具");
        assert_eq!(plugin.category(), "mcp");
        assert_eq!(plugin.icon(), "wrench");
        assert_eq!(plugin.color(), "#F59E0B");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mcp_read_resource_plugin_metadata() {
        let plugin = McpReadResourcePlugin;
        assert_eq!(plugin.type_id(), "mcp_read_resource");
        assert_eq!(plugin.display_name(), "MCP 读取资源");
        assert_eq!(plugin.category(), "mcp");
        assert_eq!(plugin.icon(), "file-search");
        assert_eq!(plugin.color(), "#F59E0B");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mcp_close_plugin_metadata() {
        let plugin = McpClosePlugin;
        assert_eq!(plugin.type_id(), "mcp_close");
        assert_eq!(plugin.display_name(), "MCP 关闭");
        assert_eq!(plugin.category(), "mcp");
        assert_eq!(plugin.icon(), "unplug");
        assert_eq!(plugin.color(), "#F59E0B");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    #[test]
    fn test_mcp_connect_resolve_port() {
        let plugin = McpConnectPlugin;
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
    fn test_mcp_call_tool_resolve_port() {
        let plugin = McpCallToolPlugin;
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
    fn test_mcp_read_resource_resolve_port() {
        let plugin = McpReadResourcePlugin;
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
    fn test_mcp_close_resolve_port() {
        let plugin = McpClosePlugin;
        // McpClose 始终返回 "out"
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
    fn test_default_config_not_empty() {
        let connect_plugin = McpConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("url").is_some());
        assert!(config.get("protocol_version").is_some());

        let call_plugin = McpCallToolPlugin;
        let config = call_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("tool_name").is_some());

        let read_plugin = McpReadResourcePlugin;
        let config = read_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("resource_uri").is_some());

        let close_plugin = McpClosePlugin;
        let config = close_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    #[test]
    fn test_mcp_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        McpPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 5);

        assert!(registry.get("mcp_connect").is_some());
        assert!(registry.get("mcp_call_tool").is_some());
        assert!(registry.get("mcp_read_resource").is_some());
        assert!(registry.get("mcp_get_prompt").is_some());
        assert!(registry.get("mcp_close").is_some());
    }

    #[test]
    fn test_build_report_input() {
        let plugin = McpConnectPlugin;
        let config = serde_json::json!({
            "url": "http://localhost:8080/mcp",
            "protocol_version": "2025-11-25",
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["url"], "http://localhost:8080/mcp");
        assert_eq!(report["protocol_version"], "2025-11-25");
    }

    #[test]
    fn test_call_tool_build_report_input() {
        let plugin = McpCallToolPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "tool_name": "get_weather",
            "arguments": {"city": "Beijing"},
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["tool_name"], "get_weather");
    }

    #[test]
    fn test_read_resource_build_report_input() {
        let plugin = McpReadResourcePlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "resource_uri": "resource://config",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["resource_uri"], "resource://config");
    }

    #[test]
    fn test_close_build_report_input() {
        let plugin = McpClosePlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    #[test]
    fn test_mcp_get_prompt_plugin_metadata() {
        let plugin = McpGetPromptPlugin;
        assert_eq!(plugin.type_id(), "mcp_get_prompt");
        assert_eq!(plugin.display_name(), "MCP 获取提示词");
        assert_eq!(plugin.category(), "mcp");
        assert_eq!(plugin.icon(), "message-square");
        assert_eq!(plugin.color(), "#F59E0B");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_mcp_get_prompt_resolve_port() {
        let plugin = McpGetPromptPlugin;
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
    fn test_get_prompt_build_report_input() {
        let plugin = McpGetPromptPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "prompt_name": "greet",
            "arguments": {"name": "Alice"},
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-1");
        assert_eq!(report["prompt_name"], "greet");
    }

    #[test]
    fn test_get_prompt_default_config() {
        let plugin = McpGetPromptPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("prompt_name").is_some());
        assert!(config.get("arguments").is_some());
    }

    #[test]
    fn test_connect_resolve_port_edge_cases() {
        let plugin = McpConnectPlugin;

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
}
