//! Flow Engine - 流程执行引擎核心库
//!
//! 独立于 Tauri 的业务逻辑层，可在 CLI 和测试环境中直接使用。

pub use flow_engine_executor::commands;

// Re-exports from flow-engine-core
pub use flow_engine_core::dataset;
pub use flow_engine_core::email_types;
pub use flow_engine_core::email_auth;
pub use flow_engine_core::connection;
pub use flow_engine_core::plugin;
pub use flow_engine_core::registry;
pub use flow_engine_core::error;
pub use flow_engine_core::execution_event;
pub use flow_engine_core::flow;
pub use flow_engine_core::log_event;
pub use flow_engine_core::node;
pub use flow_engine_core::node_config;
pub use flow_engine_core::node_types;
pub use flow_engine_core::report;
pub use flow_engine_core::statistics;
pub use flow_engine_core::traits;
pub use flow_engine_core::utils;
pub use flow_engine_core::variable;
pub use flow_engine_core::ws_stream;

// Wave 2: 无依赖的基础模块
pub use flow_engine_crypto::encryption;
pub use flow_engine_executor::command;
pub use flow_engine_executor::storage;
pub use flow_engine_ws::websocket_manager;

// Wave 3: 执行器模块（需要修改 Tauri 依赖）
pub use flow_engine_assertion::assertion_executor;
pub use flow_engine_core::context;
pub use flow_engine_db::db;
pub use flow_engine_db::db_begin_executor;
pub use flow_engine_db::db_close_executor;
pub use flow_engine_db::db_commit_executor;
pub use flow_engine_db::db_connect_executor;
pub use flow_engine_db::db_query_executor;
pub use flow_engine_db::db_rollback_executor;
pub use flow_engine_executor::conditional_executor;
pub use flow_engine_executor::debug_event_adapter;
pub use flow_engine_executor::entry_executor;
pub use flow_engine_executor::executor;
pub use flow_engine_executor::validator;
pub use flow_engine_http::http;
pub use flow_engine_http::http_executor;
pub use flow_engine_ws::ws_close_executor;
pub use flow_engine_ws::ws_connect_executor;
pub use flow_engine_ws::ws_send_collect_executor;
pub use flow_engine_grpc::grpc_close_executor;
pub use flow_engine_grpc::grpc_connect_executor;
pub use flow_engine_grpc::grpc_connection_manager;
pub use flow_engine_grpc::grpc_call_executor;
pub use flow_engine_grpc::proto_parser;
pub use flow_engine_grpc::reflection;
// TCP (flow-engine-tcp)
pub use flow_engine_tcp::tcp_close_executor;
pub use flow_engine_tcp::tcp_connect_executor;
pub use flow_engine_tcp::tcp_connection_manager;
pub use flow_engine_tcp::tcp_send_executor;
pub use flow_engine_tcp::templates;
pub use flow_engine_tcp::parser;
pub use flow_engine_tcp::recording;
// UDP (flow-engine-tcp)
pub use flow_engine_tcp::udp_bind_executor;
pub use flow_engine_tcp::udp_close_executor;
pub use flow_engine_tcp::udp_recv_from_executor;
pub use flow_engine_tcp::udp_send_to_executor;
pub use flow_engine_tcp::udp_socket_manager;
// MCP (flow-engine-mcp)
pub use flow_engine_mcp;
// MQTT (flow-engine-mqtt)
pub use flow_engine_mqtt::mqtt_connect_executor;
pub use flow_engine_mqtt::mqtt_publish_executor;
pub use flow_engine_mqtt::mqtt_subscribe_executor;
pub use flow_engine_mqtt::mqtt_unsubscribe_executor;
pub use flow_engine_mqtt::mqtt_disconnect_executor;
pub use flow_engine_mqtt::mqtt_wait_message_executor;
pub use flow_engine_mqtt::mqtt_request_executor;
pub use flow_engine_mqtt::mqtt_connection_manager;
// Kafka (flow-engine-kafka)
pub use flow_engine_kafka::kafka_connect_executor;
pub use flow_engine_kafka::kafka_produce_executor;
pub use flow_engine_kafka::kafka_consume_executor;
pub use flow_engine_kafka::kafka_consume_batch_executor;
pub use flow_engine_kafka::kafka_admin_executor;
pub use flow_engine_kafka::kafka_schema_registry_executor;
pub use flow_engine_kafka::kafka_close_executor;
pub use flow_engine_kafka::kafka_connection_manager;
// SSE (flow-engine-sse)
pub use flow_engine_sse::sse_connect_executor;
pub use flow_engine_sse::sse_listen_executor;
pub use flow_engine_sse::sse_disconnect_executor;
pub use flow_engine_sse::sse_connection_manager;
// GraphQL (flow-engine-graphql)
pub use flow_engine_graphql::graphql_connect_executor;
pub use flow_engine_graphql::graphql_query_executor;
pub use flow_engine_graphql::graphql_subscribe_executor;
pub use flow_engine_graphql::graphql_unsubscribe_executor;
pub use flow_engine_graphql::graphql_introspect_executor;
pub use flow_engine_graphql::graphql_disconnect_executor;
pub use flow_engine_graphql::graphql_connection_manager;
// AMQP (flow-engine-amqp)
pub use flow_engine_amqp::amqp091::amqp_connect_executor;
pub use flow_engine_amqp::amqp091::amqp_close_executor;
pub use flow_engine_amqp::amqp091::amqp_exchange_declare_executor;
pub use flow_engine_amqp::amqp091::amqp_exchange_delete_executor;
pub use flow_engine_amqp::amqp091::amqp_queue_declare_executor;
pub use flow_engine_amqp::amqp091::amqp_queue_delete_executor;
pub use flow_engine_amqp::amqp091::amqp_queue_bind_executor;
pub use flow_engine_amqp::amqp091::amqp_publish_executor;
pub use flow_engine_amqp::amqp091::amqp_consume_executor;
pub use flow_engine_amqp::amqp091::amqp_get_executor;
pub use flow_engine_amqp::amqp091::amqp_ack_executor;
pub use flow_engine_amqp::AmqpConnectionManager;
// SMTP (flow-engine-smtp)
pub use flow_engine_smtp::smtp_connection_manager;
pub use flow_engine_smtp::smtp_connect_executor;
pub use flow_engine_smtp::smtp_send_executor;
pub use flow_engine_smtp::smtp_disconnect_executor;
pub use flow_engine_smtp::plugin as smtp_plugin;
// IMAP (flow-engine-imap)
pub use flow_engine_imap::imap_connection_manager;
pub use flow_engine_imap::imap_connect_executor;
pub use flow_engine_imap::imap_select_executor;
pub use flow_engine_imap::imap_search_executor;
pub use flow_engine_imap::imap_fetch_executor;
pub use flow_engine_imap::imap_store_flags_executor;
pub use flow_engine_imap::imap_copy_executor;
pub use flow_engine_imap::imap_move_executor;
pub use flow_engine_imap::imap_expunge_executor;
pub use flow_engine_imap::imap_list_executor;
pub use flow_engine_imap::imap_folder_create_executor;
pub use flow_engine_imap::imap_folder_delete_executor;
pub use flow_engine_imap::imap_idle_executor;
pub use flow_engine_imap::imap_close_executor;
pub use flow_engine_imap::plugin as imap_plugin;
// Wave 4: 调试模块
pub use flow_engine_executor::debug;
// Wave 5: 断言模块
pub use flow_engine_assertion::assertion;

