//! HTTP 客户端 - 使用 hyper 1.x 支持 HTTP/1.1 + HTTP/2
//!
//! 本模块提供带详细计时的 HTTP 客户端实现：
//! - 自动 HTTP/1.1 和 HTTP/2 协商
//! - DNS/TCP/TLS/TTFB/Download 各阶段计时
//! - 支持多种请求体类型

use crate::http::connector::TimingConnector;
use crate::http::error::HttpError;
use crate::http::timing::{HttpTiming, TimingContext};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::{Method, Request};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;
use std::sync::Arc;

// 定义统一的 Body 类型
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty() -> BoxBody {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

/// HTTP 请求配置
#[derive(Debug, Clone)]
pub struct HttpRequestConfig {
    /// 请求 URL
    pub url: String,
    /// 请求方法
    pub method: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体
    pub body: Option<Vec<u8>>,
}

/// HTTP 客户端执行结果
#[derive(Debug)]
pub struct HttpResult {
    /// 响应对象
    pub response: HttpResponse,
    /// 计时数据
    pub timing: HttpTiming,
}

/// HTTP 响应
#[derive(Debug)]
pub struct HttpResponse {
    /// 状态码
    pub status: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// 获取响应体文本（UTF-8）
    pub fn body_text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }

    /// 检查是否为 JSON 响应
    pub fn is_json(&self) -> bool {
        self.headers
            .get("Content-Type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// 解析响应体为 JSON
    pub fn body_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        let text = self.body_text().map_err(|e| {
            serde_json::from_str::<serde_json::Value>(&format!("无效的 UTF-8: {}", e))
                .unwrap_err()
        })?;
        serde_json::from_str(&text)
    }
}

/// HTTP 客户端（使用 hyper 1.x，支持 HTTP/1.1 + HTTP/2）
pub struct HttpClient {
    timeout_ms: u64,
}

impl HttpClient {
    /// 创建新的 HTTP 客户端
    ///
    /// # 参数
    /// - `timeout_ms`: 各阶段超时时间（毫秒）
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// 执行 HTTP 请求
    ///
    /// # 参数
    /// - `config`: 请求配置
    ///
    /// # 返回
    /// 包含响应和计时数据的结果
    pub async fn execute(&self, config: &HttpRequestConfig) -> Result<HttpResult, HttpError> {
        // 创建计时上下文
        let timing = Arc::new(std::sync::Mutex::new(TimingContext::new()));

        // 创建自定义连接器
        let connector = TimingConnector::new(self.timeout_ms, timing.clone());

        // 创建 hyper 客户端
        let client: Client<TimingConnector, BoxBody> =
            Client::builder(TokioExecutor::new()).build(connector);

        // 构建 hyper 请求
        let method = match config.method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            _ => return Err(HttpError::UnsupportedMethod(config.method.clone())),
        };

        let uri: http::Uri = config
            .url
            .parse()
            .map_err(|e| HttpError::InvalidUrl(format!("无效的 URI: {}", e)))?;

        let mut builder = Request::builder().method(method).uri(uri.clone());

        // 添加请求头
        for (key, value) in &config.headers {
            builder = builder.header(key, value);
        }

        // 构建请求体（使用 BoxBody 统一类型）
        let request = if let Some(body) = &config.body {
            builder
                .body(full(Bytes::copy_from_slice(body)))
                .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
        } else {
            builder
                .body(empty())
                .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
        };

        log::debug!("发送请求: {} {}", config.method, config.url);

        // 发送请求
        let response = client.request(request).await.map_err(|e| {
            // 根据错误类型返回更具体的错误
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("dns") || error_msg.contains("resolve") {
                HttpError::DnsFailed(error_msg)
            } else if error_msg.contains("connect") {
                // "Connect" 错误通常表示 DNS 解析或 TCP 连接失败
                // 由于 DNS 是连接的第一步，将其归类为 DNS 错误
                HttpError::DnsFailed(error_msg)
            } else if error_msg.contains("tcp") {
                HttpError::TcpFailed(error_msg)
            } else if error_msg.contains("tls")
                || error_msg.contains("ssl")
                || error_msg.contains("certificate")
            {
                HttpError::TlsFailed(error_msg)
            } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
                HttpError::TcpTimeout
            } else {
                HttpError::WriteFailed(error_msg)
            }
        })?;

        // 标记首字节时间
        {
            let mut t = timing
                .lock()
                .map_err(|_| HttpError::ReadFailed("计时锁获取失败".into()))?;
            t.mark_first_byte();
        }

        // 提取状态码和响应头
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        log::debug!("响应状态: {}", status);

        // 收集响应体
        let body_bytes = response
            .collect()
            .await
            .map_err(|e| HttpError::ReadFailed(e.to_string()))?
            .to_bytes();

        let body = body_bytes.to_vec();

        // 标记下载完成
        {
            let mut t = timing
                .lock()
                .map_err(|_| HttpError::ReadFailed("计时锁获取失败".into()))?;
            t.mark_download_complete();
        }

        log::debug!("响应体大小: {} bytes", body.len());

        // 计算计时数据
        let timing_data = {
            let t = timing
                .lock()
                .map_err(|_| HttpError::ReadFailed("计时锁获取失败".into()))?;
            t.calculate()
        };

        log::debug!(
            "计时: DNS={:?}ms, TCP={:?}ms, TLS={:?}ms, TTFB={:?}ms, Download={:?}ms",
            timing_data.dns_ms,
            timing_data.connect_ms,
            timing_data.tls_ms,
            timing_data.ttfb_ms,
            timing_data.download_ms
        );

        Ok(HttpResult {
            response: HttpResponse {
                status,
                headers,
                body,
            },
            timing: timing_data,
        })
    }
}

