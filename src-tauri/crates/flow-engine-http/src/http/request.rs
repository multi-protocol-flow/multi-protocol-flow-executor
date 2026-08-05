use crate::http::error::HttpError;
use std::collections::HashMap;

/// HTTP 请求构建器
pub struct RequestBuilder {
    method: String,
    path: String,
    host: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    /// 创建新的请求构建器
    ///
    /// # 参数
    /// - `method`: HTTP 方法（GET、POST、PUT、DELETE、PATCH）
    /// - `url`: 请求 URL
    pub fn new(method: &str, url: &str) -> Result<Self, HttpError> {
        let parsed = url::Url::parse(url).map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| HttpError::InvalidUrl("缺少主机名".into()))?
            .to_string();
        let port = parsed
            .port()
            .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
        let path_and_query = if let Some(query) = parsed.query() {
            format!("{}?{}", parsed.path(), query)
        } else {
            parsed.path().to_string()
        };

        Ok(Self {
            method: method.to_uppercase(),
            path: path_and_query,
            host: if port != 80 && port != 443 {
                format!("{}:{}", host, port)
            } else {
                host
            },
            headers: HashMap::new(),
            body: None,
        })
    }

    /// 添加请求头
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 添加多个请求头
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        for (key, value) in headers {
            self.headers.insert(key, value);
        }
        self
    }

    /// 设置请求体
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// 设置文本请求体
    pub fn body_text(self, text: impl Into<String>) -> Self {
        self.body(text.into().into_bytes())
    }

    /// 构建原始 HTTP 请求字节
    pub fn build(self) -> Vec<u8> {
        let mut request = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\n",
            self.method, self.path, self.host
        );

        // 添加请求头
        for (key, value) in &self.headers {
            request.push_str(&format!("{}: {}\r\n", key, value));
        }

        // 添加请求体长度
        if let Some(body) = &self.body {
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }

        request.push_str("\r\n");

        let mut bytes = request.into_bytes();
        if let Some(body) = self.body {
            bytes.extend(body);
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder_get() {
        let builder = RequestBuilder::new("GET", "https://example.com/path?query=value")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"));

        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));

        assert!(request_str.starts_with("GET /path?query=value HTTP/1.1\r\n"));
        assert!(request_str.contains("Host: example.com\r\n"));
        assert!(request_str.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_request_builder_post_with_body() {
        let body = r#"{"key":"value"}"#;
        let builder = RequestBuilder::new("POST", "https://example.com/api")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"))
            .header("Content-Type", "application/json")
            .body_text(body);

        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));

        assert!(request_str.starts_with("POST /api HTTP/1.1\r\n"));
        assert!(request_str.contains("Host: example.com\r\n"));
        assert!(request_str.contains("Content-Type: application/json\r\n"));
        assert!(request_str.contains("Content-Length: 15\r\n"));
        assert!(request_str.ends_with(&format!("\r\n\r\n{}", body)));
    }

    #[test]
    fn test_request_builder_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let builder = RequestBuilder::new("GET", "https://api.example.com/data")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"))
            .headers(headers);

        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));

        assert!(request_str.contains("Authorization: Bearer token123\r\n"));
        assert!(request_str.contains("X-Custom-Header: custom-value\r\n"));
    }

    #[test]
    fn test_request_builder_with_port() {
        let builder = RequestBuilder::new("GET", "https://example.com:8080/path")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"));

        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));

        assert!(request_str.contains("Host: example.com:8080\r\n"));
    }

    #[test]
    fn test_request_builder_default_port() {
        // HTTPS 默认端口 443（不显示）
        let builder = RequestBuilder::new("GET", "https://example.com/path")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"));
        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));
        assert!(request_str.contains("Host: example.com\r\n"));
        assert!(!request_str.contains("Host: example.com:443\r\n"));

        // HTTP 默认端口 80（不显示）
        let builder = RequestBuilder::new("GET", "http://example.com/path")
            .unwrap_or_else(|e| panic!("创建Failed: {e}"));
        let request = builder.build();
        let request_str = String::from_utf8(request).unwrap_or_else(|e| panic!("无效 UTF-8: {e}"));
        assert!(request_str.contains("Host: example.com\r\n"));
        assert!(!request_str.contains("Host: example.com:80\r\n"));
    }

    #[test]
    fn test_request_builder_invalid_url() {
        let result = RequestBuilder::new("GET", "not-a-valid-url");
        assert!(result.is_err());
    }
}
