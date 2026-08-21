//! HTTP 模块 - 提供带详细计时的 HTTP 客户端（支持 HTTP/1.1 + HTTP/2）
//!
//! 本模块使用 hyper 1.x + tokio-rustls 实现 HTTP/HTTPS 请求，提供以下功能：
//! - DNS 解析计时
//! - TCP 连接计时
//! - TLS 握手计时（HTTPS）
//! - 首字节时间（TTFB）计时
//! - 响应体下载计时
//! - HTTP/1.1 和 HTTP/2 自动协商（ALPN）
//!
//! # 示例
//! ```rust,no_run
//! use flow_engine_http::http::{HttpClient, HttpRequestConfig, BodyType};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = HttpClient::new(30000);
//!     let config = HttpRequestConfig {
//!         url: "https://example.com/api".to_string(),
//!         method: "GET".to_string(),
//!         headers: HashMap::new(),
//!         body: BodyType::None,
//!         multipart_boundary: None,
//!         accept_encoding: true,
//!         follow_redirects: true,
//!         max_redirects: 10,
//!     };
//!
//!     let result = client.execute(&config).await?;
//!
//!     println!("Status: {}", result.response.status);
//!     println!("DNS: {:?}ms", result.timing.dns_ms);
//!     println!("TCP: {:?}ms", result.timing.connect_ms);
//!     println!("TLS: {:?}ms", result.timing.tls_ms);
//!     println!("TTFB: {:?}ms", result.timing.ttfb_ms);
//!     println!("Download: {:?}ms", result.timing.download_ms);
//!
//!     Ok(())
//! }
//! ```

pub mod body;
pub mod client;
pub mod connector;
/// Cookie 管理模块，提供 Cookie Jar 实现
pub mod cookie;
/// HTTP Error类型定义，涵盖 DNS/TCP/TLS/HTTP 协议各阶段的Error
pub mod error;
/// HTTP 代理模块，支持通过 HTTP 代理发送请求
pub mod proxy;
/// HTTP 请求各阶段计时，记录 DNS/TCP/TLS/TTFB/下载耗时
pub mod timing;

// Re-export 公共类型
pub use body::{BodyType, MultipartBody, MultipartField, UrlEncoder};
pub use client::{HttpClient, HttpRequestConfig, HttpResponse, HttpResult};
pub use connector::{TimedStream, TimingConnector, TlsOptions};
pub use error::HttpError;
pub use timing::{HttpTiming, TimingContext};