// Wave 6: 脚本节点 API 模块
pub use flow_engine_script::script_api;
pub use flow_engine_script::script_executor;

// Wave 7: 压力测试模块
pub use flow_engine_executor::stress_test;

// Wave 8: 节点注册表
pub use flow_engine_executor::default_registry;

// 数据集执行模块
pub use flow_engine_executor::dataset_executor;
pub use flow_engine_executor::csv_parser;

// 钩子执行器模块
pub use flow_engine_core::hook;
pub use flow_engine_executor::hook_runner;

// Re-exports
pub use traits::{
    AppDirProvider, ConsoleLogSender, DefaultAppDirProvider, DirectoryOpener, ExecutionEventSender,
    LogSender, LogWorker, NoopDirectoryOpener, NoopLogSender, QueuedLogSender, UnderlyingLogSender,
};

// Re-export execution event types
pub use execution_event::ExecutionEvent;

// Re-export ws_stream types
pub use ws_stream::{
    MessageContent, MessageDirection, WsConnectionStream, WsMessage, WsStreamSegment,
};

// Re-export db types
pub use db::{
    DatabaseConnectionManager, DatabasePool, DbConnection, DbConnectionStatus, DbDriver,
    IsolationLevel, MockPool, PoolMetadata, QueryResult, TestDbConnectionResult, TransactionInfo,
    TransactionState,
};

