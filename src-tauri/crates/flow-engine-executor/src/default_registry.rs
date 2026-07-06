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
/// | TCP | TcpReceivePlugin | `tcp_receive` |
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
/// | GraphQL | GraphqlIntrospectPlugin | `graphql_introspect` |
/// | GraphQL | GraphqlDisconnectPlugin | `graphql_disconnect` |
/// | AMQP | AmqpConnectPlugin | `amqp_connect` |
/// | AMQP | AmqpClosePlugin | `amqp_close` |
/// | AMQP | AmqpExchangeDeclarePlugin | `amqp_exchange_declare` |
/// | AMQP | AmqpExchangeDeletePlugin | `amqp_exchange_delete` |
/// | AMQP | AmqpQueueDeclarePlugin | `amqp_queue_declare` |
/// | AMQP | AmqpQueueDeletePlugin | `amqp_queue_delete` |
/// | AMQP | AmqpQueueBindPlugin | `amqp_queue_bind` |
/// | AMQP | AmqpPublishPlugin | `amqp_publish` |
/// | AMQP | AmqpConsumePlugin | `amqp_consume` |
/// | AMQP | AmqpGetPlugin | `amqp_get` |
/// | AMQP | AmqpAckPlugin | `amqp_ack` |
/// | AMQP 1.0 | Amqp10ConnectPlugin | `amqp10_connect` |
/// | AMQP 1.0 | Amqp10SendPlugin | `amqp10_send` |
/// | AMQP 1.0 | Amqp10ReceivePlugin | `amqp10_receive` |
/// | AMQP 1.0 | Amqp10AcceptPlugin | `amqp10_accept` |
/// | AMQP 1.0 | Amqp10RejectPlugin | `amqp10_reject` |
/// | AMQP 1.0 | Amqp10DisconnectPlugin | `amqp10_disconnect` |
/// | SMTP | SmtpConnectPlugin | `smtp_connect` |
/// | SMTP | SmtpSendPlugin | `smtp_send` |
/// | SMTP | SmtpDisconnectPlugin | `smtp_disconnect` |
/// | IMAP | ImapConnectPlugin | `imap_connect` |
/// | IMAP | ImapSelectPlugin | `imap_select` |
/// | IMAP | ImapSearchPlugin | `imap_search` |
/// | IMAP | ImapFetchPlugin | `imap_fetch` |
/// | IMAP | ImapStoreFlagsPlugin | `imap_store_flags` |
/// | IMAP | ImapCopyPlugin | `imap_copy` |
/// | IMAP | ImapMovePlugin | `imap_move` |
/// | IMAP | ImapExpungePlugin | `imap_expunge` |
/// | IMAP | ImapListPlugin | `imap_list` |
/// | IMAP | ImapFolderCreatePlugin | `imap_folder_create` |
/// | IMAP | ImapFolderDeletePlugin | `imap_folder_delete` |
/// | IMAP | ImapIdlePlugin | `imap_idle` |
/// | IMAP | ImapClosePlugin | `imap_close` |
/// | Tools | EndPlugin | `end` |
/// | Utility | VariableExtractorPlugin | `variable_extractor` |
/// | Utility | ProxyPlugin | `proxy` |
pub fn default_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // Logic (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(crate::plugin_entry::EntryPlugin));
    registry.register(std::sync::Arc::new(
        crate::plugin_conditional::ConditionalPlugin,
    ));

    // HTTP (flow-engine-http)
    #[cfg(feature = "protocol-http")]
    registry.register(std::sync::Arc::new(flow_engine_http::plugin::HttpPlugin));

    // WebSocket (flow-engine-ws) — 批量注册 3 个插件
    #[cfg(feature = "protocol-ws")]
    flow_engine_ws::plugin::WsPluginGroup::register_all(&mut registry);

    // Database (flow-engine-db) — 批量注册 6 个插件
    #[cfg(feature = "protocol-db")]
    flow_engine_db::plugin::DbPluginGroup::register_all(&mut registry);

    // Script (flow-engine-script)
    #[cfg(feature = "protocol-script")]
    registry.register(std::sync::Arc::new(
        flow_engine_script::plugin::ScriptPlugin,
    ));

    // Assertion (flow-engine-assertion)
    #[cfg(feature = "protocol-assertion")]
    registry.register(std::sync::Arc::new(
        flow_engine_assertion::plugin::AssertionPlugin,
    ));

    // gRPC (flow-engine-grpc) — 批量注册 3 个插件
    #[cfg(feature = "protocol-grpc")]
    flow_engine_grpc::plugin::GrpcPluginGroup::register_all(&mut registry);

    // TCP (flow-engine-tcp) — 批量注册 4 个插件
    #[cfg(feature = "protocol-tcp")]
    flow_engine_tcp::plugin::TcpPluginGroup::register_all(&mut registry);

    // UDP (flow-engine-tcp) — 批量注册 4 个插件
    #[cfg(feature = "protocol-tcp")]
    flow_engine_tcp::plugin::UdpPluginGroup::register_all(&mut registry);

    // MCP (flow-engine-mcp) — 批量注册 5 个插件
    #[cfg(feature = "protocol-mcp")]
    flow_engine_mcp::plugin::McpPluginGroup::register_all(&mut registry);

    // MQTT (flow-engine-mqtt) — 批量注册 7 个插件
    #[cfg(feature = "protocol-mqtt")]
    flow_engine_mqtt::plugin::MqttPluginGroup::register_all(&mut registry);

    // Kafka (flow-engine-kafka) — 批量注册 7 个插件
    #[cfg(feature = "protocol-kafka")]
    flow_engine_kafka::plugin::KafkaPluginGroup::register_all(&mut registry);

    // SSE (flow-engine-sse) — 批量注册 3 个插件
    #[cfg(feature = "protocol-sse")]
    flow_engine_sse::plugin::SsePluginGroup::register_all(&mut registry);

    // GraphQL (flow-engine-graphql) — 批量注册 5 个插件
    #[cfg(feature = "protocol-graphql")]
    flow_engine_graphql::plugin::GraphqlPluginGroup::register_all(&mut registry);

    // AMQP (flow-engine-amqp) — 批量注册 17 个插件（11 AMQP 0-9-1 + 6 AMQP 1.0）
    #[cfg(feature = "protocol-amqp")]
    flow_engine_amqp::amqp091::plugin::AmqpPluginGroup::register_all(&mut registry);

    // SMTP (flow-engine-smtp) — 批量注册 3 个插件
    #[cfg(feature = "protocol-smtp")]
    flow_engine_smtp::plugin::SmtpPluginGroup::register_all(&mut registry);

    // IMAP (flow-engine-imap) — 批量注册 13 个插件
    #[cfg(feature = "protocol-imap")]
    flow_engine_imap::plugin::ImapPluginGroup::register_all(&mut registry);

    // Redis (flow-engine-redis) — 批量注册 3 个插件
    #[cfg(feature = "protocol-redis")]
    flow_engine_redis::plugin::RedisPluginGroup::register_all(&mut registry);

    // End (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(crate::plugin_end::EndPlugin));

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
    fn test_plugin_count_matches_features() {
        let registry = default_registry();
        let count = registry.len();
        let mut expected = 5usize; // always-on: entry, conditional, end, variable_extractor, proxy

        #[cfg(feature = "protocol-http")]
        {
            expected += 1;
        }
        #[cfg(feature = "protocol-ws")]
        {
            expected += 3;
        }
        #[cfg(feature = "protocol-db")]
        {
            expected += 6;
        }
        #[cfg(feature = "protocol-script")]
        {
            expected += 1;
        }
        #[cfg(feature = "protocol-assertion")]
        {
            expected += 1;
        }
        #[cfg(feature = "protocol-grpc")]
        {
            expected += 3;
        }
        #[cfg(feature = "protocol-mcp")]
        {
            expected += 5;
        }
        #[cfg(feature = "protocol-tcp")]
        {
            expected += 8;
        } // 4 TCP + 4 UDP
        #[cfg(feature = "protocol-mqtt")]
        {
            expected += 7;
        }
        #[cfg(feature = "protocol-kafka")]
        {
            expected += 7;
        }
        #[cfg(feature = "protocol-sse")]
        {
            expected += 3;
        }
        #[cfg(feature = "protocol-graphql")]
        {
            expected += 5;
        }
        #[cfg(feature = "protocol-amqp")]
        {
            expected += 17;
        }
        #[cfg(feature = "protocol-smtp")]
        {
            expected += 3;
        }
        #[cfg(feature = "protocol-imap")]
        {
            expected += 13;
        }
        #[cfg(feature = "protocol-redis")]
        {
            expected += 3;
        }

        assert_eq!(
            count, expected,
            "Plugin count mismatch for active features. Expected={}, actual={}",
            expected, count
        );
    }

    #[test]
    fn test_type_ids_match_features() {
        let registry = default_registry();
        let metadata = registry.all_metadata();
        let type_ids: Vec<&str> = metadata.iter().map(|m| m.type_id.as_str()).collect();

        // Always present
        assert!(type_ids.contains(&"entry"));
        assert!(type_ids.contains(&"conditional"));
        assert!(type_ids.contains(&"end"));
        assert!(type_ids.contains(&"variable_extractor"));
        assert!(type_ids.contains(&"proxy"));

        // Conditional on features
        #[cfg(feature = "protocol-http")]
        {
            assert!(type_ids.contains(&"http"), "http should be registered");
        }
        #[cfg(not(feature = "protocol-http"))]
        {
            assert!(!type_ids.contains(&"http"), "http should NOT be registered");
        }

        #[cfg(feature = "protocol-ws")]
        {
            assert!(type_ids.contains(&"ws_connect"));
        }
        #[cfg(not(feature = "protocol-ws"))]
        {
            assert!(!type_ids.contains(&"ws_connect"));
        }

        #[cfg(feature = "protocol-db")]
        {
            assert!(type_ids.contains(&"db_connect"));
        }
        #[cfg(not(feature = "protocol-db"))]
        {
            assert!(!type_ids.contains(&"db_connect"));
        }

        #[cfg(feature = "protocol-script")]
        {
            assert!(type_ids.contains(&"script"));
        }
        #[cfg(not(feature = "protocol-script"))]
        {
            assert!(!type_ids.contains(&"script"));
        }

        #[cfg(feature = "protocol-grpc")]
        {
            assert!(type_ids.contains(&"grpc_connect"));
        }
        #[cfg(not(feature = "protocol-grpc"))]
        {
            assert!(!type_ids.contains(&"grpc_connect"));
        }

        #[cfg(feature = "protocol-kafka")]
        {
            assert!(type_ids.contains(&"kafka_connect"));
        }
        #[cfg(not(feature = "protocol-kafka"))]
        {
            assert!(!type_ids.contains(&"kafka_connect"));
        }

        #[cfg(feature = "protocol-amqp")]
        {
            assert!(type_ids.contains(&"amqp_connect"));
        }
        #[cfg(not(feature = "protocol-amqp"))]
        {
            assert!(!type_ids.contains(&"amqp_connect"));
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
