// WebSocket 连接管理器模块
//
// 管理 WebSocket 连接池，提供连接建立、关闭、消息收发功能。
// Wave 1: 基础数据结构
// Wave 2: 核心连接管理逻辑

use chrono::{DateTime, Utc};
use evalexpr::{ContextWithMutableVariables, HashMapContext, Value};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use http::Request;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json_path::JsonPath;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::Message as WebSocketMessage;
use tokio_tungstenite::{connect_async_tls_with_config, MaybeTlsStream, WebSocketStream};

use flow_engine_core::node_config::{FormField, WsMatchType, WsWaitCondition};

/// WebSocket 流类型（支持 TLS）
type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// WebSocket 发送端类型
type WsSender = SplitSink<WsStream, WebSocketMessage>;

/// WebSocket 接收端类型
type WsReceiver = SplitStream<WsStream>;

/// 连接状态
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// 正在连接
    #[default]
    Connecting,
    /// 已连接
    Connected,
    /// 正在关闭
    Closing,
    /// 已关闭
    Closed,
    /// 连接错误
    Error,
}

/// 活跃连接
///
/// 存储单个 WebSocket 连接的状态、元数据和实际的 WebSocket 流。
pub struct ActiveConnection {
    /// 节点 UUID（连接的唯一标识）
    pub uuid: String,
    /// 节点名称（作为连接的可读标识符）
    pub name: String,
    /// 连接状态
    pub status: ConnectionStatus,
    /// 连接创建时间
    pub created_at: DateTime<Utc>,
    /// WebSocket 服务端 URL
    pub url: String,
    /// 消息缓冲区（用于暂存待处理消息）
    pub message_buffer: VecDeque<WebSocketMessage>,
    /// 最后错误消息（如果有）
    pub last_error: Option<String>,

    /// 当前持有者节点 UUID（用于消息流所有权追踪）
    pub current_owner_node_uuid: Option<String>,

    /// 流段 ID（用于关联 `WsStreamSegment`）
    pub stream_segment_id: Option<String>,

    // WebSocket 连接（通过 Arc<Mutex> 实现线程安全共享）
    sender: Option<Arc<tokio::sync::Mutex<WsSender>>>,
    receiver: Option<Arc<tokio::sync::Mutex<WsReceiver>>>,
}

impl std::fmt::Debug for ActiveConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveConnection")
            .field("uuid", &self.uuid)
            .field("name", &self.name)
            .field("status", &self.status)
            .field("created_at", &self.created_at)
            .field("url", &self.url)
            .field("buffer_size", &self.message_buffer.len())
            .field("last_error", &self.last_error)
            .field("current_owner_node_uuid", &self.current_owner_node_uuid)
            .field("stream_segment_id", &self.stream_segment_id)
            .field("has_sender", &self.sender.is_some())
            .field("has_receiver", &self.receiver.is_some())
            .finish()
    }
}

impl ActiveConnection {
    /// 创建新的活跃连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    /// * `name` - 节点名称
    /// * `url` - WebSocket URL
    ///
    /// # 返回
    ///
    /// 初始化的活跃连接，状态为 Connecting
    pub fn new(uuid: String, name: String, url: String) -> Self {
        Self {
            uuid,
            name,
            status: ConnectionStatus::Connecting,
            created_at: Utc::now(),
            url,
            message_buffer: VecDeque::new(),
            last_error: None,
            current_owner_node_uuid: None,
            stream_segment_id: None,
            sender: None,
            receiver: None,
        }
    }

    /// 更新连接状态
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    /// 设置错误状态和消息
    pub fn set_error(&mut self, error: String) {
        self.status = ConnectionStatus::Error;
        self.last_error = Some(error);
    }

    /// 添加消息到缓冲区
    pub fn push_message(&mut self, message: WebSocketMessage) {
        self.message_buffer.push_back(message);
    }

    /// 从缓冲区取出消息
    pub fn pop_message(&mut self) -> Option<WebSocketMessage> {
        self.message_buffer.pop_front()
    }

    /// 获取缓冲区消息数量
    pub fn buffer_size(&self) -> usize {
        self.message_buffer.len()
    }

    /// 设置 WebSocket 流（连接成功后调用）
    pub fn set_stream(&mut self, sender: WsSender, receiver: WsReceiver) {
        self.sender = Some(Arc::new(tokio::sync::Mutex::new(sender)));
        self.receiver = Some(Arc::new(tokio::sync::Mutex::new(receiver)));
        self.status = ConnectionStatus::Connected;
    }

    /// 获取发送端的克隆
    pub fn get_sender(&self) -> Option<Arc<tokio::sync::Mutex<WsSender>>> {
        self.sender.clone()
    }

    /// 获取接收端的克隆
    pub fn get_receiver(&self) -> Option<Arc<tokio::sync::Mutex<WsReceiver>>> {
        self.receiver.clone()
    }

    /// 检查是否有活跃的 WebSocket 连接
    pub fn is_connected(&self) -> bool {
        self.status == ConnectionStatus::Connected
            && self.sender.is_some()
            && self.receiver.is_some()
    }

    /// 清理 WebSocket 资源
    pub fn clear_stream(&mut self) {
        self.sender = None;
        self.receiver = None;
    }

    /// 设置所有权
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 持有者节点 UUID
    /// * `segment_id` - 流段 ID
    pub fn set_owner(&mut self, node_uuid: &str, segment_id: &str) {
        self.current_owner_node_uuid = Some(node_uuid.to_string());
        self.stream_segment_id = Some(segment_id.to_string());
    }

    /// 释放所有权
    pub fn release_owner(&mut self) {
        self.current_owner_node_uuid = None;
        self.stream_segment_id = None;
    }
}

/// WebSocket 连接管理器
///
/// 管理所有活跃的 WebSocket 连接，提供线程安全的连接池操作。
/// 使用 Arc<Mutex<>> 实现跨线程共享。
#[derive(Debug, Clone)]
pub struct WebSocketConnectionManager {
    /// 连接池：UUID -> `ActiveConnection`
    connections: Arc<Mutex<HashMap<String, ActiveConnection>>>,
}

impl WebSocketConnectionManager {
    /// 创建新的连接管理器
    ///
    /// # 返回
    ///
    /// 初始化的连接管理器，包含空的连接池
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 建立 WebSocket 连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID（连接标识）
    /// * `name` - 节点名称（可读标识）
    /// * `url` - WebSocket URL (ws:// 或 wss://)
    /// * `headers` - 自定义请求头
    /// * `subprotocols` - WebSocket 子协议列表
    /// * `connect_timeout_ms` - 连接超时（毫秒）
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接成功
    /// * `Err(msg)` - 连接失败，包含错误消息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let manager = WebSocketConnectionManager::new();
    /// manager.connect("conn-1", "测试连接", "wss://echo.websocket.org", vec![], vec![], 30000).await?;
    /// ```
    pub async fn connect(
        &self,
        uuid: String,
        name: String,
        url: String,
        headers: Vec<FormField>,
        subprotocols: Vec<String>,
        connect_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 创建 ActiveConnection (status: Connecting)
        let mut connection = ActiveConnection::new(uuid.clone(), name, url.clone());

        // 2. 验证 URL 格式
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            let error_msg = format!("无效的 WebSocket URL（必须以 ws:// 或 wss:// 开头）: {}", url);
            connection.set_error(error_msg.clone());
            self.upsert_connection(connection);
            return Err(error_msg);
        }

