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
    Https(Box<TokioIo<tokio_rustls::client::TlsStream<TcpStream>>>, bool),
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

/// 确保 CryptoProvider 已安装
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
    let root_store = rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    );
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

    let mut root_store = rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    );

    if let Some(ca_path) = &options.ca_cert_path {
        let ca_bytes = std::fs::read(ca_path)
            .map_err(|e| format!("读取 CA 证书失败 ({}): {}", ca_path, e))?;
        let mut cursor = std::io::Cursor::new(&ca_bytes[..]);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("解析 CA 证书 PEM 失败: {}", e))?;
        for cert in certs {
            root_store.add(cert).map_err(|e| format!("添加 CA 证书失败: {}", e))?;
        }
    }

    let has_client_cert = options.client_cert_path.is_some() && options.client_key_path.is_some();
    if has_client_cert {
        let cert_path = options.client_cert_path.as_ref().ok_or("客户端证书路径未设置")?;
        let key_path = options.client_key_path.as_ref().ok_or("客户端私钥路径未设置")?;
        let cert_bytes = std::fs::read(cert_path)
            .map_err(|e| format!("读取客户端证书失败 ({}): {}", cert_path, e))?;
        let mut cert_cursor = std::io::Cursor::new(&cert_bytes[..]);
        let client_certs: Vec<_> = rustls_pemfile::certs(&mut cert_cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("解析客户端证书 PEM 失败: {}", e))?;
        let key_bytes = std::fs::read(key_path)
            .map_err(|e| format!("读取客户端私钥失败 ({}): {}", key_path, e))?;
        let mut key_cursor = std::io::Cursor::new(&key_bytes[..]);
        let client_key = rustls_pemfile::private_key(&mut key_cursor)
            .map_err(|e| format!("解析客户端私钥 PEM 失败: {}", e))?
            .ok_or_else(|| "客户端私钥文件为空".to_string())?;
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| format!("配置客户端证书失败: {}", e))?;
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
        Self { tls_config: Arc::new(config), timeout_ms, timing, proxy_state: None }
    }

    /// 创建带自定义 TLS 配置的计时连接器
    pub fn new_with_tls(
        timeout_ms: u64,
        timing: Arc<std::sync::Mutex<TimingContext>>,
        tls_options: &TlsOptions,
    ) -> Result<Self, String> {
        let config = build_custom_tls_config(tls_options)?;
        Ok(Self { tls_config: Arc::new(config), timeout_ms, timing, proxy_state: None })
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
                .ok_or_else(|| HttpError::InvalidUrl("缺少主机名".into()))?
                .to_string();

            let port = uri.port_u16().unwrap_or(
                if uri.scheme_str() == Some("https") { 443 } else { 80 }
            );

            let is_https = uri.scheme_str() == Some("https");

            log::debug!(
                "开始建立连接: {}://{}:{}{}",
                if is_https { "https" } else { "http" },
                host,
                port,
                if proxy_state.is_some() { " (via proxy)" } else { "" }
            );

            // 获取 TCP 连接：直连或通过代理隧道
            let tcp_stream = if let Some(ref ps) = proxy_state {
                // 通过代理建立隧道
                crate::http::proxy::connect_via_proxy(ps, &host, port, timeout_ms)
                    .await
                    .map_err(HttpError::ProxyFailed)?
            } else {
                // 直连：DNS 解析
                let addr = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    tokio::net::lookup_host((host.as_str(), port)),
                )
                .await
                .map_err(|_| HttpError::DnsTimeout)?
                .map_err(|e| HttpError::DnsFailed(e.to_string()))?
                .next()
                .ok_or(HttpError::DnsNoAddress)?;

                {
                    let mut timing = timing
                        .lock()
                        .map_err(|_| HttpError::DnsFailed("计时锁获取失败".into()))?;
                    timing.mark_dns_resolved();
                }
                log::debug!("DNS 解析完成: {}", addr);

                // 直连：TCP 连接
                let stream = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    TcpStream::connect(&addr),
                )
                .await
                .map_err(|_| HttpError::TcpTimeout)?
                .map_err(|e| HttpError::TcpFailed(e.to_string()))?;

                stream
            };

            {
                let mut timing = timing
                    .lock()
                    .map_err(|_| HttpError::TcpFailed("计时锁获取失败".into()))?;
                timing.mark_tcp_connected();
            }
            log::debug!("TCP 连接完成");

            // TLS 握手（HTTPS）
            if is_https {
                let connector = TlsConnector::from(tls_config);

                // 创建 ServerName
                let dns_name = rustls_pki_types::DnsName::try_from(host.clone())
                    .map_err(|e| HttpError::InvalidServerName(format!("{}: {:?}", host, e)))?;
                let server_name = rustls_pki_types::ServerName::from(dns_name);

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
                    let mut timing = timing
                        .lock()
                        .map_err(|_| HttpError::TlsFailed("计时锁获取失败".into()))?;
                    timing.mark_tls_handshaked();
                }
                log::debug!(
                    "TLS 握手完成, ALPN: {:?}",
                    alpn.map(|b| String::from_utf8_lossy(b).to_string())
                );

                Ok(TimedStream::Https(Box::new(TokioIo::new(tls_stream)), is_h2))
            } else {
                Ok(TimedStream::Http(TokioIo::new(tcp_stream)))
            }
        })
    }
}