// ============================================================================
// 向后兼容的旧接口（保留用于 http_executor.rs 过渡）
// ============================================================================

use crate::http::connector::LegacyConnection;
use crate::http::request::RequestBuilder;
use crate::http::response::HttpResponse as LegacyHttpResponse;

/// 旧版 HTTP 客户端（保留用于向后兼容）
pub struct LegacyHttpClient {
    connector: Arc<crate::http::connector::HttpConnector>,
}

impl LegacyHttpClient {
    /// 创建新的 HTTP 客户端
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            connector: Arc::new(crate::http::connector::HttpConnector::new(timeout_ms)),
        }
    }

    /// 执行 HTTP 请求（旧版接口）
    pub async fn execute_legacy(
        &self,
        config: &HttpRequestConfig,
    ) -> Result<LegacyHttpResult, HttpError> {
        let mut timing = TimingContext::new();

        // 1. 建立连接
        let mut connection = self.connector.connect(&config.url, &mut timing).await?;

        // 2. 构建请求
        let mut builder = RequestBuilder::new(&config.method, &config.url)?;

        // 添加请求头
        for (key, value) in &config.headers {
            builder = builder.header(key, value);
        }

        // 添加请求体
        if let Some(body) = &config.body {
            builder = builder.body(body.clone());
        }

        let request_bytes = builder.build();

        log::debug!("发送请求: {} {}", config.method, config.url);

        // 3. 发送请求
        use tokio::io::AsyncWriteExt;
        match &mut connection {
            LegacyConnection::Http(stream) => {
                stream
                    .write_all(&request_bytes)
                    .await
                    .map_err(|e| HttpError::WriteFailed(e.to_string()))?;
                stream
                    .flush()
                    .await
                    .map_err(|e| HttpError::WriteFailed(e.to_string()))?;
            }
            LegacyConnection::Https(stream) => {
                stream
                    .write_all(&request_bytes)
                    .await
                    .map_err(|e| HttpError::WriteFailed(e.to_string()))?;
                stream
                    .flush()
                    .await
                    .map_err(|e| HttpError::WriteFailed(e.to_string()))?;
            }
        }

        log::debug!("请求已发送，等待响应...");

        // 4. 读取响应
        let response = match &mut connection {
            LegacyConnection::Http(stream) => {
                LegacyHttpResponse::from_stream(stream, &mut timing).await?
            }
            LegacyConnection::Https(stream) => {
                LegacyHttpResponse::from_stream(stream, &mut timing).await?
            }
        };

        let timing_data = timing.calculate();

        log::debug!(
            "响应接收完成: 状态 {}, 大小 {} bytes",
            response.status,
            response.body.len()
        );

        Ok(LegacyHttpResult {
            response,
            timing: timing_data,
        })
    }
}

