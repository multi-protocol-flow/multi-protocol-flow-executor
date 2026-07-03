// gRPC 节点插件 — NodePlugin trait 实现
//
// 提供 3 个 gRPC 节点的元数据、端口定义、配置验证和执行能力：
// - GrpcConnectPlugin: gRPC 连接
// - GrpcCallPlugin: gRPC 调用
// - GrpcClosePlugin: gRPC 关闭
// 以及 GrpcPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;

/// gRPC 连接节点插件
pub struct GrpcConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for GrpcConnectPlugin {
    fn type_id(&self) -> &str {
        "grpc_connect"
    }

    fn display_name(&self) -> &str {
        "gRPC 连接"
    }

    fn category(&self) -> &str {
        "grpc"
    }

    fn icon(&self) -> &str {
        "cable"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
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
            "use_tls": false,
            "tls_skip_verify": false,
            "enable_reflection": false,
            "proto_files": [],
            "default_metadata": [],
            "connect_timeout_ms": 30000,
        })
    }

    /// `GrpcConnect` 检查 $.success → "true" / "false"
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
            "use_tls": config.get("use_tls"),
            "tls_skip_verify": config.get("tls_skip_verify"),
            "enable_reflection": config.get("enable_reflection"),
            "proto_files": config.get("proto_files"),
            "default_metadata": config.get("default_metadata"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
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
        let grpc_manager = context
            .get_provider::<crate::grpc_connection_manager::GrpcConnectionManager>("grpc")
            .unwrap_or_else(|| {
                let mgr = Arc::new(crate::grpc_connection_manager::GrpcConnectionManager::new());
                context.set_provider("grpc", mgr.clone());
                mgr
            });
        let executor = crate::grpc_connect_executor::GrpcConnectExecutor::new(grpc_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// gRPC 调用节点插件
pub struct GrpcCallPlugin;

#[async_trait::async_trait]
impl NodePlugin for GrpcCallPlugin {
    fn type_id(&self) -> &str {
        "grpc_call"
    }

    fn display_name(&self) -> &str {
        "gRPC 调用"
    }

    fn category(&self) -> &str {
        "grpc"
    }

    fn icon(&self) -> &str {
        "phone-outgoing"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
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
            "service_name": "",
            "method_name": "",
            "request_json": "{}",
            "timeout_ms": 30000,
            "metadata": [],
            "request_messages": [],
        })
    }

    /// `GrpcCall` 检查 $.success → "true" / "false"
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
            "service_name": config.get("service_name"),
            "method_name": config.get("method_name"),
            "request_json": config.get("request_json"),
            "timeout_ms": config.get("timeout_ms"),
            "metadata": config.get("metadata"),
            "request_messages": config.get("request_messages"),
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
        match config.get("service_name").and_then(|v| v.as_str()) {
            Some(name) if !name.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "service_name".into(),
                message: "Service name is required".into(),
            }),
        }
        match config.get("method_name").and_then(|v| v.as_str()) {
            Some(name) if !name.is_empty() => {}
            _ => errors.push(ValidationError {
                field: "method_name".into(),
                message: "Method name is required".into(),
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
        let grpc_manager = context
            .get_provider::<crate::grpc_connection_manager::GrpcConnectionManager>("grpc")
            .unwrap_or_else(|| {
                let mgr = Arc::new(crate::grpc_connection_manager::GrpcConnectionManager::new());
                context.set_provider("grpc", mgr.clone());
                mgr
            });
        let executor = crate::grpc_call_executor::GrpcCallExecutor::new(grpc_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// gRPC 关闭节点插件
pub struct GrpcClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for GrpcClosePlugin {
    fn type_id(&self) -> &str {
        "grpc_close"
    }

    fn display_name(&self) -> &str {
        "gRPC 关闭"
    }

    fn category(&self) -> &str {
        "grpc"
    }

    fn icon(&self) -> &str {
        "phone-off"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
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
                label: "Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_id": "",
        })
    }

    /// `GrpcClose` 检查 $.success → "true" / "false"
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

    #[allow(deprecated)]
    fn supports_false_port(&self) -> bool {
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
        let grpc_manager = context
            .get_provider::<crate::grpc_connection_manager::GrpcConnectionManager>("grpc")
            .unwrap_or_else(|| {
                let mgr = Arc::new(crate::grpc_connection_manager::GrpcConnectionManager::new());
                context.set_provider("grpc", mgr.clone());
                mgr
            });
        let executor = crate::grpc_close_executor::GrpcCloseExecutor::new(grpc_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

/// gRPC 插件组 — 批量注册辅助
pub struct GrpcPluginGroup;

impl GrpcPluginGroup {
    /// 注册全部 gRPC 节点插件到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(GrpcConnectPlugin));
        registry.register(Arc::new(GrpcCallPlugin));
        registry.register(Arc::new(GrpcClosePlugin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_connect_plugin_metadata() {
        let plugin = GrpcConnectPlugin;
        assert_eq!(plugin.type_id(), "grpc_connect");
        assert_eq!(plugin.display_name(), "gRPC 连接");
        assert_eq!(plugin.category(), "grpc");
        assert_eq!(plugin.icon(), "cable");
        assert_eq!(plugin.color(), "#8B5CF6");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_grpc_call_plugin_metadata() {
        let plugin = GrpcCallPlugin;
        assert_eq!(plugin.type_id(), "grpc_call");
        assert_eq!(plugin.display_name(), "gRPC 调用");
        assert_eq!(plugin.category(), "grpc");
        assert_eq!(plugin.icon(), "phone-outgoing");
        assert_eq!(plugin.color(), "#8B5CF6");
        assert!(plugin.is_protocol_node());

        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_grpc_close_plugin_metadata() {
        let plugin = GrpcClosePlugin;
        assert_eq!(plugin.type_id(), "grpc_close");
        assert_eq!(plugin.display_name(), "gRPC 关闭");
        assert_eq!(plugin.category(), "grpc");
        assert_eq!(plugin.icon(), "phone-off");
        assert_eq!(plugin.color(), "#8B5CF6");
        let ports = plugin.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_grpc_connect_resolve_port() {
        let plugin = GrpcConnectPlugin;
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
    fn test_grpc_call_resolve_port() {
        let plugin = GrpcCallPlugin;
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
    fn test_grpc_close_resolve_port() {
        let plugin = GrpcClosePlugin;
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
    fn test_default_config_not_empty() {
        let connect_plugin = GrpcConnectPlugin;
        let config = connect_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("url").is_some());
        assert!(config.get("connect_timeout_ms").is_some());

        let call_plugin = GrpcCallPlugin;
        let config = call_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
        assert!(config.get("service_name").is_some());

        let close_plugin = GrpcClosePlugin;
        let config = close_plugin.default_config();
        assert!(config.is_object());
        assert!(config.get("connection_id").is_some());
    }

    #[test]
    fn test_grpc_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());

        GrpcPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 3);

        assert!(registry.get("grpc_connect").is_some());
        assert!(registry.get("grpc_call").is_some());
        assert!(registry.get("grpc_close").is_some());
    }

    #[test]
    fn test_build_report_input() {
        let plugin = GrpcConnectPlugin;
        let config = serde_json::json!({
            "url": "http://localhost:50051",
            "use_tls": true,
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report["url"], "http://localhost:50051");
        assert_eq!(report["use_tls"], true);
    }

    // ========================================================================
    // GrpcCallPlugin 验证测试
    // ========================================================================

    #[test]
    fn test_call_default_config_contains_request_messages() {
        let plugin = GrpcCallPlugin;
        let config = plugin.default_config();
        assert!(config.is_object());
        assert!(
            config.get("request_messages").is_some(),
            "default_config 应包含 request_messages 字段"
        );
        assert_eq!(config["request_messages"], serde_json::json!([]));
    }

    #[test]
    fn test_call_default_config_does_not_contain_collect_config() {
        let plugin = GrpcCallPlugin;
        let config = plugin.default_config();
        assert!(
            config.get("collect_config").is_none(),
            "default_config 不应包含 collect_config 字段"
        );
    }

    #[test]
    fn test_call_default_config_contains_all_fields() {
        let plugin = GrpcCallPlugin;
        let config = plugin.default_config();
        // 验证所有关键字段存在
        assert!(config.get("connection_id").is_some());
        assert!(config.get("service_name").is_some());
        assert!(config.get("method_name").is_some());
        assert!(config.get("request_json").is_some());
        assert!(config.get("timeout_ms").is_some());
        assert!(config.get("metadata").is_some());
        assert!(config.get("request_messages").is_some());
        // collect_config 已移除
        assert!(config.get("collect_config").is_none());
    }

    #[test]
    fn test_call_build_report_input_includes_new_fields() {
        let plugin = GrpcCallPlugin;
        let config = serde_json::json!({
            "connection_id": "conn-1",
            "service_name": "test.Service",
            "method_name": "GetUser",
            "request_json": "{}",
            "timeout_ms": 30000,
            "metadata": [{"key": "auth", "value": "token123"}],
            "request_messages": [{"enabled": true, "content": "{}"}],
        });
        let report = plugin.build_report_input(&config);
        assert!(report.is_some());
        let report = report.unwrap();
        // 验证新字段包含在报告输入中
        assert!(report.get("request_messages").is_some(), "报告应包含 request_messages");
        // collect_config 已移除
        assert!(report.get("collect_config").is_none(), "报告不应包含 collect_config");
        // 验证基本字段
        assert!(report.get("connection_id").is_some());
        assert!(report.get("service_name").is_some());
        assert!(report.get("method_name").is_some());
    }

    #[test]
    fn test_call_resolve_port_with_streaming_output() {
        let plugin = GrpcCallPlugin;

        // 模拟 streaming Success输出（有 responses 数组）
        let streaming_success = serde_json::json!({
            "success": true,
            "pattern": "server_streaming",
            "responses": [{"chunk": 1}, {"chunk": 2}],
        });
        assert_eq!(plugin.resolve_port(&Some(streaming_success)), "true");

        // 模拟 streaming Failed输出
        let streaming_error = serde_json::json!({
            "success": false,
            "pattern": "bidi_streaming",
            "error": "超时",
        });
        assert_eq!(plugin.resolve_port(&Some(streaming_error)), "false");
    }

    #[test]
    fn test_connect_resolve_port_with_streaming_output() {
        let plugin = GrpcConnectPlugin;

        // 连接Success
        let success = serde_json::json!({"success": true, "services": []});
        assert_eq!(plugin.resolve_port(&Some(success)), "true");

        // connection failed
        let failure = serde_json::json!({"success": false, "error": "连接超时"});
        assert_eq!(plugin.resolve_port(&Some(failure)), "false");
    }

    #[test]
    fn test_close_resolve_port_with_various_outputs() {
        let plugin = GrpcClosePlugin;

        // 关闭Success
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
            "true"
        );

        // 关闭Failed
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
            "false"
        );

        // 非 object 输出 → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!("string"))),
            "false"
        );

        // array 输出 → false
        assert_eq!(
            plugin.resolve_port(&Some(serde_json::json!([1, 2, 3]))),
            "false"
        );
    }

    #[test]
    fn test_call_resolve_port_edge_cases() {
        let plugin = GrpcCallPlugin;

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
    }

    #[test]
    fn test_connect_build_report_input_includes_all_fields() {
        let plugin = GrpcConnectPlugin;
        let config = serde_json::json!({
            "url": "http://localhost:50051",
            "use_tls": false,
            "tls_skip_verify": false,
            "enable_reflection": true,
            "proto_files": [],
            "default_metadata": [{"key": "auth", "value": "Bearer test"}],
            "connect_timeout_ms": 5000,
        });
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["url"], "http://localhost:50051");
        assert_eq!(report["enable_reflection"], true);
        assert_eq!(report["connect_timeout_ms"], 5000);
    }

    #[test]
    fn test_close_build_report_input() {
        let plugin = GrpcClosePlugin;
        let config = serde_json::json!({"connection_id": "conn-123"});
        let report = plugin.build_report_input(&config).expect("报告应存在");
        assert_eq!(report["connection_id"], "conn-123");
    }
}