        // 3. 临时存储连接（状态为 Connecting）
        self.upsert_connection(connection);

        // 4. 构建带 headers 的 WebSocket 请求
        let request = self.build_ws_request(&url, &headers, &subprotocols)?;

        // 5. 使用 tokio::time::timeout 包装 connect_async_tls_with_config
        let connect_timeout = Duration::from_millis(connect_timeout_ms.clamp(1000, 60000));
        let connect_result = timeout(
            connect_timeout,
            connect_async_tls_with_config(request, None, false, None),
        )
        .await;

        match connect_result {
            Ok(Ok((ws_stream, _response))) => {
                // 6. 连接成功，分割流
                let (sender, receiver) = ws_stream.split();

                // 7. 更新连接状态
                let mut connections = self.connections.lock().map_err(|e| {
                    format!("获取连接锁失败: {}", e)
                })?;

                if let Some(conn) = connections.get_mut(&uuid) {
                    conn.set_stream(sender, receiver);
                    log::info!(
                        "[WebSocket] 连接成功: uuid={}, url={}",
                        uuid,
                        url
                    );
                }
                Ok(())
            }
            Ok(Err(e)) => {
                // 连接失败（网络错误、TLS 错误等）
                let error_msg = format!("WebSocket 连接失败 ({}): {}", url, e);
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                // 连接超时
                let error_msg = format!(
                    "WebSocket 连接超时 ({}ms): {}",
                    connect_timeout_ms, url
                );
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
        }
    }

    /// 构建 WebSocket 握手请求
    ///
    /// # 参数
    ///
    /// * `url` - WebSocket URL
    /// * `headers` - 自定义请求头
    /// * `subprotocols` - WebSocket 子协议列表
    ///
    /// # 返回
    ///
    /// 构建好的 HTTP Request 对象
    fn build_ws_request(
        &self,
        url: &str,
        headers: &[FormField],
        subprotocols: &[String],
    ) -> Result<Request<()>, String> {
        // 1. 解析 URL 获取 host 和 port
        let uri: http::Uri = url.parse().map_err(|e| format!("解析 URL 失败: {}", e))?;

        let host = uri.host().ok_or("URL 缺少 host")?;
        let default_port = if uri.scheme_str() == Some("wss") {
            443
        } else {
            80
        };
        let port = uri.port_u16().unwrap_or(default_port);

        // 2. 构建基础请求
        let mut builder = Request::builder()
            .method("GET")
            .uri(url)
            .header("Host", format!("{}:{}", host, port))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key());

        // 3. 添加自定义 headers
        for header in headers {
            builder = builder.header(&header.key, &header.value);
        }

        // 4. 添加子协议（Sec-WebSocket-Protocol）
        if !subprotocols.is_empty() {
            builder = builder.header("Sec-WebSocket-Protocol", subprotocols.join(", "));
        }