// ============================================================================
// 旧版连接器（向后兼容）
// ============================================================================

/// 连接结果（支持 HTTP 和 HTTPS）
#[derive(Debug)]
pub enum LegacyConnection {
    /// HTTP 连接（明文）
    Http(TcpStream),
    /// HTTPS 连接（TLS 加密）
    Https(Box<tokio_rustls::client::TlsStream<TcpStream>>),
}

/// 旧版连接器，用于向后兼容
pub struct HttpConnector {
    tls_config: Arc<rustls::ClientConfig>,
    timeout_ms: u64,
}

impl HttpConnector {
    /// 创建新的连接器
    ///
    /// # 参数
    /// - `timeout_ms`: 各阶段超时时间（毫秒）
    pub fn new(timeout_ms: u64) -> Self {
        // 构建根证书存储
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // 构建 TLS 配置
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Self {
            tls_config: Arc::new(config),
            timeout_ms,
        }
    }

    /// 建立连接（DNS + TCP + 可选 TLS）
    ///
    /// # 计时点
    /// - DNS 解析开始: 函数入口
    /// - DNS 解析完成: `mark_dns_resolved()`
    /// - TCP 连接完成: `mark_tcp_connected()`
    /// - TLS 握手完成（HTTPS）: `mark_tls_handshaked()`
    ///
    /// # 参数
    /// - `url`: 请求 URL
    /// - `timing`: 计时上下文
    pub async fn connect(
        &self,
        url: &str,
        timing: &mut TimingContext,
    ) -> Result<LegacyConnection, HttpError> {
        // 1. 解析 URL
        let parsed = url::Url::parse(url).map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| HttpError::InvalidUrl("缺少主机名".into()))?
            .to_string();
        let port = parsed.port().unwrap_or(if parsed.scheme() == "https" {
            443
        } else {
            80
        });
        let is_https = parsed.scheme() == "https";

        log::debug!(
            "开始建立连接: {}://{}:{}",
            if is_https { "https" } else { "http" },
            host,
            port
        );

