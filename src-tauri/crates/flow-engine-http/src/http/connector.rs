//! 自定义连接器 - 支持 DNS + TCP + TLS 计时 + HTTP/2 ALPN 协商
//!
//! 本模块实现 hyper 的自定义连接器，提供：
//! - DNS 解析计时
//! - TCP 连接计时
//! - TLS 握手计时（支持 ALPN HTTP/2 协商）
//! - HTTP/1.1 和 HTTP/2 自动切换

use crate::http::error::HttpError;
use crate::http::timing::TimingContext;
use http::Uri;
use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::client::legacy::connect::{Connected, Connection as HyperConnection};
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tower_service::Service;

// ============================================================================
// 新版 hyper 连接器（支持 HTTP/2）
// ============================================================================

/// 计时流（支持 HTTP 和 HTTPS）
pub enum TimedStream {
    /// HTTP 连接（明文）
    Http(TokioIo<TcpStream>),
    /// HTTPS 连接（TLS 加密），包含是否协商为 HTTP/2 的标志
    Https(
        Box<TokioIo<tokio_rustls::client::TlsStream<TcpStream>>>,
        bool,
    ),
}

impl HyperConnection for TimedStream {
    fn connected(&self) -> Connected {
        match self {
            TimedStream::Http(_) => Connected::new(),
            TimedStream::Https(_, is_h2) => {
                if *is_h2 {
                    Connected::new().negotiated_h2()
                } else {
                    Connected::new()
                }
            }
        }
    }
}

impl Read for TimedStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            TimedStream::Http(stream) => Pin::new(stream).poll_read(cx, buf),
            TimedStream::Https(stream, _) => Pin::new(&mut **stream).poll_read(cx, buf),
        }
    }
}

impl Write for TimedStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            TimedStream::Http(stream) => Pin::new(stream).poll_write(cx, buf),
            TimedStream::Https(stream, _) => Pin::new(&mut **stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            TimedStream::Http(stream) => Pin::new(stream).poll_flush(cx),
            TimedStream::Https(stream, _) => Pin::new(&mut **stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            TimedStream::Http(stream) => Pin::new(stream).poll_shutdown(cx),
            TimedStream::Https(stream, _) => Pin::new(&mut **stream).poll_shutdown(cx),
        }
    }
}

/// TLS 配置选项
pub struct TlsOptions {
    /// 是否跳过证书验证
    pub skip_verify: bool,
    /// 自定义 CA 证书 PEM 文件路径
    pub ca_cert_path: Option<String>,
    /// 客户端证书 PEM 文件路径
    pub client_cert_path: Option<String>,
    /// 客户端私钥 PEM 文件路径
    pub client_key_path: Option<String>,
}

/// 确保 `CryptoProvider` 已安装
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// 证书验证跳过器
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

