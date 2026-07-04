//! HTTP 客户端 - 使用 hyper 1.x 支持 HTTP/1.1 + HTTP/2
//!
//! 本模块提供带详细计时的 HTTP 客户端实现：
//! - 自动 HTTP/1.1 和 HTTP/2 协商
//! - DNS/TCP/TLS/TTFB/Download 各阶段计时
//! - 支持多种请求体类型（None, Raw, Urlencoded, Multipart）

use crate::http::body::{BodyType, MultipartBody, UrlEncoder};
use crate::http::connector::TimingConnector;
use crate::http::cookie::CookieJar;
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

/// 重定向记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedirectEntry {
    /// HTTP 状态码
    pub status: u16,
    /// 重定向目标 URL
    pub location: String,
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
    /// 请求体类型
    pub body: BodyType,
    /// Multipart boundary（仅当 body 为 Multipart 时需要）
    pub multipart_boundary: Option<String>,
    /// 是否接受压缩编码
    pub accept_encoding: bool,
    /// 是否跟随重定向
    pub follow_redirects: bool,
    /// 最大重定向次数
    pub max_redirects: u32,
}

impl Default for HttpRequestConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: String::new(),
            headers: HashMap::new(),
            body: BodyType::None,
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        }
    }
}

/// HTTP 客户端执行结果
#[derive(Debug)]
pub struct HttpResult {
    /// 响应对象
    pub response: HttpResponse,
    /// 计时数据
    pub timing: HttpTiming,
    /// 重定向历史记录
    pub redirect_history: Vec<RedirectEntry>,
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
            serde_json::from_str::<serde_json::Value>(&format!("Invalid UTF-8: {}", e))
                .unwrap_err()
        })?;
        serde_json::from_str(&text)
    }
}

/// HTTP 客户端（使用 hyper 1.x，支持 HTTP/1.1 + HTTP/2）
pub struct HttpClient {
    timeout_ms: u64,
    tls_options: Option<crate::http::connector::TlsOptions>,
    proxy_state: Option<Arc<flow_engine_core::context::ProxyState>>,
    /// 共享 Cookie Jar（跨重定向、跨节点持久化）
    cookie_jar: Option<Arc<std::sync::Mutex<CookieJar>>>,
}

impl HttpClient {
    /// 创建新的 HTTP 客户端
    ///
    /// # 参数
    /// - `timeout_ms`: 各阶段超时时间（毫秒）
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms, tls_options: None, proxy_state: None, cookie_jar: None }
    }

    /// 创建带自定义 TLS 配置的 HTTP 客户端
    pub fn with_tls(timeout_ms: u64, tls_options: crate::http::connector::TlsOptions) -> Self {
        Self { timeout_ms, tls_options: Some(tls_options), proxy_state: None, cookie_jar: None }
    }

    /// 设置代理状态（链式调用）
    ///
    /// 设置后，HTTP 请求将通过指定的代理服务器路由。
    /// 支持 HTTP 代理（CONNECT 隧道）和 SOCKS5 代理。
    pub fn with_proxy(mut self, proxy_state: Arc<flow_engine_core::context::ProxyState>) -> Self {
        self.proxy_state = Some(proxy_state);
        self
    }

    /// 设置共享 Cookie Jar（链式调用）
    ///
    /// Cookie Jar 会在重定向链路中自动处理 Set-Cookie，
    /// 并将 Cookie 注入到后续请求中。多个 HTTP 节点共享同一个 Jar 实现跨节点 Cookie 持久化。
    pub fn with_cookie_jar(mut self, jar: Arc<std::sync::Mutex<CookieJar>>) -> Self {
        self.cookie_jar = Some(jar);
        self
    }
}

/// 解压缩响应体
fn decompress_body(body: &[u8], content_encoding: Option<&str>) -> Vec<u8> {
    match content_encoding {
        Some("gzip") => {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(body);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                decompressed
            } else {
                body.to_vec()
            }
        }
        Some("deflate") => {
            use std::io::Read;
            let mut decoder = flate2::read::DeflateDecoder::new(body);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                decompressed
            } else {
                body.to_vec()
            }
        }
        _ => body.to_vec(),
    }
}

