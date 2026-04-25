//! TCP/UDP 协议测试服务器
//!
//! 为 flow-engine 的 TCP/UDP 节点提供集成测试目标服务。
//! 包含 4 个 TCP 服务和 2 个 UDP 服务，全部并发运行。

use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::signal;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

// ============================================================================
// 模块: TCP Echo Server (port 9001)
// ============================================================================

mod tcp_echo {
    use super::*;

    /// TCP Echo 服务：原样回显所有收到的数据
    ///
    /// 测试目标: tcp_connect (Manual/Timeout framing) + tcp_send + tcp_close
    pub async fn run(port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("TCP Echo: 绑定端口 {} 失败", port))?;
        info!("TCP Echo 服务启动 → 0.0.0.0:{}", port);

        let conn_counter = Arc::new(AtomicU64::new(0));

        loop {
            let (stream, addr) = listener.accept().await?;
            let conn_id = conn_counter.fetch_add(1, Ordering::Relaxed);
            info!(conn_id, remote = %addr, "TCP Echo: 新连接");

            tokio::spawn(async move {
                if let Err(e) = handle_echo_connection(stream, addr, conn_id).await {
                    error!(conn_id, remote = %addr, "TCP Echo: 连接错误: {}", e);
                }
            });
        }
    }

    async fn handle_echo_connection(
        mut stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        conn_id: u64,
    ) -> Result<()> {
        let read_timeout = env_read_timeout();
        let mut buf = vec![0u8; 8192];

        loop {
            let read_result = if read_timeout > Duration::ZERO {
                timeout(read_timeout, stream.read(&mut buf)).await
            } else {
                Ok(stream.read(&mut buf).await)
            };

            match read_result {
                Ok(Ok(0)) => {
                    info!(conn_id, remote = %addr, "TCP Echo: 连接关闭 (EOF)");
                    break;
                }
                Ok(Ok(n)) => {
                    info!(conn_id, remote = %addr, bytes = n, "TCP Echo: 回显数据");
                    stream.write_all(&buf[..n]).await?;
                    stream.flush().await?;
                }
                Ok(Err(e)) => {
                    error!(conn_id, remote = %addr, "TCP Echo: 读取错误: {}", e);
                    break;
                }
                Err(_) => {
                    info!(conn_id, remote = %addr, "TCP Echo: 读取超时，关闭连接");
                    break;
                }
            }
        }

        let _ = stream.shutdown().await;
        Ok(())
    }
}

// ============================================================================
// 模块: TCP Line Protocol Server (port 9002)
// ============================================================================

mod tcp_line {
    use super::*;

    /// TCP Line Protocol 服务：基于换行符的文本命令协议
    ///
    /// 命令: ECHO <text>, PING, UPPER <text>, LOWER <text>, LEN <text>, QUIT
    /// 测试目标: tcp_connect (Delimiter framing, delimiter="\\n")
    pub async fn run(port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("TCP Line: 绑定端口 {} 失败", port))?;
        info!("TCP Line Protocol 服务启动 → 0.0.0.0:{}", port);

        let conn_counter = Arc::new(AtomicU64::new(0));

