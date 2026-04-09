//! Flow Engine WebSocket sub-crate
//!
//! WebSocket 连接管理器和执行器模块。

/// WebSocket 连接池管理器，管理流程级别的连接生命周期
pub mod websocket_manager;
/// WebSocket 连接节点执行器，负责建立新连接
pub mod ws_connect_executor;
/// WebSocket 发送等待节点执行器，发送消息并等待响应
pub mod ws_send_wait_executor;
/// WebSocket 关闭节点执行器，关闭指定连接
pub mod ws_close_executor;
