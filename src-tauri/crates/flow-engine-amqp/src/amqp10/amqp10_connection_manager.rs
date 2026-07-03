//! AMQP 1.0 连接管理器
//!
//! 管理 AMQP 1.0 连接池，每个连接持有 fe2o3-amqp 的 Connection + Session + Links。
//! 使用 `Arc<RwLock<HashMap>>` 保护连接池，支持并发访问。
//!
//! # 架构
//!
//! ```text
//! Amqp10ConnectionManager
//!   └─ connections: Arc<RwLock<HashMap<String, Amqp10Connection>>>
//!        └─ Amqp10Connection
//!             ├─ connection: Mutex<Amqp10ConnectionHandle>
//!             ├─ session: Mutex<Amqp10SessionHandle>
//!             ├─ senders: Mutex<HashMap<String, Sender>>
//!             ├─ receivers: Mutex<HashMap<String, Receiver>>
//!             └─ pending_deliveries: Mutex<HashMap<u32, PendingDelivery>>
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use fe2o3_amqp::Delivery;
use log::{info, warn};
use tokio::sync::{Mutex, RwLock};

use flow_engine_core::node_config::{
    Amqp10AcceptConfig, Amqp10ConnectConfig, Amqp10DisconnectConfig, Amqp10ReceiveConfig,
    Amqp10RejectConfig, Amqp10SendConfig,
};
use flow_engine_core::traits::{
    Amqp10AcceptOutput, Amqp10ConnectOutput, Amqp10Message, Amqp10ReceiveOutput,
    Amqp10RejectOutput, Amqp10SendOutput, AmqpConnectionInfo,
};

use crate::amqp10::amqp10_client;
use crate::amqp10::amqp10_client::{Amqp10Connection as Amqp10ConnHandle, Amqp10Session};

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// AMQP 1.0 连接状态
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Amqp10ConnectionStatus {
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 已断开
    Disconnected,
    /// Error状态
    Error(String),
}

impl std::fmt::Display for Amqp10ConnectionStatus {
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
// PendingDelivery
// ---------------------------------------------------------------------------

/// 待处理的 Delivery（等待 Accept/Reject）
struct PendingDelivery {
    /// 原始 Delivery 对象（用于 accept/reject）
    delivery: Delivery<String>,
    /// 对应的 Receiver 地址
    receiver_address: String,
}

// ---------------------------------------------------------------------------
// Amqp10Connection
// ---------------------------------------------------------------------------

/// AMQP 1.0 连接
///
/// 持有 fe2o3-amqp 的 Connection + Session + Sender/Receiver Links。
/// 由于 fe2o3-amqp 的类型需要 `&mut self`，使用 `Mutex` 包装以保证线程安全。
struct Amqp10Connection {
    /// fe2o3-amqp 连接
    connection: Mutex<Amqp10ConnHandle>,
    /// fe2o3-amqp 会话
    session: Mutex<Amqp10Session>,
    /// 缓存的 Sender Links: `address` → `Sender`
    senders: Mutex<HashMap<String, fe2o3_amqp::Sender>>,
    /// 缓存的 Receiver Links: `address` → `Receiver`
    receivers: Mutex<HashMap<String, fe2o3_amqp::Receiver>>,
    /// 待处理的 Deliveries: `delivery_id` → `PendingDelivery`
    pending_deliveries: Mutex<HashMap<u32, PendingDelivery>>,
    /// Broker URL
    url: String,
    /// 连接状态
    status: Arc<RwLock<Amqp10ConnectionStatus>>,
    /// 创建时间（毫秒时间戳）
    #[allow(dead_code)]
    created_at: i64,
}

// ---------------------------------------------------------------------------
// Amqp10ConnectionManager
// ---------------------------------------------------------------------------

/// AMQP 1.0 连接管理器（per-flow-execution 作用域）
///
/// 管理 AMQP 1.0 连接池，线程安全。
/// 使用 `Arc<RwLock<HashMap>>` 保护连接池。
pub struct Amqp10ConnectionManager {
    /// 活跃连接池: `connection_id` → [`Amqp10Connection`]
    connections: Arc<RwLock<HashMap<String, Amqp10Connection>>>,
    /// 是否为共享模式（压力测试）
    is_shared: bool,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl Amqp10ConnectionManager {
    /// 创建新的连接管理器（普通模式）
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            is_shared: false,
        }
    }

    /// 创建共享模式的连接管理器（压测场景）
    ///
    /// 共享模式下 disconnect 为 no-op，避免并发虚拟用户互相关闭连接。
    pub fn new_shared() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            is_shared: true,
        }
    }
}

