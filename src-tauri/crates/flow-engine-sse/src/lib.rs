//! SSE 协议支持模块
//!
//! 提供 SSE (Server-Sent Events) 连接管理、事件监听、断开连接等功能。

pub mod plugin;
pub mod sse_client;
pub mod sse_connection_manager;
pub mod sse_connect_executor;
pub mod sse_disconnect_executor;
pub mod sse_listen_executor;

// SsePluginGroup 将在后续任务中实现

pub use sse_connection_manager::{SseConnection, SseConnectionManager};