        // 5. 构建请求体
        builder.body(()).map_err(|e| format!("构建 WebSocket 请求失败: {}", e))
    }

    /// 关闭 WebSocket 连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 连接 UUID
    /// * `close_code` - 关闭码（默认 1000）
    /// * `close_reason` - 关闭原因（可选）
    /// * `wait_close_timeout_ms` - 等待关闭超时
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 关闭成功
    /// * `Err(msg)` - 关闭失败或连接不存在
    pub async fn close(
        &self,
        uuid: &str,
        close_code: u16,
        close_reason: Option<String>,
        wait_close_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 获取连接，检查是否存在
        let sender = {
            let connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;

            let conn = connections.get(uuid).ok_or_else(|| {
                format!("连接不存在: {}", uuid)
            })?;

            // 2. 检查连接状态
            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "连接状态不是 Connected（当前状态: {:?}）: {}",
                    conn.status, uuid
                ));
            }

            // 3. 获取发送端克隆
            conn.get_sender().ok_or_else(|| {
                format!("连接没有活跃的 WebSocket 流: {}", uuid)
            })?
        };

        // 4. 更新状态为 Closing
        self.update_status(uuid, ConnectionStatus::Closing)?;

        // 5. 发送 Close 消息
        let close_msg = WebSocketMessage::Close(Some(
            tokio_tungstenite::tungstenite::protocol::frame::CloseFrame {
                code: CloseCode::from(close_code),
                reason: close_reason.unwrap_or_default().into(),
            },
        ));

        {
            let mut sender_guard = sender.lock().await;
            if let Err(e) = sender_guard.send(close_msg).await {
                let error_msg = format!("发送关闭消息失败: {}", e);
                self.update_connection_error(uuid, &error_msg);
                return Err(error_msg);
            }
        }

        // 6. 等待服务器确认关闭（超时控制）
        let receiver = {
            let connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;
            connections
                .get(uuid)
                .and_then(|c| c.get_receiver())
        };

        if let Some(receiver) = receiver {
            let close_timeout = Duration::from_millis(wait_close_timeout_ms.clamp(1000, 30000));

            let mut receiver_guard = receiver.lock().await;
            match timeout(close_timeout, receiver_guard.next()).await {
                Ok(Some(Ok(WebSocketMessage::Close(_)))) => {
                    log::info!("[WebSocket] 收到服务器关闭确认: uuid={}", uuid);
                }
                Ok(Some(Ok(_))) => {
                    // 收到其他消息，忽略
                }
                Ok(Some(Err(e))) => {
                    log::warn!("[WebSocket] 等待关闭确认时出错: {}", e);
                }
                Ok(None) => {
                    // 流已结束
                }
                Err(_) => {
                    log::warn!(
                        "[WebSocket] 等待关闭确认超时 ({}ms): uuid={}",
                        wait_close_timeout_ms,
                        uuid
                    );
                }
            }
        }

        // 7. 更新状态为 Closed
        self.update_status(uuid, ConnectionStatus::Closed)?;

        // 8. 清理资源
        {
            let mut connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;
            if let Some(conn) = connections.get_mut(uuid) {
                conn.clear_stream();
            }
        }

        log::info!("[WebSocket] 连接已关闭: uuid={}, code={}", uuid, close_code);
        Ok(())
    }

    /// 发送消息
    ///
    /// # 参数
    ///
    /// * `uuid` - 连接 UUID
    /// * `message` - 消息内容
    /// * `is_binary` - 是否为二进制消息
    /// * `send_timeout_ms` - 发送超时
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 发送成功
    /// * `Err(msg)` - 发送失败或连接不存在
    pub async fn send(
        &self,
        uuid: &str,
        message: String,
        is_binary: bool,
        send_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 获取连接，检查状态是否为 Connected
        let sender = {
            let connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;

            let conn = connections.get(uuid).ok_or_else(|| {
                format!("连接不存在: {}", uuid)
            })?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "连接状态不是 Connected（当前状态: {:?}）: {}",
                    conn.status, uuid
                ));
            }

            conn.get_sender().ok_or_else(|| {
                format!("连接没有活跃的 WebSocket 流: {}", uuid)
            })?
        };

        // 2. 构造 WebSocket Message (Text 或 Binary)
        let ws_message = if is_binary {
            // 尝试将字符串解析为十六进制或直接作为字节
            let bytes = hex_to_bytes(&message).unwrap_or_else(|_| message.as_bytes().to_vec());
            WebSocketMessage::Binary(bytes.into())
        } else {
            WebSocketMessage::Text(message.into())
        };

        // 3. 使用 SinkExt::send 发送消息（超时控制）
        let send_timeout = Duration::from_millis(send_timeout_ms.clamp(1000, 30000));
        let mut sender_guard = sender.lock().await;

        match timeout(send_timeout, sender_guard.send(ws_message)).await {
            Ok(Ok(())) => {
                log::debug!("[WebSocket] 消息发送成功: uuid={}", uuid);
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("发送消息失败: {}", e);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = format!("发送消息超时 ({}ms)", send_timeout_ms);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
        }
    }

    /// 接收消息
    ///
    /// 自动处理 Ping/Pong 心跳帧：收到 Ping 时自动回复 Pong 并继续读取下一条消息，
    /// 收到 Pong 时静默丢弃并继续读取。只返回 Text/Binary 等业务消息给调用者。
    ///
    /// # 参数
    ///
    /// * `uuid` - 连接 UUID
    /// * `receive_timeout_ms` - 接收超时
    ///
    /// # 返回
    ///
    /// * `Ok(Some(message))` - 接收到消息（返回文本内容）
    /// * `Ok(None)` - 超时未收到消息
    /// * `Err(msg)` - 接收失败或连接不存在
    pub async fn receive(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        // 1. 获取连接，检查状态是否为 Connected
        let (receiver, sender) = {
            let connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;

            let conn = connections.get(uuid).ok_or_else(|| {
                format!("连接不存在: {}", uuid)
            })?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "连接状态不是 Connected（当前状态: {:?}）: {}",
                    conn.status, uuid
                ));
            }

            let receiver = conn.get_receiver().ok_or_else(|| {
                format!("连接没有活跃的 WebSocket 流: {}", uuid)
            })?;

            // 获取 sender 用于回复 Ping
            let sender = conn.get_sender();

            (receiver, sender)
        };

        // 2. 循环读取消息，自动处理 Ping/Pong 心跳帧
        let total_timeout = Duration::from_millis(receive_timeout_ms.clamp(1000, 30000));
        let start = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= total_timeout {
                log::debug!(
                    "[WebSocket] 接收超时 ({}ms): uuid={}",
                    receive_timeout_ms,
                    uuid
                );
                return Ok(None);
            }

            let remaining = total_timeout - elapsed;
            let mut receiver_guard = receiver.lock().await;

            match timeout(remaining, receiver_guard.next()).await {
                Ok(Some(Ok(message))) => {
                    match message {
                        WebSocketMessage::Text(text) => {
                            log::debug!("[WebSocket] 收到文本消息: uuid={}", uuid);
                            return Ok(Some(text.to_string()));
                        }
                        WebSocketMessage::Binary(data) => {
                            log::debug!(
                                "[WebSocket] 收到二进制消息: uuid={}, len={}",
                                uuid,
                                data.len()
                            );
                            return Ok(Some(bytes_to_hex(&data)));
                        }
                        WebSocketMessage::Ping(data) => {
                            // 自动响应 Ping：发送 Pong 并继续读取下一条消息
                            log::debug!(
                                "[WebSocket] 收到 Ping，自动响应 Pong: uuid={}",
                                uuid
                            );
                            if let Some(sender) = &sender {
                                let mut sender_guard = sender.lock().await;
                                let pong = WebSocketMessage::Pong(data);
                                if let Err(e) = sender_guard.send(pong).await {
                                    log::warn!(
                                        "[WebSocket] 发送 Pong 失败: uuid={}, error={}",
                                        uuid,
                                        e
                                    );
                                }
                            }
                            // 释放 receiver 锁，继续读取下一条消息
                            drop(receiver_guard);
                            continue;
                        }
                        WebSocketMessage::Pong(_) => {
                            // 静默丢弃 Pong（心跳回复），继续读取下一条消息
                            log::debug!("[WebSocket] 收到 Pong，忽略: uuid={}", uuid);
                            drop(receiver_guard);
                            continue;
                        }
                        WebSocketMessage::Close(frame) => {
                            log::info!(
                                "[WebSocket] 收到 Close 帧: uuid={}, frame={:?}",
                                uuid,
                                frame
                            );
                            // 更新连接状态
                            self.update_status(uuid, ConnectionStatus::Closed)?;
                            return Ok(None);
                        }
                        WebSocketMessage::Frame(_) => {
                            // 原始帧，通常不会直接处理
                            return Ok(None);
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let error_msg = format!("接收消息失败: {}", e);
                    self.update_connection_error(uuid, &error_msg);
                    return Err(error_msg);
                }
                Ok(None) => {
                    // 流已结束
                    log::info!("[WebSocket] 流已结束: uuid={}", uuid);
                    self.update_status(uuid, ConnectionStatus::Closed)?;
                    return Ok(None);
                }
                Err(_) => {
                    // 超时未收到消息
                    log::debug!(
                        "[WebSocket] 接收超时 ({}ms): uuid={}",
                        receive_timeout_ms,
                        uuid
                    );
                    return Ok(None);
                }
            }
        }
    }

    /// 清空连接中待处理的消息
    ///
    /// 在发送消息后调用此方法，清空发送前到达的所有消息（如欢迎消息），
    /// 确保后续的 `receive()` 只读取发送后的响应消息。
    ///
    /// # 参数
    ///
    /// * `uuid` - 连接 UUID
    ///
    /// # 返回
    ///
    /// * `Ok(count)` - 清空的消息数量
    /// * `Err(msg)` - 连接不存在或状态错误
    pub async fn drain_pending_messages(&self, uuid: &str) -> Result<usize, String> {
        // 1. 获取连接，检查状态是否为 Connected
        let (receiver, sender) = {
            let connections = self.connections.lock().map_err(|e| {
                format!("获取连接锁失败: {}", e)
            })?;

            let conn = connections.get(uuid).ok_or_else(|| {
                format!("连接不存在: {}", uuid)
            })?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "连接状态不是 Connected（当前状态: {:?}）: {}",
                    conn.status, uuid
                ));
            }

            let receiver = conn.get_receiver().ok_or_else(|| {
                format!("连接没有活跃的 WebSocket 流: {}", uuid)
            })?;

            let sender = conn.get_sender();

            (receiver, sender)
        };

        // 2. 使用非常短的超时（1ms）非阻塞地读取所有待处理消息
        let mut drained_count = 0;
        let drain_timeout = Duration::from_millis(1);
        let mut receiver_guard = receiver.lock().await;

        loop {
            match timeout(drain_timeout, receiver_guard.next()).await {
                Ok(Some(Ok(message))) => {
                    // 记录被清空的消息（用于调试）
                    match &message {
                        WebSocketMessage::Text(text) => {
                            log::debug!(
                                "[WebSocket] drain_pending_messages: 清空文本消息 uuid={}, len={}, preview={}",
                                uuid,
                                text.len(),
                                &text.chars().take(50).collect::<String>()
                            );
                        }
                        WebSocketMessage::Binary(data) => {
                            log::debug!(
                                "[WebSocket] drain_pending_messages: 清空二进制消息 uuid={}, len={}",
                                uuid,
                                data.len()
                            );
                        }
                        WebSocketMessage::Ping(data) => {
                            // 收到 Ping 时自动回复 Pong
                            log::debug!(
                                "[WebSocket] drain_pending_messages: 收到 Ping，自动回复 Pong uuid={}",
                                uuid
                            );
                            if let Some(sender) = &sender {
                                let mut sender_guard = sender.lock().await;
                                let pong = WebSocketMessage::Pong(data.clone());
                                if let Err(e) = sender_guard.send(pong).await {
                                    log::warn!(
                                        "[WebSocket] drain_pending_messages: 发送 Pong 失败 uuid={}, error={}",
                                        uuid,
                                        e
                                    );
                                }
                            }
                        }
                        WebSocketMessage::Pong(_) => {
                            // 静默丢弃 Pong
                            log::debug!(
                                "[WebSocket] drain_pending_messages: 忽略 Pong uuid={}",
                                uuid
                            );
                        }
                        WebSocketMessage::Close(_) => {
                            // 收到 Close 帧，更新状态并退出
                            log::info!(
                                "[WebSocket] drain_pending_messages: 收到 Close 帧 uuid={}",
                                uuid
                            );
                            self.update_status(uuid, ConnectionStatus::Closed)?;
                            break;
                        }
                        WebSocketMessage::Frame(_) => {
                            // 原始帧，忽略
                        }
                    }
                    drained_count += 1;
                    // 继续尝试读取更多消息
                }
                Ok(Some(Err(e))) => {
                    log::warn!(
                        "[WebSocket] drain_pending_messages: 读取消息出错 uuid={}, error={}",
                        uuid,
                        e
                    );
                    // 出错时停止清空
                    break;
                }
                Ok(None) => {
                    // 流已结束
                    log::debug!(
                        "[WebSocket] drain_pending_messages: 流已结束 uuid={}",
                        uuid
                    );
                    break;
                }
                Err(_) => {
                    // 超时，没有更多待处理消息
                    break;
                }
            }
        }

        if drained_count > 0 {
            log::info!(
                "[WebSocket] drain_pending_messages: 清空了 {} 条待处理消息 uuid={}",
                drained_count,
                uuid
            );
        }

        Ok(drained_count)
    }

    /// 根据 UUID 获取连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 连接的克隆，如果不存在则返回 None
    pub fn get_connection(&self, uuid: &str) -> Option<ActiveConnection> {
        // 注意：由于 ActiveConnection 包含非 Clone 的字段，
        // 这里只能返回元数据，不能返回完整的连接
        let connections = self.connections.lock().ok()?;
        connections.get(uuid).map(|c| ActiveConnectionMeta::from(c).into())
    }

    /// 获取连接状态
    pub fn get_status(&self, uuid: &str) -> Option<ConnectionStatus> {
        let connections = self.connections.lock().ok()?;
        connections.get(uuid).map(|conn| conn.status.clone())
    }

    /// 获取所有连接
    ///
    /// # 返回
    ///
    /// 所有活跃连接的列表（仅元数据）
    pub fn get_all_connections(&self) -> Vec<ActiveConnection> {
        let connections = self.connections.lock().ok();
        match connections {
            Some(conns) => conns
                .values()
                .map(|c| ActiveConnectionMeta::from(c).into())
                .collect(),
            None => vec![],
        }
    }

    /// 添加或更新连接
    ///
    /// # 参数
    ///
    /// * `connection` - 要添加的连接
    ///
    /// # 返回
    ///
    /// 如果已存在同名 UUID 的连接，返回旧连接
    pub fn upsert_connection(&self, connection: ActiveConnection) -> Option<ActiveConnection> {
        let mut connections = self.connections.lock().ok()?;
        connections
            .insert(connection.uuid.clone(), connection)
            .map(|c| ActiveConnectionMeta::from(&c).into())
    }

    /// 移除连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 被移除的连接，如果不存在则返回 None
    pub fn remove_connection(&self, uuid: &str) -> Option<ActiveConnection> {
        let mut connections = self.connections.lock().ok()?;
        connections
            .remove(uuid)
            .map(|c| ActiveConnectionMeta::from(&c).into())
    }

    /// 更新连接状态
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    /// * `status` - 新状态
    ///
    /// # 返回
    ///
    /// 操作是否成功（连接是否存在）
    pub fn update_status(&self, uuid: &str, status: ConnectionStatus) -> Result<(), String> {
        let mut connections = self.connections.lock().map_err(|e| {
            format!("获取连接锁失败: {}", e)
        })?;
        if let Some(conn) = connections.get_mut(uuid) {
            conn.set_status(status);
            Ok(())
        } else {
            Err(format!("连接不存在: {}", uuid))
        }
    }

    /// 更新连接错误状态（内部方法）
    fn update_connection_error(&self, uuid: &str, error: &str) {
        if let Ok(mut connections) = self.connections.lock() {
            if let Some(conn) = connections.get_mut(uuid) {
                conn.set_error(error.to_string());
            }
        }
    }

    /// 获取连接数量
    pub fn connection_count(&self) -> usize {
        let connections = self.connections.lock().ok();
        match connections {
            Some(conns) => conns.len(),
            None => 0,
        }
    }

    /// 清理所有已关闭的连接
    pub fn cleanup_closed(&self) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.retain(|_, conn| conn.status != ConnectionStatus::Closed);
        }
    }

    /// 清空所有连接
    pub fn clear(&self) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.clear();
        }
    }

    /// 获取活跃连接数量（状态为 Connected）
    pub fn active_connection_count(&self) -> usize {
        let connections = self.connections.lock().ok();
        match connections {
            Some(conns) => conns
                .values()
                .filter(|c| c.status == ConnectionStatus::Connected)
                .count(),
            None => 0,
        }
    }

    // ========================================================================
    // 消息匹配功能（Wave 3）
    // ========================================================================

    /// 匹配 WebSocket 消息
    ///
    /// # 参数
    ///
    /// * `message` - 消息内容（文本）
    /// * `condition` - 匹配条件
    ///
    /// # 返回
    ///
    /// * `Ok(true)`: 消息匹配成功
    /// * `Ok(false)`: 消息不匹配
    /// * `Err(msg)`: 匹配过程出错
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let manager = WebSocketConnectionManager::new();
    /// let condition = WsWaitCondition {
    ///     match_type: WsMatchType::JsonPath,
    ///     expression: "$.status".to_string(),
    /// };
    /// let result = manager.match_message(r#"{"status":"ok"}"#, &condition)?;
    /// assert!(result);
    /// ```
    pub fn match_message(
        &self,
        message: &str,
        condition: &WsWaitCondition,
    ) -> Result<bool, String> {
        match condition.match_type {
            WsMatchType::JsonPath => self.match_jsonpath(message, &condition.expression),
            WsMatchType::Regex => self.match_regex(message, &condition.expression),
            WsMatchType::Expression => self.match_expression(message, &condition.expression),
        }
    }

    /// `JSONPath` 匹配
    ///
    /// 使用 `JSONPath` 表达式查询 JSON 消息，非空结果表示匹配成功。
    ///
    /// # 参数
    ///
    /// * `message` - JSON 格式的消息文本
    /// * `expression` - `JSONPath` 表达式（例如：`$.status`）
    ///
    /// # 返回
    ///
    /// * `Ok(true)`: `JSONPath` 查询结果非空
    /// * `Ok(false)`: `JSONPath` 查询结果为空
    /// * `Err(msg)`: JSON 解析失败或 `JSONPath` 语法错误
    fn match_jsonpath(&self, message: &str, expression: &str) -> Result<bool, String> {
        // 1. 解析 JSON
        let json: serde_json::Value = serde_json::from_str(message)
            .map_err(|e| format!("JSON 解析失败: {}", e))?;

        // 2. 编译 JSONPath 表达式
        let path = JsonPath::parse(expression)
            .map_err(|e| format!("JSONPath 语法错误: {}", e))?;

        // 3. 查询 JSON
        let result = path.query(&json);

        // 4. 判断是否匹配（非空结果 = 匹配）
        Ok(!result.is_empty())
    }

    /// 正则表达式匹配
    ///
    /// 使用正则表达式匹配消息文本。
    ///
    /// # 参数
    ///
    /// * `message` - 消息文本
    /// * `pattern` - 正则表达式模式
    ///
    /// # 返回
    ///
    /// * `Ok(true)`: 消息匹配正则表达式
    /// * `Ok(false)`: 消息不匹配正则表达式
    /// * `Err(msg)`: 正则表达式语法错误
    fn match_regex(&self, message: &str, pattern: &str) -> Result<bool, String> {
        // 1. 编译正则表达式
        let regex = Regex::new(pattern)
            .map_err(|e| format!("正则表达式语法错误: {}", e))?;

        // 2. 匹配消息
        Ok(regex.is_match(message))
    }

    /// 表达式匹配（使用 evalexpr）
    ///
    /// 使用布尔表达式匹配消息，支持 JSON 字段引用。
    ///
    /// # 参数
    ///
    /// * `message` - 消息文本（可以是 JSON 或纯文本）
    /// * `expression` - 布尔表达式（例如：`status == 200`）
    ///
    /// # 返回
    ///
    /// * `Ok(true)`: 表达式求值为 true
    /// * `Ok(false)`: 表达式求值为 false
    /// * `Err(msg)`: 表达式语法错误或求值失败
    ///
    /// # 说明
    ///
    /// 如果消息是 JSON 格式，会自动解析并将顶层字段添加到表达式上下文中。
    /// 同时提供 `message` 变量引用原始消息文本。
    fn match_expression(&self, message: &str, expression: &str) -> Result<bool, String> {
        // 1. 尝试解析 JSON（如果消息是 JSON 格式）
        let json_value: Option<serde_json::Value> =
            serde_json::from_str(message).ok();

        // 2. 创建表达式上下文
        let mut context = HashMapContext::new();

        // 3. 添加变量到上下文
        // 消息文本
        context
            .set_value("message".to_string(), Value::String(message.to_string()))
            .map_err(|e| format!("设置变量失败: {}", e))?;

        // 如果是 JSON，添加解析后的字段
        if let Some(serde_json::Value::Object(map)) = json_value {
            for (key, value) in map {
                let evalexpr_value = json_to_evalexpr_value(&value);
                context
                    .set_value(key.clone(), evalexpr_value)
                    .map_err(|e| format!("设置变量失败: {}", e))?;
            }
        }

        // 4. 求值表达式
        let eval_result = evalexpr::eval_with_context(expression, &context)
            .map_err(|e| format!("表达式求值失败: {}", e))?;

        // 5. 转换为布尔值
        let result = match eval_result {
            Value::Boolean(b) => b,
            Value::Int(i) => i != 0,
            Value::Float(f) => f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Empty => false,
            _ => {
                return Err(format!("表达式返回不支持的类型: {:?}", eval_result));
            }
        };

        Ok(result)
    }
}

