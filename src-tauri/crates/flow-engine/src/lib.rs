//! Flow Engine - 流程执行引擎核心库
//!
//! 独立于 Tauri 的业务逻辑层，可在 CLI 和测试环境中直接使用。

pub mod commands;
pub mod traits;

// Wave 2: 无依赖的基础模块
pub mod command;
pub mod connection;
pub mod error;
pub mod flow;
pub mod log_event;
pub mod node;
pub mod node_config;
pub mod report;
pub mod statistics;
pub mod storage;
pub mod variable;
pub mod websocket_manager;

// Wave 3: 执行器模块（需要修改 Tauri 依赖）
pub mod context;
pub mod entry_executor;
pub mod executor;
pub mod http_executor;
pub mod conditional_executor;
pub mod ws_connect_executor;
pub mod validator;

// Re-exports
pub use traits::{
    AppDirProvider, ConsoleLogSender, DefaultAppDirProvider, DirectoryOpener, LogSender,
    NoopDirectoryOpener, NoopLogSender,
};

// ============================================================================
// 测试工具函数
// ============================================================================

#[cfg(test)]
pub mod test_utils {
    use crate::context::ExecutionContext;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::{
        WsCloseConfig, WsConnectConfig, WsSendWaitConfig, WsMessageType, WsWaitMode,
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
            node_type: NodeType::WsConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsConnect(WsConnectConfig {
                url: url.to_string(),
                headers: vec![],
                subprotocols: vec![],
                connect_timeout_ms: 30000,
                wait_for_message: false,
                wait_mode: WsWaitMode::None,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![],
            }),
            ports: None,
        }
    }

    /// 创建 WebSocket Send+Wait 节点
    ///
    /// # 参数
    /// - `uuid`: 节点唯一标识符
    /// - `connection_id`: WebSocket 连接 ID
    /// - `message`: 要发送的消息内容
    ///
    /// # 返回
    /// 配置好的 WebSocket Send+Wait 节点
    pub fn create_ws_send_wait_node(uuid: &str, connection_id: &str, message: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::WsSendWait,
            name: format!("WS Send+Wait {}", uuid),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: connection_id.to_string(),
                message: message.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: true,
                wait_mode: WsWaitMode::FirstMessage,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![],
            }),
            ports: None,
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
            node_type: NodeType::WsClose,
            name: format!("WS Close {}", uuid),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::WsClose(WsCloseConfig {
                connection_id: connection_id.to_string(),
                close_code: 1000,
                close_reason: None,
                wait_close_timeout_ms: 5000,
            }),
            ports: None,
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
        assert_eq!(node.node_type, crate::node::NodeType::WsConnect);
    }

    #[test]
    fn test_create_ws_send_wait_node() {
        let node = create_ws_send_wait_node("ws-002", "conn-001", "Hello, WebSocket!");
        
        assert_eq!(node.uuid, "ws-002");
        assert_eq!(node.node_type, crate::node::NodeType::WsSendWait);
        assert!(node.name.contains("ws-002"));
    }

    #[test]
    fn test_create_ws_close_node() {
        let node = create_ws_close_node("ws-003", "conn-001");
        
        assert_eq!(node.uuid, "ws-003");
        assert_eq!(node.node_type, crate::node::NodeType::WsClose);
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
