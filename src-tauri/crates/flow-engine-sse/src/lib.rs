//! SSE 协议支持模块
//!
//! 提供 SSE (Server-Sent Events) 连接管理、事件监听、断开连接等功能。

/// SSE 节点插件注册（NodePlugin 实现）
pub mod plugin;
/// SSE 客户端封装，处理 Server-Sent Events 流解析
pub mod sse_client;
/// SSE 连接管理器，管理流程级别的连接生命周期
pub mod sse_connection_manager;
/// SSE 连接节点执行器，建立与服务端的 SSE 连接
pub mod sse_connect_executor;
/// SSE 断开连接节点执行器，关闭指定连接
pub mod sse_disconnect_executor;
/// SSE 监听节点执行器，监听并采集服务端推送的事件
pub mod sse_listen_executor;

// SsePluginGroup 将在后续任务中实现

pub use sse_connection_manager::{SseConnection, SseConnectionManager};