impl Default for WebSocketConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl flow_engine_core::traits::WsProvider for WebSocketConnectionManager {
    async fn connect(
        &self,
        uuid: String,
        name: String,
        url: String,
        headers: Vec<FormField>,
        subprotocols: Vec<String>,
        connect_timeout_ms: u64,
    ) -> Result<(), String> {
        WebSocketConnectionManager::connect(self, uuid, name, url, headers, subprotocols, connect_timeout_ms).await
    }

    async fn send(
        &self,
        uuid: &str,
        message: String,
        is_binary: bool,
        send_timeout_ms: u64,
    ) -> Result<(), String> {
        WebSocketConnectionManager::send(self, uuid, message, is_binary, send_timeout_ms).await
    }

    async fn receive(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        WebSocketConnectionManager::receive(self, uuid, receive_timeout_ms).await
    }

    async fn close(
        &self,
        uuid: &str,
        close_code: u16,
        close_reason: Option<String>,
        wait_close_timeout_ms: u64,
    ) -> Result<(), String> {
        WebSocketConnectionManager::close(self, uuid, close_code, close_reason, wait_close_timeout_ms).await
    }

    async fn drain_pending_messages(&self, uuid: &str) -> Result<usize, String> {
        WebSocketConnectionManager::drain_pending_messages(self, uuid).await
    }

