//! TCP 连接管理器模块
//!
//! 管理 TCP 连接池，实现 `TcpProvider` trait。
//! 使用 `tokio::net::TcpStream` + `tokio::io::split()` 实现连接的读写分离。
//! 支持 TLS 加密连接（通过 tokio-rustls），提供三种模式：
//! - 明文 TCP（`use_tls=false`）
//! - 标准 TLS（`use_tls=true, tls_skip_verify=false`）
//! - 跳过验证 TLS（`use_tls=true, tls_skip_verify=true`）
//!
//! 线程安全，使用 `Arc<Mutex<>>` 保护连接池。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use flow_engine_core::node_config::TcpKeepaliveConfig;
use flow_engine_core::traits::{FramingConfig, TcpConnectionInfo, TcpProvider};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

// ---------------------------------------------------------------------------
// 类型别名（统一明文/TLS 流）
// ---------------------------------------------------------------------------

/// 装箱的异步读取端（支持明文 TCP 和 TLS 流的统一存储）
type BoxedReader = Box<dyn AsyncRead + Unpin + Send>;

/// 装箱的异步写入端（支持明文 TCP 和 TLS 流的统一存储）
type BoxedWriter = Box<dyn AsyncWrite + Unpin + Send>;

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// TCP 连接状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 已关闭
    Closed,
    /// 连接Error
    Error,
}

impl ConnectionStatus {
    /// 转换为字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatus::Connecting => "Connecting",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Closed => "Closed",
            ConnectionStatus::Error => "Error",
        }
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// 活跃 TCP 连接
// ---------------------------------------------------------------------------

/// 活跃的 TCP 连接
///
/// 存储单个 TCP 连接的状态、元数据和实际的读写流。
pub struct ActiveTcpConnection {
    /// 连接唯一标识（节点 UUID）
    pub uuid: String,
    /// 连接名称
    pub name: String,
    /// 连接状态
    pub status: ConnectionStatus,
    /// 目标主机地址
    pub host: String,
    /// 目标端口
    pub port: u16,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: u64,
    /// 是否使用 TLS
    pub use_tls: bool,
    /// 最近一次Error信息
    pub last_error: Option<String>,
    /// 帧定界配置
    pub framing_config: FramingConfig,

    /// TCP 读取端（通过 Arc<Mutex> 实现线程安全共享）
    read_half: Option<Arc<tokio::sync::Mutex<BoxedReader>>>,
    /// TCP 写入端（通过 Arc<Mutex> 实现线程安全共享）
    write_half: Option<Arc<tokio::sync::Mutex<BoxedWriter>>>,
}

impl std::fmt::Debug for ActiveTcpConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveTcpConnection")
            .field("uuid", &self.uuid)
            .field("name", &self.name)
            .field("status", &self.status)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("created_at", &self.created_at)
            .field("use_tls", &self.use_tls)
            .field("last_error", &self.last_error)
            .field("has_read_half", &self.read_half.is_some())
            .field("has_write_half", &self.write_half.is_some())
            .finish()
    }
}

impl ActiveTcpConnection {
    /// 创建新的活跃连接（状态为 Connecting）
    pub fn new(
        uuid: String,
        name: String,
        host: String,
        port: u16,
        use_tls: bool,
        framing_config: FramingConfig,
    ) -> Self {
        let created_at = flow_engine_core::utils::unix_timestamp_ms();

        Self {
            uuid,
            name,
            status: ConnectionStatus::Connecting,
            host,
            port,
            created_at,
            use_tls,
            last_error: None,
            framing_config,
            read_half: None,
            write_half: None,
        }
    }

    /// 设置连接流（连接Success后调用）
    ///
    /// 接受装箱的读写端，支持明文 TCP 和 TLS 流的统一处理。
    pub fn set_stream(&mut self, read_half: BoxedReader, write_half: BoxedWriter) {
        self.read_half = Some(Arc::new(tokio::sync::Mutex::new(read_half)));
        self.write_half = Some(Arc::new(tokio::sync::Mutex::new(write_half)));
        self.status = ConnectionStatus::Connected;
    }

    /// 设置Error状态
    pub fn set_error(&mut self, error: String) {
        self.status = ConnectionStatus::Error;
        self.last_error = Some(error);
    }

    /// 获取读取端的克隆
    pub fn get_read_half(&self) -> Option<Arc<tokio::sync::Mutex<BoxedReader>>> {
        self.read_half.clone()
    }

    /// 获取写入端的克隆
    pub fn get_write_half(&self) -> Option<Arc<tokio::sync::Mutex<BoxedWriter>>> {
        self.write_half.clone()
    }

