//! MQTT 连接管理器
//!
//! 实现 [`MqttProvider`] trait，管理 MQTT 连接的生命周期。
//! 使用 rumqttc [`AsyncClient`](rumqttc::AsyncClient) 进行 MQTT 通信，
//! 后台 tokio 任务持续 poll [`EventLoop`](rumqttc::EventLoop) 分发消息。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{info, warn};
use rumqttc::{Event, Packet, QoS};
use tokio::sync::{RwLock, broadcast, Mutex};

use flow_engine_core::node_config::{
    MqttConnectConfig, MqttDisconnectConfig, MqttPublishConfig, MqttRequestConfig,
    MqttSubscribeConfig, MqttWaitMessageConfig,
};
use flow_engine_core::traits::{
    MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput, MqttProvider,
    MqttPublishOutput, MqttReceivedMessage, MqttSubscribeOutput, MqttTimingInfo,
};

use crate::mqtt_client;

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// MQTT 连接状态
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum MqttConnectionStatus {
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 已断开
    Disconnected,
    /// Error状态
    Error(String),
}

impl std::fmt::Display for MqttConnectionStatus {
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
// MqttConnection
// ---------------------------------------------------------------------------

/// MQTT 连接
///
/// 持有 rumqttc 异步客户端、后台 `EventLoop` poll 任务句柄和元数据。
pub struct MqttConnection {
    /// rumqttc 异步客户端
    client: rumqttc::AsyncClient,
    /// 后台 `EventLoop` poll 任务的 `JoinHandle`
    eventloop_handle: tokio::task::JoinHandle<()>,
    /// Broker URL
    url: String,
    /// 客户端 ID
    client_id: String,
    /// 连接状态
    status: Arc<RwLock<MqttConnectionStatus>>,
    /// 创建时间（毫秒时间戳）
    created_at: i64,
    /// 消息接收通道
    message_rx: Arc<Mutex<broadcast::Receiver<MqttReceivedMessage>>>,
}

// ---------------------------------------------------------------------------
// MqttConnectionManager
// ---------------------------------------------------------------------------

/// MQTT 连接管理器（per-flow-execution 作用域）
///
/// 管理 MQTT 连接池，实现 `MqttProvider` trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
/// 消息通过 `broadcast` 通道分发给多个消费者。
pub struct MqttConnectionManager {
    /// 活跃连接池: `connection_id` → [`MqttConnection`]
    connections: Arc<RwLock<HashMap<String, MqttConnection>>>,
    /// 消息广播发送端（所有连接共享）
    message_tx: Arc<broadcast::Sender<MqttReceivedMessage>>,
    /// 是否为共享模式（压力测试）
    is_shared: bool,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl MqttConnectionManager {
    /// 创建新的连接管理器（普通模式）
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_tx: Arc::new(tx),
            is_shared: false,
        }
    }

    /// 创建共享模式的连接管理器（压测场景）
    ///
    /// 共享模式下 disconnect 为 no-op，避免并发虚拟用户互相关闭连接。
    pub fn new_shared() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_tx: Arc::new(tx),
            is_shared: true,
        }
    }
}