        loop {
            let (stream, addr) = listener.accept().await?;
            let conn_id = conn_counter.fetch_add(1, Ordering::Relaxed);
            info!(conn_id, remote = %addr, "TCP Line: 新连接");

            tokio::spawn(async move {
                if let Err(e) = handle_line_connection(stream, addr, conn_id).await {
                    error!(conn_id, remote = %addr, "TCP Line: 连接错误: {}", e);
                }
            });
        }
    }

    async fn handle_line_connection(
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        conn_id: u64,
    ) -> Result<()> {
        let read_timeout = env_read_timeout();
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = tokio::io::BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();

            let read_result = if read_timeout > Duration::ZERO {
                timeout(read_timeout, buf_reader.read_line(&mut line)).await
            } else {
                Ok(buf_reader.read_line(&mut line).await)
            };

            match read_result {
                Ok(Ok(0)) => {
                    info!(conn_id, remote = %addr, "TCP Line: 连接关闭 (EOF)");
                    break;
                }
                Ok(Ok(_)) => {
                    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
                    let response = process_command(trimmed);
                    info!(
                        conn_id,
                        remote = %addr,
                        cmd = trimmed,
                        resp = %response.trim_end_matches('\n'),
                        "TCP Line: 命令处理"
                    );

                    if response == "QUIT\n" {
                        let _ = writer.write_all(response.as_bytes()).await;
                        let _ = writer.flush().await;
                        info!(conn_id, remote = %addr, "TCP Line: 客户端请求关闭");
                        break;
                    }

                    writer.write_all(response.as_bytes()).await?;
                    writer.flush().await?;
                }
                Ok(Err(e)) => {
                    error!(conn_id, remote = %addr, "TCP Line: 读取错误: {}", e);
                    break;
                }
                Err(_) => {
                    info!(conn_id, remote = %addr, "TCP Line: 读取超时，关闭连接");
                    break;
                }
            }
        }

        Ok(())
    }

    fn process_command(input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return "ERR empty command\n".to_string();
        }

        // 分离命令和参数
        let (cmd, arg) = if let Some(space_pos) = input.find(' ') {
            (&input[..space_pos], &input[space_pos + 1..])
        } else {
            (input, "")
        };

        match cmd.to_lowercase().as_str() {
            "echo" => {
                if arg.is_empty() {
                    "ERR ECHO requires an argument\n".to_string()
                } else {
                    format!("{}\n", arg)
                }
            }
            "ping" => "PONG\n".to_string(),
            "upper" => {
                if arg.is_empty() {
                    "ERR UPPER requires an argument\n".to_string()
                } else {
                    format!("{}\n", arg.to_uppercase())
                }
            }
            "lower" => {
                if arg.is_empty() {
                    "ERR LOWER requires an argument\n".to_string()
                } else {
                    format!("{}\n", arg.to_lowercase())
                }
            }
            "len" => {
                if arg.is_empty() {
                    "ERR LEN requires an argument\n".to_string()
                } else {
                    format!("{}\n", arg.len())
                }
            }
            "quit" => "QUIT\n".to_string(),
            _ => "ERR unknown command\n".to_string(),
        }
    }
}

// ============================================================================
// 通用读取辅助
// ============================================================================

/// 读取结果枚举
enum ReadResult {
    Data(Vec<u8>),
    Eof,
    Timeout,
    Error(std::io::Error),
}

/// 带超时的精确读取，兼容所有 AsyncReadExt 实现
async fn read_with_timeout<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    n: usize,
    read_timeout: Duration,
    conn_id: u64,
    addr: std::net::SocketAddr,
    label: &str,
) -> ReadResult {
    let read_future = async {
        let mut buf = vec![0u8; n];
        reader.read_exact(&mut buf).await.map(|_| buf)
    };

    if read_timeout > Duration::ZERO {
        match timeout(read_timeout, read_future).await {
            Ok(Ok(buf)) => ReadResult::Data(buf),
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => ReadResult::Eof,
            Ok(Err(e)) => ReadResult::Error(e),
            Err(_) => {
                info!(conn_id, remote = %addr, "{}: 读取超时", label);
                ReadResult::Timeout
            }
        }
    } else {
        match read_future.await {
            Ok(buf) => ReadResult::Data(buf),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => ReadResult::Eof,
            Err(e) => ReadResult::Error(e),
        }
    }
}

// ============================================================================
// 模块: TCP LengthPrefix Server (port 9003)
// ============================================================================

mod tcp_prefix {
    use super::*;

    const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

    /// TCP LengthPrefix 服务：4 字节大端长度前缀协议
    ///
    /// 收到消息后原样回显（带长度前缀）。超过 1MB 的消息会关闭连接。
    /// 测试目标: tcp_connect (LengthPrefix framing, prefix_bytes=4)
    pub async fn run(port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("TCP Prefix: 绑定端口 {} 失败", port))?;
        info!("TCP LengthPrefix 服务启动 → 0.0.0.0:{}", port);

        let conn_counter = Arc::new(AtomicU64::new(0));