    /// 检查是否有活跃的 TCP 连接
    pub fn is_connected(&self) -> bool {
        self.status == ConnectionStatus::Connected
            && self.read_half.is_some()
            && self.write_half.is_some()
    }

    /// 清理流资源
    pub fn clear_stream(&mut self) {
        self.read_half = None;
        self.write_half = None;
    }

    /// 转换为 `TcpConnectionInfo`
    pub fn to_connection_info(&self) -> TcpConnectionInfo {
        TcpConnectionInfo {
            uuid: self.uuid.clone(),
            name: self.name.clone(),
            status: self.status.to_string(),
            host: self.host.clone(),
            port: self.port,
            created_at: self.created_at,
            use_tls: self.use_tls,
            last_error: self.last_error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// TCP 连接管理器
// ---------------------------------------------------------------------------

/// TCP 连接管理器
///
/// 管理所有活跃的 TCP 连接，提供线程安全的连接池操作。
/// 使用 `Arc<Mutex<>>` 实现跨线程共享。
#[derive(Clone)]
pub struct TcpConnectionManager {
    /// 连接池：UUID → `ActiveTcpConnection`
    connections: Arc<Mutex<HashMap<String, ActiveTcpConnection>>>,
    /// 录制管理器（记录 send/receive 交互）
    recording: Arc<tokio::sync::Mutex<crate::recording::RecordingManager>>,
}

impl std::fmt::Debug for TcpConnectionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpConnectionManager")
            .field("connections", &self.connections)
            .field("recording", &"<RecordingManager>")
            .finish()
    }
}

impl TcpConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            recording: Arc::new(tokio::sync::Mutex::new(
                crate::recording::RecordingManager::new(),
            )),
        }
    }

    /// 插入或更新连接
    fn upsert_connection(&self, connection: ActiveTcpConnection) {
        if let Ok(mut conns) = self.connections.lock() {
            conns.insert(connection.uuid.clone(), connection);
        }
    }

    /// 更新连接Error状态
    fn update_connection_error(&self, uuid: &str, error: &str) {
        if let Ok(mut conns) = self.connections.lock() {
            if let Some(conn) = conns.get_mut(uuid) {
                conn.set_error(error.to_string());
            }
        }
    }

    /// 开始录制 TCP 交互
    ///
    /// 返回录制 ID。如果已有活跃录制，先停止旧录制并丢弃。
    pub async fn start_recording(&self, name: &str) -> String {
        let mut rec = self.recording.lock().await;
        rec.start_recording(name)
    }

    /// 停止录制并返回结果
    ///
    /// 如果没有活跃录制，返回 None。
    pub async fn stop_recording(&self) -> Option<crate::recording::TcpRecording> {
        let mut rec = self.recording.lock().await;
        rec.stop_recording()
    }

    /// 是否正在录制
    pub async fn is_recording(&self) -> bool {
        let rec = self.recording.lock().await;
        rec.is_recording()
    }

    /// 建立 TCP 连接（支持 `NoDelay` / Keepalive / 自定义 CA 证书）
    ///
    /// 与 `TcpProvider::connect` 功能相同，但额外支持：
    /// - `no_delay=true` → 禁用 Nagle 算法
    /// - `keepalive=Some(...)` → 配置 TCP Keepalive
    /// - `tls_ca_cert=Some(...)` → 自定义 CA 证书（PEM 格式，追加到系统根证书）
    ///
    /// Socket 选项在 `TcpStream` 创建后、TLS 握手前设置。
    pub async fn connect_with_options(
        &self,
        uuid: String,
        name: String,
        host: String,
        port: u16,
        use_tls: bool,
        tls_skip_verify: bool,
        connect_timeout_ms: u64,
        framing_config: FramingConfig,
        no_delay: bool,
        keepalive: Option<TcpKeepaliveConfig>,
        tls_ca_cert: Option<String>,
    ) -> Result<(), String> {
        // 1. 创建 ActiveTcpConnection（状态为 Connecting）
        let connection = ActiveTcpConnection::new(
            uuid.clone(),
            name,
            host.clone(),
            port,
            use_tls,
            framing_config,
        );

        // 2. 临时存储连接（状态为 Connecting）
        self.upsert_connection(connection);

        // 3. 构建目标地址
        let addr = format!("{}:{}", host, port);

        // 4. 使用超时包装 TcpStream::connect
        let connect_timeout = Duration::from_millis(connect_timeout_ms.clamp(1000, 120000));
        let connect_result = timeout(connect_timeout, TcpStream::connect(&addr)).await;

        match connect_result {
            Ok(Ok(tcp_stream)) => {
                // 5. 设置 socket 选项（必须在 TLS 握手之前）

                // 设置 NoDelay（禁用 Nagle 算法）
                if no_delay {
                    tcp_stream.set_nodelay(true).map_err(|e| {
                        let error_msg = format!("Setting NoDelay failed: {}", e);
                        self.update_connection_error(&uuid, &error_msg);
                        error_msg
                    })?;
                }

                // 设置 Keepalive
                if let Some(ref ka) = keepalive {
                    let socket = socket2::SockRef::from(&tcp_stream);
                    let keepalive_cfg = socket2::TcpKeepalive::new()
                        .with_time(std::time::Duration::from_secs(ka.idle_secs))
                        .with_interval(std::time::Duration::from_secs(ka.interval_secs));
                    socket.set_tcp_keepalive(&keepalive_cfg).map_err(|e| {
                        let error_msg = format!("Setting Keepalive failed: {}", e);
                        self.update_connection_error(&uuid, &error_msg);
                        error_msg
                    })?;
                }

                // 6. 根据 TLS 配置处理流
                let stream_result: Result<(BoxedReader, BoxedWriter), String> = if use_tls {
                    let ca_bytes = tls_ca_cert.as_deref().map(|s| s.as_bytes());
                    let connector =
                        match crate::tls::create_tls_connector_with_ca(tls_skip_verify, ca_bytes) {
                            Ok(c) => c,
                            Err(e) => {
                                self.update_connection_error(&uuid, &e);
                                return Err(e);
                            }
                        };

                    let server_name = match rustls::pki_types::ServerName::try_from(host.clone()) {
                        Ok(name) => name,
                        Err(e) => {
                            let error_msg = format!("Invalid TLS server name '{}': {}", host, e);
                            self.update_connection_error(&uuid, &error_msg);
                            return Err(error_msg);
                        }
                    };

                    match connector.connect(server_name, tcp_stream).await {
                        Ok(tls_stream) => {
                            let (r, w) = tokio::io::split(tls_stream);
                            Ok((Box::new(r) as BoxedReader, Box::new(w) as BoxedWriter))
                        }
                        Err(e) => {
                            let error_msg = format!("TLS handshake failed: {}", e);
                            self.update_connection_error(&uuid, &error_msg);
                            Err(error_msg)
                        }
                    }
                } else {
                    let (r, w) = tokio::io::split(tcp_stream);
                    Ok((Box::new(r) as BoxedReader, Box::new(w) as BoxedWriter))
                };

                match stream_result {
                    Ok((read_half, write_half)) => {
                        // 7. 更新连接状态
                        let mut connections = self
                            .connections
                            .lock()
                            .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

                        if let Some(conn) = connections.get_mut(&uuid) {
                            conn.set_stream(read_half, write_half);
                            log::info!(
                                "[TCP{}] Connection successful: uuid={}, addr={}, no_delay={}, keepalive={}, custom_ca={}",
                                if use_tls { "+TLS" } else { "" },
                                uuid,
                                addr,
                                no_delay,
                                keepalive.is_some(),
                                tls_ca_cert.is_some()
                            );
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            Ok(Err(e)) => {
                let error_msg = format!("TCP connection failed ({}): {}", addr, e);
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = format!(
                    "TCP connection timeout ({}ms): {}",
                    connect_timeout_ms, addr
                );
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
        }
    }
}

impl Default for TcpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TcpProvider for TcpConnectionManager {
    /// 建立 TCP 连接
    ///
    /// 连接到目标 host:port，支持三种模式：
    /// - `use_tls=false` → 明文 TCP
    /// - `use_tls=true, tls_skip_verify=false` → 标准 TLS（webpki-roots 验证）
    /// - `use_tls=true, tls_skip_verify=true` → TLS 跳过证书验证
    ///
    /// TLS 模式下，先建立 TCP 连接再进行 TLS 握手。
    /// 读写端使用 `Box<dyn AsyncRead/AsyncWrite>` 统一存储。
    async fn connect(
        &self,
        uuid: String,
        name: String,
        host: String,
        port: u16,
        use_tls: bool,
        tls_skip_verify: bool,
        connect_timeout_ms: u64,
        framing_config: FramingConfig,
    ) -> Result<(), String> {
        // 1. 创建 ActiveTcpConnection（状态为 Connecting）
        let connection = ActiveTcpConnection::new(
            uuid.clone(),
            name,
            host.clone(),
            port,
            use_tls,
            framing_config,
        );

        // 2. 临时存储连接（状态为 Connecting）
        self.upsert_connection(connection);

        // 3. 构建目标地址
        let addr = format!("{}:{}", host, port);

        // 4. 使用超时包装 TcpStream::connect
        let connect_timeout = Duration::from_millis(connect_timeout_ms.clamp(1000, 120000));
        let connect_result = timeout(connect_timeout, TcpStream::connect(&addr)).await;

        match connect_result {
            Ok(Ok(tcp_stream)) => {
                // 5. 根据 TLS 配置处理流
                let stream_result: Result<(BoxedReader, BoxedWriter), String> = if use_tls {
                    // TLS 模式：创建 TLS 连接器并进行握手
                    let connector = match crate::tls::create_tls_connector(tls_skip_verify) {
                        Ok(c) => c,
                        Err(e) => {
                            self.update_connection_error(&uuid, &e);
                            return Err(e);
                        }
                    };

                    let server_name = match rustls::pki_types::ServerName::try_from(host.clone()) {
                        Ok(name) => name,
                        Err(e) => {
                            let error_msg = format!("Invalid TLS server name '{}': {}", host, e);
                            self.update_connection_error(&uuid, &error_msg);
                            return Err(error_msg);
                        }
                    };

                    match connector.connect(server_name, tcp_stream).await {
                        Ok(tls_stream) => {
                            let (r, w) = tokio::io::split(tls_stream);
                            Ok((Box::new(r) as BoxedReader, Box::new(w) as BoxedWriter))
                        }
                        Err(e) => {
                            let error_msg = format!("TLS handshake failed: {}", e);
                            self.update_connection_error(&uuid, &error_msg);
                            Err(error_msg)
                        }
                    }
                } else {
                    // 明文模式：直接分割 TCP 流
                    let (r, w) = tokio::io::split(tcp_stream);
                    Ok((Box::new(r) as BoxedReader, Box::new(w) as BoxedWriter))
                };

                match stream_result {
                    Ok((read_half, write_half)) => {
                        // 6. 更新连接状态
                        let mut connections = self
                            .connections
                            .lock()
                            .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;

                        if let Some(conn) = connections.get_mut(&uuid) {
                            conn.set_stream(read_half, write_half);
                            log::info!(
                                "[TCP{}] Connection successful: uuid={}, addr={}",
                                if use_tls { "+TLS" } else { "" },
                                uuid,
                                addr
                            );
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            Ok(Err(e)) => {
                // connection failed（网络Error、拒绝连接等）
                let error_msg = format!("TCP connection failed ({}): {}", addr, e);
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                // 连接超时
                let error_msg = format!(
                    "TCP connection timeout ({}ms): {}",
                    connect_timeout_ms, addr
                );
                self.update_connection_error(&uuid, &error_msg);
                Err(error_msg)
            }
        }
    }

    /// 发送数据
    ///
    /// 查找连接池中的连接，通过写入端发送原始字节数据。
    async fn send(&self, uuid: &str, data: Vec<u8>, send_timeout_ms: u64) -> Result<(), String> {
        // 1. 获取写入端（克隆后释放锁）
        let write_half = {
            let conns = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("TCP connection '{}' not found", uuid))?;

            if !conn.is_connected() {
                return Err(format!(
                    "TCP connection '{}' is not in connected state",
                    uuid
                ));
            }

            conn.get_write_half()
                .ok_or_else(|| format!("TCP connection '{}' write half unavailable", uuid))?
        }; // 锁在此释放

        // 2. 带超时发送数据
        let send_timeout = Duration::from_millis(send_timeout_ms.clamp(100, 120000));
        let mut writer = write_half.lock().await;

        let send_result = timeout(send_timeout, writer.write_all(&data)).await;

        match send_result {
            Ok(Ok(_)) => {
                // 记录发送数据（仅在录制状态下）
                if let Ok(mut rec) = self.recording.try_lock() {
                    if rec.is_recording() {
                        rec.record_send(uuid, &data);
                    }
                }
                log::debug!("[TCP] Send successful: uuid={}, bytes={}", uuid, data.len());
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("TCP send failed ({}): {}", uuid, e);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = format!("TCP send timeout ({}ms): uuid={}", send_timeout_ms, uuid);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
        }
    }

    /// 接收数据
    ///
    /// 查找连接池中的连接，从读取端读取原始字节数据。
    /// 返回原始字节，不做帧处理（帧处理由 T10 实现）。
    async fn receive(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<Vec<u8>>, String> {
        // 1. 获取读取端（克隆后释放锁）
        let read_half = {
            let conns = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("TCP connection '{}' not found", uuid))?;

            if !conn.is_connected() {
                return Err(format!(
                    "TCP connection '{}' is not in connected state",
                    uuid
                ));
            }

            conn.get_read_half()
                .ok_or_else(|| format!("TCP connection '{}' read half unavailable", uuid))?
        }; // 锁在此释放

        // 2. 带超时读取数据
        let receive_timeout = Duration::from_millis(receive_timeout_ms.clamp(100, 120000));
        let mut reader = read_half.lock().await;

        // 使用 8KB 缓冲区读取
        let mut buf = vec![0u8; 8192];

        let read_result = timeout(receive_timeout, reader.read(&mut buf)).await;

        match read_result {
            Ok(Ok(0)) => {
                // 连接已关闭（EOF）
                log::info!("[TCP] Connection closed (EOF): uuid={}", uuid);
                self.update_connection_error(
                    uuid,
                    "Connection closed (peer closed the connection)",
                );
                Ok(None)
            }
            Ok(Ok(n)) => {
                // Success读取 n 字节
                buf.truncate(n);
                // 记录接收数据（仅在录制状态下）
                if let Ok(mut rec) = self.recording.try_lock() {
                    if rec.is_recording() {
                        rec.record_receive(uuid, &buf);
                    }
                }
                log::debug!("[TCP] Receive successful: uuid={}, bytes={}", uuid, n);
                Ok(Some(buf))
            }
            Ok(Err(e)) => {
                let error_msg = format!("TCP receive failed ({}): {}", uuid, e);
                self.update_connection_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                // 超时，返回 None（表示在超时时间内没有数据可读）
                log::debug!(
                    "[TCP] Receive timeout ({}ms): uuid={}",
                    receive_timeout_ms,
                    uuid
                );
                Ok(None)
            }
        }
    }

    /// 关闭连接
    ///
    /// 从连接池中移除连接，TcpStream 在 drop 时自动关闭。
    async fn close(&self, uuid: &str, _close_timeout_ms: u64) -> Result<(), String> {
        // 先从连接池中移除（释放锁后再进行异步关闭操作）
        let conn = {
            let mut conns = self
                .connections
                .lock()
                .map_err(|e| format!("Failed to acquire connection lock: {}", e))?;
            conns
                .remove(uuid)
                .ok_or_else(|| format!("TCP connection '{}' not found", uuid))?
        }; // 锁在此释放

        // 尝试优雅关闭写入端
        if let Some(write_half) = conn.get_write_half() {
            let mut writer = write_half.lock().await;
            // 尝试 shutdown，忽略Error（连接可能已断开）
            let _ = writer.shutdown().await;
        }

        log::info!("[TCP] Connection closed: uuid={}", uuid);
        Ok(())
    }

    /// 获取连接状态
    ///
    /// 返回连接状态的字符串表示，如果连接不存在则返回 None。
    fn get_status(&self, uuid: &str) -> Option<String> {
        let conns = self.connections.lock().ok()?;
        conns.get(uuid).map(|c| c.status.to_string())
    }

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<TcpConnectionInfo> {
        match self.connections.lock() {
            Ok(conns) => conns.values().map(|c| c.to_connection_info()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// 测试连接管理器创建
    #[test]
    fn test_connection_manager_new() {
        let manager = TcpConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
        assert!(manager.list_connections().is_empty());
    }

    /// 测试连接管理器 Default trait
    #[test]
    fn test_connection_manager_default() {
        let manager = TcpConnectionManager::default();
        assert!(manager.get_status("non-existent").is_none());
    }

    /// 测试连接管理器 Clone
    #[test]
    fn test_connection_manager_clone() {
        let manager = TcpConnectionManager::new();
        let cloned = manager.clone();
        assert!(cloned.get_status("non-existent").is_none());
        assert!(cloned.list_connections().is_empty());
    }

    /// 测试 `ConnectionStatus` 转换
    #[test]
    fn test_connection_status_as_str() {
        assert_eq!(ConnectionStatus::Connecting.as_str(), "Connecting");
        assert_eq!(ConnectionStatus::Connected.as_str(), "Connected");
        assert_eq!(ConnectionStatus::Closed.as_str(), "Closed");
        assert_eq!(ConnectionStatus::Error.as_str(), "Error");
    }

    /// 测试 `ConnectionStatus` Display
    #[test]
    fn test_connection_status_display() {
        assert_eq!(format!("{}", ConnectionStatus::Connecting), "Connecting");
        assert_eq!(format!("{}", ConnectionStatus::Connected), "Connected");
    }

    /// 测试 `ActiveTcpConnection` 创建
    #[test]
    fn test_active_connection_new() {
        let conn = ActiveTcpConnection::new(
            "uuid-1".to_string(),
            "测试连接".to_string(),
            "127.0.0.1".to_string(),
            8080,
            false,
            FramingConfig::default(),
        );

        assert_eq!(conn.uuid, "uuid-1");
        assert_eq!(conn.name, "测试连接");
        assert_eq!(conn.host, "127.0.0.1");
        assert_eq!(conn.port, 8080);
        assert!(!conn.use_tls);
        assert_eq!(conn.status, ConnectionStatus::Connecting);
        assert!(conn.last_error.is_none());
        assert!(!conn.is_connected());
        assert!(conn.created_at > 0);
    }

    /// 测试 `ActiveTcpConnection` Error状态
    #[test]
    fn test_active_connection_error() {
        let mut conn = ActiveTcpConnection::new(
            "uuid-2".to_string(),
            "Error连接".to_string(),
            "192.168.1.1".to_string(),
            9090,
            false,
            FramingConfig::default(),
        );

        conn.set_error("Connection refused".to_string());
        assert_eq!(conn.status, ConnectionStatus::Error);
        assert_eq!(conn.last_error, Some("Connection refused".to_string()));
        assert!(!conn.is_connected());
    }

    /// 测试 `ActiveTcpConnection` `to_connection_info`
    #[test]
    fn test_active_connection_to_info() {
        let conn = ActiveTcpConnection::new(
            "uuid-3".to_string(),
            "信息连接".to_string(),
            "10.0.0.1".to_string(),
            3000,
            true,
            FramingConfig::default(),
        );

        let info = conn.to_connection_info();
        assert_eq!(info.uuid, "uuid-3");
        assert_eq!(info.name, "信息连接");
        assert_eq!(info.host, "10.0.0.1");
        assert_eq!(info.port, 3000);
        assert_eq!(info.status, "Connecting");
        assert!(info.use_tls);
        assert!(info.last_error.is_none());
    }

    /// 测试 `ActiveTcpConnection` Debug 输出
    #[test]
    fn test_active_connection_debug() {
        let conn = ActiveTcpConnection::new(
            "uuid-debug".to_string(),
            "调试连接".to_string(),
            "localhost".to_string(),
            1234,
            false,
            FramingConfig::default(),
        );

        let debug_str = format!("{:?}", conn);
        assert!(debug_str.contains("uuid-debug"));
        assert!(debug_str.contains("调试连接"));
        assert!(debug_str.contains("localhost"));
        assert!(debug_str.contains("1234"));
    }

    /// 测试 TLS 握手Failed（连接到不说话 TLS 的服务器）
    #[tokio::test]
    async fn test_tcp_connect_tls_handshake_failure() {
        // 创建一个明文 TCP 服务器（不说话 TLS）
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let server_addr = listener.local_addr().expect("获取地址应Success");
        let server_port = server_addr.port();

        // 接受连接但不进行 TLS 握手
        tokio::spawn(async move {
            if let Ok((_stream, _)) = listener.accept().await {
                // 保持连接但不发送 TLS 响应，导致握手超时或协议Error
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let manager = TcpConnectionManager::new();
        let result = manager
            .connect(
                "uuid-tls-fail".to_string(),
                "TLS handshake failure test".to_string(),
                "127.0.0.1".to_string(),
                server_port,
                true,  // use_tls
                false, // tls_skip_verify
                5000,
                FramingConfig::default(),
            )
            .await;

        assert!(result.is_err(), "TLS 握手应Failed");
        let err = result.expect_err("应返回 TLS 握手FailedError");
        assert!(err.contains("TLS"), "Error应包含 TLS 相关信息: {}", err);

        // 验证连接池中有Error状态的连接
        let status = manager.get_status("uuid-tls-fail");
        assert_eq!(status, Some("Error".to_string()));
    }

    /// 测试 TLS `握手Failed（skip_verify` 模式同样会Failed）
    #[tokio::test]
    async fn test_tcp_connect_tls_skip_verify_handshake_failure() {
        // 创建一个明文 TCP 服务器（不说话 TLS）
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let server_addr = listener.local_addr().expect("获取地址应Success");
        let server_port = server_addr.port();

        tokio::spawn(async move {
            if let Ok((_stream, _)) = listener.accept().await {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let manager = TcpConnectionManager::new();
        let result = manager
            .connect(
                "uuid-tls-skip".to_string(),
                "TLS skip verify handshake failure".to_string(),
                "127.0.0.1".to_string(),
                server_port,
                true, // use_tls
                true, // tls_skip_verify
                5000,
                FramingConfig::default(),
            )
            .await;

        assert!(result.is_err(), "TLS 握手应Failed");
        let err = result.expect_err("应返回 TLS 握手FailedError");
        assert!(err.contains("TLS"), "Error应包含 TLS 相关信息: {}", err);
    }

    /// 测试对不存在连接发送数据
    #[tokio::test]
    async fn test_tcp_send_connection_not_found() {
        let manager = TcpConnectionManager::new();
        let result = manager.send("non-existent", vec![1, 2, 3], 5000).await;

        assert!(result.is_err());
        let err = result.expect_err("应返回连接不存在Error");
        assert!(err.contains("not found"));
    }

    /// 测试对不存在连接接收数据
    #[tokio::test]
    async fn test_tcp_receive_connection_not_found() {
        let manager = TcpConnectionManager::new();
        let result = manager.receive("non-existent", 5000).await;

        assert!(result.is_err());
        let err = result.expect_err("应返回连接不存在Error");
        assert!(err.contains("not found"));
    }

    /// 测试关闭不存在的连接
    #[tokio::test]
    async fn test_tcp_close_connection_not_found() {
        let manager = TcpConnectionManager::new();
        let result = manager.close("non-existent", 5000).await;

        assert!(result.is_err());
        let err = result.expect_err("应返回连接不存在Error");
        assert!(err.contains("not found"));
    }

    /// 测试 TCP 连接池生命周期：连接 → 发送 → 接收 → 关闭
    ///
    /// 使用本地 TCP echo 服务器进行测试。
    #[tokio::test]
    async fn test_tcp_pool_lifecycle() {
        // 1. 创建本地 echo 服务器
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let server_addr = listener.local_addr().expect("获取地址应Success");
        let server_port = server_addr.port();

        // 启动 echo 服务器任务
        let echo_handle = tokio::spawn(async move {
            // 只处理一个连接
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            // Echo 回去
                            if stream.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        });

        // 2. 创建管理器并连接
        let manager = TcpConnectionManager::new();
        let connect_result = manager
            .connect(
                "lifecycle-1".to_string(),
                "生命周期测试".to_string(),
                "127.0.0.1".to_string(),
                server_port,
                false,
                false,
                5000,
                FramingConfig::default(),
            )
            .await;

        assert!(connect_result.is_ok(), "连接应Success");
        assert_eq!(
            manager.get_status("lifecycle-1"),
            Some("Connected".to_string())
        );

        // 3. 验证连接列表
        let connections = manager.list_connections();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].uuid, "lifecycle-1");
        assert_eq!(connections[0].port, server_port);
        assert_eq!(connections[0].status, "Connected");

        // 4. 发送数据
        let test_data = b"Hello, TCP!".to_vec();
        let send_result = manager.send("lifecycle-1", test_data.clone(), 5000).await;
        assert!(send_result.is_ok(), "发送应Success");

        // 5. 接收 echo 回来的数据
        let recv_result = manager.receive("lifecycle-1", 5000).await;
        assert!(recv_result.is_ok(), "接收应Success");
        let received = recv_result.expect("接收应Success");
        assert!(received.is_some(), "应接收到数据");
        let received_data = received.expect("应接收到数据");
        assert_eq!(received_data, test_data);

        // 6. 关闭连接
        let close_result = manager.close("lifecycle-1", 5000).await;
        assert!(close_result.is_ok(), "关闭应Success");
        assert!(manager.get_status("lifecycle-1").is_none());

        // 清理 echo 服务器
        echo_handle.abort();
    }

    /// 测试connection failed（连接拒绝）
    #[tokio::test]
    async fn test_tcp_connect_refused() {
        let manager = TcpConnectionManager::new();

        // 使用一个不太可能正在监听的端口
        let result = manager
            .connect(
                "refused-1".to_string(),
                "拒绝连接".to_string(),
                "127.0.0.1".to_string(),
                19999,
                false,
                false,
                3000,
                FramingConfig::default(),
            )
            .await;

        assert!(result.is_err(), "连接应Failed");
        let err = result.expect_err("连接应Failed");
        assert!(err.contains("connection failed"));

        // 验证连接池中有Error状态的连接
        let status = manager.get_status("refused-1");
        assert_eq!(status, Some("Error".to_string()));
    }

    /// 测试多连接并发访问
    #[tokio::test]
    async fn test_tcp_pool_concurrent_access() {
        // 创建多个本地 echo 服务器
        let mut listeners = Vec::new();
        let mut ports = Vec::new();

        for _ in 0..3 {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("绑定端口应Success");
            let port = listener.local_addr().expect("获取地址应Success").port();
            ports.push(port);
            listeners.push(listener);
        }

        // 启动 echo 服务器
        for listener in listeners {
            tokio::spawn(async move {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 4096];
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                if stream.write_all(&buf[..n]).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            });
        }

        let manager = TcpConnectionManager::new();

        // 并发连接 3 个
        let mut handles = Vec::new();
        for (i, port) in ports.iter().enumerate() {
            let mgr = manager.clone();
            let port = *port;
            handles.push(tokio::spawn(async move {
                let uuid = format!("concurrent-{}", i);
                mgr.connect(
                    uuid.clone(),
                    format!("并发连接 {}", i),
                    "127.0.0.1".to_string(),
                    port,
                    false,
                    false,
                    5000,
                    FramingConfig::default(),
                )
                .await
                .map(|_| uuid)
            }));
        }

        // 等待所有连接完成
        let mut connected_uuids = Vec::new();
        for handle in handles {
            let result = handle.await.expect("任务应完成");
            if let Ok(uuid) = result {
                connected_uuids.push(uuid);
            }
        }

        assert_eq!(connected_uuids.len(), 3, "应有 3 个连接Success");

        // 验证所有连接都存在
        for uuid in &connected_uuids {
            assert_eq!(manager.get_status(uuid), Some("Connected".to_string()));
        }

        let connections = manager.list_connections();
        assert_eq!(connections.len(), 3);

        // 并发发送数据
        let mut send_handles = Vec::new();
        for uuid in &connected_uuids {
            let mgr = manager.clone();
            let uuid = uuid.clone();
            send_handles.push(tokio::spawn(async move {
                let data = format!("data-from-{}", uuid).into_bytes();
                mgr.send(&uuid, data, 5000).await
            }));
        }

        for handle in send_handles {
            let result = handle.await.expect("任务应完成");
            assert!(result.is_ok(), "发送应Success");
        }

        // 并发接收数据
        let mut recv_handles = Vec::new();
        for uuid in &connected_uuids {
            let mgr = manager.clone();
            let uuid = uuid.clone();
            recv_handles.push(tokio::spawn(async move { mgr.receive(&uuid, 5000).await }));
        }

        for handle in recv_handles {
            let result = handle.await.expect("任务应完成");
            assert!(result.is_ok(), "接收应Success");
        }

        // 并发关闭
        let mut close_handles = Vec::new();
        for uuid in connected_uuids {
            let mgr = manager.clone();
            close_handles.push(tokio::spawn(async move { mgr.close(&uuid, 5000).await }));
        }

        for handle in close_handles {
            let result = handle.await.expect("任务应完成");
            assert!(result.is_ok(), "关闭应Success");
        }

        assert!(manager.list_connections().is_empty());
    }

    /// 测试重复连接同一 UUID 会覆盖旧连接
    #[tokio::test]
    async fn test_tcp_connect_duplicate_uuid() {
        // 创建两个本地服务器
        let listener1 = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let port1 = listener1.local_addr().expect("获取地址应Success").port();

        let listener2 = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let port2 = listener2.local_addr().expect("获取地址应Success").port();

        // 启动两个 echo 服务器
        for listener in [listener1, listener2] {
            tokio::spawn(async move {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 4096];
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                if stream.write_all(&buf[..n]).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            });
        }

        let manager = TcpConnectionManager::new();

        // 第一次连接
        let result1 = manager
            .connect(
                "dup-uuid".to_string(),
                "第一次连接".to_string(),
                "127.0.0.1".to_string(),
                port1,
                false,
                false,
                5000,
                FramingConfig::default(),
            )
            .await;
        assert!(result1.is_ok());

        // 第二次连接（同一 UUID，不同端口）
        let result2 = manager
            .connect(
                "dup-uuid".to_string(),
                "第二次连接".to_string(),
                "127.0.0.1".to_string(),
                port2,
                false,
                false,
                5000,
                FramingConfig::default(),
            )
            .await;
        assert!(result2.is_ok());

        // 验证连接池只有 1 个连接，且是第二次连接的端口
        let connections = manager.list_connections();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].uuid, "dup-uuid");
        assert_eq!(connections[0].port, port2);
        assert_eq!(connections[0].name, "第二次连接");
    }

    /// 测试接收超时返回 None
    #[tokio::test]
    async fn test_tcp_receive_timeout_returns_none() {
        // 创建一个不发送数据的服务器
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口应Success");
        let server_addr = listener.local_addr().expect("获取地址应Success");
        let server_port = server_addr.port();

        // 启动一个接受连接但什么都不发的服务器
        tokio::spawn(async move {
            if let Ok((_stream, _)) = listener.accept().await {
                // 保持连接但不发送数据
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let manager = TcpConnectionManager::new();

        // 连接
        let connect_result = manager
            .connect(
                "timeout-1".to_string(),
                "超时测试".to_string(),
                "127.0.0.1".to_string(),
                server_port,
                false,
                false,
                5000,
                FramingConfig::default(),
            )
            .await;
        assert!(connect_result.is_ok());

        // 使用短超时接收，应返回 None
        let recv_result = manager.receive("timeout-1", 500).await;
        assert!(recv_result.is_ok(), "接收应Success");
        assert!(
            recv_result.expect("接收应Success").is_none(),
            "超时应返回 None"
        );
    }
}