impl HttpClient {
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

        // 创建连接器（支持 TLS 配置和可选代理）
        let connector = match &self.tls_options {
            Some(opts) => {
                let c = TimingConnector::new_with_tls(self.timeout_ms, timing.clone(), opts)
                    .map_err(HttpError::TlsFailed)?;
                match &self.proxy_state {
                    Some(ps) => c.with_proxy(ps.clone()),
                    None => c,
                }
            }
            None => {
                let c = TimingConnector::new(self.timeout_ms, timing.clone());
                match &self.proxy_state {
                    Some(ps) => c.with_proxy(ps.clone()),
                    None => c,
                }
            }
        };

        // 创建 hyper 客户端
        let client: Client<TimingConnector, BoxBody> =
            Client::builder(TokioExecutor::new()).build(connector);
        // 重定向跟踪状态
        let mut current_url = config.url.clone();
        let mut current_method = config.method.clone();
        let mut current_body = config.body.clone();
        let mut redirect_history: Vec<RedirectEntry> = Vec::new();
        let max_redirects = if config.follow_redirects { config.max_redirects } else { 0 };
        let mut redirect_count: u32 = 0;

        loop {
            let method = match current_method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH,
                "HEAD" => Method::HEAD,
                "OPTIONS" => Method::OPTIONS,
                _ => return Err(HttpError::UnsupportedMethod(current_method.clone())),
            };

            // 解析并规范化 URL（自动编码特殊字符）
            let parsed_url = url::Url::parse(&current_url)
                .map_err(|e| HttpError::InvalidUrl(format!("Invalid URL: {}", e)))?;
            let normalized_url = parsed_url.to_string();

            let uri: http::Uri = normalized_url
                .parse()
                .map_err(|e| HttpError::InvalidUrl(format!("Invalid URI: {}", e)))?;

            let mut builder = Request::builder().method(method).uri(uri);

            // 提取用户设置的 Content-Type（供 Raw body 类型使用）
            let user_content_type = config.headers.iter()
                .find(|(k, _)| k.to_lowercase() == "content-type")
                .map(|(_, v)| v.clone());

            // 添加请求头（不包含 Content-Type，由 body 类型决定）
            let mut has_accept_encoding = false;
            for (key, value) in &config.headers {
                if key.to_lowercase() != "content-type" {
                    builder = builder.header(key, value);
                }
                if key.to_lowercase() == "accept-encoding" {
                    has_accept_encoding = true;
                }
            }

            // 如果启用压缩且用户未手动设置 Accept-Encoding
            if config.accept_encoding && !has_accept_encoding {
                builder = builder.header("Accept-Encoding", "gzip, deflate");
            }

            // 注入 Cookie 头（从共享 Jar 中读取，覆盖重定向前后的所有请求）
            if let Some(ref jar) = self.cookie_jar {
                if let Ok(guard) = jar.lock() {
                    if !guard.is_empty() {
                        let cookie_val = guard.cookie_header(&current_url);
                        if !cookie_val.is_empty() {
                            builder = builder.header("Cookie", &cookie_val);
                        }
                    }
                }
            }

