//! AMQP 1.0 协议子模块
//!
//! 基于 fe2o3-amqp 实现 AMQP 1.0 客户端和连接管理器。
//! AMQP 1.0 使用 Connection → Session → Link (Sender/Receiver) 模型，
//! 适用于 Azure Service Bus、ActiveMQ Artemis 等 AMQP 1.0 Broker。

pub mod amqp10_accept_executor;
pub mod amqp10_client;
pub mod amqp10_connect_executor;
pub mod amqp10_connection_manager;
pub mod amqp10_disconnect_executor;
pub mod amqp10_receive_executor;
pub mod amqp10_reject_executor;
pub mod amqp10_send_executor;

pub use amqp10_connection_manager::Amqp10ConnectionManager;
