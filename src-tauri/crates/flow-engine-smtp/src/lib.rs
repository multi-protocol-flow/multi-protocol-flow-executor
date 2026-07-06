//! SMTP 协议支持模块
//!
//! 提供 SMTP 邮件发送、连接管理、OAuth2 认证等功能。

/// `OAuth2` Token 流程（PKCE、Device Flow、Token 刷新、加密存储）
pub mod oauth2;
/// SMTP 节点插件注册
pub mod plugin;
/// SMTP 连接节点执行器
pub mod smtp_connect_executor;
/// SMTP 连接池管理器
pub mod smtp_connection_manager;
/// SMTP 断开连接节点执行器
pub mod smtp_disconnect_executor;
/// SMTP 发送邮件节点执行器
pub mod smtp_send_executor;

pub use plugin::SmtpPluginGroup;
pub use smtp_connection_manager::SmtpConnectionManager;