// ============================================================================
// 测试工具函数
// ============================================================================

#[cfg(test)]
pub mod test_utils {
    use crate::context::ExecutionContext;
    use crate::node::{Node, NodePosition};
    use crate::node_config::{
        WsCloseConfig, WsCollectMode, WsConnectConfig, WsMessageType, WsSendCollectConfig,
    };

    /// 创建 WebSocket Connect 节点
    ///
    /// # 参数
    /// - `uuid`: 节点唯一标识符
    /// - `name`: 节点名称
    /// - `url`: WebSocket 连接 URL
    ///
    /// # 返回
    /// 配置好的 WebSocket Connect 节点
    pub fn create_ws_connect_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: crate::node_types::WS_CONNECT.to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsConnect(WsConnectConfig {
                url: url.to_string(),
                headers: vec![],
                subprotocols: vec![],
                connect_timeout_ms: 30000,
                collect_message: false,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 10000,
                use_tls: false,
                tls_skip_verify: false,
                ping_interval_ms: None,
                ping_timeout_ms: None,
                output_mappings: vec![],
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建 WebSocket Send+Collect 节点
    ///
    /// # 参数
    /// - `uuid`: 节点唯一标识符
    /// - `connection_id`: WebSocket 连接 ID
    /// - `message`: 要发送的消息内容
    ///
    /// # 返回
    /// 配置好的 WebSocket Send+Collect 节点
    pub fn create_ws_send_collect_node(uuid: &str, connection_id: &str, message: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: crate::node_types::WS_SEND_COLLECT.to_string(),
            name: format!("WS Send+Collect {}", uuid),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: connection_id.to_string(),
                message: message.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                collect_response: true,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 10000,
                output_mappings: vec![],
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建 WebSocket Close 节点
    ///
    /// # 参数
    /// - `uuid`: 节点唯一标识符
    /// - `connection_id`: WebSocket 连接 ID
    ///
    /// # 返回
    /// 配置好的 WebSocket Close 节点
    pub fn create_ws_close_node(uuid: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: crate::node_types::WS_CLOSE.to_string(),
            name: format!("WS Close {}", uuid),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsClose(WsCloseConfig {
                connection_id: connection_id.to_string(),
                close_code: 1000,
                close_reason: None,
                close_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建带 WebSocket 管理器的执行上下文
    ///
    /// # 参数
    /// - `flow_id`: 流程 ID
    ///
    /// # 返回
    /// 配置好的执行上下文（不包含 WebSocket 管理器，因为 WebSocketConnectionManager 在 Task 4）
    pub fn create_context_with_ws_manager(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(flow_id.to_string(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;

    #[test]
    fn test_create_ws_connect_node() {
        let node = create_ws_connect_node("ws-001", "Test WS Connect", "wss://example.com/ws");

        assert_eq!(node.uuid, "ws-001");
        assert_eq!(node.name, "Test WS Connect");
        assert_eq!(node.node_type, crate::node_types::WS_CONNECT);
    }

    #[test]
    fn test_create_ws_send_collect_node() {
        let node = create_ws_send_collect_node("ws-002", "conn-001", "Hello, WebSocket!");

        assert_eq!(node.uuid, "ws-002");
        assert_eq!(node.node_type, crate::node_types::WS_SEND_COLLECT);
        assert!(node.name.contains("ws-002"));
    }

    #[test]
    fn test_create_ws_close_node() {
        let node = create_ws_close_node("ws-003", "conn-001");

        assert_eq!(node.uuid, "ws-003");
        assert_eq!(node.node_type, crate::node_types::WS_CLOSE);
        assert!(node.name.contains("ws-003"));
    }

    #[test]
    fn test_create_context_with_ws_manager() {
        let context = create_context_with_ws_manager("test-flow-001");

        assert_eq!(context.flow_uuid, "test-flow-001");
        assert!(context.current_node_uuid.is_none());
        assert!(context.node_reports.is_empty());
    }
}
