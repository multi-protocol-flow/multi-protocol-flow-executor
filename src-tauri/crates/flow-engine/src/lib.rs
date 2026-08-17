//! Flow Engine - 流程执行引擎核心库
//!
//! 独立于 Tauri 的业务逻辑层，可在 CLI 和测试环境中直接使用。

pub use flow_engine_executor::commands;

// Re-exports from flow-engine-core
pub use flow_engine_core::connection;
pub use flow_engine_core::dataset;
pub use flow_engine_core::error;
pub use flow_engine_core::execution_event;
pub use flow_engine_core::flow;
pub use flow_engine_core::log_event;
pub use flow_engine_core::node;
pub use flow_engine_core::node_config;
pub use flow_engine_core::node_types;
pub use flow_engine_core::plugin;
pub use flow_engine_core::registry;
pub use flow_engine_core::report;
pub use flow_engine_core::statistics;
pub use flow_engine_core::traits;
pub use flow_engine_core::utils;
pub use flow_engine_core::variable;
pub use flow_engine_core::ws_stream;

// Wave 2: 无依赖的基础模块
pub use flow_engine_executor::command;
pub use flow_engine_executor::storage;

// Wave 3: 执行器模块（协议 crate 通过 executor 条件转发）
pub use flow_engine_core::context;
pub use flow_engine_executor::conditional_executor;
pub use flow_engine_executor::debug_event_adapter;
pub use flow_engine_executor::entry_executor;
pub use flow_engine_executor::executor;
pub use flow_engine_executor::validator;

// HTTP (flow-engine-http)
#[cfg(feature = "protocol-http")]
pub use flow_engine_executor::flow_engine_http;
#[cfg(feature = "protocol-http")]
pub use flow_engine_executor::flow_engine_http::{http, http_executor};

// WebSocket (flow-engine-ws)
#[cfg(feature = "protocol-ws")]
pub use flow_engine_executor::flow_engine_ws;
#[cfg(feature = "protocol-ws")]
pub use flow_engine_executor::flow_engine_ws::{
    websocket_manager, ws_close_executor, ws_connect_executor, ws_send_collect_executor,
};

// gRPC (flow-engine-grpc)
#[cfg(feature = "protocol-grpc")]
pub use flow_engine_executor::flow_engine_grpc;
#[cfg(feature = "protocol-grpc")]
pub use flow_engine_executor::flow_engine_grpc::{
    grpc_call_executor, grpc_close_executor, grpc_connect_executor, grpc_connection_manager,
    proto_parser, reflection,
};

// TCP/UDP (flow-engine-tcp)
#[cfg(feature = "protocol-tcp")]
pub use flow_engine_executor::flow_engine_tcp;
#[cfg(feature = "protocol-tcp")]
pub use flow_engine_executor::flow_engine_tcp::{
    parser, recording, tcp_close_executor, tcp_connect_executor, tcp_connection_manager,
    tcp_send_executor, templates, udp_bind_executor, udp_close_executor, udp_recv_from_executor,
    udp_send_to_executor, udp_socket_manager,
};


// SSE (flow-engine-sse)
#[cfg(feature = "protocol-sse")]
pub use flow_engine_executor::flow_engine_sse;
#[cfg(feature = "protocol-sse")]
pub use flow_engine_executor::flow_engine_sse::{
    sse_connect_executor, sse_connection_manager, sse_disconnect_executor, sse_listen_executor,
};

// GraphQL (flow-engine-graphql)
#[cfg(feature = "protocol-graphql")]
pub use flow_engine_executor::flow_engine_graphql;
#[cfg(feature = "protocol-graphql")]
pub use flow_engine_executor::flow_engine_graphql::{
    graphql_connect_executor, graphql_connection_manager, graphql_disconnect_executor,
    graphql_introspect_executor, graphql_query_executor, graphql_subscribe_executor,
};

// Script (flow-engine-script)
#[cfg(feature = "protocol-script")]
pub use flow_engine_executor::flow_engine_script;
#[cfg(feature = "protocol-script")]
pub use flow_engine_executor::flow_engine_script::{script_api, script_executor};

// Assertion (flow-engine-assertion)
#[cfg(feature = "protocol-assertion")]
pub use flow_engine_executor::flow_engine_assertion;
#[cfg(feature = "protocol-assertion")]
pub use flow_engine_executor::flow_engine_assertion::{assertion, assertion_executor};

// Wave 4: 调试模块
pub use flow_engine_executor::debug;

// Wave 7: 压力测试模块
pub use flow_engine_executor::stress_test;

// Wave 8: 节点注册表
pub use flow_engine_executor::default_registry;

// 插件系统（sidecar 插件进程；`plugin-manager` 特性门控，与 `protocol-*` 无关）
#[cfg(feature = "plugin-manager")]
pub use flow_engine_executor::default_registry::default_registry_with_plugins;
#[cfg(feature = "plugin-manager")]
pub use flow_engine_plugin::manager::PluginManager;
#[cfg(feature = "plugin-manager")]
pub use flow_engine_plugin::sidecar::SidecarPlugin;
// 插件目录扫描（已安装列表；GUI 与 CLI 共用）
#[cfg(feature = "plugin-manager")]
pub use flow_engine_plugin::discovery::{PluginScan, ScanEntry};
// 插件市场客户端 + 安装器（GUI 市场 Tab 与 `mpe plugin install` 共用）
#[cfg(feature = "plugin-manager")]
pub use flow_engine_plugin_marketplace;

// 数据集执行模块
pub use flow_engine_executor::csv_parser;
pub use flow_engine_executor::dataset_executor;

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

// ============================================================================
// 测试工具函数
// ============================================================================

#[cfg(test)]
/// 测试工具模块：提供 WebSocket 节点创建辅助函数。
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
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            on_error: None,
        }
    }

    /// 创建带 WebSocket 管理器的执行上下文
    ///
    /// # 参数
    /// - `flow_id`: 流程 ID
    ///
    /// # 返回
    /// 配置好的执行上下文（不包含 WebSocket 管理器，因为 `WebSocketConnectionManager` 在 Task 4）
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
