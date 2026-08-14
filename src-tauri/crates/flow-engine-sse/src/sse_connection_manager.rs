//! SSE 连接管理器
//!
//! 实现 [`SseProvider`] trait，管理 SSE 连接的生命周期。
//! 使用 reqwest 流式 GET 请求接收 SSE 事件，
//! 后台 tokio 任务持续解析事件并通过 broadcast 通道分发给多个消费者。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{info, warn};
use tokio::sync::{broadcast, RwLock};

use flow_engine_core::node_config::{SseConnectConfig, SseDisconnectConfig, SseListenConfig};
use flow_engine_core::traits::{
    SseConnectOutput, SseConnectionInfo, SseEventItem, SseListenOutput, SseProvider,
};

use crate::sse_client::{SseClient, SseClientConfig, SseEvent};

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// SSE 连接状态
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum SseConnectionStatus {
    /// 正在连接
    Connecting,
    /// 已连接（正在接收事件流）
    Connected,
    /// 已断开
    Disconnected,
    /// Error状态
    Error(String),
}

impl std::fmt::Display for SseConnectionStatus {
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
// SseConnection
// ---------------------------------------------------------------------------

/// SSE 连接
///
/// 持有连接元数据、后台事件读取任务句柄和 broadcast 事件通道。
pub struct SseConnection {
    /// SSE 端点 URL
    pub url: String,
    /// 连接状态
    status: Arc<RwLock<SseConnectionStatus>>,
    /// 创建时间（毫秒时间戳）
    pub created_at: i64,
    /// 自定义请求头
    pub headers: HashMap<String, String>,
    /// 最后接收到的事件 ID（用于断线续传）
    pub last_event_id: Arc<RwLock<Option<String>>>,
    /// 后台事件读取任务句柄
    listen_handle: Option<tokio::task::JoinHandle<()>>,
    /// 事件广播发送端（多个 listen 可同时订阅）
    event_tx: broadcast::Sender<SseEvent>,
    /// 取消令牌
    cancel_token: tokio_util::sync::CancellationToken,
}

// ---------------------------------------------------------------------------
// SseConnectionManager
// ---------------------------------------------------------------------------

/// SSE 连接管理器（per-flow-execution 作用域）
///
/// 管理 SSE 连接池，实现 `SseProvider` trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
/// 事件通过 `broadcast` 通道分发给多个消费者（listen 节点）。
pub struct SseConnectionManager {
    /// 活跃连接池: `connection_id` → [`SseConnection`]
    connections: Arc<RwLock<HashMap<String, SseConnection>>>,
    /// 是否为共享模式（压力测试）
    is_shared: bool,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl SseConnectionManager {
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

impl Default for SseConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SseConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, conn) in connections.drain() {
                conn.cancel_token.cancel();
                if let Some(handle) = conn.listen_handle {
                    handle.abort();
                }
                info!("SSE connection {} cleaned up on manager release", id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 工具方法
// ---------------------------------------------------------------------------

impl SseConnectionManager {
    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    /// 将 `SseEvent` 转换为 `SseEventItem`，可选 JSON 解码
    fn sse_event_to_item(event: &SseEvent, decode_json: bool) -> SseEventItem {
        let data_decoded = if decode_json {
            serde_json::from_str::<serde_json::Value>(&event.data).ok()
        } else {
            None
        };

        SseEventItem {
            event: event.event.clone(),
            id: event.id.clone(),
            data: event.data.clone(),
            data_decoded,
            received_at: flow_engine_core::utils::unix_timestamp_ms(),
        }
    }
}

// ---------------------------------------------------------------------------
// SseProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl SseProvider for SseConnectionManager {
    /// 建立 SSE 连接
    ///
    /// 使用 reqwest 发送 GET 请求，启动后台事件读取任务，
    /// 将连接存入连接池。
    async fn connect(
        &self,
        uuid: &str,
        config: &SseConnectConfig,
    ) -> Result<SseConnectOutput, String> {
        info!("SSE connecting: url={}, id={}", config.url, uuid);

        // UUID 去重：如果已存在且已连接，直接返回Success
        {
            let conns = self.connections.read().await;
            if let Some(existing) = conns.get(uuid) {
                let s = existing.status.read().await;
                if *s == SseConnectionStatus::Connected {
                    info!("SSE reusing existing connection: id={}", uuid);
                    let last_id = existing.last_event_id.read().await.clone();
                    return Ok(SseConnectOutput {
                        success: true,
                        connection_id: uuid.to_string(),
                        server_headers: None,
                        last_event_id: last_id,
                        error: None,
                    });
                }
            }
        } // 读锁释放

        // 构建客户端配置
        let client_config = SseClientConfig {
            url: config.url.clone(),
            headers: config.headers.clone().unwrap_or_default(),
            last_event_id: config.last_event_id.clone(),
            timeout: if config.timeout_ms > 0 {
                Duration::from_millis(config.timeout_ms)
            } else {
                Duration::from_secs(30)
            },
        };

        // 发送 SSE 连接请求
        let response = match SseClient::connect(&client_config).await {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(SseConnectOutput {
                    success: false,
                    connection_id: uuid.to_string(),
                    server_headers: None,
                    last_event_id: None,
                    error: Some(e),
                });
            }
        };

        // 提取服务器响应头
        let server_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // 创建 broadcast 事件通道（允许同时有多个 listen 订阅）
        let (event_tx, _) = broadcast::channel::<SseEvent>(256);
        let cancel_token = tokio_util::sync::CancellationToken::new();

        // 设置连接状态
        let status = Arc::new(RwLock::new(SseConnectionStatus::Connected));

        // 启动后台事件读取任务
        let listen_handle =
            SseClient::spawn_event_reader(response, cancel_token.clone(), event_tx.clone());

        let created_at = Self::now_ms();
        let last_event_id = Arc::new(RwLock::new(config.last_event_id.clone()));

        let connection = SseConnection {
            url: config.url.clone(),
            status: Arc::clone(&status),
            created_at,
            headers: config.headers.clone().unwrap_or_default(),
            last_event_id: Arc::clone(&last_event_id),
            listen_handle: Some(listen_handle),
            event_tx,
            cancel_token,
        };

        // 存入连接池
        {
            let mut conns = self.connections.write().await;
            info!("SSE adding connection: id={}", uuid);
            conns.insert(uuid.to_string(), connection);
        }

        Ok(SseConnectOutput {
            success: true,
            connection_id: uuid.to_string(),
            server_headers: Some(server_headers),
            last_event_id: config.last_event_id.clone(),
            error: None,
        })
    }

