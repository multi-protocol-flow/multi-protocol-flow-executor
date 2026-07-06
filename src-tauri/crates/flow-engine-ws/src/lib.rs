//! Flow Engine WebSocket sub-crate
//!
//! WebSocket 连接管理器和执行器模块。

/// WebSocket 节点插件（WsConnectPlugin, `WsSendCollectPlugin`, `WsClosePlugin`）
pub mod plugin;
/// WebSocket 连接池管理器，管理流程级别的连接生命周期
pub mod websocket_manager;
/// WebSocket 关闭节点执行器，关闭指定连接
pub mod ws_close_executor;
/// WebSocket 连接节点执行器，负责建立新连接
pub mod ws_connect_executor;
/// WebSocket 发送采集节点执行器，发送消息并采集响应
pub mod ws_send_collect_executor;
