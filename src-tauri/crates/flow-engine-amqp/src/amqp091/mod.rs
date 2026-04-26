//! AMQP 0-9-1 协议子模块
//!
//! 基于 lapin 实现 AMQP 0-9-1 客户端和连接管理器。

pub mod amqp_ack_executor;
pub mod amqp_client;
pub mod amqp_close_executor;
pub mod amqp_connect_executor;
pub mod amqp_connection_manager;
pub mod amqp_consume_executor;
pub mod amqp_exchange_declare_executor;
pub mod amqp_exchange_delete_executor;
pub mod amqp_get_executor;
#[cfg(feature = "management")]
pub mod amqp_management_executor;
pub mod amqp_publish_executor;
pub mod amqp_queue_bind_executor;
pub mod amqp_queue_declare_executor;
pub mod amqp_queue_delete_executor;
/// AMQP 0-9-1 协议插件注册表
pub mod plugin;
