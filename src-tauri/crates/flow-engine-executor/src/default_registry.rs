//! 默认节点注册表 — 注册全部内置节点插件
//!
//! 此模块位于 `flow-engine-executor` crate 中，因为它需要依赖所有协议 crate
//! （http、ws、db、script、assertion、grpc、mcp、tcp、mqtt、kafka、sse），而 `flow-engine-core` 作为叶子 crate
//! 不能依赖它们。

use flow_engine_core::registry::NodeRegistry;

/// 创建包含全部内置节点的默认注册表
///
/// # 注册的插件
///
/// | 分类 | 插件 | type_id |
/// |------|------|---------|
/// | Logic | EntryPlugin | `entry` |
/// | Logic | ConditionalPlugin | `conditional` |
/// | HTTP | HttpPlugin | `http` |
/// | WebSocket | WsConnectPlugin | `ws_connect` |
/// | WebSocket | WsSendCollectPlugin | `ws_send_collect` |
/// | WebSocket | WsClosePlugin | `ws_close` |
/// | Database | DbConnectPlugin | `db_connect` |
/// | Database | DbQueryPlugin | `db_query` |
/// | Database | DbBeginPlugin | `db_begin` |
/// | Database | DbCommitPlugin | `db_commit` |
/// | Database | DbRollbackPlugin | `db_rollback` |
/// | Database | DbClosePlugin | `db_close` |
/// | Script | ScriptPlugin | `script` |
/// | Assertion | AssertionPlugin | `assertion` |
/// | gRPC | GrpcConnectPlugin | `grpc_connect` |
/// | gRPC | GrpcCallPlugin | `grpc_call` |
/// | gRPC | GrpcClosePlugin | `grpc_close` |
/// | MCP | McpConnectPlugin | `mcp_connect` |
/// | MCP | McpCallToolPlugin | `mcp_call_tool` |
/// | MCP | McpReadResourcePlugin | `mcp_read_resource` |
/// | MCP | McpGetPromptPlugin | `mcp_get_prompt` |
/// | MCP | McpClosePlugin | `mcp_close` |
/// | TCP | TcpConnectPlugin | `tcp_connect` |
/// | TCP | TcpSendPlugin | `tcp_send` |
/// | TCP | TcpClosePlugin | `tcp_close` |
/// | UDP | UdpBindPlugin | `udp_bind` |
/// | UDP | UdpSendToPlugin | `udp_send_to` |
/// | UDP | UdpRecvFromPlugin | `udp_recv_from` |
/// | UDP | UdpClosePlugin | `udp_close` |
/// | MQTT | MqttConnectPlugin | `mqtt_connect` |
/// | MQTT | MqttPublishPlugin | `mqtt_publish` |
/// | MQTT | MqttSubscribePlugin | `mqtt_subscribe` |
/// | MQTT | MqttUnsubscribePlugin | `mqtt_unsubscribe` |
/// | MQTT | MqttDisconnectPlugin | `mqtt_disconnect` |
/// | MQTT | MqttWaitMessagePlugin | `mqtt_wait_message` |
/// | MQTT | MqttRequestPlugin | `mqtt_request` |
/// | Kafka | KafkaConnectPlugin | `kafka_connect` |
/// | Kafka | KafkaProducePlugin | `kafka_produce` |
/// | Kafka | KafkaConsumePlugin | `kafka_consume` |
/// | Kafka | KafkaConsumeBatchPlugin | `kafka_consume_batch` |
/// | Kafka | KafkaAdminPlugin | `kafka_admin` |
/// | Kafka | KafkaSchemaRegistryPlugin | `kafka_schema_registry` |
/// | Kafka | KafkaClosePlugin | `kafka_close` |
/// | SSE | SseConnectPlugin | `sse_connect` |
/// | SSE | SseListenPlugin | `sse_listen` |
/// | SSE | SseDisconnectPlugin | `sse_disconnect` |
/// | GraphQL | GraphqlConnectPlugin | `graphql_connect` |
/// | GraphQL | GraphqlQueryPlugin | `graphql_query` |
/// | GraphQL | GraphqlSubscribePlugin | `graphql_subscribe` |
/// | GraphQL | GraphqlUnsubscribePlugin | `graphql_unsubscribe` |
/// | GraphQL | GraphqlIntrospectPlugin | `graphql_introspect` |
/// | GraphQL | GraphqlDisconnectPlugin | `graphql_disconnect` |
pub fn default_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // Logic (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(crate::plugin_entry::EntryPlugin));
    registry.register(std::sync::Arc::new(
        crate::plugin_conditional::ConditionalPlugin,
    ));

    // HTTP (flow-engine-http)
    registry.register(std::sync::Arc::new(flow_engine_http::plugin::HttpPlugin));

    // WebSocket (flow-engine-ws) — 批量注册 3 个插件
    flow_engine_ws::plugin::WsPluginGroup::register_all(&mut registry);

    // Database (flow-engine-db) — 批量注册 6 个插件
    flow_engine_db::plugin::DbPluginGroup::register_all(&mut registry);

    // Script (flow-engine-script)
    registry.register(std::sync::Arc::new(
        flow_engine_script::plugin::ScriptPlugin,
    ));

    // Assertion (flow-engine-assertion)
    registry.register(std::sync::Arc::new(
        flow_engine_assertion::plugin::AssertionPlugin,
    ));

    // gRPC (flow-engine-grpc) — 批量注册 3 个插件
    flow_engine_grpc::plugin::GrpcPluginGroup::register_all(&mut registry);

    // TCP (flow-engine-tcp) — 批量注册 3 个插件
    flow_engine_tcp::plugin::TcpPluginGroup::register_all(&mut registry);

    // UDP (flow-engine-tcp) — 批量注册 4 个插件
    flow_engine_tcp::plugin::UdpPluginGroup::register_all(&mut registry);

    // MCP (flow-engine-mcp) — 批量注册 5 个插件
    flow_engine_mcp::plugin::McpPluginGroup::register_all(&mut registry);

    // MQTT (flow-engine-mqtt) — 批量注册 7 个插件
    flow_engine_mqtt::plugin::MqttPluginGroup::register_all(&mut registry);

    // Kafka (flow-engine-kafka) — 批量注册 7 个插件
    flow_engine_kafka::plugin::KafkaPluginGroup::register_all(&mut registry);

    // SSE (flow-engine-sse) — 批量注册 3 个插件
    flow_engine_sse::plugin::SsePluginGroup::register_all(&mut registry);

    // GraphQL (flow-engine-graphql) — 批量注册 6 个插件
    flow_engine_graphql::plugin::GraphqlPluginGroup::register_all(&mut registry);

    // AMQP (flow-engine-amqp) — 批量注册 17 个插件（11 AMQP 0-9-1 + 6 AMQP 1.0）
    flow_engine_amqp::amqp091::plugin::AmqpPluginGroup::register_all(&mut registry);

    // SMTP (flow-engine-smtp) — 批量注册 3 个插件
    flow_engine_smtp::plugin::SmtpPluginGroup::register_all(&mut registry);

    // IMAP (flow-engine-imap) — 批量注册 13 个插件
    flow_engine_imap::plugin::ImapPluginGroup::register_all(&mut registry);

    // Variable Extractor (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(
        crate::plugin_variable_extractor::VariableExtractorPlugin,
    ));

    // Proxy (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(crate::plugin_proxy::ProxyPlugin));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_18_plugins() {
        let registry = default_registry();
        assert_eq!(registry.len(), 88);

        let metadata = registry.all_metadata();
        assert_eq!(metadata.len(), 88);

        // 验证每个 type_id 都已注册
        let type_ids: Vec<&str> = metadata.iter().map(|m| m.type_id.as_str()).collect();
        let expected = [
            "entry",
            "conditional",
            "http",
            "ws_connect",
            "ws_send_collect",
            "ws_close",
            "assertion",
            "db_connect",
            "db_query",
            "db_begin",
            "db_commit",
            "db_rollback",
            "db_close",
            "script",
            "grpc_connect",
            "grpc_call",
            "grpc_close",
            "tcp_connect",
            "tcp_send",
            "tcp_close",
            "udp_bind",
            "udp_send_to",
            "udp_recv_from",
            "udp_close",
            "mcp_connect",
            "mcp_call_tool",
            "mcp_read_resource",
            "mcp_get_prompt",
            "mcp_close",
            "mqtt_connect",
            "mqtt_publish",
            "mqtt_subscribe",
            "mqtt_unsubscribe",
            "mqtt_disconnect",
            "mqtt_wait_message",
            "mqtt_request",
            "kafka_connect",
            "kafka_produce",
            "kafka_consume",
            "kafka_consume_batch",
            "kafka_admin",
            "kafka_schema_registry",
            "kafka_close",
            "sse_connect",
            "sse_listen",
            "sse_disconnect",
            "graphql_connect",
            "graphql_query",
            "graphql_subscribe",
            "graphql_unsubscribe",
            "graphql_introspect",
            "graphql_disconnect",
            "amqp_connect",
            "amqp_close",
            "amqp_exchange_declare",
            "amqp_exchange_delete",
            "amqp_queue_declare",
            "amqp_queue_delete",
            "amqp_queue_bind",
            "amqp_publish",
            "amqp_consume",
            "amqp_get",
            "amqp_ack",
            "amqp10_connect",
            "amqp10_send",
            "amqp10_receive",
            "amqp10_accept",
            "amqp10_reject",
            "amqp10_disconnect",
            "variable_extractor",
            "proxy",
        ];

        for exp in &expected {
            assert!(
                type_ids.contains(exp),
                "Missing plugin: {}",
                exp
            );
        }
    }

    #[test]
    fn test_default_registry_all_plugins_have_metadata() {
        let registry = default_registry();
        let metadata = registry.all_metadata();

        for meta in &metadata {
            // 验证元数据基本完整性
            assert!(!meta.type_id.is_empty(), "type_id 不能为空");
            assert!(!meta.display_name.is_empty(), "display_name 不能为空");
            assert!(!meta.category.is_empty(), "category 不能为空");
            assert!(!meta.icon.is_empty(), "icon 不能为空");
            assert!(!meta.color.is_empty(), "color 不能为空");
            assert!(!meta.ports.is_empty(), "ports 不能为空");
        }
    }
}