    fn match_message(&self, message: &str, condition: &WsWaitCondition) -> Result<bool, String> {
        WebSocketConnectionManager::match_message(self, message, condition)
    }

    fn get_status(&self, uuid: &str) -> Option<String> {
        WebSocketConnectionManager::get_status(self, uuid).map(|s| format!("{:?}", s))
    }
}

// ============================================================================
// 辅助结构和函数
// ============================================================================

/// 连接元数据（用于序列化和克隆）
#[derive(Clone)]
struct ActiveConnectionMeta {
    uuid: String,
    name: String,
    status: ConnectionStatus,
    created_at: DateTime<Utc>,
    url: String,
    last_error: Option<String>,
    current_owner_node_uuid: Option<String>,
    stream_segment_id: Option<String>,
}

impl From<&ActiveConnection> for ActiveConnectionMeta {
    fn from(conn: &ActiveConnection) -> Self {
        Self {
            uuid: conn.uuid.clone(),
            name: conn.name.clone(),
            status: conn.status.clone(),
            created_at: conn.created_at,
            url: conn.url.clone(),
            last_error: conn.last_error.clone(),
            current_owner_node_uuid: conn.current_owner_node_uuid.clone(),
            stream_segment_id: conn.stream_segment_id.clone(),
        }
    }
}

impl From<ActiveConnectionMeta> for ActiveConnection {
    fn from(meta: ActiveConnectionMeta) -> Self {
        Self {
            uuid: meta.uuid,
            name: meta.name,
            status: meta.status,
            created_at: meta.created_at,
            url: meta.url,
            message_buffer: VecDeque::new(),
            last_error: meta.last_error,
            current_owner_node_uuid: meta.current_owner_node_uuid,
            stream_segment_id: meta.stream_segment_id,
            sender: None,
            receiver: None,
        }
    }
}