        loop {
            let (stream, addr) = listener.accept().await?;
            let conn_id = conn_counter.fetch_add(1, Ordering::Relaxed);
            info!(conn_id, remote = %addr, "TCP Prefix: 新连接");

            tokio::spawn(async move {
                if let Err(e) = handle_prefix_connection(stream, addr, conn_id).await {
                    error!(conn_id, remote = %addr, "TCP Prefix: 连接错误: {}", e);
                }
            });
        }
    }

    async fn handle_prefix_connection(
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        conn_id: u64,
    ) -> Result<()> {
        let read_timeout = env_read_timeout();
        let (mut reader, mut writer) = stream.into_split();

        loop {
            // 读取 4 字节长度前缀
            let len_buf_result = read_with_timeout(
                &mut reader,
                4,
                read_timeout,
                conn_id,
                addr,
                "TCP Prefix",
            ).await;

            let len_bytes = match len_buf_result {
                ReadResult::Data(buf) => buf,
                ReadResult::Eof => {
                    info!(conn_id, remote = %addr, "TCP Prefix: 连接关闭 (EOF)");
                    break;
                }
                ReadResult::Timeout => {
                    info!(conn_id, remote = %addr, "TCP Prefix: 读取超时，关闭连接");
                    break;
                }
                ReadResult::Error(e) => {
                    error!(conn_id, remote = %addr, "TCP Prefix: 读取长度前缀错误: {}", e);
                    break;
                }
            };

            let msg_len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                as usize;

            if msg_len > MAX_MESSAGE_SIZE {
                error!(
                    conn_id,
                    remote = %addr,
                    msg_len,
                    max = MAX_MESSAGE_SIZE,
                    "TCP Prefix: 消息过大，关闭连接"
                );
                let err_msg = b"ERROR: message too large\n";
                let err_len = (err_msg.len() as u32).to_be_bytes();
                let _ = writer.write_all(&err_len).await;
                let _ = writer.write_all(err_msg).await;
                let _ = writer.flush().await;
                break;
            }

            // 读取消息体
            let body_result = read_with_timeout(
                &mut reader,
                msg_len,
                read_timeout,
                conn_id,
                addr,
                "TCP Prefix",
            ).await;

            let body = match body_result {
                ReadResult::Data(buf) => buf,
                ReadResult::Eof => {
                    info!(conn_id, remote = %addr, "TCP Prefix: 消息体 EOF");
                    break;
                }
                ReadResult::Timeout => {
                    info!(conn_id, remote = %addr, "TCP Prefix: 消息体读取超时");
                    break;
                }
                ReadResult::Error(e) => {
                    error!(conn_id, remote = %addr, "TCP Prefix: 读取消息体错误: {}", e);
                    break;
                }
            };

            info!(
                conn_id,
                remote = %addr,
                msg_len,
                "TCP Prefix: 回显消息"
            );

            // 回显：写入长度前缀 + 消息体
            writer.write_all(&len_bytes).await?;
            writer.write_all(&body).await?;
            writer.flush().await?;
        }

        Ok(())
    }
}

// ============================================================================
// 模块: TCP TLS Echo Server (port 9004)
// ============================================================================

mod tcp_tls {
    use super::*;
    use std::fs::File;
    use std::io::BufReader as StdBufReader;
    use tokio_rustls::TlsAcceptor;

    /// TCP TLS Echo 服务：与 TCP Echo 相同，但通过 TLS 加密
    ///
    /// 证书路径: /app/certs/server.crt + server.key (Docker)
    /// 备用路径: ./certs/server.crt + server.key (本地开发)
    /// 测试目标: tcp_connect (use_tls=true)
    pub async fn run(port: u16) -> Result<()> {
        let cert_path = env::var("TLS_CERT_PATH").unwrap_or_else(|_| "/app/certs/server.crt".to_string());
        let key_path = env::var("TLS_KEY_PATH").unwrap_or_else(|_| "/app/certs/server.key".to_string());

        let config = match load_tls_config(&cert_path, &key_path) {
            Ok(config) => config,
            Err(e) => {
                warn!(
                    "TCP TLS Echo: 加载证书失败 (cert={}, key={}): {}. 跳过 TLS 服务。",
                    cert_path, key_path, e
                );
                return Ok(());
            }
        };

        let acceptor = TlsAcceptor::from(config);
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("TCP TLS Echo: 绑定端口 {} 失败", port))?;
        info!("TCP TLS Echo 服务启动 → 0.0.0.0:{}", port);

