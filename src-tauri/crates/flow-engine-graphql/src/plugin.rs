// GraphQL 节点插件 — NodePlugin trait 实现
//
// 提供 6 个 GraphQL 节点的元数据、端口定义、配置验证和执行能力：
// - GraphqlConnectPlugin: GraphQL 连接
// - GraphqlQueryPlugin: GraphQL 查询/变更
// - GraphqlSubscribePlugin: GraphQL 订阅
// - GraphqlUnsubscribePlugin: GraphQL 取消订阅
// - GraphqlIntrospectPlugin: GraphQL Schema 探索
// - GraphqlDisconnectPlugin: GraphQL 断开连接
// 以及 GraphqlPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// GraphqlConnectPlugin
// ---------------------------------------------------------------------------

/// GraphQL 连接节点插件
pub struct GraphqlConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for GraphqlConnectPlugin {
    fn type_id(&self) -> &str {
        "graphql_connect"
    }

    fn display_name(&self) -> &str {
        "GraphQL 连接"
    }

    fn category(&self) -> &str {
        "graphql"
    }

    fn icon(&self) -> &str {
        "plug"
    }

    fn color(&self) -> &str {
        "#6366f1"
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
            "endpoint": "https://example.com/graphql",
            "headers": [],
        })
    }

    /// `GraphqlConnect` 检查 $.success → "true" / "false"
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
            "endpoint": config.get("endpoint"),
            "headers": config.get("headers"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let graphql_manager = context.graphql_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::graphql_connection_manager::GraphqlConnectionManager::new())
        });
        let executor = crate::graphql_connect_executor::GraphqlConnectExecutor::new(graphql_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// GraphqlQueryPlugin
// ---------------------------------------------------------------------------

/// GraphQL 查询/变更节点插件
pub struct GraphqlQueryPlugin;

#[async_trait::async_trait]
impl NodePlugin for GraphqlQueryPlugin {
    fn type_id(&self) -> &str {
        "graphql_query"
    }

    fn display_name(&self) -> &str {
        "GraphQL 查询"
    }

    fn category(&self) -> &str {
        "graphql"
    }

    fn icon(&self) -> &str {
        "search"
    }

    fn color(&self) -> &str {
        "#6366f1"
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
            "query": "{ __typename }",
            "variables": {},
        })
    }

    /// `GraphqlQuery` 检查 $.success → "true" / "false"
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
            "query": config.get("query"),
            "variables": config.get("variables"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let graphql_manager = context.graphql_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::graphql_connection_manager::GraphqlConnectionManager::new())
        });
        let executor = crate::graphql_query_executor::GraphqlQueryExecutor::new(graphql_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// GraphqlSubscribePlugin
// ---------------------------------------------------------------------------

/// GraphQL 订阅节点插件
pub struct GraphqlSubscribePlugin;

#[async_trait::async_trait]
impl NodePlugin for GraphqlSubscribePlugin {
    fn type_id(&self) -> &str {
        "graphql_subscribe"
    }

    fn display_name(&self) -> &str {
        "GraphQL 订阅"
    }

    fn category(&self) -> &str {
        "graphql"
    }

    fn icon(&self) -> &str {
        "eye"
    }

    fn color(&self) -> &str {
        "#6366f1"
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
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "subscription": "subscription { messageAdded { id text } }",
            "variables": {},
        })
    }

    /// `GraphqlSubscribe` 检查 $.success → "true" / "false"
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
            "subscription": config.get("subscription"),
            "variables": config.get("variables"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let graphql_manager = context.graphql_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::graphql_connection_manager::GraphqlConnectionManager::new())
        });
        let executor = crate::graphql_subscribe_executor::GraphqlSubscribeExecutor::new(graphql_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// GraphqlIntrospectPlugin
// ---------------------------------------------------------------------------

/// GraphQL Schema 探索节点插件
pub struct GraphqlIntrospectPlugin;

#[async_trait::async_trait]
impl NodePlugin for GraphqlIntrospectPlugin {
    fn type_id(&self) -> &str {
        "graphql_introspect"
    }

    fn display_name(&self) -> &str {
        "GraphQL Schema 探索"
    }

    fn category(&self) -> &str {
        "graphql"
    }

    fn icon(&self) -> &str {
        "file-search"
    }

    fn color(&self) -> &str {
        "#6366f1"
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
            "endpoint": "https://example.com/graphql",
        })
    }

    /// `GraphqlIntrospect` 检查 $.success → "true" / "false"
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
            "endpoint": config.get("endpoint"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let graphql_manager = context.graphql_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::graphql_connection_manager::GraphqlConnectionManager::new())
        });
        let executor = crate::graphql_introspect_executor::GraphqlIntrospectExecutor::new(graphql_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// GraphqlDisconnectPlugin
// ---------------------------------------------------------------------------

/// GraphQL 断开连接节点插件
pub struct GraphqlDisconnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for GraphqlDisconnectPlugin {
    fn type_id(&self) -> &str {
        "graphql_disconnect"
    }

    fn display_name(&self) -> &str {
        "GraphQL 断开连接"
    }

    fn category(&self) -> &str {
        "graphql"
    }

    fn icon(&self) -> &str {
        "unplug"
    }

    fn color(&self) -> &str {
        "#6366f1"
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

    /// `GraphqlDisconnect` 始终返回 "out" 端口
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
        let graphql_manager = context.graphql_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::graphql_connection_manager::GraphqlConnectionManager::new())
        });
        let executor = crate::graphql_disconnect_executor::GraphqlDisconnectExecutor::new(graphql_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ---------------------------------------------------------------------------
// GraphqlPluginGroup
// ---------------------------------------------------------------------------

/// GraphQL 插件组 — 批量注册辅助
pub struct GraphqlPluginGroup;

impl GraphqlPluginGroup {
    /// 注册全部 GraphQL 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(GraphqlConnectPlugin));
        registry.register(Arc::new(GraphqlQueryPlugin));
        registry.register(Arc::new(GraphqlSubscribePlugin));
        registry.register(Arc::new(GraphqlIntrospectPlugin));
        registry.register(Arc::new(GraphqlDisconnectPlugin));
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
    fn test_graphql_connect_plugin_metadata() {
        let plugin = GraphqlConnectPlugin;
        assert_eq!(plugin.type_id(), "graphql_connect");
        assert_eq!(plugin.display_name(), "GraphQL 连接");
        assert_eq!(plugin.category(), "graphql");
        assert_eq!(plugin.icon(), "plug");
        assert_eq!(plugin.color(), "#6366f1");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_graphql_query_plugin_metadata() {
        let plugin = GraphqlQueryPlugin;
        assert_eq!(plugin.type_id(), "graphql_query");
        assert_eq!(plugin.display_name(), "GraphQL 查询");
        assert_eq!(plugin.category(), "graphql");
        assert_eq!(plugin.icon(), "search");
        assert_eq!(plugin.color(), "#6366f1");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_graphql_subscribe_plugin_metadata() {
        let plugin = GraphqlSubscribePlugin;
        assert_eq!(plugin.type_id(), "graphql_subscribe");
        assert_eq!(plugin.display_name(), "GraphQL 订阅");
        assert_eq!(plugin.category(), "graphql");
        assert_eq!(plugin.icon(), "eye");
        assert_eq!(plugin.color(), "#6366f1");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_graphql_introspect_plugin_metadata() {
        let plugin = GraphqlIntrospectPlugin;
        assert_eq!(plugin.type_id(), "graphql_introspect");
        assert_eq!(plugin.display_name(), "GraphQL Schema 探索");
        assert_eq!(plugin.category(), "graphql");
        assert_eq!(plugin.icon(), "file-search");
        assert_eq!(plugin.color(), "#6366f1");
        assert!(plugin.is_protocol_node());
        assert!(plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_graphql_disconnect_plugin_metadata() {
        let plugin = GraphqlDisconnectPlugin;
        assert_eq!(plugin.type_id(), "graphql_disconnect");
        assert_eq!(plugin.display_name(), "GraphQL 断开连接");
        assert_eq!(plugin.category(), "graphql");
        assert_eq!(plugin.icon(), "unplug");
        assert_eq!(plugin.color(), "#6366f1");
        assert!(plugin.is_protocol_node());
        assert!(!plugin.supports_false_port());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_graphql_connect_resolve_port() {
        let plugin = GraphqlConnectPlugin;
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
    fn test_graphql_query_resolve_port() {
        let plugin = GraphqlQueryPlugin;
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
    fn test_graphql_subscribe_resolve_port() {
        let plugin = GraphqlSubscribePlugin;
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
    fn test_graphql_introspect_resolve_port() {
        let plugin = GraphqlIntrospectPlugin;
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
    fn test_graphql_disconnect_resolve_port() {
        let plugin = GraphqlDisconnectPlugin;
        // GraphqlDisconnect 始终返回 "out"
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
    fn test_default_config_not_empty() {
        let connect_plugin = GraphqlConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("endpoint").is_some());

        let query_plugin = GraphqlQueryPlugin;
        let config = query_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("query").is_some());

        let subscribe_plugin = GraphqlSubscribePlugin;
        let config = subscribe_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("subscription").is_some());

        let introspect_plugin = GraphqlIntrospectPlugin;
        let config = introspect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("endpoint").is_some());

        let disconnect_plugin = GraphqlDisconnectPlugin;
        let config = disconnect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    // ---- build_report_input 测试 ----

    #[test]
    fn test_connect_build_report_input() {
        let plugin = GraphqlConnectPlugin;
        let config = serde_json::json!({
            "endpoint": "https://api.example.com/graphql",
            "headers": [{"key": "Authorization", "value": "Bearer token"}],
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["endpoint"], "https://api.example.com/graphql");
    }

    #[test]
    fn test_query_build_report_input() {
        let plugin = GraphqlQueryPlugin;
        let config = serde_json::json!({
            "query": "{ users { id name } }",
            "variables": {"limit": 10},
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["query"], "{ users { id name } }");
    }

    #[test]
    fn test_subscribe_build_report_input() {
        let plugin = GraphqlSubscribePlugin;
        let config = serde_json::json!({
            "subscription": "subscription { messageAdded { id } }",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["subscription"], "subscription { messageAdded { id } }");
    }

    #[test]
    fn test_introspect_build_report_input() {
        let plugin = GraphqlIntrospectPlugin;
        let config = serde_json::json!({
            "endpoint": "https://api.example.com/graphql",
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["endpoint"], "https://api.example.com/graphql");
    }

    #[test]
    fn test_disconnect_build_report_input() {
        let plugin = GraphqlDisconnectPlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }

    // ---- GraphqlPluginGroup.register_all 测试 ----

    #[test]
    fn test_graphql_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        GraphqlPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 5);

        assert!(registry.get("graphql_connect").is_some());
        assert!(registry.get("graphql_query").is_some());
        assert!(registry.get("graphql_subscribe").is_some());
        assert!(registry.get("graphql_introspect").is_some());
        assert!(registry.get("graphql_disconnect").is_some());
    }
}