/// 将字节转换为十六进制字符串
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 将十六进制字符串转换为字节
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    // 移除可能的 0x 前缀
    let hex = hex.strip_prefix("0x").unwrap_or(hex);

    if !hex.len().is_multiple_of(2) {
        return Err("十六进制字符串长度必须是偶数".to_string());
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("解析十六进制失败: {}", e))
        })
        .collect()
}

/// 将 JSON Value 转换为 evalexpr Value
///
/// 用于在表达式匹配中引用 JSON 字段。
fn json_to_evalexpr_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Empty,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            // 优先检查是否为整数
            if let Some(i) = n.as_i64() {
                // 检查是否真的是整数（没有小数部分）
                if let Some(f) = n.as_f64() {
                    if f == i as f64 {
                        return Value::Int(i);
                    }
                }
            }
            // 否则作为浮点数
            if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Empty
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // 复杂类型转为字符串
            Value::String(json.to_string())
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::{WsMatchType, WsWaitCondition};

    fn create_test_connection(uuid: &str, name: &str, url: &str) -> ActiveConnection {
        ActiveConnection::new(uuid.to_string(), name.to_string(), url.to_string())
    }

    #[test]
    fn test_connection_status_default() {
        let status = ConnectionStatus::default();
        assert_eq!(status, ConnectionStatus::Connecting);
    }

    #[test]
    fn test_connection_status_serialization() {
        let status = ConnectionStatus::Connected;
        let json = serde_json::to_string(&status).expect("序列化失败");
        assert_eq!(json, r#""connected""#);

        let deserialized: ConnectionStatus =
            serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized, ConnectionStatus::Connected);
    }

    #[test]
    fn test_active_connection_creation() {
        let conn = create_test_connection("uuid-1", "测试连接", "wss://example.com/ws");

        assert_eq!(conn.uuid, "uuid-1");
        assert_eq!(conn.name, "测试连接");
        assert_eq!(conn.url, "wss://example.com/ws");
        assert_eq!(conn.status, ConnectionStatus::Connecting);
        assert!(conn.buffer_size() == 0);
        assert!(conn.last_error.is_none());
        assert!(!conn.is_connected());
    }

    #[test]
    fn test_active_connection_set_status() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        conn.set_status(ConnectionStatus::Connected);
        assert_eq!(conn.status, ConnectionStatus::Connected);

        conn.set_status(ConnectionStatus::Closing);
        assert_eq!(conn.status, ConnectionStatus::Closing);
    }

    #[test]
    fn test_active_connection_set_error() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        conn.set_error("连接失败".to_string());
        assert_eq!(conn.status, ConnectionStatus::Error);
        assert_eq!(conn.last_error, Some("连接失败".to_string()));
    }

    #[test]
    fn test_active_connection_message_buffer() {
        use tokio_tungstenite::tungstenite::{Bytes, Utf8Bytes};

        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        // 添加消息
        conn.push_message(WebSocketMessage::Text(Utf8Bytes::from("hello")));
        conn.push_message(WebSocketMessage::Binary(Bytes::from(vec![1, 2, 3])));
        assert_eq!(conn.buffer_size(), 2);

        // 取出消息（FIFO）
        let msg1 = conn.pop_message();
        assert!(
            matches!(msg1, Some(WebSocketMessage::Text(ref t)) if t.as_str() == "hello")
        );
        assert_eq!(conn.buffer_size(), 1);

        let msg2 = conn.pop_message();
        assert!(
            matches!(msg2, Some(WebSocketMessage::Binary(ref b)) if b.as_ref() == &[1, 2, 3])
        );
        assert_eq!(conn.buffer_size(), 0);

        // 空缓冲区
        let msg3 = conn.pop_message();
        assert!(msg3.is_none());
    }

    #[test]
    fn test_active_connection_ownership() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        // 初始状态：没有所有者
        assert!(conn.current_owner_node_uuid.is_none());
        assert!(conn.stream_segment_id.is_none());

        // 设置所有权
        conn.set_owner("node-123", "segment-456");
        assert_eq!(conn.current_owner_node_uuid, Some("node-123".to_string()));
        assert_eq!(conn.stream_segment_id, Some("segment-456".to_string()));

        // 释放所有权
        conn.release_owner();
        assert!(conn.current_owner_node_uuid.is_none());
        assert!(conn.stream_segment_id.is_none());
    }

    #[test]
    fn test_active_connection_ownership_overwrite() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        // 设置第一次所有权
        conn.set_owner("node-1", "segment-1");
        assert_eq!(conn.current_owner_node_uuid, Some("node-1".to_string()));
        assert_eq!(conn.stream_segment_id, Some("segment-1".to_string()));

        // 覆盖所有权
        conn.set_owner("node-2", "segment-2");
        assert_eq!(conn.current_owner_node_uuid, Some("node-2".to_string()));
        assert_eq!(conn.stream_segment_id, Some("segment-2".to_string()));
    }

    #[test]
    fn test_active_connection_meta_ownership() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");
        conn.set_owner("node-123", "segment-456");

        // 测试 From<ActiveConnection> for ActiveConnectionMeta
        let meta = ActiveConnectionMeta::from(&conn);
        assert_eq!(meta.current_owner_node_uuid, Some("node-123".to_string()));
        assert_eq!(meta.stream_segment_id, Some("segment-456".to_string()));

        // 测试 From<ActiveConnectionMeta> for ActiveConnection
        let conn_from_meta: ActiveConnection = meta.into();
        assert_eq!(conn_from_meta.current_owner_node_uuid, Some("node-123".to_string()));
        assert_eq!(conn_from_meta.stream_segment_id, Some("segment-456".to_string()));
        // 注意：sender 和 receiver 应该是 None
        assert!(conn_from_meta.sender.is_none());
        assert!(conn_from_meta.receiver.is_none());
    }

    #[test]
    fn test_manager_creation() {
        let manager = WebSocketConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_all_connections().is_empty());
    }

    #[test]
    fn test_manager_default() {
        let manager = WebSocketConnectionManager::default();
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn test_manager_upsert_connection() {
        let manager = WebSocketConnectionManager::new();
        let conn1 = create_test_connection("uuid-1", "连接1", "wss://example.com/ws1");

        // 添加新连接
        let result = manager.upsert_connection(conn1);
        assert!(result.is_none());
        assert_eq!(manager.connection_count(), 1);

        // 更新已存在的连接
        let conn2 = create_test_connection("uuid-1", "连接1更新", "wss://example.com/ws2");
        let old = manager.upsert_connection(conn2);
        assert!(old.is_some());
        assert_eq!(old.unwrap().name, "连接1");
        assert_eq!(manager.connection_count(), 1);

        // 验证更新成功
        let current = manager.get_connection("uuid-1").unwrap();
        assert_eq!(current.name, "连接1更新");
        assert_eq!(current.url, "wss://example.com/ws2");
    }

    #[test]
    fn test_manager_get_connection() {
        let manager = WebSocketConnectionManager::new();
        let conn = create_test_connection("uuid-1", "连接1", "wss://example.com/ws");

        manager.upsert_connection(conn);

        // 存在的连接
        let found = manager.get_connection("uuid-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "连接1");

        // 不存在的连接
        let not_found = manager.get_connection("uuid-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_manager_get_all_connections() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));
        manager.upsert_connection(create_test_connection(
            "uuid-2",
            "连接2",
            "wss://b.com",
        ));
        manager.upsert_connection(create_test_connection(
            "uuid-3",
            "连接3",
            "wss://c.com",
        ));

        let all = manager.get_all_connections();
        assert_eq!(all.len(), 3);

        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"连接1"));
        assert!(names.contains(&"连接2"));
        assert!(names.contains(&"连接3"));
    }

    #[test]
    fn test_manager_remove_connection() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));

        // 移除存在的连接
        let removed = manager.remove_connection("uuid-1");
        assert!(removed.is_some());
        assert_eq!(manager.connection_count(), 0);

        // 移除不存在的连接
        let not_removed = manager.remove_connection("uuid-999");
        assert!(not_removed.is_none());
    }

    #[test]
    fn test_manager_update_status() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));

        // 更新存在的连接状态
        let success = manager.update_status("uuid-1", ConnectionStatus::Connected);
        assert!(success.is_ok());

        let conn = manager.get_connection("uuid-1").unwrap();
        assert_eq!(conn.status, ConnectionStatus::Connected);

        // 更新不存在的连接
        let fail = manager.update_status("uuid-999", ConnectionStatus::Connected);
        assert!(fail.is_err());
    }

    #[test]
    fn test_manager_clear() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));
        manager.upsert_connection(create_test_connection(
            "uuid-2",
            "连接2",
            "wss://b.com",
        ));

        assert_eq!(manager.connection_count(), 2);

        manager.clear();

        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_all_connections().is_empty());
    }

    #[test]
    fn test_manager_clone() {
        let manager1 = WebSocketConnectionManager::new();
        manager1.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));

        // 克隆管理器（共享同一内部状态）
        let manager2 = manager1.clone();

        // 通过 manager2 修改
        manager2.upsert_connection(create_test_connection(
            "uuid-2",
            "连接2",
            "wss://b.com",
        ));

        // manager1 应该看到变化（Arc 共享）
        assert_eq!(manager1.connection_count(), 2);
        assert_eq!(manager2.connection_count(), 2);
    }

    #[test]
    fn test_manager_cleanup_closed() {
        let manager = WebSocketConnectionManager::new();

        // 添加连接
        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));
        manager.upsert_connection(create_test_connection(
            "uuid-2",
            "连接2",
            "wss://b.com",
        ));

        // 将 uuid-1 设置为 Closed
        manager
            .update_status("uuid-1", ConnectionStatus::Closed)
            .unwrap();

        assert_eq!(manager.connection_count(), 2);

        // 清理已关闭的连接
        manager.cleanup_closed();

        assert_eq!(manager.connection_count(), 1);
        assert!(manager.get_connection("uuid-1").is_none());
        assert!(manager.get_connection("uuid-2").is_some());
    }

    #[test]
    fn test_manager_active_connection_count() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection(
            "uuid-1",
            "连接1",
            "wss://a.com",
        ));
        manager.upsert_connection(create_test_connection(
            "uuid-2",
            "连接2",
            "wss://b.com",
        ));

        assert_eq!(manager.active_connection_count(), 0);

        manager
            .update_status("uuid-1", ConnectionStatus::Connected)
            .unwrap();

        assert_eq!(manager.active_connection_count(), 1);

        manager
            .update_status("uuid-2", ConnectionStatus::Connected)
            .unwrap();

        assert_eq!(manager.active_connection_count(), 2);
    }

    #[test]
    fn test_bytes_to_hex() {
        let bytes = vec![0x01, 0x02, 0xff, 0xab];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "0102ffab");
    }

    #[test]
    fn test_hex_to_bytes() {
        let hex = "0102ffab";
        let bytes = hex_to_bytes(hex).unwrap();
        assert_eq!(bytes, vec![0x01, 0x02, 0xff, 0xab]);

        // 带 0x 前缀
        let hex_with_prefix = "0x0102ffab";
        let bytes_with_prefix = hex_to_bytes(hex_with_prefix).unwrap();
        assert_eq!(bytes_with_prefix, vec![0x01, 0x02, 0xff, 0xab]);

        // 奇数长度应该失败
        let odd_hex = "010";
        assert!(hex_to_bytes(odd_hex).is_err());
    }

    // ============================================================================
    // 异步测试（连接到公共 WebSocket 服务器）
    // ============================================================================

    #[tokio::test]
    async fn test_connect_invalid_url() {
        let manager = WebSocketConnectionManager::new();

        // 测试无效 URL
        let result = manager
            .connect(
                "conn-1".to_string(),
                "测试连接".to_string(),
                "http://invalid.com".to_string(), // 无效协议
                vec![],                           // headers
                vec![],                           // subprotocols
                5000,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的 WebSocket URL"));

        // 验证连接状态为 Error
        let conn = manager.get_connection("conn-1").unwrap();
        assert_eq!(conn.status, ConnectionStatus::Error);
    }

    #[tokio::test]
    async fn test_connect_timeout() {
        let manager = WebSocketConnectionManager::new();

        // 使用不可达的地址测试超时
        // 注意：这个测试可能会因为网络环境不同而表现不同
        let result = manager
            .connect(
                "conn-timeout".to_string(),
                "超时测试".to_string(),
                "ws://192.0.2.1:81/ws".to_string(), // RFC 3330 测试地址，不可路由
                vec![],                            // headers
                vec![],                            // subprotocols
                2000,                              // 2 秒超时
            )
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("超时") || error.contains("失败"),
            "错误消息应该包含超时或失败: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager.close("nonexistent", 1000, None, 5000).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("连接不存在"));
    }

    #[tokio::test]
    async fn test_send_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager
            .send("nonexistent", "hello".to_string(), false, 5000)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("连接不存在"));
    }

    #[tokio::test]
    async fn test_receive_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager.receive("nonexistent", 5000).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("连接不存在"));
    }

    #[tokio::test]
    async fn test_connect_to_echo_server() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "echo-conn".to_string(),
                "Echo 测试".to_string(),
                "wss://echo.websocket.org".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
            )
            .await;

        // 网络测试：如果连接失败，跳过而不是失败
        match result {
            Ok(()) => {
                // 验证连接状态
                let status = manager.get_status("echo-conn").unwrap();
                assert_eq!(status, ConnectionStatus::Connected);

                // 接收并丢弃初始问候消息（echo.websocket.org 会发送欢迎消息）
                let _ = manager.receive("echo-conn", 3000).await;

                // 发送消息
                let send_result = manager
                    .send("echo-conn", "Hello, WebSocket!".to_string(), false, 10000)
                    .await;
                assert!(send_result.is_ok());

                // 接收回显消息
                let recv_result = manager.receive("echo-conn", 10000).await;
                assert!(recv_result.is_ok());
                let message = recv_result.unwrap();
                assert!(message.is_some());
                assert_eq!(message.unwrap(), "Hello, WebSocket!");

                // 关闭连接
                let close_result = manager.close("echo-conn", 1000, Some("测试完成".to_string()), 5000).await;
                assert!(close_result.is_ok());

                // 验证连接状态
                let status = manager.get_status("echo-conn").unwrap();
                assert_eq!(status, ConnectionStatus::Closed);
            }
            Err(e) => {
                // 网络不可用时跳过测试
                println!("跳过测试：无法连接到 echo 服务器: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_send_binary_message() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "binary-conn".to_string(),
                "二进制测试".to_string(),
                "wss://echo.websocket.org".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
            )
            .await;

        // 网络测试：如果连接失败，跳过而不是失败
        if result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = manager.receive("binary-conn", 3000).await;

            // 发送二进制消息（使用十六进制字符串）
            let send_result = manager
                .send("binary-conn", "0102030405".to_string(), true, 10000)
                .await;
            assert!(send_result.is_ok());

            // 接收回显消息
            let recv_result = manager.receive("binary-conn", 10000).await;
            assert!(recv_result.is_ok());
            let message = recv_result.unwrap();
            assert!(message.is_some());
            // 二进制消息会被转换为十六进制字符串
            assert_eq!(message.unwrap(), "0102030405");

            // 清理
            let _ = manager.close("binary-conn", 1000, None, 5000).await;
        } else {
            println!("跳过测试：无法连接到 echo 服务器");
        }
    }

    #[tokio::test]
    async fn test_receive_timeout() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "timeout-conn".to_string(),
                "超时测试".to_string(),
                "wss://echo.websocket.org".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
            )
            .await;

        // 网络测试：如果连接失败，跳过而不是失败
        if result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = manager.receive("timeout-conn", 3000).await;

            // 不发送任何消息，直接接收，应该超时
            let recv_result = manager.receive("timeout-conn", 2000).await;
            assert!(recv_result.is_ok());
            // 超时应该返回 None
            assert!(recv_result.unwrap().is_none());

            // 清理
            let _ = manager.close("timeout-conn", 1000, None, 5000).await;
        } else {
            println!("跳过测试：无法连接到 echo 服务器");
        }
    }

    #[test]
    fn test_status_tracking() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection(
            "track-1",
            "跟踪测试",
            "wss://a.com",
        ));

        // 初始状态
        assert_eq!(
            manager.get_status("track-1"),
            Some(ConnectionStatus::Connecting)
        );

        // 更新状态
        manager
            .update_status("track-1", ConnectionStatus::Connected)
            .unwrap();
        assert_eq!(
            manager.get_status("track-1"),
            Some(ConnectionStatus::Connected)
        );

        manager
            .update_status("track-1", ConnectionStatus::Closing)
            .unwrap();
        assert_eq!(
            manager.get_status("track-1"),
            Some(ConnectionStatus::Closing)
        );

        manager
            .update_status("track-1", ConnectionStatus::Closed)
            .unwrap();
        assert_eq!(
            manager.get_status("track-1"),
            Some(ConnectionStatus::Closed)
        );
    }

    // ============================================================================
    // 消息匹配测试（Wave 3）
    // ============================================================================

    #[test]
    fn test_match_jsonpath_success() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok","data":{"id":123}}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.status".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_jsonpath_nested_field() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok","data":{"id":123}}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.data.id".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_jsonpath_not_found() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok"}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.error".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // 空结果 = 不匹配
    }

    #[test]
    fn test_match_jsonpath_invalid_json() {
        let manager = WebSocketConnectionManager::new();
        let message = "not a json";
        let condition = WsWaitCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.status".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON 解析失败"));
    }

    #[test]
    fn test_match_jsonpath_invalid_expression() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok"}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.[invalid".to_string(), // 无效的 JSONPath
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSONPath 语法错误"));
    }

    #[test]
    fn test_match_regex_success() {
        let manager = WebSocketConnectionManager::new();
        let message = "Hello World 123";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Regex,
            expression: r"\d+".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_regex_complex_pattern() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":200,"message":"success"}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Regex,
            expression: r#""status":\s*\d{3}"#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_regex_no_match() {
        let manager = WebSocketConnectionManager::new();
        let message = "Hello World";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Regex,
            expression: r"\d+".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_match_regex_invalid_pattern() {
        let manager = WebSocketConnectionManager::new();
        let message = "test";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Regex,
            expression: r"[".to_string(), // 无效的正则表达式
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("正则表达式语法错误"));
    }

    #[test]
    fn test_match_expression_simple() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":200}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "status == 200".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_with_message_var() {
        let manager = WebSocketConnectionManager::new();
        let message = "success";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: r#"message == "success""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_false() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"code":404}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "code == 200".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_match_expression_complex() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"temperature":25,"humidity":60}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "temperature > 20 && humidity < 70".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_string_comparison() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok"}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: r#"status == "ok""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_or_logic() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":200}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "status == 200 || status == 201".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_numeric_comparison() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"count":10}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "count >= 5 && count <= 15".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_invalid_syntax() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":200}"#;
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: "status ==".to_string(), // 无效的表达式
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("表达式求值失败"));
    }

    #[test]
    fn test_match_expression_non_json() {
        let manager = WebSocketConnectionManager::new();
        let message = "plain text message";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: r#"message == "plain text message""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_contains_string() {
        let manager = WebSocketConnectionManager::new();
        let message = "Error: Connection timeout";
        let condition = WsWaitCondition {
            match_type: WsMatchType::Expression,
            expression: r#"message == "Error: Connection timeout""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_json_to_evalexpr_value_null() {
        let json = serde_json::Value::Null;
        let value = json_to_evalexpr_value(&json);
        assert_eq!(value, Value::Empty);
    }

    #[test]
    fn test_json_to_evalexpr_value_boolean() {
        let json = serde_json::json!(true);
        let value = json_to_evalexpr_value(&json);
        assert_eq!(value, Value::Boolean(true));

        let json = serde_json::json!(false);
        let value = json_to_evalexpr_value(&json);
        assert_eq!(value, Value::Boolean(false));
    }

    #[test]
    fn test_json_to_evalexpr_value_number() {
        let json = serde_json::json!(42);
        let value = json_to_evalexpr_value(&json);
        assert_eq!(value, Value::Int(42));

        let json = serde_json::json!(3.14);
        let value = json_to_evalexpr_value(&json);
        if let Value::Float(f) = value {
            assert!((f - 3.14).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
    }

    #[test]
    fn test_json_to_evalexpr_value_string() {
        let json = serde_json::json!("hello");
        let value = json_to_evalexpr_value(&json);
        assert_eq!(value, Value::String("hello".to_string()));
    }

    #[test]
    fn test_json_to_evalexpr_value_complex() {
        let json = serde_json::json!({"key": "value"});
        let value = json_to_evalexpr_value(&json);
        // 复杂类型转为字符串
        if let Value::String(s) = value {
            assert!(s.contains("key"));
            assert!(s.contains("value"));
        } else {
            panic!("Expected String value for complex JSON");
        }
    }
}