impl Default for MqttConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MqttConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, conn) in connections.drain() {
                conn.eventloop_handle.abort();
                info!("MQTT 连接 {} 已随 Manager 释放而清理", id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 工具方法
// ---------------------------------------------------------------------------

impl MqttConnectionManager {
    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    /// 将 rumqttc `QoS` 值转换为 u8
    fn qos_to_u8(qos: QoS) -> u8 {
        match qos {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce => 1,
            QoS::ExactlyOnce => 2,
        }
    }

    /// 将 u8 转换为 rumqttc `QoS`
    fn u8_to_qos(qos: u8) -> QoS {
        match qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        }
    }

    /// 启动后台 `EventLoop` poll 任务
    ///
    /// 持续 poll EventLoop，将 Publish 消息分发到 broadcast 通道，
    /// 处理 ConnAck、Disconnect 等事件更新连接状态。
    fn spawn_eventloop_poll(
        mut eventloop: rumqttc::EventLoop,
        status: Arc<RwLock<MqttConnectionStatus>>,
        message_tx: Arc<broadcast::Sender<MqttReceivedMessage>>,
        connection_id: String,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(packet)) => {
                        match packet {
                            Packet::ConnAck(connack) => {
                                let code = connack.code;
                                let is_success = matches!(
                                    code,
                                    rumqttc::ConnectReturnCode::Success
                                );
                                if is_success {
                                    info!(
                                        "MQTT 连接 {} 收到 ConnAck: session_present={}",
                                        connection_id, connack.session_present
                                    );
                                    let mut s = status.write().await;
                                    *s = MqttConnectionStatus::Connected;
                                } else {
                                    warn!(
                                        "MQTT 连接 {} ConnAck Failed: code={:?}",
                                        connection_id, code
                                    );
                                    let mut s = status.write().await;
                                    *s = MqttConnectionStatus::Error(format!("{:?}", code));
                                }
                            }
                            Packet::Publish(publish) => {
                                let msg = MqttReceivedMessage {
                                    topic: String::from_utf8_lossy(&publish.topic)
                                        .to_string(),
                                    payload: publish.payload.to_vec(),
                                    qos: Self::qos_to_u8(publish.qos),
                                    retain: publish.retain,
                                    payload_format: None,
                                    timestamp_ms: Self::now_ms(),
                                };
                                // broadcast 发送Failed说明没有接收者，忽略即可
                                let _ = message_tx.send(msg);
                            }
                            Packet::Disconnect(_) => {
                                info!("MQTT 连接 {} 收到 Disconnect", connection_id);
                                let mut s = status.write().await;
                                *s = MqttConnectionStatus::Disconnected;
                            }
                            _ => {
                                // 其他 Incoming 包（PingResp、SubAck 等）忽略
                            }
                        }
                    }
                    Ok(Event::Outgoing(_)) => {
                        // Outgoing 事件不需要处理
                    }
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        // RequestsDone 表示客户端已 drop，正常退出
                        if err_msg.contains("RequestsDone") {
                            info!(
                                "MQTT 连接 {} EventLoop 正常结束（客户端已关闭）",
                                connection_id
                            );
                            let mut s = status.write().await;
                            *s = MqttConnectionStatus::Disconnected;
                            return;
                        }
                        warn!(
                            "MQTT 连接 {} EventLoop 轮询Error: {}",
                            connection_id, err_msg
                        );
                        {
                            let mut s = status.write().await;
                            *s = MqttConnectionStatus::Error(err_msg);
                        }
                        // 短暂休眠后继续轮询，避免Error时 CPU 空转
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// MqttProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl MqttProvider for MqttConnectionManager {
    /// 建立 MQTT 连接
    ///
    /// 调用 [`mqtt_client::create_mqtt_client`] 创建客户端和 `EventLoop`，
    /// 启动后台 poll 任务，将连接存入连接池。
    async fn connect(
        &self,
        uuid: &str,
        config: &MqttConnectConfig,
    ) -> Result<MqttConnectOutput, String> {
        info!("MQTT 开始连接: url={}, id={}", config.url, uuid);

        // UUID 去重：如果已存在直接返回Success
        {
            let conns = self.connections.read().await;
            if let Some(existing) = conns.get(uuid) {
                let s = existing.status.read().await;
                if *s == MqttConnectionStatus::Connected {
                    info!("MQTT 复用已有连接: id={}", uuid);
                    return Ok(MqttConnectOutput {
                        success: true,
                        broker_info: None,
                        reason_code: Some(0),
                        error: None,
                    });
                }
            }
        } // 读锁释放

        // 创建 MQTT 客户端
        let result = mqtt_client::create_mqtt_client(config)?;

        let status = Arc::new(RwLock::new(MqttConnectionStatus::Connecting));
        let message_rx = self.message_tx.subscribe();

        // 启动后台 EventLoop poll 任务
        let eventloop_handle = Self::spawn_eventloop_poll(
            result.eventloop,
            Arc::clone(&status),
            Arc::clone(&self.message_tx),
            uuid.to_string(),
        );

        let client_id = config
            .client_id
            .clone()
            .unwrap_or_else(|| "mpe-mqtt-client".to_string());

        let created_at = Self::now_ms();

        let connection = MqttConnection {
            client: result.client,
            eventloop_handle,
            url: config.url.clone(),
            client_id: client_id.clone(),
            status: Arc::clone(&status),
            created_at,
            message_rx: Arc::new(Mutex::new(message_rx)),
        };

        // 存入连接池
        {
            let mut conns = self.connections.write().await;
            info!("MQTT 添加连接: id={}", uuid);
            conns.insert(uuid.to_string(), connection);
        }

        // 等待连接状态变为 Connected 或 Error（带超时）
        let timeout_ms = config.connect_timeout_ms.unwrap_or(5000);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            {
                let s = status.read().await;
                match &*s {
                    MqttConnectionStatus::Connected => {
                        return Ok(MqttConnectOutput {
                            success: true,
                            broker_info: None,
                            reason_code: Some(0),
                            error: None,
                        });
                    }
                    MqttConnectionStatus::Error(e) => {
                        // connection failed，清理
                        let mut conns = self.connections.write().await;
                        if let Some(conn) = conns.remove(uuid) {
                            conn.eventloop_handle.abort();
                        }
                        return Ok(MqttConnectOutput {
                            success: false,
                            broker_info: None,
                            reason_code: None,
                            error: Some(e.clone()),
                        });
                    }
                    MqttConnectionStatus::Disconnected => {
                        let mut conns = self.connections.write().await;
                        if let Some(conn) = conns.remove(uuid) {
                            conn.eventloop_handle.abort();
                        }
                        return Ok(MqttConnectOutput {
                            success: false,
                            broker_info: None,
                            reason_code: None,
                            error: Some("连接被断开".to_string()),
                        });
                    }
                    MqttConnectionStatus::Connecting => {
                        // 继续等待
                    }
                }
            }

            if tokio::time::Instant::now() >= deadline {
                // 超时，不立即清理，让连接继续在后台尝试
                return Ok(MqttConnectOutput {
                    success: false,
                    broker_info: None,
                    reason_code: None,
                    error: Some(format!("连接超时（{}ms）", timeout_ms)),
                });
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    /// 发布消息
    ///
    /// 从连接池获取客户端，发布消息到指定主题。
    async fn publish(
        &self,
        uuid: &str,
        config: &MqttPublishConfig,
    ) -> Result<MqttPublishOutput, String> {
        // 从连接池获取客户端引用后释放锁
        let client = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("MQTT connection '{}' not found", uuid))?;
            conn.client.clone()
        }; // 读锁释放

        let topic = &config.topic;
        let qos = Self::u8_to_qos(config.qos.unwrap_or(0));
        let retain = config.retain.unwrap_or(false);

        // 编码 payload
        let payload = config
            .payload
            .as_deref()
            .unwrap_or("")
            .as_bytes()
            .to_vec();

        client
            .publish(topic, qos, retain, payload)
            .await
            .map_err(|e| format!("MQTT publish failed: {}", e))?;

        info!("MQTT 发布Success: topic={}, qos={}", topic, config.qos.unwrap_or(0));

        Ok(MqttPublishOutput {
            success: true,
            reason_code: Some(0),
            error: None,
        })
    }

    /// 订阅主题
    ///
    /// 订阅指定主题，等待确认后可选地收集消息。
    async fn subscribe(
        &self,
        uuid: &str,
        config: &MqttSubscribeConfig,
    ) -> Result<MqttSubscribeOutput, String> {
        // 从连接池获取客户端引用和消息接收端
        let (client, message_rx_arc) = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("MQTT connection '{}' not found", uuid))?;
            (conn.client.clone(), Arc::clone(&conn.message_rx))
        };