    /// 监听 SSE 事件
    ///
    /// 从已建立的连接获取 broadcast 接收端，在指定超时内收集事件，
    /// 支持事件过滤、最大事件数限制和 JSON 解码。
    async fn listen(
        &self,
        uuid: &str,
        config: &SseListenConfig,
    ) -> Result<SseListenOutput, String> {
        let start = std::time::Instant::now();

        // 从连接池获取 broadcast 发送端并克隆（以便释放读锁）
        let event_tx = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("SSE connection '{}' does not exist", uuid))?;
            conn.event_tx.clone()
        };

        let mut rx = event_tx.subscribe();
        let event_filter = config.event_filter.clone();
        let max_events = config.max_events;
        let timeout_ms = config.timeout_ms;
        let decode_json = config.decode_json;

        // 计算超时时刻
        let timeout = if timeout_ms > 0 {
            Duration::from_millis(timeout_ms)
        } else {
            Duration::from_secs(3600) // 0 = 无限等待
        };
        let deadline = tokio::time::Instant::now() + timeout;

        let mut events: Vec<SseEventItem> = Vec::new();

        loop {
            // 检查是否已超时
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(event)) => {
                    // 事件过滤
                    if let Some(ref filter) = event_filter {
                        let event_type = event.event.as_deref().unwrap_or("message");
                        if event_type != filter.as_str() {
                            continue;
                        }
                    }

                    // 转换为 SseEventItem
                    let item = Self::sse_event_to_item(&event, decode_json);
                    events.push(item);

                    // 最大事件数限制
                    if max_events > 0 && events.len() >= max_events as usize {
                        break;
                    }
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    // 通道已关闭（连接被断开）
                    break;
                }
                Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                    warn!("SSE receiver lagged, dropped {} events", n);
                    continue;
                }
                Err(_elapsed) => {
                    // 超时
                    break;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let event_count = events.len() as u32;

        Ok(SseListenOutput {
            success: true,
            events,
            event_count,
            duration_ms,
            error: None,
        })
    }

    /// 断开 SSE 连接
    ///
    /// 取消后台任务，从连接池移除。
    /// 共享模式下为 no-op。
    async fn disconnect(&self, uuid: &str, _config: &SseDisconnectConfig) -> Result<(), String> {
        if self.is_shared {
            info!("ignoring disconnect request in shared mode: {}", uuid);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(uuid);
        match removed {
            Some(conn) => {
                conn.cancel_token.cancel();
                if let Some(handle) = conn.listen_handle {
                    handle.abort();
                }
                info!("SSE disconnecting: id={}", uuid);
                Ok(())
            }
            None => {
                warn!("SSE disconnect failed: connection '{}' does not exist", uuid);
                Err(format!("SSE connection '{}' does not exist", uuid))
            }
        }
    }

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String> {
        let conns = self.connections.try_read().ok()?;
        let conn = conns.get(uuid)?;
        let status = conn.status.try_read().ok()?;
        Some(status.to_string())
    }

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<SseConnectionInfo> {
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
                SseConnectionInfo {
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
        let manager = SseConnectionManager::new();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = SseConnectionManager::new_shared();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = SseConnectionManager::default();
        assert!(manager.list_connections().is_empty());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = SseConnectionManager::new();
        let config = SseDisconnectConfig::default();
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn test_shared_mode_disconnect_is_noop() {
        let manager = SseConnectionManager::new_shared();
        let config = SseDisconnectConfig::default();
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_status_nonexistent_connection() {
        let manager = SseConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_list_connections_empty() {
        let manager = SseConnectionManager::new();
        let connections = manager.list_connections();
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_listen_nonexistent_connection() {
        let manager = SseConnectionManager::new();
        let config = SseListenConfig::default();
        let result = manager.listen("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    // ---- 状态显示测试 ----

    #[test]
    fn test_connection_status_display() {
        assert_eq!(SseConnectionStatus::Connecting.to_string(), "connecting");
        assert_eq!(SseConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(
            SseConnectionStatus::Disconnected.to_string(),
            "disconnected"
        );
        let err_status = SseConnectionStatus::Error("timeout".to_string());
        assert!(err_status.to_string().contains("timeout"));
    }
}