/// 构建标准 TLS 配置
fn build_standard_tls_config() -> rustls::ClientConfig {
    ensure_crypto_provider();
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

/// 构建自定义 TLS 配置
fn build_custom_tls_config(options: &TlsOptions) -> Result<rustls::ClientConfig, String> {
    ensure_crypto_provider();

    if options.skip_verify {
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        return Ok(config);
    }

    let mut root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    if let Some(ca_path) = &options.ca_cert_path {
        let ca_bytes = std::fs::read(ca_path)
            .map_err(|e| format!("Failed to read CA certificate ({}): {}", ca_path, e))?;
        let mut cursor = std::io::Cursor::new(&ca_bytes[..]);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to parse CA certificate PEM: {}", e))?;
        for cert in certs {
            root_store
                .add(cert)
                .map_err(|e| format!("Failed to add CA certificate: {}", e))?;
        }
    }

    let has_client_cert = options.client_cert_path.is_some() && options.client_key_path.is_some();
    if has_client_cert {
        let cert_path = options
            .client_cert_path
            .as_ref()
            .ok_or("Client certificate path not set")?;
        let key_path = options
            .client_key_path
            .as_ref()
            .ok_or("Client private key path not set")?;
        let cert_bytes = std::fs::read(cert_path)
            .map_err(|e| format!("Failed to read client certificate ({}): {}", cert_path, e))?;
        let mut cert_cursor = std::io::Cursor::new(&cert_bytes[..]);
        let client_certs: Vec<_> = rustls_pemfile::certs(&mut cert_cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to parse client certificate PEM: {}", e))?;
        let key_bytes = std::fs::read(key_path)
            .map_err(|e| format!("Failed to read client private key ({}): {}", key_path, e))?;
        let mut key_cursor = std::io::Cursor::new(&key_bytes[..]);
        let client_key = rustls_pemfile::private_key(&mut key_cursor)
            .map_err(|e| format!("Failed to parse client private key PEM: {}", e))?
            .ok_or_else(|| "Client private key file is empty".to_string())?;
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| format!("Failed to configure client certificate: {}", e))?;
        Ok(config)
    } else {
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        Ok(config)
    }
}

/// 自定义连接器，支持计时、HTTP/2 ALPN 协商和代理
#[derive(Clone)]
pub struct TimingConnector {
    tls_config: Arc<rustls::ClientConfig>,
    timeout_ms: u64,
    timing: Arc<std::sync::Mutex<TimingContext>>,
    proxy_state: Option<Arc<flow_engine_core::context::ProxyState>>,
}

impl TimingConnector {
    /// 创建新的计时连接器
    ///
    /// # 参数
    /// - `timeout_ms`: 各阶段超时时间（毫秒）
    /// - `timing`: 计时上下文（Arc<Mutex> 包装以便在 Service 中使用）
    pub fn new(timeout_ms: u64, timing: Arc<std::sync::Mutex<TimingContext>>) -> Self {
        let config = build_standard_tls_config();
        Self {
            tls_config: Arc::new(config),
            timeout_ms,
            timing,
            proxy_state: None,
        }
    }

    /// 创建带自定义 TLS 配置的计时连接器
    pub fn new_with_tls(
        timeout_ms: u64,
        timing: Arc<std::sync::Mutex<TimingContext>>,
        tls_options: &TlsOptions,
    ) -> Result<Self, String> {
        let config = build_custom_tls_config(tls_options)?;
        Ok(Self {
            tls_config: Arc::new(config),
            timeout_ms,
            timing,
            proxy_state: None,
        })
    }

    /// 设置代理状态（链式调用）
    ///
    /// 设置后，连接器将通过代理服务器建立隧道。
    pub fn with_proxy(mut self, proxy_state: Arc<flow_engine_core::context::ProxyState>) -> Self {
        self.proxy_state = Some(proxy_state);
        self
    }
}

impl Service<Uri> for TimingConnector {
    type Response = TimedStream;
    type Error = HttpError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let tls_config = self.tls_config.clone();
        let timeout_ms = self.timeout_ms;
        let timing = self.timing.clone();
        let proxy_state = self.proxy_state.clone();

        Box::pin(async move {
            // 提取主机和端口
            let host = uri
                .host()
                .ok_or_else(|| HttpError::InvalidUrl("Missing hostname".into()))?
                .to_string();

            let port = uri
                .port_u16()
                .unwrap_or(if uri.scheme_str() == Some("https") {
                    443
                } else {
                    80
                });

            let is_https = uri.scheme_str() == Some("https");

            log::debug!(
                "Starting connection: {}://{}:{}{}",
                if is_https { "https" } else { "http" },
                host,
                port,
                if proxy_state.is_some() {
                    " (via proxy)"
                } else {
                    ""
                }
            );

            // 获取 TCP 连接：直连或通过代理隧道
            let tcp_stream = if let Some(ref ps) = proxy_state {
                // 通过代理建立隧道
                crate::http::proxy::connect_via_proxy(ps, &host, port, timeout_ms)
                    .await
                    .map_err(HttpError::ProxyFailed)?
            } else {
                // 直连：DNS 解析 — 收集所有解析地址
                let addrs: Vec<_> = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    tokio::net::lookup_host((host.as_str(), port)),
                )
                .await
                .map_err(|_| HttpError::DnsTimeout)?
                .map_err(|e| HttpError::DnsFailed(e.to_string()))?
                .collect();

                if addrs.is_empty() {
                    return Err(HttpError::DnsNoAddress);
                }

                {
                    let mut timing = timing.lock().map_err(|_| {
                        HttpError::DnsFailed("Failed to acquire timing lock".into())
                    })?;
                    timing.mark_dns_resolved();
                }
                log::debug!(
                    "DNS resolution complete, {} addresses: {:?}",
                    addrs.len(),
                    addrs
                );

                // 直连：TCP 连接 — 依次尝试所有地址
                let stream = {
                    let mut last_err = None;
                    let mut connected = None;
                    for addr in &addrs {
                        log::debug!("Trying connection: {}", addr);
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(timeout_ms),
                            TcpStream::connect(addr),
                        )
                        .await
                        {
                            Ok(Ok(s)) => {
                                connected = Some(s);
                                break;
                            }
                            Ok(Err(e)) => {
                                log::debug!("Connection {} failed: {}", addr, e);
                                last_err = Some(e);
                            }
                            Err(_) => {
                                log::debug!("Connection {} timed out", addr);
                                last_err = Some(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    format!("Connection {} timed out", addr),
                                ));
                            }
                        }
                    }
                    connected.ok_or_else(|| {
                        let msg = last_err
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "All addresses failed to connect".to_string());
                        HttpError::TcpFailed(msg)
                    })?
                };

                stream
            };

            {
                let mut timing = timing
                    .lock()
                    .map_err(|_| HttpError::TcpFailed("Failed to acquire timing lock".into()))?;
                timing.mark_tcp_connected();
            }
            log::debug!("TCP connection established");

            // TLS 握手（HTTPS）
            if is_https {
                let connector = TlsConnector::from(tls_config);

                // 创建 ServerName（兼容 DNS 名称和 IP 地址）
                let server_name = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                    rustls_pki_types::ServerName::IpAddress(ip.into())
                } else {
                    let dns_name = rustls_pki_types::DnsName::try_from(host.clone())
                        .map_err(|e| HttpError::InvalidServerName(format!("{}: {:?}", host, e)))?;
                    rustls_pki_types::ServerName::from(dns_name)
                };

                let tls_stream = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    connector.connect(server_name, tcp_stream),
                )
                .await
                .map_err(|_| HttpError::TlsTimeout)?
                .map_err(|e| HttpError::TlsFailed(e.to_string()))?;

                // 检查 ALPN 协商结果
                let (_, session) = tls_stream.get_ref();
                let alpn = session.alpn_protocol();
                let is_h2 = alpn.map(|p| p == b"h2").unwrap_or(false);

                {
                    let mut timing = timing.lock().map_err(|_| {
                        HttpError::TlsFailed("Failed to acquire timing lock".into())
                    })?;
                    timing.mark_tls_handshaked();
                }
                log::debug!(
                    "TLS handshake complete, ALPN: {:?}",
                    alpn.map(|b| String::from_utf8_lossy(b).to_string())
                );

                Ok(TimedStream::Https(
                    Box::new(TokioIo::new(tls_stream)),
                    is_h2,
                ))
            } else {
                Ok(TimedStream::Http(TokioIo::new(tcp_stream)))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_timing_connector_http() {
        let timing = Arc::new(std::sync::Mutex::new(TimingContext::new()));
        let mut connector = TimingConnector::new(10000, timing.clone());

        let uri: Uri = "http://localhost:19090/api/status"
            .parse()
            .unwrap_or_else(|e| panic!("{e}"));
        let result = Service::<Uri>::call(&mut connector, uri).await;

        assert!(result.is_ok(), "HTTP 连接应该Success");
        let stream = result.unwrap_or_else(|e| panic!("{e}"));

        // 检查是否为 HTTP 流
        assert!(matches!(stream, TimedStream::Http(_)));

        // 验证计时数据
        let timing_data = {
            let t = timing.lock().unwrap_or_else(|e| panic!("{e}"));
            t.calculate()
        };
        assert!(timing_data.dns_ms.is_some());
        assert!(timing_data.connect_ms.is_some());
        assert!(timing_data.tls_ms.is_none()); // HTTP 无 TLS
    }
}