impl Default for Amqp10ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Amqp10ConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, conn) in connections.drain() {
                let url = conn.url.clone();
                // 关闭连接（忽略异步结果，drop 场景下无法 await）
                tokio::spawn(async move {
                    let mut connection = conn.connection.into_inner();
                    let _ = connection.close().await;
                    info!("AMQP 1.0 连接 {} ({}) 已随 Manager 释放而清理", id, url);
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 工具方法
// ---------------------------------------------------------------------------

impl Amqp10ConnectionManager {
    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

// ---------------------------------------------------------------------------
// AMQP 1.0 操作方法
// ---------------------------------------------------------------------------

impl Amqp10ConnectionManager {
    /// 建立 AMQP 1.0 连接
    pub async fn connect(
        &self,
        uuid: &str,
        config: &Amqp10ConnectConfig,
    ) -> Result<Amqp10ConnectOutput, String> {
        info!("AMQP 1.0 开始连接: url={}, id={}", config.url, uuid);

        // UUID 去重：如果已存在直接返回Success
        {
            let conns = self.connections.read().await;
            if let Some(existing) = conns.get(uuid) {
                let s = existing.status.read().await;
                if *s == Amqp10ConnectionStatus::Connected {
                    info!("AMQP 1.0 复用已有连接: id={}", uuid);
                    return Ok(Amqp10ConnectOutput {
                        success: true,
                        connection_id: uuid.to_string(),
                        error: None,
                    });
                }
            }
        } // 读锁释放

        // 确定容器 ID
        let container_id = match config.container_id.as_deref() {
            Some(id) => id.to_string(),
            None => amqp10_client::generate_container_id(),
        };

        // 创建连接
        let mut connection =
            amqp10_client::connect_async(&container_id, &config.url).await?;

        // 创建会话
        let session = amqp10_client::begin_session(&mut connection).await?;

        let status = Arc::new(RwLock::new(Amqp10ConnectionStatus::Connected));
        let created_at = Self::now_ms();

        let amqp_conn = Amqp10Connection {
            connection: Mutex::new(connection),
            session: Mutex::new(session),
            senders: Mutex::new(HashMap::new()),
            receivers: Mutex::new(HashMap::new()),
            pending_deliveries: Mutex::new(HashMap::new()),
            url: config.url.clone(),
            status: Arc::clone(&status),
            created_at,
        };

        // 存入连接池
        {
            let mut conns = self.connections.write().await;
            info!("AMQP 1.0 添加连接: id={}", uuid);
            conns.insert(uuid.to_string(), amqp_conn);
        }

        Ok(Amqp10ConnectOutput {
            success: true,
            connection_id: uuid.to_string(),
            error: None,
        })
    }

    /// AMQP 1.0 发送消息
    pub async fn send(
        &self,
        uuid: &str,
        config: &Amqp10SendConfig,
    ) -> Result<Amqp10SendOutput, String> {
        // 获取连接（读锁）
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

        // 获取或创建 Sender（需要先检查缓存）
        {
            let mut senders = conn.senders.lock().await;
            if let Some(sender) = senders.get_mut(&config.address) {
                // 发送消息
                let body = config.message_body.as_deref().unwrap_or("");

                // 如果有属性，使用带属性发送
                if let Some(props_config) = &config.message_properties {
                    let props = amqp10_client::build_properties(props_config);
                    amqp10_client::send_message_with_properties(sender, body, props).await?;
                } else {
                    amqp10_client::send_string(sender, body).await?;
                }

                return Ok(Amqp10SendOutput {
                    success: true,
                    error: None,
                });
            }
        }

        // 缓存中无 Sender，需要创建
        drop(conns); // 释放读锁

        // 获取连接（重新获取读锁）
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

        // 创建新 Sender
        let sender = {
            let mut session = conn.session.lock().await;
            let link_name = amqp10_client::generate_link_name("sender");
            amqp10_client::attach_sender(&mut session, &link_name, &config.address).await?
        };

        // 存入缓存
        conn.senders.lock().await.insert(config.address.clone(), sender);

        // 再次获取 sender 发送
        let mut senders = conn.senders.lock().await;
        let sender = senders
            .get_mut(&config.address)
            .expect("刚刚插入的 sender");

        let body = config.message_body.as_deref().unwrap_or("");
        if let Some(props_config) = &config.message_properties {
            let props = amqp10_client::build_properties(props_config);
            amqp10_client::send_message_with_properties(sender, body, props).await?;
        } else {
            amqp10_client::send_string(sender, body).await?;
        }

        Ok(Amqp10SendOutput {
            success: true,
            error: None,
        })
    }

    /// AMQP 1.0 接收消息
    pub async fn receive(
        &self,
        uuid: &str,
        config: &Amqp10ReceiveConfig,
    ) -> Result<Amqp10ReceiveOutput, String> {
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

        // 获取或创建 Receiver
        {
            let mut receivers = conn.receivers.lock().await;
            if receivers.get_mut(&config.address).is_none() {
                // 需要创建新 Receiver
                drop(receivers);
                drop(conns); // 释放锁

                let conns = self.connections.read().await;
                let conn = conns
                    .get(uuid)
                    .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

                let receiver = {
                    let mut session = conn.session.lock().await;
                    let link_name = amqp10_client::generate_link_name("receiver");
                    amqp10_client::attach_receiver(&mut session, &link_name, &config.address)
                        .await?
                };

                conn.receivers.lock().await.insert(config.address.clone(), receiver);
            }
        }

        // 现在获取 receiver 接收消息
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

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

        let mut receivers = conn.receivers.lock().await;
        let receiver = receivers
            .get_mut(&config.address)
            .ok_or_else(|| format!("AMQP 1.0 Receiver '{}' 不存在", config.address))?;

        loop {
            if messages.len() >= max_messages {
                break;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, amqp10_client::receive_string(receiver)).await {
                Ok(Ok(delivery)) => {
                    let delivery_id = delivery.delivery_id();
                    let body_str = delivery.body().clone();
                    let body_decoded = serde_json::from_str::<serde_json::Value>(&body_str).ok();

                    let properties = amqp10_client::extract_delivery_properties(&delivery);
                    let annotations = amqp10_client::extract_delivery_annotations(&delivery);

                    let msg = Amqp10Message {
                        properties,
                        body: body_str,
                        body_decoded,
                        message_annotations: annotations,
                        delivery_id: Some(*delivery_id as u64),
                    };

                    // 自动接受
                    if config.auto_accept {
                        let _ = amqp10_client::accept_delivery(receiver, &delivery).await;
                    } else {
                        // 存入 pending_deliveries 供后续 accept/reject
                        let mut pending = conn.pending_deliveries.lock().await;
                        pending.insert(
                            *delivery_id,
                            PendingDelivery {
                                delivery,
                                receiver_address: config.address.clone(),
                            },
                        );
                    }

                    messages.push(msg);
                }
                Ok(Err(_)) | Err(_) => {
                    // 接收Error或超时
                    break;
                }
            }
        }

        let message_count = messages.len() as u32;

        Ok(Amqp10ReceiveOutput {
            success: !messages.is_empty(),
            messages,
            message_count,
            error: None,
        })
    }

    /// AMQP 1.0 确认消息
    pub async fn accept(
        &self,
        uuid: &str,
        config: &Amqp10AcceptConfig,
    ) -> Result<Amqp10AcceptOutput, String> {
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

        // 取出 pending delivery
        let pending = {
            let mut pending_map = conn.pending_deliveries.lock().await;
            pending_map.remove(&(config.delivery_id as u32))
        };

        match pending {
            Some(pending) => {
                let mut receivers = conn.receivers.lock().await;
                let receiver = receivers
                    .get_mut(&pending.receiver_address)
                    .ok_or_else(|| {
                        format!(
                            "AMQP 1.0 Receiver '{}' 不存在（连接 {}）",
                            pending.receiver_address, uuid
                        )
                    })?;

                amqp10_client::accept_delivery(receiver, &pending.delivery).await?;

                Ok(Amqp10AcceptOutput {
                    success: true,
                    error: None,
                })
            }
            None => Err(format!(
                "AMQP 1.0 delivery_id {} 不存在或已处理（连接 {}）",
                config.delivery_id, uuid
            )),
        }
    }

    /// AMQP 1.0 拒绝消息
    pub async fn reject(
        &self,
        uuid: &str,
        config: &Amqp10RejectConfig,
    ) -> Result<Amqp10RejectOutput, String> {
        let conns = self.connections.read().await;
        let conn = conns
            .get(uuid)
            .ok_or_else(|| format!("AMQP 1.0 连接 '{}' 不存在", uuid))?;

        // 取出 pending delivery
        let pending = {
            let mut pending_map = conn.pending_deliveries.lock().await;
            pending_map.remove(&(config.delivery_id as u32))
        };

        match pending {
            Some(pending) => {
                let mut receivers = conn.receivers.lock().await;
                let receiver = receivers
                    .get_mut(&pending.receiver_address)
                    .ok_or_else(|| {
                        format!(
                            "AMQP 1.0 Receiver '{}' 不存在（连接 {}）",
                            pending.receiver_address, uuid
                        )
                    })?;

                // 构建可选的 Error（暂不传 error condition，简单实现）
                let _ = &config.error_condition;
                let _ = &config.error_description;
                amqp10_client::reject_delivery(receiver, &pending.delivery, None).await?;

                Ok(Amqp10RejectOutput {
                    success: true,
                    error: None,
                })
            }
            None => Err(format!(
                "AMQP 1.0 delivery_id {} 不存在或已处理（连接 {}）",
                config.delivery_id, uuid
            )),
        }
    }

    /// AMQP 1.0 断开连接
    pub async fn disconnect(
        &self,
        uuid: &str,
        _config: &Amqp10DisconnectConfig,
    ) -> Result<(), String> {
        if self.is_shared {
            info!("共享模式下忽略 disconnect 请求: {}", uuid);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(uuid);
        match removed {
            Some(conn) => {
                // 关闭所有 Sender
                let senders = conn.senders.into_inner();
                for (_, sender) in senders {
                    let _ = sender.close().await;
                }

                // 关闭所有 Receiver
                let receivers = conn.receivers.into_inner();
                for (_, receiver) in receivers {
                    let _ = receiver.close().await;
                }

                // 关闭 Session
                let mut session = conn.session.into_inner();
                let _ = session.end().await;

                // 关闭 Connection
                let mut connection = conn.connection.into_inner();
                let _ = connection.close().await;

                info!("AMQP 1.0 断开连接: id={}", uuid);
                Ok(())
            }
            None => {
                warn!("AMQP 1.0 断开connection failed: 连接 '{}' 不存在", uuid);
                Err(format!("AMQP 1.0 连接 '{}' 不存在", uuid))
            }
        }
    }

    // ===== 通用操作 =====

    /// 获取连接状态
    pub fn get_status(&self, uuid: &str) -> Option<String> {
        let conns = self.connections.try_read().ok()?;
        let conn = conns.get(uuid)?;
        let status = conn.status.try_read().ok()?;
        Some(status.to_string())
    }

    /// 列出所有连接信息
    pub fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
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
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = Amqp10ConnectionManager::new();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = Amqp10ConnectionManager::new_shared();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = Amqp10ConnectionManager::default();
        assert!(manager.list_connections().is_empty());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10DisconnectConfig {
            connection_id: "non-existent".to_string(),
        };
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_shared_mode_disconnect_is_noop() {
        let manager = Amqp10ConnectionManager::new_shared();
        let config = Amqp10DisconnectConfig {
            connection_id: "non-existent".to_string(),
        };
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_status_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_list_connections_empty() {
        let manager = Amqp10ConnectionManager::new();
        let connections = manager.list_connections();
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_send_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10SendConfig::default();
        let result = manager.send("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_receive_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10ReceiveConfig::default();
        let result = manager.receive("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_accept_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10AcceptConfig::default();
        let result = manager.accept("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_reject_nonexistent_connection() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10RejectConfig::default();
        let result = manager.reject("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    // ---- 状态显示测试 ----

    #[test]
    fn test_connection_status_display() {
        assert_eq!(Amqp10ConnectionStatus::Connecting.to_string(), "connecting");
        assert_eq!(Amqp10ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(
            Amqp10ConnectionStatus::Disconnected.to_string(),
            "disconnected"
        );
        let err_status = Amqp10ConnectionStatus::Error("timeout".to_string());
        assert!(err_status.to_string().contains("timeout"));
    }

    // ---- 内部锁竞争测试 ----

    #[tokio::test]
    async fn test_concurrent_status_checks() {
        let manager = Arc::new(Amqp10ConnectionManager::new());

        let mut handles = vec![];
        for i in 0..5 {
            let m = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                m.get_status(&format!("conn-{}", i))
            }));
        }

        for handle in handles {
            assert!(handle.await.expect("task panicked").is_none());
        }
    }

    // ---- accept/reject 无 pending delivery 测试 ----

    #[tokio::test]
    async fn test_accept_no_pending_delivery() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10AcceptConfig {
            connection_id: "any".to_string(),
            delivery_id: 42,
        };
        let result = manager.accept("any", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reject_no_pending_delivery() {
        let manager = Amqp10ConnectionManager::new();
        let config = Amqp10RejectConfig {
            connection_id: "any".to_string(),
            delivery_id: 42,
            error_condition: Some("amqp:not-found".to_string()),
            error_description: Some("test error".to_string()),
        };
        let result = manager.reject("any", &config).await;
        assert!(result.is_err());
    }
}
