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

use flow_engine_core::node_config::{FormField, WsCollectCondition, WsMatchType};

// ---------------------------------------------------------------------------
// TLS 辅助
// ---------------------------------------------------------------------------

/// 不验证任何证书的 ServerCertVerifier（仅用于 `tls_skip_verify=true`）
///
/// **警告**: 仅用于开发/测试环境，切勿在生产环境使用。
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

/// 确保已安装 CryptoProvider（幂等，多次调用安全）
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// 创建 TLS `ClientConfig`
///
/// # 参数
/// - `skip_verify`: 是否跳过证书验证
fn create_tls_client_config(skip_verify: bool) -> Result<Arc<rustls::ClientConfig>, String> {
    ensure_crypto_provider();

    let config = if skip_verify {
        // 跳过验证：使用自定义 NoVerifier
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth()
    } else {
        // 标准验证：使用 webpki-roots 根证书
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    Ok(Arc::new(config))
}

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
    /// 连接Error
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
    /// 最后Error消息（如果有）
    pub last_error: Option<String>,

    /// 当前持有者节点 UUID（用于消息流所有权追踪）
    pub current_owner_node_uuid: Option<String>,

    /// 流段 ID（用于关联 `WsStreamSegment`）
    pub stream_segment_id: Option<String>,

    // WebSocket 连接（通过 Arc<Mutex> 实现线程安全共享）
    sender: Option<Arc<tokio::sync::Mutex<WsSender>>>,
    receiver: Option<Arc<tokio::sync::Mutex<WsReceiver>>>,

    /// Ping 心跳后台任务句柄
    ping_task_handle: Option<tokio::task::JoinHandle<()>>,
    /// 最后一次收到 Pong 的时间
    last_pong_received: Option<std::time::Instant>,
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
            .field(
                "ping_task_alive",
                &self
                    .ping_task_handle
                    .as_ref()
                    .is_some_and(|h| !h.is_finished()),
            )
            .field("last_pong_received", &self.last_pong_received.is_some())
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
            ping_task_handle: None,
            last_pong_received: None,
        }
    }

    /// 更新连接状态
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    /// 设置Error状态和消息
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

    /// 设置 WebSocket 流（连接Success后调用）
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
        // 中止 Ping 心跳后台任务
        if let Some(handle) = self.ping_task_handle.take() {
            handle.abort();
        }
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
    /// * `use_tls` - 是否使用 TLS 加密
    /// * `tls_skip_verify` - 是否跳过 TLS 证书验证
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接Success
    /// * `Err(msg)` - connection failed，包含Error消息
    ///
    /// # TLS 模式
    ///
    /// - `wss://` URL 自动启用 TLS（无论 `use_tls` 参数值如何）
    /// - `use_tls=false` + `ws://` → 明文连接
    /// - `use_tls=true, tls_skip_verify=false` → 标准 TLS（webpki-roots 验证）
    /// - `use_tls=true, tls_skip_verify=true` → 跳过证书验证（仅开发/测试）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let manager = WebSocketConnectionManager::new();
    /// manager.connect("conn-1", "测试连接", "wss://echo.websocket.org", vec![], vec![], 30000, true, false).await?;
    /// ```
    pub async fn connect(
        &self,
        uuid: String,
        name: String,
        url: String,
        headers: Vec<FormField>,
        subprotocols: Vec<String>,
        connect_timeout_ms: u64,
        use_tls: bool,
        tls_skip_verify: bool,
        ping_interval_ms: Option<u64>,
        _ping_timeout_ms: Option<u64>,
    ) -> Result<(), String> {
        // 1. 创建 ActiveConnection (status: Connecting)
        let mut connection = ActiveConnection::new(uuid.clone(), name, url.clone());

        // 2. 验证 URL 格式
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            let error_msg = format!(
                "Invalid WebSocket URL (must start with ws:// or wss://): {}",
                url
            );
            connection.set_error(error_msg.clone());
            self.upsert_connection(connection);
            return Err(error_msg);
        }

        // 3. 自动检测 TLS：wss:// URL 自动启用 TLS（避免 use_tls=false 导致 wss:// 降级为明文）
        let use_tls = use_tls || url.starts_with("wss://");

        // 4. 根据 use_tls 调整 URL scheme
        let effective_url = if use_tls {
            // TLS 模式：确保使用 wss://
            if url.starts_with("ws://") {
                format!("wss{}", &url[2..])
            } else {
                url.clone()
            }
        } else {
            // 明文模式：确保使用 ws://
            url.clone()
        };

        // 5. 临时存储连接（状态为 Connecting）
        self.upsert_connection(connection);

        // 6. 构建带 headers 的 WebSocket 请求
        let request = self.build_ws_request(&effective_url, &headers, &subprotocols)?;

        // 7. 根据 TLS 配置进行 3-way 分发
        let connect_timeout = Duration::from_millis(connect_timeout_ms.clamp(1000, 60000));

        let connector = if use_tls {
            // TLS 模式：创建自定义 TLS 配置
            match create_tls_client_config(tls_skip_verify) {
                Ok(config) => Some(tokio_tungstenite::Connector::Rustls(config)),
                Err(e) => {
                    self.update_connection_error(&uuid, &e);
                    return Err(e);
                }
            }
        } else {
            // 明文模式：使用 Plain 连接器
            Some(tokio_tungstenite::Connector::Plain)
        };

        let connect_result = timeout(
            connect_timeout,
            connect_async_tls_with_config(request, None, false, connector),
        )
        .await;

        match connect_result {
            Ok(Ok((ws_stream, _response))) => {
                // 8. 连接Success，分割流
                let (sender, receiver) = ws_stream.split();

                // 9. 更新连接状态
                let mut connections = self
                    .connections
                    .lock()
                    .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

                if let Some(conn) = connections.get_mut(&uuid) {
                    conn.set_stream(sender, receiver);

                    // Ping 心跳：如果配置了 ping_interval_ms，启动后台定期发送 Ping 的任务
                    if let Some(interval) = ping_interval_ms {
                        if let Some(sender) = conn.get_sender() {
                            let ping_handle = tokio::spawn(async move {
                                let mut ticker =
                                    tokio::time::interval(Duration::from_millis(interval));
                                loop {
                                    ticker.tick().await;
                                    let mut guard = sender.lock().await;
                                    if guard
                                        .send(WebSocketMessage::Ping(vec![].into()))
                                        .await
                                        .is_err()
                                    {
                                        break; // 连接已关闭
                                    }
                                }
                            });
                            conn.ping_task_handle = Some(ping_handle);
                        }
                    }

                    log::info!(
                        "[WebSocket{}] Connection established: uuid={}, url={}",
                        if use_tls { "+TLS" } else { "" },
                        uuid,
                        url
                    );
                }
                Ok(())
            }
            Ok(Err(e)) => {
                // connection failed（网络Error、TLS Error等）
                let error_msg = format!("WebSocket connection failed ({}): {}", url, e);
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                // 连接超时
                let error_msg = format!(
                    "WebSocket connection timeout ({}ms): {}",
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
        let uri: http::Uri = url
            .parse()
            .map_err(|e| format!("Failed to parse URL: {}", e))?;

        let host = uri.host().ok_or("URL missing host")?;
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
        builder
            .body(())
            .map_err(|e| format!("Failed to build WebSocket request: {}", e))
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
    /// * `Ok(())` - 关闭Success
    /// * `Err(msg)` - 关闭Failed或连接不存在
    pub async fn close(
        &self,
        uuid: &str,
        close_code: u16,
        close_reason: Option<String>,
        wait_close_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 获取连接，检查是否存在
        let sender = {
            let connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

            let conn = connections
                .get(uuid)
                .ok_or_else(|| format!("Connection not found: {}", uuid))?;

            // 2. 检查连接状态
            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "Connection status is not Connected (current: {:?}): {}",
                    conn.status, uuid
                ));
            }

            // 3. 获取发送端克隆
            conn.get_sender()
                .ok_or_else(|| format!("Connection has no active WebSocket stream: {}", uuid))?
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
                let error_msg = format!("Failed to send close message: {}", e);
                self.update_connection_error(uuid, &error_msg);
                return Err(error_msg);
            }
        }

        // 6. 等待服务器确认关闭（超时控制）
        let receiver = {
            let connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
            connections.get(uuid).and_then(|c| c.get_receiver())
        };

        if let Some(receiver) = receiver {
            let close_timeout = Duration::from_millis(wait_close_timeout_ms.clamp(1000, 30000));

            let mut receiver_guard = receiver.lock().await;
            match timeout(close_timeout, receiver_guard.next()).await {
                Ok(Some(Ok(WebSocketMessage::Close(_)))) => {
                    log::info!(
                        "[WebSocket] Server close acknowledgment received: uuid={}",
                        uuid
                    );
                }
                Ok(Some(Ok(_))) => {
                    // Other message received, ignore
                }
                Ok(Some(Err(e))) => {
                    log::warn!(
                        "[WebSocket] Error while waiting for close acknowledgment: {}",
                        e
                    );
                }
                Ok(None) => {
                    // Stream ended
                }
                Err(_) => {
                    log::warn!(
                        "[WebSocket] Close acknowledgment timeout ({}ms): uuid={}",
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
            let mut connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
            if let Some(conn) = connections.get_mut(uuid) {
                conn.clear_stream();
            }
        }

        log::info!(
            "[WebSocket] Connection closed: uuid={}, code={}",
            uuid,
            close_code
        );
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
    /// * `Ok(())` - 发送Success
    /// * `Err(msg)` - 发送Failed或连接不存在
    pub async fn send(
        &self,
        uuid: &str,
        message: String,
        is_binary: bool,
        send_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 获取连接，检查状态是否为 Connected
        let sender = {
            let connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

            let conn = connections
                .get(uuid)
                .ok_or_else(|| format!("Connection not found: {}", uuid))?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "Connection status is not Connected (current: {:?}): {}",
                    conn.status, uuid
                ));
            }

            conn.get_sender()
                .ok_or_else(|| format!("Connection has no active WebSocket stream: {}", uuid))?
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
                log::debug!("[WebSocket] Message sent successfully: uuid={}", uuid);
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("Failed to send message: {}", e);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = format!("Send message timeout ({}ms)", send_timeout_ms);
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
    /// * `Err(msg)` - 接收Failed或连接不存在
    pub async fn receive(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        // 1. 获取连接，检查状态是否为 Connected
        let (receiver, sender) = {
            let connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

            let conn = connections
                .get(uuid)
                .ok_or_else(|| format!("Connection not found: {}", uuid))?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "Connection status is not Connected (current: {:?}): {}",
                    conn.status, uuid
                ));
            }

            let receiver = conn
                .get_receiver()
                .ok_or_else(|| format!("Connection has no active WebSocket stream: {}", uuid))?;

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
                    "[WebSocket] Receive timeout ({}ms): uuid={}",
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
                            log::debug!("[WebSocket] Text message received: uuid={}", uuid);
                            return Ok(Some(text.to_string()));
                        }
                        WebSocketMessage::Binary(data) => {
                            log::debug!(
                                "[WebSocket] Binary message received: uuid={}, len={}",
                                uuid,
                                data.len()
                            );
                            return Ok(Some(bytes_to_hex(&data)));
                        }
                        WebSocketMessage::Ping(data) => {
                            // Auto-respond to Ping: send Pong and continue reading next message
                            log::debug!(
                                "[WebSocket] Ping received, auto-responding with Pong: uuid={}",
                                uuid
                            );
                            if let Some(sender) = &sender {
                                let mut sender_guard = sender.lock().await;
                                let pong = WebSocketMessage::Pong(data);
                                if let Err(e) = sender_guard.send(pong).await {
                                    log::warn!(
                                        "[WebSocket] Failed to send Pong: uuid={}, error={}",
                                        uuid,
                                        e
                                    );
                                }
                            }
                            // Release receiver lock and continue reading next message
                            drop(receiver_guard);
                            continue;
                        }
                        WebSocketMessage::Pong(_) => {
                            // Update last_pong_received timestamp
                            log::debug!(
                                "[WebSocket] Pong received, updating timestamp: uuid={}",
                                uuid
                            );
                            if let Ok(mut conns) = self.connections.lock() {
                                if let Some(conn) = conns.get_mut(uuid) {
                                    conn.last_pong_received = Some(std::time::Instant::now());
                                }
                            }
                            drop(receiver_guard);
                            continue;
                        }
                        WebSocketMessage::Close(frame) => {
                            log::info!(
                                "[WebSocket] Close frame received: uuid={}, frame={:?}",
                                uuid,
                                frame
                            );
                            // Update connection status
                            self.update_status(uuid, ConnectionStatus::Closed)?;
                            return Ok(None);
                        }
                        WebSocketMessage::Frame(_) => {
                            // Raw frame, usually not handled directly
                            return Ok(None);
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let error_msg = format!("Failed to receive message: {}", e);
                    self.update_connection_error(uuid, &error_msg);
                    return Err(error_msg);
                }
                Ok(None) => {
                    // Stream ended
                    log::info!("[WebSocket] Stream ended: uuid={}", uuid);
                    self.update_status(uuid, ConnectionStatus::Closed)?;
                    return Ok(None);
                }
                Err(_) => {
                    // Timeout - no message received
                    log::debug!(
                        "[WebSocket] Receive timeout ({}ms): uuid={}",
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
    /// * `Err(msg)` - 连接不存在或状态Error
    pub async fn drain_pending_messages(&self, uuid: &str) -> Result<usize, String> {
        // 1. 获取连接，检查状态是否为 Connected
        let (receiver, sender) = {
            let connections = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

            let conn = connections
                .get(uuid)
                .ok_or_else(|| format!("Connection not found: {}", uuid))?;

            if conn.status != ConnectionStatus::Connected {
                return Err(format!(
                    "Connection status is not Connected (current: {:?}): {}",
                    conn.status, uuid
                ));
            }

            let receiver = conn
                .get_receiver()
                .ok_or_else(|| format!("Connection has no active WebSocket stream: {}", uuid))?;

            let sender = conn.get_sender();

            (receiver, sender)
        };

        // 2. Use a very short timeout (1ms) to non-blockingly read all pending messages
        let mut drained_count = 0;
        let drain_timeout = Duration::from_millis(1);
        let mut receiver_guard = receiver.lock().await;

        loop {
            match timeout(drain_timeout, receiver_guard.next()).await {
                Ok(Some(Ok(message))) => {
                    // Log discarded messages (for debugging)
                    match &message {
                        WebSocketMessage::Text(text) => {
                            log::debug!(
                                "[WebSocket] drain_pending_messages: Discarding text message uuid={}, len={}, preview={}",
                                uuid,
                                text.len(),
                                &text.chars().take(50).collect::<String>()
                            );
                        }
                        WebSocketMessage::Binary(data) => {
                            log::debug!(
                                "[WebSocket] drain_pending_messages: Discarding binary message uuid={}, len={}",
                                uuid,
                                data.len()
                            );
                        }
                        WebSocketMessage::Ping(data) => {
                            // Auto-reply to Ping with Pong
                            log::debug!(
                                "[WebSocket] drain_pending_messages: Ping received, auto-replying with Pong uuid={}",
                                uuid
                            );
                            if let Some(sender) = &sender {
                                let mut sender_guard = sender.lock().await;
                                let pong = WebSocketMessage::Pong(data.clone());
                                if let Err(e) = sender_guard.send(pong).await {
                                    log::warn!(
                                        "[WebSocket] drain_pending_messages: Failed to send Pong uuid={}, error={}",
                                        uuid,
                                        e
                                    );
                                }
                            }
                        }
                        WebSocketMessage::Pong(_) => {
                            // Update last_pong_received timestamp
                            log::debug!(
                                "[WebSocket] drain_pending_messages: Pong received, updating timestamp uuid={}",
                                uuid
                            );
                            if let Ok(mut conns) = self.connections.lock() {
                                if let Some(conn) = conns.get_mut(uuid) {
                                    conn.last_pong_received = Some(std::time::Instant::now());
                                }
                            }
                        }
                        WebSocketMessage::Close(_) => {
                            // Close frame received, update status and exit
                            log::info!(
                                "[WebSocket] drain_pending_messages: Close frame received uuid={}",
                                uuid
                            );
                            self.update_status(uuid, ConnectionStatus::Closed)?;
                            break;
                        }
                        WebSocketMessage::Frame(_) => {
                            // Raw frame, ignore
                        }
                    }
                    drained_count += 1;
                    // Try to read more messages
                }
                Ok(Some(Err(e))) => {
                    log::warn!(
                        "[WebSocket] drain_pending_messages: Error reading message uuid={}, error={}",
                        uuid,
                        e
                    );
                    // Stop draining on error
                    break;
                }
                Ok(None) => {
                    // Stream ended
                    log::debug!(
                        "[WebSocket] drain_pending_messages: Stream ended uuid={}",
                        uuid
                    );
                    break;
                }
                Err(_) => {
                    // Timeout, no more pending messages
                    break;
                }
            }
        }

        if drained_count > 0 {
            log::info!(
                "[WebSocket] drain_pending_messages: Discarded {} pending message(s) uuid={}",
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
        connections
            .get(uuid)
            .map(|c| ActiveConnectionMeta::from(c).into())
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
    /// 操作是否Success（连接是否存在）
    pub fn update_status(&self, uuid: &str, status: ConnectionStatus) -> Result<(), String> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
        if let Some(conn) = connections.get_mut(uuid) {
            conn.set_status(status);
            Ok(())
        } else {
            Err(format!("Connection not found: {}", uuid))
        }
    }

    /// 更新连接Error状态（内部方法）
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
    /// * `Ok(true)`: 消息匹配Success
    /// * `Ok(false)`: 消息不匹配
    /// * `Err(msg)`: 匹配过程出错
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let manager = WebSocketConnectionManager::new();
    /// let condition = WsCollectCondition {
    ///     match_type: WsMatchType::JsonPath,
    ///     expression: "$.status".to_string(),
    /// };
    /// let result = manager.match_message(r#"{"status":"ok"}"#, &condition)?;
    /// assert!(result);
    /// ```
    pub fn match_message(
        &self,
        message: &str,
        condition: &WsCollectCondition,
    ) -> Result<bool, String> {
        match condition.match_type {
            WsMatchType::JsonPath => self.match_jsonpath(message, &condition.expression),
            WsMatchType::Regex => self.match_regex(message, &condition.expression),
            WsMatchType::Expression => self.match_expression(message, &condition.expression),
        }
    }

    /// `JSONPath` 匹配
    ///
    /// 使用 `JSONPath` 表达式查询 JSON 消息，非空结果表示匹配Success。
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
    /// * `Err(msg)`: JSON 解析Failed或 `JSONPath` 语法Error
    fn match_jsonpath(&self, message: &str, expression: &str) -> Result<bool, String> {
        // 1. 解析 JSON
        let json: serde_json::Value =
            serde_json::from_str(message).map_err(|e| format!("JSON parse failed: {}", e))?;

        // 2. 编译 JSONPath 表达式
        let path =
            JsonPath::parse(expression).map_err(|e| format!("JSONPath syntax error: {}", e))?;

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
    /// * `Err(msg)`: 正则表达式语法Error
    fn match_regex(&self, message: &str, pattern: &str) -> Result<bool, String> {
        // 1. 编译正则表达式
        let regex = Regex::new(pattern).map_err(|e| format!("Regex syntax error: {}", e))?;

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
    /// * `Err(msg)`: `表达式语法Error或求值Failed`
    ///
    /// # 说明
    ///
    /// 如果消息是 JSON 格式，会自动解析并将顶层字段添加到表达式上下文中。
    /// 同时提供 `message` 变量引用原始消息文本。
    fn match_expression(&self, message: &str, expression: &str) -> Result<bool, String> {
        // 1. 尝试解析 JSON（如果消息是 JSON 格式）
        let json_value: Option<serde_json::Value> = serde_json::from_str(message).ok();

        // 2. 创建表达式上下文
        let mut context = HashMapContext::new();

        // 3. 添加变量到上下文
        // 消息文本
        context
            .set_value("message".to_string(), Value::String(message.to_string()))
            .map_err(|e| format!("Failed to set variable: {}", e))?;

        // 如果是 JSON，添加解析后的字段
        if let Some(serde_json::Value::Object(map)) = json_value {
            // 添加顶层字段（兼容旧行为：直接比较顶层值）
            for (key, value) in &map {
                let evalexpr_value = json_to_evalexpr_value(value);
                context
                    .set_value(key.clone(), evalexpr_value)
                    .map_err(|e| format!("Failed to set variable: {}", e))?;
            }

            // 递归展开嵌套字段为扁平路径（支持 data.stepKey 访问方式）
            for (key, value) in &map {
                if matches!(value, serde_json::Value::Object(_)) {
                    flatten_json_to_context(value, key, &mut context)?;
                }
            }
        }

        // 4. 预处理表达式：将 x.y.z 转换为 x__y__z（保留字符串字面量不变）
        let processed_expression = preprocess_expression(expression)?;

        // 5. 求值表达式
        let eval_result = evalexpr::eval_with_context(&processed_expression, &context)
            .map_err(|e| format!("Expression evaluation failed: {}", e))?;

        // 6. 转换为布尔值
        let result = match eval_result {
            Value::Boolean(b) => b,
            Value::Int(i) => i != 0,
            Value::Float(f) => f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Empty => false,
            _ => {
                return Err(format!(
                    "Expression returned unsupported type: {:?}",
                    eval_result
                ));
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
        use_tls: bool,
        tls_skip_verify: bool,
        _ping_interval_ms: Option<u64>,
        _ping_timeout_ms: Option<u64>,
    ) -> Result<(), String> {
        WebSocketConnectionManager::connect(
            self,
            uuid,
            name,
            url,
            headers,
            subprotocols,
            connect_timeout_ms,
            use_tls,
            tls_skip_verify,
            _ping_interval_ms,
            _ping_timeout_ms,
        )
        .await
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

    async fn receive(&self, uuid: &str, receive_timeout_ms: u64) -> Result<Option<String>, String> {
        WebSocketConnectionManager::receive(self, uuid, receive_timeout_ms).await
    }

    async fn close(
        &self,
        uuid: &str,
        close_code: u16,
        close_reason: Option<String>,
        wait_close_timeout_ms: u64,
    ) -> Result<(), String> {
        WebSocketConnectionManager::close(
            self,
            uuid,
            close_code,
            close_reason,
            wait_close_timeout_ms,
        )
        .await
    }

    async fn drain_pending_messages(&self, uuid: &str) -> Result<usize, String> {
        WebSocketConnectionManager::drain_pending_messages(self, uuid).await
    }

    fn match_message(&self, message: &str, condition: &WsCollectCondition) -> Result<bool, String> {
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
            ping_task_handle: None,
            last_pong_received: None,
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
        return Err("Hex string length must be even".to_string());
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Failed to parse hex: {}", e))
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

/// 递归展开 JSON 嵌套对象为扁平的变量路径（使用 `__` 分隔符）
///
/// 将嵌套的 JSON 对象展开为扁平的变量名，以便在 evalexpr 表达式中通过
/// 预处理后的 `x__y__z` 方式访问嵌套字段。
///
/// # 示例
///
/// ```ignore
/// // 输入 JSON: {"stepKey": "test", "result": {"title": "hello"}}
/// // prefix = "data"
/// // 展开结果:
/// //   data__stepKey = "test"
/// //   data__result__title = "hello"
/// ```
fn flatten_json_to_context(
    json: &serde_json::Value,
    prefix: &str,
    context: &mut HashMapContext,
) -> Result<(), String> {
    if let serde_json::Value::Object(map) = json {
        for (key, value) in map {
            let flat_key = format!("{}__{}", prefix, key);
            match value {
                serde_json::Value::Object(_) => {
                    // 递归展开嵌套对象
                    flatten_json_to_context(value, &flat_key, context)?;
                    // 同时保留对象的字符串表示（兼容直接比较整个对象）
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Failed to set variable: {}", e))?;
                }
                serde_json::Value::Array(arr) => {
                    // 展开数组元素（支持索引访问，如 items.0.name）
                    for (i, item) in arr.iter().enumerate() {
                        let item_key = format!("{}__{}", flat_key, i);
                        match item {
                            serde_json::Value::Object(_) => {
                                flatten_json_to_context(item, &item_key, context)?;
                            }
                            _ => {
                                let eval_value = json_to_evalexpr_value(item);
                                context
                                    .set_value(item_key, eval_value)
                                    .map_err(|e| format!("Failed to set variable: {}", e))?;
                            }
                        }
                    }
                    // 保留整个数组的字符串表示
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Failed to set variable: {}", e))?;
                }
                _ => {
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Failed to set variable: {}", e))?;
                }
            }
        }
    }
    Ok(())
}

/// 预处理表达式：将标识符链中的点号转换为双下划线
///
/// 自动将表达式中的 `x.y.z` 格式转换为 `x__y__z`，
/// 以便与 `flatten_json_to_context` 展开的扁平变量名对应。
/// 字符串字面量内的内容不会被修改。
///
/// # 示例
///
/// ```ignore
/// preprocess_expression(r#"data.stepKey == "intent_unsupported""#)
/// // → r#"data__stepKey == "intent_unsupported""#
/// ```
fn preprocess_expression(expr: &str) -> Result<String, String> {
    // 匹配双引号字符串字面量（处理转义字符）
    let string_re = Regex::new(r#""(?:[^"\\]|\\.)*""#)
        .map_err(|e| format!("Regex syntax error in preprocess_expression: {}", e))?;

    let mut result = String::with_capacity(expr.len() * 2);
    let mut last_end = 0;

    for cap in string_re.find_iter(expr) {
        // 处理字符串字面量之前的非字符串部分
        if cap.start() > last_end {
            result.push_str(&flatten_identifier_chains(&expr[last_end..cap.start()])?);
        }

        // 保留字符串字面量不变
        result.push_str(cap.as_str());

        last_end = cap.end();
    }

    // 处理最后一段非字符串部分
    if last_end < expr.len() {
        result.push_str(&flatten_identifier_chains(&expr[last_end..])?);
    }

    Ok(result)
}

/// 将标识符链中的点号替换为双下划线
///
/// 匹配 `[a-zA-Z_]\w*(?:\.\w+)+` 模式（至少包含一个点号的标识符链），
/// 将其中的 `.` 替换为 `__`。不含点号的标识符不会被修改。
///
/// # 示例
///
/// ```ignore
/// flatten_identifier_chains("data.stepKey == 200")  // → "data__stepKey == 200"
/// flatten_identifier_chains("code == 200")           // → "code == 200"（无变化）
/// ```
fn flatten_identifier_chains(s: &str) -> Result<String, String> {
    // 匹配: 字母/下划线开头，后跟一个或多个 `.word` 模式
    // 例如: data.stepKey, data.result.title, items.0.name
    // 不匹配: 3.14（不以字母/下划线开头）
    let re = Regex::new(r"[a-zA-Z_]\w*(?:\.\w+)+")
        .map_err(|e| format!("Regex syntax error in flatten_identifier_chains: {}", e))?;
    Ok(re
        .replace_all(s, |caps: &regex::Captures| caps[0].replace('.', "__"))
        .into_owned())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::{WsCollectCondition, WsMatchType};

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
        let json = serde_json::to_string(&status).expect("Serialization failed");
        assert_eq!(json, r#""connected""#);

        let deserialized: ConnectionStatus =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized, ConnectionStatus::Connected);
    }

    #[test]
    fn test_active_connection_creation() {
        let conn = create_test_connection("uuid-1", "test connection", "wss://example.com/ws");

        assert_eq!(conn.uuid, "uuid-1");
        assert_eq!(conn.name, "test connection");
        assert_eq!(conn.url, "wss://example.com/ws");
        assert_eq!(conn.status, ConnectionStatus::Connecting);
        assert!(conn.buffer_size() == 0);
        assert!(conn.last_error.is_none());
        assert!(!conn.is_connected());
    }

    #[test]
    fn test_active_connection_set_status() {
        let mut conn = create_test_connection("uuid-1", "test", "wss://example.com/ws");

        conn.set_status(ConnectionStatus::Connected);
        assert_eq!(conn.status, ConnectionStatus::Connected);

        conn.set_status(ConnectionStatus::Closing);
        assert_eq!(conn.status, ConnectionStatus::Closing);
    }

    #[test]
    fn test_active_connection_set_error() {
        let mut conn = create_test_connection("uuid-1", "test", "wss://example.com/ws");

        conn.set_error("Connection failed".to_string());
        assert_eq!(conn.status, ConnectionStatus::Error);
        assert_eq!(conn.last_error, Some("Connection failed".to_string()));
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
        assert!(matches!(msg1, Some(WebSocketMessage::Text(ref t)) if t.as_str() == "hello"));
        assert_eq!(conn.buffer_size(), 1);

        let msg2 = conn.pop_message();
        assert!(matches!(msg2, Some(WebSocketMessage::Binary(ref b)) if b.as_ref() == [1, 2, 3]));
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
        let mut conn = create_test_connection("uuid-1", "test", "wss://example.com/ws");
        conn.set_owner("node-123", "segment-456");

        // 测试 From<ActiveConnection> for ActiveConnectionMeta
        let meta = ActiveConnectionMeta::from(&conn);
        assert_eq!(meta.current_owner_node_uuid, Some("node-123".to_string()));
        assert_eq!(meta.stream_segment_id, Some("segment-456".to_string()));

        // 测试 From<ActiveConnectionMeta> for ActiveConnection
        let conn_from_meta: ActiveConnection = meta.into();
        assert_eq!(
            conn_from_meta.current_owner_node_uuid,
            Some("node-123".to_string())
        );
        assert_eq!(
            conn_from_meta.stream_segment_id,
            Some("segment-456".to_string())
        );
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
        let conn1 = create_test_connection("uuid-1", "conn1", "wss://example.com/ws1");

        // 添加新连接
        let result = manager.upsert_connection(conn1);
        assert!(result.is_none());
        assert_eq!(manager.connection_count(), 1);

        // 更新已存在的连接
        let conn2 = create_test_connection("uuid-1", "conn1-updated", "wss://example.com/ws2");
        let old = manager.upsert_connection(conn2);
        assert!(old.is_some());
        assert_eq!(old.unwrap().name, "conn1");
        assert_eq!(manager.connection_count(), 1);

        // 验证更新Success
        let current = manager.get_connection("uuid-1").unwrap();
        assert_eq!(current.name, "conn1-updated");
        assert_eq!(current.url, "wss://example.com/ws2");
    }

    #[test]
    fn test_manager_get_connection() {
        let manager = WebSocketConnectionManager::new();
        let conn = create_test_connection("uuid-1", "conn1", "wss://example.com/ws");

        manager.upsert_connection(conn);

        // 存在的连接
        let found = manager.get_connection("uuid-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "conn1");

        // 不存在的连接
        let not_found = manager.get_connection("uuid-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_manager_get_all_connections() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "conn2", "wss://b.com"));
        manager.upsert_connection(create_test_connection("uuid-3", "conn3", "wss://c.com"));

        let all = manager.get_all_connections();
        assert_eq!(all.len(), 3);

        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"conn1"));
        assert!(names.contains(&"conn2"));
        assert!(names.contains(&"conn3"));
    }

    #[test]
    fn test_manager_remove_connection() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));

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
        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));

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
        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "conn2", "wss://b.com"));

        assert_eq!(manager.connection_count(), 2);

        manager.clear();

        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_all_connections().is_empty());
    }

    #[test]
    fn test_manager_clone() {
        let manager1 = WebSocketConnectionManager::new();
        manager1.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));

        // 克隆管理器（共享同一内部状态）
        let manager2 = manager1.clone();

        // 通过 manager2 修改
        manager2.upsert_connection(create_test_connection("uuid-2", "conn2", "wss://b.com"));

        // manager1 应该看到变化（Arc 共享）
        assert_eq!(manager1.connection_count(), 2);
        assert_eq!(manager2.connection_count(), 2);
    }

    #[test]
    fn test_manager_cleanup_closed() {
        let manager = WebSocketConnectionManager::new();

        // 添加连接
        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "conn2", "wss://b.com"));

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

        manager.upsert_connection(create_test_connection("uuid-1", "conn1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "conn2", "wss://b.com"));

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

        // 奇数长度应该Failed
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
                "test connection".to_string(),
                "http://invalid.com".to_string(), // 无效协议
                vec![],                           // headers
                vec![],                           // subprotocols
                5000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid WebSocket URL"));

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
                "timeout test".to_string(),
                "ws://192.0.2.1:81/ws".to_string(), // RFC 3330 测试地址，不可路由
                vec![],                             // headers
                vec![],                             // subprotocols
                2000,                               // 2 秒超时
                false,                              // use_tls
                false,                              // tls_skip_verify
                None,                               // ping_interval_ms
                None,                               // ping_timeout_ms
            )
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("timeout") || error.contains("failed"),
            "Error消息应该包含超时或Failed: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager.close("nonexistent", 1000, None, 5000).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Connection not found"));
    }

    #[tokio::test]
    async fn test_send_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager
            .send("nonexistent", "hello".to_string(), false, 5000)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Connection not found"));
    }

    #[tokio::test]
    async fn test_receive_nonexistent_connection() {
        let manager = WebSocketConnectionManager::new();

        let result = manager.receive("nonexistent", 5000).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Connection not found"));
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_connect_to_echo_server() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "echo-conn".to_string(),
                "Echo Test".to_string(),
                "ws://localhost:19091".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        // 网络测试：如果connection failed，跳过而不是Failed
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
                let close_result = manager
                    .close("echo-conn", 1000, Some("测试完成".to_string()), 5000)
                    .await;
                assert!(close_result.is_ok());

                // 验证连接状态
                let status = manager.get_status("echo-conn").unwrap();
                assert_eq!(status, ConnectionStatus::Closed);
            }
            Err(e) => {
                // 网络不可用时跳过测试
                println!("Skipping test: unable to connect to echo server: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_send_binary_message() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "binary-conn".to_string(),
                "binary test".to_string(),
                "ws://localhost:19091".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        // 网络测试：如果connection failed，跳过而不是Failed
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
            println!("Skipping test: unable to connect to echo server");
        }
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_receive_timeout() {
        let manager = WebSocketConnectionManager::new();

        // 连接到公共 WebSocket echo 服务器
        let result = manager
            .connect(
                "timeout-conn".to_string(),
                "timeout test".to_string(),
                "ws://localhost:19091".to_string(),
                vec![], // headers
                vec![], // subprotocols
                30000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        // 网络测试：如果connection failed，跳过而不是Failed
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
            println!("Skipping test: unable to connect to echo server");
        }
    }

    #[test]
    fn test_status_tracking() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection(
            "track-1",
            "tracking test",
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.status".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON parse failed"));
    }

    #[test]
    fn test_match_jsonpath_invalid_expression() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":"ok"}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::JsonPath,
            expression: "$.[invalid".to_string(), // 无效的 JSONPath
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSONPath syntax error"));
    }

    #[test]
    fn test_match_regex_success() {
        let manager = WebSocketConnectionManager::new();
        let message = "Hello World 123";
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
            match_type: WsMatchType::Regex,
            expression: r"[".to_string(), // 无效的正则表达式
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Regex syntax error"));
    }

    #[test]
    fn test_match_expression_simple() {
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"status":200}"#;
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: "status ==".to_string(), // 无效的表达式
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expression evaluation failed"));
    }

    #[test]
    fn test_match_expression_non_json() {
        let manager = WebSocketConnectionManager::new();
        let message = "plain text message";
        let condition = WsCollectCondition {
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
        let condition = WsCollectCondition {
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

    // ============================================================================
    // 嵌套 JSON 字段访问测试
    // ============================================================================

    #[test]
    fn test_match_expression_nested_field() {
        // 模拟用户场景: data.stepKey == "intent_unsupported"
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"code": 0, "data": {"stepKey": "intent_unsupported", "taskID": 102555}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: r#"data.stepKey == "intent_unsupported""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_nested_field_no_match() {
        // data.stepKey 不等于目标值
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"code": 0, "data": {"stepKey": "intent_message", "taskID": 102555}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: r#"data.stepKey == "intent_unsupported""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_match_expression_deeply_nested_field() {
        // 三层嵌套: data.result.title
        let manager = WebSocketConnectionManager::new();
        let message =
            r#"{"code": 0, "data": {"result": {"title": "意图消息", "content": "你好"}}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: r#"data.result.title == "意图消息""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_nested_numeric_field() {
        // 嵌套数值字段比较
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"data": {"taskID": 102555}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: "data.taskID > 100000".to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_top_level_and_nested_mixed() {
        // 同时使用顶层字段和嵌套字段
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"code": 0, "success": true, "data": {"stepKey": "intent_unsupported"}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: r#"code == 0 && data.stepKey == "intent_unsupported""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_expression_nested_with_string_literal_containing_dots() {
        // 字符串字面量内的点号不应被转换
        let manager = WebSocketConnectionManager::new();
        let message = r#"{"data": {"url": "http://example.com/api"}}"#;
        let condition = WsCollectCondition {
            match_type: WsMatchType::Expression,
            expression: r#"data.url == "http://example.com/api""#.to_string(),
        };

        let result = manager.match_message(message, &condition);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_preprocess_expression_basic() {
        // 基本点号转换
        assert_eq!(
            preprocess_expression(r#"data.stepKey == "test""#).unwrap(),
            r#"data__stepKey == "test""#
        );
    }

    #[test]
    fn test_preprocess_expression_deeply_nested() {
        // 多层嵌套
        assert_eq!(
            preprocess_expression(r#"data.result.title == "hello""#).unwrap(),
            r#"data__result__title == "hello""#
        );
    }

    #[test]
    fn test_preprocess_expression_preserves_strings() {
        // 字符串字面量内的点号不被转换
        assert_eq!(
            preprocess_expression(r#"data.url == "http://example.com""#).unwrap(),
            r#"data__url == "http://example.com""#
        );
    }

    #[test]
    fn test_preprocess_expression_no_dots() {
        // 没有点号的表达式不变
        assert_eq!(
            preprocess_expression("code == 200").unwrap(),
            "code == 200"
        );
    }

    #[test]
    fn test_preprocess_expression_multiple_chains() {
        // 多个标识符链
        assert_eq!(
            preprocess_expression(
                r#"data.stepKey == "test" && data.result.code == 0"#
            )
            .unwrap(),
            r#"data__stepKey == "test" && data__result__code == 0"#
        );
    }

    #[test]
    fn test_flatten_identifier_chains_basic() {
        assert_eq!(
            flatten_identifier_chains("data.stepKey == 200").unwrap(),
            "data__stepKey == 200"
        );
    }

    #[test]
    fn test_flatten_identifier_chains_no_match() {
        // 不含点号的标识符不修改
        assert_eq!(
            flatten_identifier_chains("code == 200").unwrap(),
            "code == 200"
        );
    }

    #[test]
    fn test_flatten_identifier_chains_float_not_matched() {
        // 浮点数字面量不以字母开头，不会被匹配
        assert_eq!(flatten_identifier_chains("3.14").unwrap(), "3.14");
    }
}
