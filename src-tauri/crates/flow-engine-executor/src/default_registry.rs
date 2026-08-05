//! 默认节点注册表 — 注册全部内置节点插件
//!
//! 此模块位于 `flow-engine-executor` crate 中，因为它需要依赖所有协议 crate
//! （http、ws、db、script、assertion、grpc、mcp、tcp、mqtt、kafka、sse），而 `flow-engine-core` 作为叶子 crate
//! 不能依赖它们。

use flow_engine_core::registry::NodeRegistry;

#[cfg(feature = "plugin-manager")]
use flow_engine_plugin::manager::PluginManager;

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

/// 构建包含全部内置节点和已注册插件节点的注册表
///
/// 先运行 [`default_registry`] 注册全部内置节点，再将 `manager` 中已登记
/// （`add_registration` / `scan_and_describe`）的插件节点包装为
/// [`flow_engine_plugin::sidecar::SidecarPlugin`] 并注册。
///
/// 冲突策略与 `flow-engine-plugin` 的 `PluginManager::register_nodes_with`
/// 保持一致：内置 `type_id` 永不覆盖；插件之间先到先得。所有
/// `SidecarPlugin` 共享同一个 `manager` Arc，因此插件进程在多次执行、
/// 数据驱动与压测场景之间复用。
///
/// # 参数
/// - `manager`: 已完成插件登记（`add_registration` / `scan_and_describe`）的管理器
///
/// # 返回
/// 注册了全部内置节点和插件节点的 [`NodeRegistry`]
#[cfg(feature = "plugin-manager")]
pub fn default_registry_with_plugins(manager: &std::sync::Arc<PluginManager>) -> NodeRegistry {
    use flow_engine_plugin::reserved::is_reserved_type_id;
    use flow_engine_plugin::sidecar::SidecarPlugin;

    let mut registry = default_registry();
    // 本地 claimed 集合复刻 `register_nodes_with` 的单次调用语义（先到先得），
    // 因为 `register_nodes_with` 需要 `&mut manager`，而 `SidecarPlugin` 持有
    // `Arc<PluginManager>` —— 两者无法在同一借用中同时获得。
    let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for registration in manager.registrations() {
        for node in &registration.nodes {
            let reserved = is_reserved_type_id(&node.type_id)
                || (registry.get(&node.type_id).is_some() && !claimed.contains(&node.type_id));
            if reserved || claimed.contains(&node.type_id) {
                log::warn!(
                    "plugin {} node {} rejected by collision policy (reserved or first-wins)",
                    registration.name,
                    node.type_id
                );
                continue;
            }
            registry.register(SidecarPlugin::new(node.clone(), std::sync::Arc::clone(manager)));
            claimed.insert(node.type_id.clone());
        }
    }
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
