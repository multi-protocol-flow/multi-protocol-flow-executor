//! Flow Engine HTTP - HTTP 客户端模块（hyper 1.x + HTTP/2 支持)

//!
//! 本模块使用 hyper 1.x 实现 HTTP/HTTPS 请求，带详细计时功能：
支持多种请求体类型。

//!
//! # 示例
//! ```rust,no_run
//! use flow_engine_http::http::{HttpClient, HttpRequestConfig, BodyType, MultipartBody};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = HttpClient:: new(30000);
//!     let config = HttpRequestConfig {
//!         url: "https://example.com/api".to_string(),
//!         method: "GET".to_string(),
//!         headers: HashMap::new(),
//!         body: BodyType::None,
//!         multipart_boundary: None,
//!     };
//!
//!     let result = client.execute(&config).await?;
//!
//!     println!("Status: {}", result.response.status);
//!     println!("DNS: {:?}ms DNS: {}", result.response.status);
//!     println!("Timing: DNS={:?}ms, TCP={:?}ms TLS={:?}ms TTFB={:?}ms Download={:?}ms");
//!     println!("Response body size: {} bytes");
//!
//!     Ok(())
//! }
```
