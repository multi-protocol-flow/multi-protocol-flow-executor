use crate::http::error::HttpError;
use crate::http::timing::TimingContext;
use serde::de::Error as SerdeError;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;

/// HTTP 响应
#[derive(Debug)]
pub struct HttpResponse {
    /// 状态码
    pub status: u16,
    /// 状态文本
    pub status_text: String,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// 从字节流解析响应
    ///
    /// # 计时点
    /// - 首字节到达: `mark_first_byte()`
    /// - 下载完成: `mark_download_complete()`
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    /// - `timing`: 计时上下文
    pub async fn from_stream<S>(
        stream: &mut S,
        timing: &mut TimingContext,
    ) -> Result<Self, HttpError>
    where
        S: AsyncReadExt + Unpin,
    {
        let mut buffer = vec![0u8; 8192];
        let mut total_read = 0;

        // 读取响应头（直到 \r\n\r\n）
        let header_end = loop {
            let n = stream
                .read(&mut buffer[total_read..])
                .await
                .map_err(|e| HttpError::ReadFailed(e.to_string()))?;

            if n == 0 {
                return Err(HttpError::ConnectionClosed);
            }

            total_read += n;

            // 标记首字节时间
            if !timing.has_first_byte() {
                timing.mark_first_byte();
            }

            // 查找 header 结束位置
            if let Some(pos) = Self::find_header_end(&buffer[..total_read]) {
                break pos;
            }

            if total_read >= buffer.len() {
                buffer.resize(buffer.len() * 2, 0);
            }
        };

        // 解析响应头
        let header_bytes = &buffer[..header_end];
        let header_str = std::str::from_utf8(header_bytes)
            .map_err(|e| HttpError::InvalidHeader(e.to_string()))?;

        let (status, status_text, headers) = Self::parse_headers(header_str)?;

        log::debug!("Response status: {} {}", status, status_text);

        // 确定响应体长度
        let body_length = Self::get_body_length(&headers);
        let body_already_read = total_read - header_end;

        // 读取剩余响应体
        let mut body = buffer[header_end..total_read].to_vec();

        if let Some(expected_length) = body_length {
            let remaining = expected_length.saturating_sub(body_already_read);
            if remaining > 0 {
                body.reserve(remaining);
                let mut remaining_buf = vec![0u8; remaining];
                stream
                    .read_exact(&mut remaining_buf)
                    .await
                    .map_err(|e| HttpError::ReadFailed(e.to_string()))?;
                body.extend(remaining_buf);
            }
        } else {
            // Chunked 或 Connection: close 模式，读取到连接关闭
            loop {
                let mut temp_buf = vec![0u8; 8192];
                let n = stream
                    .read(&mut temp_buf)
                    .await
                    .map_err(|e| HttpError::ReadFailed(e.to_string()))?;
                if n == 0 {
                    break;
                }
                body.extend(&temp_buf[..n]);
            }
        }

        timing.mark_download_complete();
        log::debug!("Response body size: {} bytes", body.len());

        Ok(Self {
            status,
            status_text,
            headers,
            body,
        })
    }

    /// 查找响应头结束位置（\r\n\r\n）
    fn find_header_end(bytes: &[u8]) -> Option<usize> {
        for i in 0..bytes.len().saturating_sub(3) {
            if &bytes[i..i + 4] == b"\r\n\r\n" {
                return Some(i + 4);
            }
        }
        None
    }

    /// 解析响应头
    fn parse_headers(
        header_str: &str,
    ) -> Result<(u16, String, HashMap<String, String>), HttpError> {
        let mut lines = header_str.lines();

        // 解析状态行
        let status_line = lines
            .next()
            .ok_or_else(|| HttpError::InvalidHeader("Empty response".into()))?;
        let mut parts = status_line.split_whitespace();
        let _version = parts
            .next()
            .ok_or_else(|| HttpError::InvalidHeader("Missing HTTP version".into()))?;
        let status: u16 = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| HttpError::InvalidHeader("Invalid status code".into()))?;
        let status_text = parts.collect::<Vec<_>>().join(" ");

        // 解析头部
        let mut headers = HashMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        Ok((status, status_text, headers))
    }

    /// 获取响应体长度（Content-Length）
    fn get_body_length(headers: &HashMap<String, String>) -> Option<usize> {
        headers.get("Content-Length").and_then(|v| v.parse().ok())
    }

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
            <serde_json::Error as SerdeError>::custom(format!("Invalid UTF-8: {}", e))
        })?;
        serde_json::from_str(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn test_response_parse_simple() {
        let raw_response = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
        let mut stream = BufReader::new(&raw_response[..]);
        let mut timing = TimingContext::new();

        let response = HttpResponse::from_stream(&mut stream, &mut timing)
            .await
            .expect("解析Failed");

        assert_eq!(response.status, 200);
        assert_eq!(response.status_text, "OK");
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&"13".to_string())
        );
        assert_eq!(response.body_text().unwrap(), "Hello, World!");

        // 验证计时数据
        let timing_data = timing.calculate();
        assert!(timing_data.ttfb_ms.is_some());
        assert!(timing_data.download_ms.is_some());
    }

    #[tokio::test]
    async fn test_response_parse_with_headers() {
        let raw_response = b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nX-Custom-Header: value\r\n\r\n{}";
        let mut stream = BufReader::new(&raw_response[..]);
        let mut timing = TimingContext::new();

        let response = HttpResponse::from_stream(&mut stream, &mut timing)
            .await
            .expect("解析Failed");

        assert_eq!(response.status, 201);
        assert_eq!(response.status_text, "Created");
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            response.headers.get("X-Custom-Header"),
            Some(&"value".to_string())
        );
        assert!(response.is_json());
    }

    #[tokio::test]
    async fn test_response_parse_error_status() {
        let raw_response = b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
        let mut stream = BufReader::new(&raw_response[..]);
        let mut timing = TimingContext::new();

        let response = HttpResponse::from_stream(&mut stream, &mut timing)
            .await
            .expect("解析Failed");

        assert_eq!(response.status, 404);
        assert_eq!(response.status_text, "Not Found");
    }

    #[tokio::test]
    async fn test_response_parse_chunked() {
        // 无 Content-Length，读取到连接关闭
        let raw_response = b"HTTP/1.1 200 OK\r\n\r\nChunked content";
        let mut stream = BufReader::new(&raw_response[..]);
        let mut timing = TimingContext::new();

        let response = HttpResponse::from_stream(&mut stream, &mut timing)
            .await
            .expect("解析Failed");

        assert_eq!(response.status, 200);
        assert_eq!(response.body_text().unwrap(), "Chunked content");
    }

    #[test]
    fn test_find_header_end() {
        let bytes = b"Header1: Value1\r\nHeader2: Value2\r\n\r\nBody";
        let pos = HttpResponse::find_header_end(bytes);
        // \r\n\r\n 位于索引 32-35，所以返回 36（\r\n\r\n 之后的第一个字节）
        assert_eq!(pos, Some(36));
    }

    #[test]
    fn test_find_header_end_not_found() {
        let bytes = b"Header1: Value1\r\nHeader2: Value2";
        let pos = HttpResponse::find_header_end(bytes);
        assert_eq!(pos, None);
    }

    #[test]
    fn test_body_json() {
        let response = HttpResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: br#"{"key":"value"}"#.to_vec(),
        };

        let json = response.body_json().expect("JSON 解析Failed");
        assert_eq!(json["key"], "value");
    }
}
