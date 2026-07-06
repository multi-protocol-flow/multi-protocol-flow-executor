//! MQTT 协议支持模块
//!
//! 提供 MQTT 5.0 连接管理、发布/订阅、请求/响应等功能。

/// MQTT 客户端封装，支持 TCP/TLS/WebSocket 传输
pub mod mqtt_client;
/// MQTT 连接节点执行器，建立与 Broker 的连接
pub mod mqtt_connect_executor;
/// MQTT 连接池管理器，管理流程级别的连接生命周期
pub mod mqtt_connection_manager;
/// MQTT 断开连接节点执行器，关闭指定连接
pub mod mqtt_disconnect_executor;
/// MQTT 发布节点执行器，发布消息到指定主题
pub mod mqtt_publish_executor;
/// MQTT 请求/响应节点执行器，实现发布-订阅-等待模式
pub mod mqtt_request_executor;
/// MQTT 订阅节点执行器，订阅主题并可选收集消息
pub mod mqtt_subscribe_executor;
/// MQTT 取消订阅节点执行器，取消订阅指定主题
pub mod mqtt_unsubscribe_executor;
/// MQTT 等待消息节点执行器，等待匹配主题的消息
pub mod mqtt_wait_message_executor;
/// MQTT 节点插件注册（NodePlugin 实现 + [`MqttPluginGroup`]）
pub mod plugin;

pub use mqtt_connection_manager::{MqttConnection, MqttConnectionManager};
pub use plugin::MqttPluginGroup;
