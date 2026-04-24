//! MCP 协议支持 crate
//!
//! 提供 MCP (Model Context Protocol) 客户端连接管理功能。
//! 封装 rmcp SDK，实现 [`McpProvider`] trait。
//!
//! 所有 rmcp 类型保持内部可见，公共 API 仅暴露 core 类型。

pub mod mcp_client;
pub mod mcp_connection_manager;
/// MCP 连接节点执行器
pub mod mcp_connect_executor;
/// MCP 调用工具节点执行器
pub mod mcp_call_tool_executor;
/// MCP 读取资源节点执行器
pub mod mcp_read_resource_executor;
/// MCP 关闭节点执行器
pub mod mcp_close_executor;
/// MCP 节点插件（NodePlugin 实现 + [`McpPluginGroup`]）
pub mod plugin;

pub use mcp_connection_manager::{McpConnection, McpConnectionManager};
pub use plugin::{McpPluginGroup, McpConnectPlugin, McpCallToolPlugin, McpReadResourcePlugin, McpClosePlugin};
