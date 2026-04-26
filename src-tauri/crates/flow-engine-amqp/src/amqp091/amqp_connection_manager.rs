//! AMQP 0-9-1 连接管理器
//!
//! 实现 [`AmqpProvider`] trait，管理 AMQP 连接的生命周期。
//! 使用 lapin [`Connection`](lapin::Connection) 进行 AMQP 通信。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use lapin::options::BasicQosOptions;
use log::{info, warn};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use flow_engine_core::node_config::{
    Amqp10AcceptConfig, Amqp10ConnectConfig, Amqp10DisconnectConfig, Amqp10ReceiveConfig,
    Amqp10RejectConfig, Amqp10SendConfig, AmqpAckConfig, AmqpCloseConfig, AmqpConnectConfig,
    AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig, AmqpGetConfig,
    AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig, AmqpQueueDeclareConfig,
    AmqpQueueDeleteConfig,
};
use flow_engine_core::traits::{
    Amqp10AcceptOutput, Amqp10ConnectOutput, Amqp10ReceiveOutput, Amqp10RejectOutput,
    Amqp10SendOutput, AmqpAckOutput, AmqpConnectOutput, AmqpConnectionInfo, AmqpConsumeOutput,
    AmqpExchangeOutput, AmqpGetOutput, AmqpManagementOutput, AmqpMessage, AmqpProvider,
    AmqpPublishOutput, AmqpQueueDeclareOutput, AmqpQueueDeleteOutput,
};

use crate::amqp091::amqp_client;

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// AMQP 连接状态
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum AmqpConnectionStatus {
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 已断开
    Disconnected,
    /// 错误状态
    Error(String),
}

impl std::fmt::Display for AmqpConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Error(e) => write!(f, "error: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// AmqpConnection
// ---------------------------------------------------------------------------

/// AMQP 连接
///
/// 持有 lapin Connection 和 Channel，以及元数据。
pub struct AmqpConnection {
    /// lapin 连接
    connection: lapin::Connection,
    /// 默认通道
    channel: lapin::Channel,
    /// Broker URL
    url: String,
    /// 连接状态
    status: Arc<RwLock<AmqpConnectionStatus>>,
    /// 创建时间（毫秒时间戳）
    #[allow(dead_code)]
    created_at: i64,
}

// ---------------------------------------------------------------------------
// AmqpConnectionManager
// ---------------------------------------------------------------------------

/// AMQP 连接管理器（per-flow-execution 作用域）
///
/// 管理 AMQP 连接池，实现 `AmqpProvider` trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
pub struct AmqpConnectionManager {
    /// 活跃连接池: `connection_id` → [`AmqpConnection`]
    connections: Arc<RwLock<HashMap<String, AmqpConnection>>>,
    /// 是否为共享模式（压力测试）
    is_shared: bool,
    /// AMQP 1.0 连接管理器（委托 AMQP 1.0 操作）
    amqp10_manager: Arc<crate::amqp10::Amqp10ConnectionManager>,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl AmqpConnectionManager {
    /// 创建新的连接管理器（普通模式）
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            is_shared: false,
            amqp10_manager: Arc::new(crate::amqp10::Amqp10ConnectionManager::new()),
        }
    }

    /// 创建共享模式的连接管理器（压测场景）
    ///
    /// 共享模式下 close 为 no-op，避免并发虚拟用户互相关闭连接。
    pub fn new_shared() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            is_shared: true,
            amqp10_manager: Arc::new(crate::amqp10::Amqp10ConnectionManager::new_shared()),
        }
    }
}