        let conn_counter = Arc::new(AtomicU64::new(0));

        loop {
            let (stream, addr) = listener.accept().await?;
            let conn_id = conn_counter.fetch_add(1, Ordering::Relaxed);
            info!(conn_id, remote = %addr, "TCP TLS Echo: 新连接 (TLS 握手中)");

            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!(conn_id, remote = %addr, "TCP TLS Echo: TLS 握手失败: {}", e);
                        return;
                    }
                };
                info!(conn_id, remote = %addr, "TCP TLS Echo: TLS 握手完成");

                if let Err(e) = handle_tls_echo(tls_stream, addr, conn_id).await {
                    error!(conn_id, remote = %addr, "TCP TLS Echo: 连接错误: {}", e);
                }
            });
        }
    }

    fn load_tls_config(
        cert_path: &str,
        key_path: &str,
    ) -> Result<Arc<tokio_rustls::rustls::ServerConfig>> {
        // 尝试加载证书，如果主路径失败则尝试备用路径
        let (certs, key) = load_certs_and_key(cert_path, key_path)
            .or_else(|_| {
                let fallback_cert = "./certs/server.crt";
                let fallback_key = "./certs/server.key";
                warn!(
                    "主证书路径加载失败，尝试备用路径: cert={}, key={}",
                    fallback_cert, fallback_key
                );
                load_certs_and_key(fallback_cert, fallback_key)
            })
            .context("无法加载 TLS 证书和密钥")?;

        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("TLS 配置无效")?;

        Ok(Arc::new(config))
    }

    fn load_certs_and_key(
        cert_path: &str,
        key_path: &str,
    ) -> Result<(
        Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>>,
        tokio_rustls::rustls::pki_types::PrivateKeyDer<'static>,
    )> {
        let cert_file = File::open(cert_path)
            .with_context(|| format!("打开证书文件失败: {}", cert_path))?;
        let mut cert_reader = StdBufReader::new(cert_file);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| "解析证书 PEM 失败")?;

        if certs.is_empty() {
            anyhow::bail!("证书文件中没有找到有效的证书: {}", cert_path);
        }

        let key_file = File::open(key_path)
            .with_context(|| format!("打开密钥文件失败: {}", key_path))?;
        let mut key_reader = StdBufReader::new(key_file);

        // 自动检测密钥格式 (PKCS8/RSA/EC)
        let key = rustls_pemfile::private_key(&mut key_reader)
            .with_context(|| "解析私钥 PEM 失败")?
            .with_context(|| "密钥文件中没有找到有效的私钥")?;

        Ok((certs, key))
    }

    async fn handle_tls_echo(
        mut stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
        addr: std::net::SocketAddr,
        conn_id: u64,
    ) -> Result<()> {
        let read_timeout = env_read_timeout();
        let mut buf = vec![0u8; 8192];

        loop {
            let read_result = if read_timeout > Duration::ZERO {
                timeout(read_timeout, stream.read(&mut buf)).await
            } else {
                Ok(stream.read(&mut buf).await)
            };

            match read_result {
                Ok(Ok(0)) => {
                    info!(conn_id, remote = %addr, "TCP TLS Echo: 连接关闭 (EOF)");
                    break;
                }
                Ok(Ok(n)) => {
                    info!(conn_id, remote = %addr, bytes = n, "TCP TLS Echo: 回显数据");
                    stream.write_all(&buf[..n]).await?;
                    stream.flush().await?;
                }
                Ok(Err(e)) => {
                    error!(conn_id, remote = %addr, "TCP TLS Echo: 读取错误: {}", e);
                    break;
                }
                Err(_) => {
                    info!(conn_id, remote = %addr, "TCP TLS Echo: 读取超时，关闭连接");
                    break;
                }
            }
        }

        let _ = stream.shutdown().await;
        Ok(())
    }
}

// ============================================================================
// 模块: UDP Echo (port 9101)
// ============================================================================

mod udp_echo {
    use super::*;

