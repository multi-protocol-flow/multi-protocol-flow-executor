//! AMQP 协议支持模块
//!
//! 提供 AMQP 0-9-1 和 AMQP 1.0 连接管理、发布/消费等功能。
//! - AMQP 0-9-1: 基于 lapin，适用于 `RabbitMQ` 等传统 Broker
//! - AMQP 1.0: 基于 fe2o3-amqp，适用于 Azure Service Bus、ActiveMQ Artemis 等

pub mod amqp091;
pub mod amqp10;

pub use amqp091::amqp_connection_manager::AmqpConnectionManager;
pub use amqp091::plugin::AmqpPluginGroup;
pub use amqp10::Amqp10ConnectionManager;
