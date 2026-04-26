//! Kafka 协议支持模块
//!
//! 提供 Kafka 连接管理、生产/消费、批量消费、管理操作、Schema Registry 等功能。

/// Kafka 客户端封装
pub mod kafka_client;
/// Kafka 连接管理器
pub mod kafka_connection_manager;
/// Schema Registry HTTP 客户端
pub mod schema_registry_client;
/// Kafka Connect 执行器
pub mod kafka_connect_executor;
/// Kafka Produce 执行器
pub mod kafka_produce_executor;
/// Kafka Consume 执行器
pub mod kafka_consume_executor;
/// Kafka ConsumeBatch 执行器
pub mod kafka_consume_batch_executor;
/// Kafka Admin 执行器
pub mod kafka_admin_executor;
/// Kafka Schema Registry 执行器
pub mod kafka_schema_registry_executor;
/// Kafka Close 执行器
pub mod kafka_close_executor;
/// Kafka 节点插件注册
pub mod plugin;

pub use kafka_connection_manager::KafkaConnectionManager;
pub use plugin::KafkaPluginGroup;