    /// UDP Echo 服务：将收到的数据报原样发回发送者
    ///
    /// 测试目标: udp_bind + udp_send_to + udp_recv_from
    pub async fn run(port: u16) -> Result<()> {
        let socket = UdpSocket::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("UDP Echo: 绑定端口 {} 失败", port))?;
        info!("UDP Echo 服务启动 → 0.0.0.0:{}", port);

        let mut buf = vec![0u8; 65535];

        loop {
            let (n, addr) = socket.recv_from(&mut buf).await?;
            info!(remote = %addr, bytes = n, "UDP Echo: 收到数据报");

            socket.send_to(&buf[..n], addr).await?;
            info!(remote = %addr, bytes = n, "UDP Echo: 回显数据报");
        }
    }
}

// ============================================================================
// 模块: UDP Responder (port 9102)
// ============================================================================

mod udp_responder {
    use super::*;

    /// UDP Responder 服务：根据收到的内容返回结构化响应
    ///
    /// 测试目标: 结构化 UDP 请求/响应模式
    ///
    /// 响应规则:
    /// - "ping" → "pong"
    /// - "hello" → "world"
    /// - 其他文本 → "ACK:" + 原文
    /// - 二进制数据 → "ACK:" + hex 编码
    pub async fn run(port: u16) -> Result<()> {
        let socket = UdpSocket::bind(("0.0.0.0", port))
            .await
            .with_context(|| format!("UDP Responder: 绑定端口 {} 失败", port))?;
        info!("UDP Responder 服务启动 → 0.0.0.0:{}", port);

        let mut buf = vec![0u8; 65535];

        loop {
            let (n, addr) = socket.recv_from(&mut buf).await?;
            let data = &buf[..n];

            let response: Vec<u8> = match std::str::from_utf8(data) {
                Ok(text) => {
                    let trimmed = text.trim();
                    match trimmed.to_lowercase().as_str() {
                        "ping" => {
                            info!(remote = %addr, req = trimmed, resp = "pong", "UDP Responder");
                            b"pong".to_vec()
                        }
                        "hello" => {
                            info!(remote = %addr, req = trimmed, resp = "world", "UDP Responder");
                            b"world".to_vec()
                        }
                        _ => {
                            let resp = format!("ACK:{}", trimmed);
                            info!(remote = %addr, req = trimmed, resp = %resp, "UDP Responder");
                            resp.into_bytes()
                        }
                    }
                }
                Err(_) => {
                    // 非 UTF-8 二进制数据 → hex 编码
                    let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                    let resp = format!("ACK:{}", hex);
                    info!(
                        remote = %addr,
                        bytes = n,
                        resp = %resp,
                        "UDP Responder (二进制数据)"
                    );
                    resp.into_bytes()
                }
            };

            socket.send_to(&response, addr).await?;
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 从环境变量读取连接超时时间，默认 60 秒
fn env_read_timeout() -> Duration {
    let secs = env::var("CONN_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    Duration::from_secs(secs)
}

/// 从环境变量读取端口，失败时使用默认值
fn env_port(name: &str, default: u16) -> u16 {
    env::var(name)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default)
}

/// 打印启动横幅
fn print_banner(
    tcp_echo: u16,
    tcp_line: u16,
    tcp_prefix: u16,
    tcp_tls: u16,
    udp_echo: u16,
    udp_responder: u16,
    tls_available: bool,
) {
    eprintln!();
    eprintln!("╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║          Protocol Test Server - 协议测试服务器              ║");
    eprintln!("╠══════════════════════════════════════════════════════════════╣");
    eprintln!("║  TCP Services:                                              ║");
    eprintln!("║    Echo         → 0.0.0.0:{:<5}  (Manual/Timeout framing) ║", tcp_echo);
    eprintln!("║    Line Proto   → 0.0.0.0:{:<5}  (Delimiter framing)      ║", tcp_line);
    eprintln!("║    LengthPrefix → 0.0.0.0:{:<5}  (LengthPrefix framing)   ║", tcp_prefix);
    if tls_available {
        eprintln!("║    TLS Echo     → 0.0.0.0:{:<5}  (TLS + Manual framing)  ║", tcp_tls);
    } else {
        eprintln!("║    TLS Echo     → DISABLED (证书未找到)                    ║");
    }
    eprintln!("║                                                              ║");
    eprintln!("║  UDP Services:                                              ║");
    eprintln!("║    Echo         → 0.0.0.0:{:<5}  (原样回显)              ║", udp_echo);
    eprintln!("║    Responder    → 0.0.0.0:{:<5}  (结构化响应)            ║", udp_responder);
    eprintln!("╠══════════════════════════════════════════════════════════════╣");
    eprintln!("║  环境变量覆盖:                                              ║");
    eprintln!("║    TCP_ECHO_PORT / TCP_LINE_PORT / TCP_PREFIX_PORT          ║");
    eprintln!("║    TCP_TLS_PORT / UDP_ECHO_PORT / UDP_RESPONDER_PORT        ║");
    eprintln!("║    TLS_CERT_PATH / TLS_KEY_PATH / CONN_TIMEOUT_SECS         ║");
    eprintln!("║    RUST_LOG (默认: info)                                     ║");
    eprintln!("╚══════════════════════════════════════════════════════════════╝");
    eprintln!();
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // 读取端口配置
    let tcp_echo_port = env_port("TCP_ECHO_PORT", 9001);
    let tcp_line_port = env_port("TCP_LINE_PORT", 9002);
    let tcp_prefix_port = env_port("TCP_PREFIX_PORT", 9003);
    let tcp_tls_port = env_port("TCP_TLS_PORT", 9004);
    let udp_echo_port = env_port("UDP_ECHO_PORT", 9101);
    let udp_responder_port = env_port("UDP_RESPONDER_PORT", 9102);

    // 检查 TLS 证书是否可用（用于横幅显示）
    let cert_path = env::var("TLS_CERT_PATH").unwrap_or_else(|_| "/app/certs/server.crt".to_string());
    let key_path = env::var("TLS_KEY_PATH").unwrap_or_else(|_| "/app/certs/server.key".to_string());
    let tls_available = std::path::Path::new(&cert_path).exists() && std::path::Path::new(&key_path).exists()
        || (std::path::Path::new("./certs/server.crt").exists()
            && std::path::Path::new("./certs/server.key").exists());

    // 打印启动横幅
    print_banner(
        tcp_echo_port,
        tcp_line_port,
        tcp_prefix_port,
        tcp_tls_port,
        udp_echo_port,
        udp_responder_port,
        tls_available,
    );

    // 启动所有服务
    let tcp_echo_task = tokio::spawn(async move {
        if let Err(e) = tcp_echo::run(tcp_echo_port).await {
            error!("TCP Echo 服务异常退出: {}", e);
        }
    });

    let tcp_line_task = tokio::spawn(async move {
        if let Err(e) = tcp_line::run(tcp_line_port).await {
            error!("TCP Line 服务异常退出: {}", e);
        }
    });

    let tcp_prefix_task = tokio::spawn(async move {
        if let Err(e) = tcp_prefix::run(tcp_prefix_port).await {
            error!("TCP Prefix 服务异常退出: {}", e);
        }
    });

    let tcp_tls_task = tokio::spawn(async move {
        if let Err(e) = tcp_tls::run(tcp_tls_port).await {
            error!("TCP TLS Echo 服务异常退出: {}", e);
        }
    });

    let udp_echo_task = tokio::spawn(async move {
        if let Err(e) = udp_echo::run(udp_echo_port).await {
            error!("UDP Echo 服务异常退出: {}", e);
        }
    });

    let udp_responder_task = tokio::spawn(async move {
        if let Err(e) = udp_responder::run(udp_responder_port).await {
            error!("UDP Responder 服务异常退出: {}", e);
        }
    });

    info!("所有服务已启动，等待 CTRL+C 信号以关闭...");

    // 等待 CTRL+C 信号
    signal::ctrl_c().await?;
    info!("收到 CTRL+C 信号，正在关闭所有服务...");

    // 取消所有任务
    tcp_echo_task.abort();
    tcp_line_task.abort();
    tcp_prefix_task.abort();
    tcp_tls_task.abort();
    udp_echo_task.abort();
    udp_responder_task.abort();

    info!("所有服务已关闭。");
    Ok(())
}