            // 根据请求体类型构建请求
            let request = match &current_body {
                BodyType::None => {
                    builder
                        .body(empty())
                        .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
                }
                BodyType::Raw(data) => {
                    // Raw body 使用用户设置的 Content-Type（来自 http_executor 自动设置或手动配置）
                    if let Some(ct) = &user_content_type {
                        builder = builder.header("Content-Type", ct);
                    }
                    builder
                        .body(full(Bytes::copy_from_slice(data)))
                        .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
                }
                BodyType::Urlencoded(fields) => {
                    builder = builder.header("Content-Type", "application/x-www-form-urlencoded");
                    let encoded = UrlEncoder::encode(fields);
                    builder
                        .body(full(encoded.into_bytes()))
                        .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
                }
                BodyType::Multipart(multipart) => {
                    let boundary = config
                        .multipart_boundary
                        .clone()
                        .unwrap_or_else(MultipartBody::generate_boundary);
                    builder = builder.header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={}", boundary),
                    );
                    let encoded = multipart.encode(&boundary);
                    builder
                        .body(full(encoded))
                        .map_err(|e| HttpError::BodyBuildFailed(e.to_string()))?
                }
            };

            log::debug!("Sending request: {} {}", current_method, current_url);

            // 发送请求
            let response = client.request(request).await.map_err(|e| {
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("dns") || error_msg.contains("resolve")
                    || error_msg.contains("connect")
                {
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
                    .map_err(|_| HttpError::ReadFailed("Failed to acquire timing lock".into()))?;
                t.mark_first_byte();
            }

            // 提取状态码和响应头
            let status = response.status().as_u16();
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            log::debug!("Response status: {}", status);

            // 处理 Set-Cookie（所有响应包括重定向中间响应都捕获，记录来源域用于 domain 匹配）
            if let Some(ref jar) = self.cookie_jar {
                if let Ok(mut guard) = jar.lock() {
                    guard.store_from_response_with_url(&headers, &current_url);
                }
            }

            // 收集响应体
            let body_bytes = response
                .collect()
                .await
                .map_err(|e| HttpError::ReadFailed(e.to_string()))?
                .to_bytes();

            // 标记下载完成
            {
                let mut t = timing
                    .lock()
                    .map_err(|_| HttpError::ReadFailed("Failed to acquire timing lock".into()))?;
                t.mark_download_complete();
            }

            // 检查是否需要重定向
            if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308)
                && redirect_count < max_redirects
            {
                let location = headers.get("Location")
                    .or_else(|| headers.get("location"));

                if let Some(location) = location {
                    redirect_count += 1;

                    // 解析重定向 URL（相对或绝对）
                    let resolved_url = if location.starts_with("http://") || location.starts_with("https://") {
                        location.clone()
                    } else {
                        let base = url::Url::parse(&current_url)
                            .map_err(|e| HttpError::InvalidUrl(format!("Redirect URL parse failed: {}", e)))?;
                        base.join(location)
                            .map(|u| u.to_string())
                            .unwrap_or_else(|_| location.clone())
                    };

                    log::debug!("Redirect ({}) -> {}", status, resolved_url);

                    redirect_history.push(RedirectEntry {
                        status,
                        location: resolved_url.clone(),
                    });

                    // 301/302/303: 非 GET/HEAD 方法改为 GET，丢弃请求体
                    if (status == 301 || status == 302 || status == 303)
                        && current_method != "GET" && current_method != "HEAD"
                    {
                        current_method = "GET".to_string();
                        current_body = BodyType::None;
                    }
                    // 307/308: 保持原方法和请求体

                    current_url = resolved_url;
                    continue;
                }
            }

            // 解压缩响应体
            let content_encoding = headers
                .get("Content-Encoding")
                .or_else(|| headers.get("content-encoding"))
                .map(|s| s.as_str());
            let body = decompress_body(&body_bytes, content_encoding);

            log::debug!("Response body size: {} bytes (decompressed)", body.len());

            // 计算计时数据
            let timing_data = {
                let t = timing
                    .lock()
                    .map_err(|_| HttpError::ReadFailed("Failed to acquire timing lock".into()))?;
                t.calculate()
            };

            log::debug!(
                "Timing: DNS={:?}ms, TCP={:?}ms, TLS={:?}ms, TTFB={:?}ms, Download={:?}ms",
                timing_data.dns_ms,
                timing_data.connect_ms,
                timing_data.tls_ms,
                timing_data.ttfb_ms,
                timing_data.download_ms
            );

            return Ok(HttpResult {
                response: HttpResponse {
                    status,
                    headers,
                    body,
                },
                timing: timing_data,
                redirect_history,
            });
        }
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
        match &config.body {
            BodyType::None => {}
            BodyType::Raw(data) => {
                builder = builder.body(data.clone());
            }
            BodyType::Urlencoded(fields) => {
                let encoded = UrlEncoder::encode(fields);
                builder = builder.body(encoded.into_bytes());
            }
            BodyType::Multipart(multipart) => {
                let boundary = config
                    .multipart_boundary
                    .clone()
                    .unwrap_or_else(MultipartBody::generate_boundary);
                let encoded = multipart.encode(&boundary);
                builder = builder.body(encoded);
            }
        }

        let request_bytes = builder.build();

        log::debug!("Sending request: {} {}", config.method, config.url);

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

        log::debug!("Request sent, waiting for response...");

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
            "Response received: Status {}, Size {} bytes",
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
    /// HTTP 响应，包含状态码、响应头和响应体
    pub response: LegacyHttpResponse,
    /// 请求各阶段计时信息（DNS/TCP/TLS/TTFB/下载）
    pub timing: HttpTiming,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_http_get() {
        let client = HttpClient::new(10000);
        let config = HttpRequestConfig {
            url: "http://localhost:19090/api/status".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: BodyType::None,
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "HTTP GET 应该Success: {:?}", result.err());
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
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_post_with_body() {
        let client = HttpClient::new(10000);
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let config = HttpRequestConfig {
            url: "http://localhost:19090/api/mirror".to_string(),
            method: "POST".to_string(),
            headers,
            body: BodyType::raw(br#"{"test":"value"}"#.to_vec()),
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "POST 应该Success: {:?}", result.err());
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 200);

        // 验证响应体包含请求数据（mirror 端点回显 json 字段）
        let body = http_result.response.body_json().expect("JSON 解析Failed");
        assert!(body["json"]["test"].as_str() == Some("value"));
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_404_error() {
        let client = HttpClient::new(10000);
        let config = HttpRequestConfig {
            url: "http://localhost:19090/api/status/404".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: BodyType::None,
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        };

        let result = client.execute(&config).await;

        assert!(result.is_ok(), "请求应该Success（状态码 404 不是Error）");
        let http_result = result.unwrap();

        assert_eq!(http_result.response.status, 404);
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_dns_failure() {
        let client = HttpClient::new(5000);
        let config = HttpRequestConfig {
            url: "http://this-domain-does-not-exist-12345.invalid".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: BodyType::None,
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        };

        let result = client.execute(&config).await;

        assert!(result.is_err(), "DNS Failed应该返回Error");
        let error = result.unwrap_err();
        assert!(matches!(
            error,
            HttpError::DnsFailed(_) | HttpError::DnsTimeout | HttpError::DnsNoAddress
        ));
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_timeout() {
        let client = HttpClient::new(1000); // 1 秒超时
        let config = HttpRequestConfig {
            url: "http://192.0.2.1:81".to_string(), // 不可路由的测试地址
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: BodyType::None,
            multipart_boundary: None,
            accept_encoding: true,
            follow_redirects: true,
            max_redirects: 10,
        };

        let result = client.execute(&config).await;

        assert!(result.is_err(), "超时应该返回Error: {:?}", result.ok());
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_client_all_methods() {
        let client = HttpClient::new(10000);

        let methods = vec![
            ("GET", "http://localhost:19090/api/mirror"),
            ("POST", "http://localhost:19090/api/mirror"),
            ("PUT", "http://localhost:19090/api/mirror"),
            ("DELETE", "http://localhost:19090/api/mirror"),
            ("PATCH", "http://localhost:19090/api/mirror"),
        ];

        for (method, url) in methods {
            let config = HttpRequestConfig {
                url: url.to_string(),
                method: method.to_string(),
                headers: HashMap::new(),
                body: BodyType::None,
                multipart_boundary: None,
                accept_encoding: true,
                follow_redirects: true,
                max_redirects: 10,
            };

            let result = client.execute(&config).await;
            assert!(result.is_ok(), "{} 请求应该Success: {:?}", method, result.err());
            let http_result = result.unwrap();
            assert_eq!(http_result.response.status, 200, "{} 状态码应为 200", method);
        }
    }
}