        // 2. DNS 解析
        let addr = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            tokio::net::lookup_host((host.as_str(), port)),
        )
        .await
        .map_err(|_| HttpError::DnsTimeout)?
        .map_err(|e| HttpError::DnsFailed(e.to_string()))?
        .next()
        .ok_or(HttpError::DnsNoAddress)?;

        timing.mark_dns_resolved();
        log::debug!("DNS 解析完成: {}", addr);

        // 3. TCP 连接
        let tcp_stream = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| HttpError::TcpTimeout)?
        .map_err(|e| HttpError::TcpFailed(e.to_string()))?;

        timing.mark_tcp_connected();
        log::debug!("TCP 连接完成");

        // 4. TLS 握手（HTTPS）
        if is_https {
            let connector = TlsConnector::from(self.tls_config.clone());

            // 创建拥有所有权的 ServerName
            let dns_name = rustls_pki_types::DnsName::try_from(host.clone())
                .map_err(|e| HttpError::InvalidServerName(format!("{}: {:?}", host, e)))?;
            let server_name = rustls_pki_types::ServerName::from(dns_name);

            let tls_stream = tokio::time::timeout(
                std::time::Duration::from_millis(self.timeout_ms),
                connector.connect(server_name, tcp_stream),
            )
            .await
            .map_err(|_| HttpError::TlsTimeout)?
            .map_err(|e| HttpError::TlsFailed(e.to_string()))?;

            timing.mark_tls_handshaked();
            log::debug!("TLS 握手完成");

            Ok(LegacyConnection::Https(Box::new(tls_stream)))
        } else {
            Ok(LegacyConnection::Http(tcp_stream))
        }
    }
}

/// 连接类型别名（向后兼容）
pub type Connection = LegacyConnection;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_http() {
        let connector = HttpConnector::new(5000);
        let mut timing = TimingContext::new();

        // 测试 HTTP 连接
        let result = connector.connect("http://httpbin.org/get", &mut timing).await;

        assert!(result.is_ok(), "HTTP 连接应该成功");
        let connection = result.unwrap();
        assert!(matches!(connection, LegacyConnection::Http(_)));

        // 验证计时数据
        let timing_data = timing.calculate();
        assert!(timing_data.dns_ms.is_some());
        assert!(timing_data.connect_ms.is_some());
        assert!(timing_data.tls_ms.is_none()); // HTTP 无 TLS
    }

    #[tokio::test]
    async fn test_connector_https() {
        let connector = HttpConnector::new(5000);
        let mut timing = TimingContext::new();

        // 测试 HTTPS 连接
        let result = connector.connect("https://httpbin.org/get", &mut timing).await;

        assert!(result.is_ok(), "HTTPS 连接应该成功");
        let connection = result.unwrap();
        assert!(matches!(connection, LegacyConnection::Https(_)));

        // 验证计时数据
        let timing_data = timing.calculate();
        assert!(timing_data.dns_ms.is_some());
        assert!(timing_data.connect_ms.is_some());
        assert!(timing_data.tls_ms.is_some()); // HTTPS 有 TLS
    }

    #[tokio::test]
    async fn test_connector_dns_failure() {
        let connector = HttpConnector::new(5000);
        let mut timing = TimingContext::new();

        // 测试 DNS 解析失败
        let result = connector
            .connect("http://this-domain-does-not-exist-12345.invalid", &mut timing)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(
            error,
            HttpError::DnsFailed(_) | HttpError::DnsTimeout | HttpError::DnsNoAddress
        ));
    }

    #[tokio::test]
    async fn test_connector_invalid_url() {
        let connector = HttpConnector::new(5000);
        let mut timing = TimingContext::new();

        // 测试无效 URL
        let result = connector.connect("not-a-valid-url", &mut timing).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, HttpError::InvalidUrl(_)));
    }

    #[tokio::test]
    async fn test_timing_connector_https() {
        let timing = Arc::new(std::sync::Mutex::new(TimingContext::new()));
        let mut connector = TimingConnector::new(10000, timing.clone());

        let uri: Uri = "https://httpbin.org/get".parse().unwrap();
        let result = Service::<Uri>::call(&mut connector, uri).await;

        assert!(result.is_ok(), "HTTPS 连接应该成功");
        let stream = result.unwrap();

        // 检查是否为 HTTPS 流
        assert!(matches!(stream, TimedStream::Https(_, _)));

        // 验证计时数据
        let timing_data = {
            let t = timing.lock().unwrap();
            t.calculate()
        };
        assert!(timing_data.dns_ms.is_some());
        assert!(timing_data.connect_ms.is_some());
        assert!(timing_data.tls_ms.is_some());
    }
}
