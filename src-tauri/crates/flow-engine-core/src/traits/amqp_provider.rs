// AMQP Provider trait - AMQP 连接管理抽象接口（覆盖 AMQP 0-9-1 和 AMQP 1.0）

use crate::node_config::{
    AmqpConnectConfig, AmqpCloseConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig,
    AmqpQueueDeclareConfig, AmqpQueueDeleteConfig, AmqpQueueBindConfig, AmqpPublishConfig,
    AmqpConsumeConfig, AmqpGetConfig, AmqpAckConfig, AmqpManagementConfig, Amqp10ConnectConfig,
    Amqp10SendConfig, Amqp10ReceiveConfig, Amqp10AcceptConfig, Amqp10RejectConfig,
    Amqp10DisconnectConfig,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================================
// AMQP 0-9-1 输出类型
// ============================================================================

/// AMQP 连接输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    pub connection_id: String,
    /// 服务器属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_properties: Option<serde_json::Value>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 交换器操作输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpExchangeOutput {
    /// 是否Success
    pub success: bool,
    /// 交换器名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    /// 交换器类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_type: Option<String>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 声明队列输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpQueueDeclareOutput {
    /// 是否Success
    pub success: bool,
    /// 队列名称（自动生成时有用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_name: Option<String>,
    /// 队列中的消息数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u32>,
    /// 队列的消费者数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer_count: Option<u32>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 删除队列输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpQueueDeleteOutput {
    /// 是否Success
    pub success: bool,
    /// 被删除的消息数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_deleted: Option<u32>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 发布输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpPublishOutput {
    /// 是否Success
    pub success: bool,
    /// 发布确认结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_result: Option<String>,
    /// 被退回的消息（mandatory 模式下）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returned_message: Option<serde_json::Value>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpMessage {
    /// 投递标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_tag: Option<u64>,
    /// 交换器名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    /// 路由键
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_key: Option<String>,
    /// 是否为重传消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redelivered: Option<bool>,
    /// 消息属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
    /// 消息体
    pub body: String,
    /// 解码后的消息体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_decoded: Option<serde_json::Value>,
}

/// AMQP 消费输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpConsumeOutput {
    /// 是否Success
    pub success: bool,
    /// 接收到的消息列表
    pub messages: Vec<AmqpMessage>,
    /// 消息数量
    pub message_count: u32,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP Get 输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpGetOutput {
    /// 是否Success
    pub success: bool,
    /// 获取到的消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AmqpMessage>,
    /// 队列剩余消息数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count_remaining: Option<u32>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 确认输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpAckOutput {
    /// 是否Success
    pub success: bool,
    /// 操作类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// 投递标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_tag: Option<u64>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 管理输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmqpManagementOutput {
    /// 是否Success
    pub success: bool,
    /// 管理操作返回数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// AMQP 1.0 输出类型
// ============================================================================

/// AMQP 1.0 连接输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10ConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    pub connection_id: String,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 1.0 发送输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10SendOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 1.0 消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10Message {
    /// 消息属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
    /// 消息体
    pub body: String,
    /// 解码后的消息体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_decoded: Option<serde_json::Value>,
    /// 消息注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_annotations: Option<serde_json::Value>,
    /// 投递 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<u64>,
}

/// AMQP 1.0 接收输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10ReceiveOutput {
    /// 是否Success
    pub success: bool,
    /// 接收到的消息列表
    pub messages: Vec<Amqp10Message>,
    /// 消息数量
    pub message_count: u32,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 1.0 确认输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10AcceptOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// AMQP 1.0 拒绝输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Amqp10RejectOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// 通用类型
// ============================================================================

/// AMQP 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// Broker URL
    pub url: String,
    /// 连接状态
    pub status: String,
}

/// AMQP 连接管理器 trait
///
/// 抽象 AMQP 连接的生命周期管理，覆盖 AMQP 0-9-1 和 AMQP 1.0 协议。
/// 使得业务逻辑可以在无具体实现依赖的环境下运行。
#[async_trait]
pub trait AmqpProvider: Send + Sync {
    // ===== AMQP 0-9-1 操作 =====

    /// 建立 AMQP 0-9-1 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &AmqpConnectConfig,
    ) -> Result<AmqpConnectOutput, String>;

    /// 关闭 AMQP 0-9-1 连接
    async fn close(&self, uuid: &str, config: &AmqpCloseConfig) -> Result<(), String>;

    /// 声明交换器
    async fn exchange_declare(
        &self,
        uuid: &str,
        config: &AmqpExchangeDeclareConfig,
    ) -> Result<AmqpExchangeOutput, String>;

    /// 删除交换器
    async fn exchange_delete(
        &self,
        uuid: &str,
        config: &AmqpExchangeDeleteConfig,
    ) -> Result<AmqpExchangeOutput, String>;

    /// 声明队列
    async fn queue_declare(
        &self,
        uuid: &str,
        config: &AmqpQueueDeclareConfig,
    ) -> Result<AmqpQueueDeclareOutput, String>;

    /// 删除队列
    async fn queue_delete(
        &self,
        uuid: &str,
        config: &AmqpQueueDeleteConfig,
    ) -> Result<AmqpQueueDeleteOutput, String>;

    /// 绑定队列到交换器
    async fn queue_bind(&self, uuid: &str, config: &AmqpQueueBindConfig) -> Result<(), String>;

    /// 发布消息
    async fn publish(
        &self,
        uuid: &str,
        config: &AmqpPublishConfig,
    ) -> Result<AmqpPublishOutput, String>;

    /// 消费消息
    async fn consume(
        &self,
        uuid: &str,
        config: &AmqpConsumeConfig,
    ) -> Result<AmqpConsumeOutput, String>;

    /// 获取单条消息
    async fn get(&self, uuid: &str, config: &AmqpGetConfig) -> Result<AmqpGetOutput, String>;

    /// 确认/拒绝消息
    async fn ack(&self, uuid: &str, config: &AmqpAckConfig) -> Result<AmqpAckOutput, String>;

    /// 管理操作（通过 Management API）
    async fn management(
        &self,
        uuid: &str,
        config: &AmqpManagementConfig,
    ) -> Result<AmqpManagementOutput, String>;

    // ===== AMQP 1.0 操作 =====

    /// 建立 AMQP 1.0 连接
    async fn amqp10_connect(
        &self,
        uuid: &str,
        config: &Amqp10ConnectConfig,
    ) -> Result<Amqp10ConnectOutput, String>;

    /// AMQP 1.0 发送消息
    async fn amqp10_send(
        &self,
        uuid: &str,
        config: &Amqp10SendConfig,
    ) -> Result<Amqp10SendOutput, String>;

    /// AMQP 1.0 接收消息
    async fn amqp10_receive(
        &self,
        uuid: &str,
        config: &Amqp10ReceiveConfig,
    ) -> Result<Amqp10ReceiveOutput, String>;

    /// AMQP 1.0 确认消息
    async fn amqp10_accept(
        &self,
        uuid: &str,
        config: &Amqp10AcceptConfig,
    ) -> Result<Amqp10AcceptOutput, String>;

    /// AMQP 1.0 拒绝消息
    async fn amqp10_reject(
        &self,
        uuid: &str,
        config: &Amqp10RejectConfig,
    ) -> Result<Amqp10RejectOutput, String>;

    /// AMQP 1.0 断开连接
    async fn amqp10_disconnect(
        &self,
        uuid: &str,
        config: &Amqp10DisconnectConfig,
    ) -> Result<(), String>;

    // ===== 通用操作 =====

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<AmqpConnectionInfo>;
}