        let topic = &config.topic;
        let qos = Self::u8_to_qos(config.qos.unwrap_or(0));

        // 发送订阅请求
        client
            .subscribe(topic, qos)
            .await
            .map_err(|e| format!("MQTT subscribe failed: {}", e))?;

        info!("MQTT 订阅Success: topic={}, qos={}", topic, config.qos.unwrap_or(0));

        // 如果配置了 max_messages 或 collect_timeout_ms，收集消息
        let max_messages = config.max_messages.unwrap_or(0);
        let collect_timeout_ms = config.collect_timeout_ms.unwrap_or(0);

        if max_messages == 0 && collect_timeout_ms == 0 {
            return Ok(MqttSubscribeOutput {
                success: true,
                reason_code: Some(0),
                messages: vec![],
                error: None,
            });
        }

        // 收集消息
        let messages = Self::collect_messages(
            &message_rx_arc,
            max_messages,
            collect_timeout_ms,
        )
        .await;

        Ok(MqttSubscribeOutput {
            success: true,
            reason_code: Some(0),
            messages,
            error: None,
        })
    }

    /// 取消订阅
    async fn unsubscribe(
        &self,
        uuid: &str,
        _connection_id: &str,
        topic: &str,
    ) -> Result<(), String> {
        let client = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("MQTT connection '{}' not found", uuid))?;
            conn.client.clone()
        };

        client
            .unsubscribe(topic)
            .await
            .map_err(|e| format!("MQTT unsubscribe failed: {}", e))?;

        info!("MQTT 取消订阅Success: topic={}", topic);
        Ok(())
    }

    /// 等待消息
    ///
    /// 监听 broadcast 通道，收集匹配主题的消息。
    async fn wait_message(
        &self,
        uuid: &str,
        config: &MqttWaitMessageConfig,
    ) -> Result<MqttMessageOutput, String> {
        let message_rx_arc = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("MQTT connection '{}' not found", uuid))?;
            Arc::clone(&conn.message_rx)
        };

        let timeout_ms = config.timeout_ms.unwrap_or(30000);
        let max_messages = config.max_messages.unwrap_or(1);
        let start = std::time::Instant::now();

        let messages = Self::collect_messages(
            &message_rx_arc,
            max_messages,
            timeout_ms,
        )
        .await;

        let total_ms = start.elapsed().as_millis() as u64;
        let is_empty = messages.is_empty();
        let first_message_ms = if is_empty {
            None
        } else {
            // 粗略估计首条消息耗时
            Some(total_ms)
        };

        Ok(MqttMessageOutput {
            success: !is_empty,
            messages,
            timing: Some(MqttTimingInfo {
                total_ms,
                first_message_ms,
            }),
            error: if is_empty {
                Some(format!("Wait message timed out（{}ms）", timeout_ms))
            } else {
                None
            },
        })
    }

    /// 请求-响应模式
    ///
    /// 订阅响应主题，发布请求消息，等待匹配的响应，最后取消订阅。
    async fn request(
        &self,
        uuid: &str,
        config: &MqttRequestConfig,
    ) -> Result<MqttMessageOutput, String> {
        let (client, message_rx_arc) = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("MQTT connection '{}' not found", uuid))?;
            (conn.client.clone(), Arc::clone(&conn.message_rx))
        };

        let response_topic = &config.response_topic;
        let request_topic = &config.request_topic;
        let timeout_ms = config.timeout_ms.unwrap_or(30000);
        let qos = Self::u8_to_qos(config.qos.unwrap_or(0));

        let start = std::time::Instant::now();

        // 订阅响应主题
        client
            .subscribe(response_topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| format!("MQTT request subscribe response topic failed: {}", e))?;

        // 发布请求
        let payload = config
            .payload
            .as_deref()
            .unwrap_or("")
            .as_bytes()
            .to_vec();

        client
            .publish(request_topic, qos, false, payload)
            .await
            .map_err(|e| format!("MQTT request publish failed: {}", e))?;

        // 等待响应消息
        let messages = Self::collect_messages(&message_rx_arc, 1, timeout_ms).await;

        // 取消订阅响应主题
        let _ = client.unsubscribe(response_topic).await;

        let total_ms = start.elapsed().as_millis() as u64;
        let is_empty = messages.is_empty();
        let first_message_ms = if is_empty {
            None
        } else {
            Some(total_ms)
        };

        Ok(MqttMessageOutput {
            success: !is_empty,
            messages,
            timing: Some(MqttTimingInfo {
                total_ms,
                first_message_ms,
            }),
            error: if is_empty {
                Some(format!("Request-response timed out（{}ms）", timeout_ms))
            } else {
                None
            },
        })
    }

    /// 断开连接
    ///
    /// 发送断开请求，中止后台 `EventLoop` 任务，从连接池移除。
    /// 共享模式下为 no-op，避免并发虚拟用户互相关闭连接。
    async fn disconnect(
        &self,
        uuid: &str,
        _config: &MqttDisconnectConfig,
    ) -> Result<(), String> {
        if self.is_shared {
            info!("共享模式下忽略 disconnect 请求: {}", uuid);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(uuid);
        match removed {
            Some(conn) => {
                // 尝试发送 disconnect 请求
                let _ = conn.client.disconnect().await;
                // 中止后台任务（防止任务泄漏）
                conn.eventloop_handle.abort();
                info!("MQTT 断开连接: id={}", uuid);
                Ok(())
            }
            None => {
                warn!("MQTT 断开connection failed: 连接 '{}' not found", uuid);
                Err(format!("MQTT connection '{}' not found", uuid))
            }
        }
    }

    /// 获取连接状态
    ///
    /// 非阻塞读取连接池，返回状态字符串。
    fn get_status(&self, uuid: &str) -> Option<String> {
        let conns = self.connections.try_read().ok()?;
        let conn = conns.get(uuid)?;
        // 尝试非阻塞读取状态
        let status = conn.status.try_read().ok()?;
        Some(status.to_string())
    }

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<MqttConnectionInfo> {
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
                MqttConnectionInfo {
                    uuid: uuid.clone(),
                    url: conn.url.clone(),
                    client_id: conn.client_id.clone(),
                    status: status_str,
                    connected_at: conn.created_at,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 内部辅助方法
// ---------------------------------------------------------------------------

impl MqttConnectionManager {
    /// 从 broadcast 通道收集消息
    ///
    /// 在指定的超时时间内收集最多 `max_messages` 条消息。
    /// `max_messages=0` 表示不限制数量（仅受超时控制）。
    async fn collect_messages(
        message_rx: &Arc<Mutex<broadcast::Receiver<MqttReceivedMessage>>>,
        max_messages: usize,
        timeout_ms: u64,
    ) -> Vec<MqttReceivedMessage> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut collected = Vec::new();

        let mut rx = message_rx.lock().await;

        loop {
            if max_messages > 0 && collected.len() >= max_messages {
                break;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(msg)) => {
                    collected.push(msg);
                }
                Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                    warn!("MQTT 消息通道落后 {} 条消息，继续接收", n);
                    continue;
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    break;
                }
                Err(_) => {
                    // 超时
                    break;
                }
            }
        }

        collected
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = MqttConnectionManager::new();
        // 新创建的 manager 应该没有连接
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = MqttConnectionManager::new_shared();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = MqttConnectionManager::default();
        assert!(manager.list_connections().is_empty());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let config = MqttDisconnectConfig::default();
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_shared_mode_disconnect_is_noop() {
        let manager = MqttConnectionManager::new_shared();
        let config = MqttDisconnectConfig::default();
        // 共享模式下 disconnect not found的连接也返回 Ok
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_status_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_list_connections_empty() {
        let manager = MqttConnectionManager::new();
        let connections = manager.list_connections();
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_publish_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let config = MqttPublishConfig::default();
        let result = manager.publish("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_subscribe_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let config = MqttSubscribeConfig::default();
        let result = manager.subscribe("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_unsubscribe_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let result = manager.unsubscribe("non-existent", "conn-1", "test/topic").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_wait_message_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let config = MqttWaitMessageConfig::default();
        let result = manager.wait_message("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_request_nonexistent_connection() {
        let manager = MqttConnectionManager::new();
        let config = MqttRequestConfig::default();
        let result = manager.request("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // ---- 状态显示测试 ----

    #[test]
    fn test_connection_status_display() {
        assert_eq!(MqttConnectionStatus::Connecting.to_string(), "connecting");
        assert_eq!(MqttConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(
            MqttConnectionStatus::Disconnected.to_string(),
            "disconnected"
        );
        let err_status = MqttConnectionStatus::Error("timeout".to_string());
        assert!(err_status.to_string().contains("timeout"));
    }

    #[test]
    fn test_qos_conversion() {
        assert_eq!(MqttConnectionManager::qos_to_u8(QoS::AtMostOnce), 0);
        assert_eq!(MqttConnectionManager::qos_to_u8(QoS::AtLeastOnce), 1);
        assert_eq!(MqttConnectionManager::qos_to_u8(QoS::ExactlyOnce), 2);

        assert!(matches!(
            MqttConnectionManager::u8_to_qos(0),
            QoS::AtMostOnce
        ));
        assert!(matches!(
            MqttConnectionManager::u8_to_qos(1),
            QoS::AtLeastOnce
        ));
        assert!(matches!(
            MqttConnectionManager::u8_to_qos(2),
            QoS::ExactlyOnce
        ));
        // 无效 QoS 默认为 AtMostOnce
        assert!(matches!(
            MqttConnectionManager::u8_to_qos(99),
            QoS::AtMostOnce
        ));
    }
}