/// 旧版 HTTP 结果
#[derive(Debug)]
pub struct LegacyHttpResult {
    pub response: LegacyHttpResponse,
    pub timing: HttpTiming,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_http_get() {
        let client = HttpClient::new(10000);
        let config = HttpRequestConfig {
            url: "http://httpbin.org/get".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "HTTP GET 应该成功: {:?}", result.err());
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 200);

        // 验证计时数据
        assert!(http_result.timing.dns_ms.is_some());
        assert!(http_result.timing.connect_ms.is_some());
        assert!(http_result.timing.tls_ms.is_none()); // HTTP 无 TLS
        assert!(http_result.timing.ttfb_ms.is_some());
        assert!(http_result.timing.download_ms.is_some());
    }

    #[tokio::test]
    async fn test_client_https_get() {
        let client = HttpClient::new(10000);
        let config = HttpRequestConfig {
            url: "https://httpbin.org/get".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "HTTPS GET 应该成功: {:?}", result.err());
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 200);

        // 验证计时数据
        assert!(http_result.timing.dns_ms.is_some());
        assert!(http_result.timing.connect_ms.is_some());
        assert!(http_result.timing.tls_ms.is_some()); // HTTPS 有 TLS
        assert!(http_result.timing.ttfb_ms.is_some());
        assert!(http_result.timing.download_ms.is_some());
    }

    #[tokio::test]
    async fn test_client_post_with_body() {
        let client = HttpClient::new(10000);
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let config = HttpRequestConfig {
            url: "https://httpbin.org/post".to_string(),
            method: "POST".to_string(),
            headers,
            body: Some(br#"{"test":"value"}"#.to_vec()),
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "POST 应该成功: {:?}", result.err());
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 200);

        // 验证响应体包含请求数据
        let body = http_result.response.body_json().expect("JSON 解析失败");
        assert!(body["json"]["test"].as_str() == Some("value"));
    }

    #[tokio::test]
    async fn test_client_404_error() {
        let client = HttpClient::new(10000);
        let config = HttpRequestConfig {
            url: "https://httpbin.org/status/404".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "请求应该成功（状态码 404 不是错误）");
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 404);
    }

    #[tokio::test]
    async fn test_client_dns_failure() {
        let client = HttpClient::new(5000);
        let config = HttpRequestConfig {
            url: "http://this-domain-does-not-exist-12345.invalid".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let result = client.execute(&config).await;

        assert!(result.is_err(), "DNS 失败应该返回错误");
        let error = result.unwrap_err();
        assert!(matches!(
            error,
            HttpError::DnsFailed(_) | HttpError::DnsTimeout | HttpError::DnsNoAddress
        ));
    }

    #[tokio::test]
    async fn test_client_timeout() {
        let client = HttpClient::new(1000); // 1 秒超时
        let config = HttpRequestConfig {
            url: "http://192.0.2.1:81".to_string(), // 不可路由的测试地址
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let result = client.execute(&config).await;

        assert!(result.is_err(), "超时应该返回错误: {:?}", result.ok());
    }

    #[tokio::test]
    async fn test_client_all_methods() {
        let client = HttpClient::new(10000);

        let methods = vec![
            ("GET", "https://httpbin.org/get"),
            ("POST", "https://httpbin.org/post"),
            ("PUT", "https://httpbin.org/put"),
            ("DELETE", "https://httpbin.org/delete"),
            ("PATCH", "https://httpbin.org/patch"),
        ];

        for (method, url) in methods {
            let config = HttpRequestConfig {
                url: url.to_string(),
                method: method.to_string(),
                headers: HashMap::new(),
                body: None,
            };

            let result = client.execute(&config).await;
            assert!(result.is_ok(), "{} 请求应该成功: {:?}", method, result.err());
            let http_result = result.unwrap();
            assert_eq!(http_result.response.status, 200, "{} 状态码应为 200", method);
        }
    }
}