impl Default for AmqpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AmqpConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, conn) in connections.drain() {
                // 关闭连接（忽略异步结果，drop 场景下无法 await）
                // 使用 tokio spawn 确保连接正确关闭
                let connection = conn.connection;
                let id_clone = id.clone();
                tokio::spawn(async move {
                    let _ = connection.close(200, "manager dropped".into()).await;
                    info!("AMQP 连接 {} 已随 Manager 释放而清理", id_clone);
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 工具方法
// ---------------------------------------------------------------------------

impl AmqpConnectionManager {
    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    /// 从连接池获取连接的通道引用（释放锁后使用）
    async fn get_channel(&self, uuid: &str) -> Result<lapin::Channel, String> {
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 连接 '{}' 不存在", uuid))?;
        Ok(conn.channel.clone())
    }
}

// ---------------------------------------------------------------------------
// AmqpProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl AmqpProvider for AmqpConnectionManager {
    // ===== AMQP 0-9-1 操作 =====

    /// 建立 AMQP 0-9-1 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &AmqpConnectConfig,
    ) -> Result<AmqpConnectOutput, String> {
        info!("AMQP 开始连接: url={}, id={}", config.url, uuid);

        // UUID 去重：如果已存在直接返回成功
        {
            let conns = self.connections.read().await;
            if let Some(existing) = conns.get(uuid) {
                let s = existing.status.read().await;
                if *s == AmqpConnectionStatus::Connected {
                    info!("AMQP 复用已有连接: id={}", uuid);
                    return Ok(AmqpConnectOutput {
                        success: true,
                        connection_id: uuid.to_string(),
                        server_properties: None,
                        error: None,
                    });
                }
            }
        } // 读锁释放

        // 创建 lapin 连接
        let connection = amqp_client::connect_async(config).await?;

        // 创建默认通道
        let channel = amqp_client::create_channel(&connection).await?;

        let status = Arc::new(RwLock::new(AmqpConnectionStatus::Connected));
        let created_at = Self::now_ms();

        let amqp_conn = AmqpConnection {
            connection,
            channel,
            url: config.url.clone(),
            status: Arc::clone(&status),
            created_at,
        };

        // 存入连接池
        {
            let mut conns = self.connections.write().await;
            info!("AMQP 添加连接: id={}", uuid);
            conns.insert(uuid.to_string(), amqp_conn);
        }

        Ok(AmqpConnectOutput {
            success: true,
            connection_id: uuid.to_string(),
            server_properties: None,
            error: None,
        })
    }

    /// 关闭 AMQP 0-9-1 连接
    async fn close(&self, uuid: &str, config: &AmqpCloseConfig) -> Result<(), String> {
        if self.is_shared {
            info!("共享模式下忽略 close 请求: {}", uuid);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(uuid);
        match removed {
            Some(conn) => {
                let reply_code = config.reply_code;
                let reply_text = config
                    .reply_text
                    .clone()
                    .unwrap_or_else(|| "normal close".to_string());
                let _ = conn.connection.close(reply_code, reply_text.into()).await;
                info!("AMQP 关闭连接: id={}", uuid);
                Ok(())
            }
            None => {
                warn!("AMQP 关闭连接失败: 连接 '{}' 不存在", uuid);
                Err(format!("AMQP 连接 '{}' 不存在", uuid))
            }
        }
    }

    /// 声明交换器
    async fn exchange_declare(
        &self,
        uuid: &str,
        config: &AmqpExchangeDeclareConfig,
    ) -> Result<AmqpExchangeOutput, String> {
        let channel = self.get_channel(uuid).await?;

        let exchange_type = config
            .exchange_type
            .as_deref()
            .unwrap_or("direct");

        amqp_client::exchange_declare(
            &channel,
            &config.exchange,
            exchange_type,
            config.passive,
            config.durable,
            config.auto_delete,
            config.internal,
            config.arguments.as_ref(),
        )
        .await?;

        Ok(AmqpExchangeOutput {
            success: true,
            exchange: Some(config.exchange.clone()),
            exchange_type: Some(exchange_type.to_string()),
            error: None,
        })
    }

    /// 删除交换器
    async fn exchange_delete(
        &self,
        uuid: &str,
        config: &AmqpExchangeDeleteConfig,
    ) -> Result<AmqpExchangeOutput, String> {
        let channel = self.get_channel(uuid).await?;

        amqp_client::exchange_delete(&channel, &config.exchange, config.if_unused).await?;

        Ok(AmqpExchangeOutput {
            success: true,
            exchange: Some(config.exchange.clone()),
            exchange_type: None,
            error: None,
        })
    }

    /// 声明队列
    async fn queue_declare(
        &self,
        uuid: &str,
        config: &AmqpQueueDeclareConfig,
    ) -> Result<AmqpQueueDeclareOutput, String> {
        let channel = self.get_channel(uuid).await?;

        let queue_name = config.queue.as_deref().unwrap_or("");

        let (name, message_count, consumer_count) = amqp_client::queue_declare(
            &channel,
            queue_name,
            config.passive,
            config.durable,
            config.exclusive,
            config.auto_delete,
            config.arguments.as_ref(),
        )
        .await?;

        Ok(AmqpQueueDeclareOutput {
            success: true,
            queue_name: Some(name),
            message_count: Some(message_count),
            consumer_count: Some(consumer_count),
            error: None,
        })
    }

    /// 删除队列
    async fn queue_delete(
        &self,
        uuid: &str,
        config: &AmqpQueueDeleteConfig,
    ) -> Result<AmqpQueueDeleteOutput, String> {
        let channel = self.get_channel(uuid).await?;

        let deleted = amqp_client::queue_delete(
            &channel,
            &config.queue,
            config.if_unused,
            config.if_empty,
        )
        .await?;

        Ok(AmqpQueueDeleteOutput {
            success: true,
            messages_deleted: Some(deleted),
            error: None,
        })
    }

    /// 绑定队列到交换器
    async fn queue_bind(&self, uuid: &str, config: &AmqpQueueBindConfig) -> Result<(), String> {
        let channel = self.get_channel(uuid).await?;

        let routing_key = config.routing_key.as_deref().unwrap_or("");

        amqp_client::queue_bind(
            &channel,
            &config.queue,
            &config.exchange,
            routing_key,
            config.arguments.as_ref(),
        )
        .await?;

        Ok(())
    }

    /// 发布消息
    async fn publish(
        &self,
        uuid: &str,
        config: &AmqpPublishConfig,
    ) -> Result<AmqpPublishOutput, String> {
        let channel = self.get_channel(uuid).await?;

        let exchange = config.exchange.as_deref().unwrap_or("");
        let routing_key = config.routing_key.as_deref().unwrap_or("");
        let body = config.body.as_deref().unwrap_or("");

        let properties = amqp_client::build_basic_properties(config);

        let confirm_result = amqp_client::basic_publish(
            &channel,
            exchange,
            routing_key,
            body.as_bytes(),
            config.mandatory,
            properties,
        )
        .await?;

        Ok(AmqpPublishOutput {
            success: true,
            confirm_result: Some(confirm_result),
            returned_message: None,
            error: None,
        })
    }

    /// 消费消息
    async fn consume(
        &self,
        uuid: &str,
        config: &AmqpConsumeConfig,
    ) -> Result<AmqpConsumeOutput, String> {
        let channel = self.get_channel(uuid).await?;

        // 设置预取数量
        if config.prefetch_count > 0 {
            channel
                .basic_qos(
                    config.prefetch_count,
                    BasicQosOptions { global: false },
                )
                .await
                .map_err(|e| format!("AMQP 设置预取数量失败: {}", e))?;
        }

        let consumer_tag = config
            .consumer_tag
            .as_deref()
            .unwrap_or("mpe-consumer");

        let mut consumer = amqp_client::basic_consume(
            &channel,
            &config.queue,
            consumer_tag,
            config.no_local,
            config.ack_mode == "auto",
            config.exclusive,
        )
        .await?;

        // 收集消息
        let max_messages = if config.max_messages > 0 {
            config.max_messages as usize
        } else {
            1
        };
        let timeout_ms = if config.timeout_ms > 0 {
            config.timeout_ms
        } else {
            30000
        };

        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut messages = Vec::new();

        loop {
            if messages.len() >= max_messages {
                break;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, consumer.next()).await {
                Ok(Some(Ok(delivery))) => {
                    let body_str = String::from_utf8_lossy(&delivery.data).to_string();
                    let body_decoded = serde_json::from_str::<serde_json::Value>(&body_str).ok();

                    let msg = AmqpMessage {
                        delivery_tag: Some(delivery.delivery_tag),
                        exchange: Some(delivery.exchange.to_string()),
                        routing_key: Some(delivery.routing_key.to_string()),
                        redelivered: Some(delivery.redelivered),
                        properties: None,
                        body: body_str,
                        body_decoded,
                    };

                    // 自动确认
                    if config.ack_mode == "auto" || config.auto_ack_on_success {
                        let _ = amqp_client::basic_ack(&channel, delivery.delivery_tag, false)
                            .await;
                    }

                    messages.push(msg);
                }
                Ok(Some(Err(e))) => {
                    warn!("AMQP 消费消息错误: {}", e);
                    if config.auto_reject_on_failure {
                        // 无法获取 delivery_tag，跳过
                    }
                    break;
                }
                Ok(None) => {
                    // 消费者结束
                    break;
                }
                Err(_) => {
                    // 超时
                    break;
                }
            }
        }

        let message_count = messages.len() as u32;

        Ok(AmqpConsumeOutput {
            success: !messages.is_empty(),
            messages,
            message_count,
            error: None,
        })
    }

    /// 获取单条消息
    async fn get(&self, uuid: &str, config: &AmqpGetConfig) -> Result<AmqpGetOutput, String> {
        let channel = self.get_channel(uuid).await?;

        let result = amqp_client::basic_get(&channel, &config.queue, config.no_ack).await?;

        match result {
            Some((delivery_tag, exchange, routing_key, redelivered, body, message_count)) => {
                let body_str = String::from_utf8_lossy(&body).to_string();
                let body_decoded = serde_json::from_str::<serde_json::Value>(&body_str).ok();

                let message = AmqpMessage {
                    delivery_tag: Some(delivery_tag),
                    exchange: Some(exchange),
                    routing_key: Some(routing_key),
                    redelivered: Some(redelivered),
                    properties: None,
                    body: body_str,
                    body_decoded,
                };

                Ok(AmqpGetOutput {
                    success: true,
                    message: Some(message),
                    message_count_remaining: Some(message_count),
                    error: None,
                })
            }
            None => Ok(AmqpGetOutput {
                success: false,
                message: None,
                message_count_remaining: None,
                error: Some("队列为空".to_string()),
            }),
        }
    }

    /// 确认/拒绝消息
    async fn ack(&self, uuid: &str, config: &AmqpAckConfig) -> Result<AmqpAckOutput, String> {
        let channel = self.get_channel(uuid).await?;

        match config.action.as_str() {
            "ack" => {
                amqp_client::basic_ack(&channel, config.delivery_tag, config.multiple).await?;
                Ok(AmqpAckOutput {
                    success: true,
                    action: Some("ack".to_string()),
                    delivery_tag: Some(config.delivery_tag),
                    error: None,
                })
            }
            "nack" => {
                amqp_client::basic_nack(
                    &channel,
                    config.delivery_tag,
                    config.multiple,
                    config.requeue,
                )
                .await?;
                Ok(AmqpAckOutput {
                    success: true,
                    action: Some("nack".to_string()),
                    delivery_tag: Some(config.delivery_tag),
                    error: None,
                })
            }
            "reject" => {
                amqp_client::basic_reject(&channel, config.delivery_tag, config.requeue).await?;
                Ok(AmqpAckOutput {
                    success: true,
                    action: Some("reject".to_string()),
                    delivery_tag: Some(config.delivery_tag),
                    error: None,
                })
            }
            _ => Err(format!("未知的确认操作: '{}'", config.action)),
        }
    }

    /// 管理操作（通过 RabbitMQ Management HTTP API）
    ///
    /// 使用 reqwest 调用 RabbitMQ Management API (默认端口 15672)。
    /// 仅在 `management` feature 启用时可用。
    async fn management(
        &self,
        _uuid: &str,
        config: &AmqpManagementConfig,
    ) -> Result<AmqpManagementOutput, String> {
        #[cfg(feature = "management")]
        {
            self.execute_management_api(config).await
        }
        #[cfg(not(feature = "management"))]
        {
            let _ = config;
            Err("AMQP 管理操作需要启用 management feature".to_string())
        }
    }

    // ===== AMQP 1.0 操作（委托给 Amqp10ConnectionManager） =====

    async fn amqp10_connect(
        &self,
        uuid: &str,
        config: &Amqp10ConnectConfig,
    ) -> Result<Amqp10ConnectOutput, String> {
        self.amqp10_manager.connect(uuid, config).await
    }

    async fn amqp10_send(
        &self,
        uuid: &str,
        config: &Amqp10SendConfig,
    ) -> Result<Amqp10SendOutput, String> {
        self.amqp10_manager.send(uuid, config).await
    }

    async fn amqp10_receive(
        &self,
        uuid: &str,
        config: &Amqp10ReceiveConfig,
    ) -> Result<Amqp10ReceiveOutput, String> {
        self.amqp10_manager.receive(uuid, config).await
    }

    async fn amqp10_accept(
        &self,
        uuid: &str,
        config: &Amqp10AcceptConfig,
    ) -> Result<Amqp10AcceptOutput, String> {
        self.amqp10_manager.accept(uuid, config).await
    }

    async fn amqp10_reject(
        &self,
        uuid: &str,
        config: &Amqp10RejectConfig,
    ) -> Result<Amqp10RejectOutput, String> {
        self.amqp10_manager.reject(uuid, config).await
    }

    async fn amqp10_disconnect(
        &self,
        uuid: &str,
        config: &Amqp10DisconnectConfig,
    ) -> Result<(), String> {
        self.amqp10_manager.disconnect(uuid, config).await
    }

    // ===== 通用操作 =====

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String> {
        let conns = self.connections.try_read().ok()?;
        let conn = conns.get(uuid)?;
        let status = conn.status.try_read().ok()?;
        Some(status.to_string())
    }

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
        let Ok(conns) = self.connections.try_read() else {
            return vec![];
        };
        conns
            .iter()
            .map(|(uuid, conn)| {
                let status_str = conn
                    .status
                    .try_read()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                AmqpConnectionInfo {
                    connection_id: uuid.clone(),
                    url: conn.url.clone(),
                    status: status_str,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// AmqpConnectionManager 辅助方法（非 trait）
// ---------------------------------------------------------------------------

impl AmqpConnectionManager {
    /// 执行 RabbitMQ Management HTTP API 请求
    ///
    /// 根据 config.action 构建 API 路径，使用 reqwest 发送 GET 请求。
    /// 支持 overview/queue_info/list_queues/list_connections/list_channels/list_exchanges/list_consumers。
    #[cfg(feature = "management")]
    pub(crate) async fn execute_management_api(
        &self,
        config: &AmqpManagementConfig,
    ) -> Result<AmqpManagementOutput, String> {
        // 构建 API 路径
        let path = build_management_path(&config.action, config.vhost.as_deref(), config.queue_name.as_deref())?;

        // 拼接完整 URL
        let base_url = config.management_url.trim_end_matches('/');
        let url = format!("{}/api{}", base_url, path);

        log::info!("[AMQP Management] 请求: {} {}", "GET", url);

        // 发送请求（带基本认证）
        let response = reqwest::Client::new()
            .get(&url)
            .basic_auth(
                config.username.as_deref().unwrap_or("guest"),
                Some(config.password.as_deref().unwrap_or("guest")),
            )
            .send()
            .await
            .map_err(|e| format!("Management API 请求失败: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Ok(AmqpManagementOutput {
                success: false,
                data: None,
                error: Some(format!("HTTP {}: {}", status, body)),
            });
        }

        // 解析 JSON 响应
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Management API 响应解析失败: {}", e))?;

        Ok(AmqpManagementOutput {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

/// 根据 action 构建管理 API 路径
///
/// RabbitMQ Management API 路径规则:
/// - overview → /overview
/// - queue_info → /queues/{vhost}/{queue}
/// - list_queues → /queues/{vhost}
/// - list_connections → /connections
/// - list_channels → /channels
/// - list_exchanges → /exchanges/{vhost}
/// - list_consumers → /consumers/{vhost}
#[cfg(feature = "management")]
fn build_management_path(
    action: &str,
    vhost: Option<&str>,
    queue_name: Option<&str>,
) -> Result<String, String> {
    match action {
        "overview" => Ok("/overview".to_string()),
        "queue_info" => {
            let v = vhost.unwrap_or("/");
            let q = queue_name.ok_or("queue_info 操作需要 queue_name 参数")?;
            Ok(format!("/queues/{}/{}", urlencoding_vhost(v), q))
        }
        "list_queues" => {
            let v = vhost.unwrap_or("/");
            Ok(format!("/queues/{}", urlencoding_vhost(v)))
        }
        "list_connections" => Ok("/connections".to_string()),
        "list_channels" => Ok("/channels".to_string()),
        "list_exchanges" => {
            let v = vhost.unwrap_or("/");
            Ok(format!("/exchanges/{}", urlencoding_vhost(v)))
        }
        "list_consumers" => {
            let v = vhost.unwrap_or("/");
            Ok(format!("/consumers/{}", urlencoding_vhost(v)))
        }
        _ => Err(format!("未知的 management action: '{}'", action)),
    }
}

/// 对 vhost 进行 URL 编码（RabbitMQ 默认 vhost "/" 编码为 "%2F"）
#[cfg(feature = "management")]
fn urlencoding_vhost(vhost: &str) -> String {
    vhost.replace('/', "%2F")
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = AmqpConnectionManager::new();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = AmqpConnectionManager::new_shared();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = AmqpConnectionManager::default();
        assert!(manager.list_connections().is_empty());
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpCloseConfig::default();
        let result = manager.close("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_shared_mode_close_is_noop() {
        let manager = AmqpConnectionManager::new_shared();
        let config = AmqpCloseConfig::default();
        let result = manager.close("non-existent", &config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_status_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_list_connections_empty() {
        let manager = AmqpConnectionManager::new();
        let connections = manager.list_connections();
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_exchange_declare_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpExchangeDeclareConfig::default();
        let result = manager.exchange_declare("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_exchange_delete_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpExchangeDeleteConfig::default();
        let result = manager.exchange_delete("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_queue_declare_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpQueueDeclareConfig::default();
        let result = manager.queue_declare("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_queue_delete_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpQueueDeleteConfig::default();
        let result = manager.queue_delete("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_queue_bind_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpQueueBindConfig::default();
        let result = manager.queue_bind("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_publish_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpPublishConfig::default();
        let result = manager.publish("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_consume_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpConsumeConfig::default();
        let result = manager.consume("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_get_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpGetConfig::default();
        let result = manager.get("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_ack_nonexistent_connection() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpAckConfig {
            connection_id: "non-existent".to_string(),
            action: "ack".to_string(),
            delivery_tag: 1,
            multiple: false,
            requeue: false,
        };
        let result = manager.ack("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_management_not_supported() {
        let manager = AmqpConnectionManager::new();
        let config = AmqpManagementConfig::default();
        let result = manager.management("any", &config).await;
        assert!(result.is_err());
    }

    // ---- AMQP 1.0 委托测试（通过 Amqp10ConnectionManager） ----

    #[tokio::test]
    async fn test_amqp10_connect_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10ConnectConfig::default();
        // 没有 broker，连接应失败
        let result = manager.amqp10_connect("any", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_amqp10_send_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10SendConfig::default();
        let result = manager.amqp10_send("any", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_amqp10_receive_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10ReceiveConfig::default();
        let result = manager.amqp10_receive("any", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_amqp10_accept_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10AcceptConfig::default();
        let result = manager.amqp10_accept("any", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_amqp10_reject_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10RejectConfig::default();
        let result = manager.amqp10_reject("any", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_amqp10_disconnect_nonexistent() {
        let manager = AmqpConnectionManager::new();
        let config = Amqp10DisconnectConfig::default();
        let result = manager.amqp10_disconnect("any", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    // ---- 状态显示测试 ----

    #[test]
    fn test_connection_status_display() {
        assert_eq!(AmqpConnectionStatus::Connecting.to_string(), "connecting");
        assert_eq!(AmqpConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(
            AmqpConnectionStatus::Disconnected.to_string(),
            "disconnected"
        );
        let err_status = AmqpConnectionStatus::Error("timeout".to_string());
        assert!(err_status.to_string().contains("timeout"));
    }

    // ---- Mock Provider 测试 ----

    /// Mock AmqpProvider 用于测试
    struct MockAmqpProvider {
        connections: Arc<RwLock<HashMap<String, String>>>,
    }

    impl MockAmqpProvider {
        fn new() -> Self {
            Self {
                connections: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl AmqpProvider for MockAmqpProvider {
        async fn connect(
            &self,
            uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            let mut conns = self.connections.write().await;
            conns.insert(uuid.to_string(), "connected".to_string());
            Ok(AmqpConnectOutput {
                success: true,
                connection_id: uuid.to_string(),
                server_properties: None,
                error: None,
            })
        }

        async fn close(&self, uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> {
            let mut conns = self.connections.write().await;
            conns
                .remove(uuid)
                .ok_or_else(|| format!("连接 '{}' 不存在", uuid))?;
            Ok(())
        }

        async fn exchange_declare(
            &self,
            _uuid: &str,
            config: &AmqpExchangeDeclareConfig,
        ) -> Result<AmqpExchangeOutput, String> {
            Ok(AmqpExchangeOutput {
                success: true,
                exchange: Some(config.exchange.clone()),
                exchange_type: config.exchange_type.clone(),
                error: None,
            })
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            config: &AmqpExchangeDeleteConfig,
        ) -> Result<AmqpExchangeOutput, String> {
            Ok(AmqpExchangeOutput {
                success: true,
                exchange: Some(config.exchange.clone()),
                exchange_type: None,
                error: None,
            })
        }

        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<AmqpQueueDeclareOutput, String> {
            Ok(AmqpQueueDeclareOutput {
                success: true,
                queue_name: Some("test-queue".to_string()),
                message_count: Some(0),
                consumer_count: Some(0),
                error: None,
            })
        }

        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<AmqpQueueDeleteOutput, String> {
            Ok(AmqpQueueDeleteOutput {
                success: true,
                messages_deleted: Some(0),
                error: None,
            })
        }

        async fn queue_bind(
            &self,
            _uuid: &str,
            _config: &AmqpQueueBindConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &AmqpPublishConfig,
        ) -> Result<AmqpPublishOutput, String> {
            Ok(AmqpPublishOutput {
                success: true,
                confirm_result: Some("not_requested".to_string()),
                returned_message: None,
                error: None,
            })
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<AmqpConsumeOutput, String> {
            Ok(AmqpConsumeOutput {
                success: true,
                messages: vec![],
                message_count: 0,
                error: None,
            })
        }

        async fn get(
            &self,
            _uuid: &str,
            _config: &AmqpGetConfig,
        ) -> Result<AmqpGetOutput, String> {
            Ok(AmqpGetOutput {
                success: false,
                message: None,
                message_count_remaining: None,
                error: Some("队列为空".to_string()),
            })
        }

        async fn ack(
            &self,
            _uuid: &str,
            config: &AmqpAckConfig,
        ) -> Result<AmqpAckOutput, String> {
            Ok(AmqpAckOutput {
                success: true,
                action: Some(config.action.clone()),
                delivery_tag: Some(config.delivery_tag),
                error: None,
            })
        }

        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<AmqpManagementOutput, String> {
            Err("不支持".to_string())
        }

        async fn amqp10_connect(
            &self,
            _uuid: &str,
            _config: &Amqp10ConnectConfig,
        ) -> Result<Amqp10ConnectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<Amqp10SendOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<Amqp10ReceiveOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<Amqp10AcceptOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<Amqp10RejectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        fn get_status(&self, uuid: &str) -> Option<String> {
            let conns = self.connections.try_read().ok()?;
            conns.get(uuid).cloned()
        }

        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            let Ok(conns) = self.connections.try_read() else {
                return vec![];
            };
            conns
                .iter()
                .map(|(id, status)| AmqpConnectionInfo {
                    connection_id: id.clone(),
                    url: format!("amqp://mock/{}", id),
                    status: status.clone(),
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn test_mock_provider_connect_and_close() {
        let provider = MockAmqpProvider::new();
        let config = AmqpConnectConfig {
            url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
            ..Default::default()
        };

        // 连接
        let result = provider.connect("conn-1", &config).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.connection_id, "conn-1");

        // 状态
        assert_eq!(provider.get_status("conn-1"), Some("connected".to_string()));

        // 列出连接
        let conns = provider.list_connections();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].connection_id, "conn-1");

        // 关闭
        let close_config = AmqpCloseConfig::default();
        let result = provider.close("conn-1", &close_config).await;
        assert!(result.is_ok());

        // 关闭后状态
        assert!(provider.get_status("conn-1").is_none());
    }

    #[tokio::test]
    async fn test_mock_provider_exchange_operations() {
        let provider = MockAmqpProvider::new();
        let connect_config = AmqpConnectConfig {
            url: "amqp://localhost:5672".to_string(),
            ..Default::default()
        };
        provider.connect("conn-1", &connect_config).await.unwrap();

        // 声明交换器
        let declare_config = AmqpExchangeDeclareConfig {
            connection_id: "conn-1".to_string(),
            exchange: "test-exchange".to_string(),
            exchange_type: Some("topic".to_string()),
            durable: true,
            ..Default::default()
        };
        let result = provider.exchange_declare("conn-1", &declare_config).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.exchange, Some("test-exchange".to_string()));

        // 删除交换器
        let delete_config = AmqpExchangeDeleteConfig {
            connection_id: "conn-1".to_string(),
            exchange: "test-exchange".to_string(),
            if_unused: true,
        };
        let result = provider.exchange_delete("conn-1", &delete_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_queue_operations() {
        let provider = MockAmqpProvider::new();
        let connect_config = AmqpConnectConfig {
            url: "amqp://localhost:5672".to_string(),
            ..Default::default()
        };
        provider.connect("conn-1", &connect_config).await.unwrap();

        // 声明队列
        let declare_config = AmqpQueueDeclareConfig {
            connection_id: "conn-1".to_string(),
            queue: Some("test-queue".to_string()),
            durable: true,
            ..Default::default()
        };
        let result = provider.queue_declare("conn-1", &declare_config).await;
        assert!(result.is_ok());

        // 绑定队列
        let bind_config = AmqpQueueBindConfig {
            connection_id: "conn-1".to_string(),
            queue: "test-queue".to_string(),
            exchange: "test-exchange".to_string(),
            routing_key: Some("test.#".to_string()),
            ..Default::default()
        };
        let result = provider.queue_bind("conn-1", &bind_config).await;
        assert!(result.is_ok());

        // 删除队列
        let delete_config = AmqpQueueDeleteConfig {
            connection_id: "conn-1".to_string(),
            queue: "test-queue".to_string(),
            ..Default::default()
        };
        let result = provider.queue_delete("conn-1", &delete_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_publish_and_ack() {
        let provider = MockAmqpProvider::new();
        let connect_config = AmqpConnectConfig {
            url: "amqp://localhost:5672".to_string(),
            ..Default::default()
        };
        provider.connect("conn-1", &connect_config).await.unwrap();

        // 发布消息
        let publish_config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("test-exchange".to_string()),
            routing_key: Some("test.key".to_string()),
            body: Some("{\"msg\":\"hello\"}".to_string()),
            content_type: Some("application/json".to_string()),
            delivery_mode: 2,
            ..Default::default()
        };
        let result = provider.publish("conn-1", &publish_config).await;
        assert!(result.is_ok());

        // 确认消息
        let ack_config = AmqpAckConfig {
            connection_id: "conn-1".to_string(),
            action: "ack".to_string(),
            delivery_tag: 1,
            multiple: false,
            requeue: false,
        };
        let result = provider.ack("conn-1", &ack_config).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.action, Some("ack".to_string()));
    }
}
