//! SSE 客户端封装层
//!
//! 使用 reqwest 流式 GET 请求实现 SSE (Server-Sent Events) 协议客户端。
//! 不依赖外部 SSE 库，直接解析 SSE 协议文本格式。
//!
//! # SSE 协议格式
//!
//! ```text
//! event: message
//! data: {"foo": "bar"}
//! id: 123
//! retry: 5000
//!
//! ```
//!
//! - `data:` 行可多行（以 `\n` 连接）
//! - `event:` 事件类型（默认 "message"）
//! - `id:` 事件 ID（用于 Last-Event-ID 重连）
//! - `retry:` 重连间隔建议（毫秒）
//! - 空行 = 事件结束
//! - `:` 前缀 = 注释（忽略）

use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use log::{info, warn};
use reqwest::Response;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// 解析后的 SSE 事件
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SseEvent {
    /// 事件类型（默认 "message"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// 事件 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 事件数据（多行 data 以 `\n` 连接）
    pub data: String,
    /// 服务器建议的重连间隔（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<u64>,
}

impl SseEvent {
    /// 默认事件类型
    pub const DEFAULT_EVENT_TYPE: &'static str = "message";
}

/// SSE 客户端连接配置
#[derive(Debug, Clone)]
pub struct SseClientConfig {
    /// SSE 端点 URL
    pub url: String,
    /// 自定义请求头
    pub headers: HashMap<String, String>,
    /// Last-Event-ID（断线续传）
    pub last_event_id: Option<String>,
    /// 连接超时
    pub timeout: Duration,
}

// ---------------------------------------------------------------------------
// SSE 客户端
// ---------------------------------------------------------------------------

/// SSE 客户端
///
/// 通过 reqwest 发送 GET 请求，读取流式响应体，解析 SSE 协议。
/// 提供连接方法返回事件流，支持取消和重连。
pub struct SseClient;

impl SseClient {
    /// 发送 SSE 连接请求，返回 reqwest Response
    ///
    /// 发送带有 `Accept: text/event-stream` 头的 GET 请求。
    /// 如果指定了 `Last-Event-ID`，会包含在请求头中用于断线续传。
    pub async fn connect(config: &SseClientConfig) -> Result<Response, String> {
        let mut builder = reqwest::Client::new()
            .get(&config.url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .timeout(config.timeout);

        // 添加自定义请求头
        for (key, value) in &config.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        // 添加 Last-Event-ID 用于断线续传
        if let Some(ref last_id) = config.last_event_id {
            builder = builder.header("Last-Event-ID", last_id.as_str());
        }

        let response = builder
            .send()
            .await
            .map_err(|e| format!("SSE 连接请求Failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("SSE connection failed: HTTP {}", status.as_u16()));
        }

        info!("SSE 连接Success: {}", config.url);
        Ok(response)
    }

    /// 从 Response 中解析 SSE 事件流
    ///
    /// 逐字节读取响应体，按 SSE 协议解析事件，
    /// 通过 broadcast channel 分发给多个消费者。
    pub fn spawn_event_reader(
        response: Response,
        cancel_token: tokio_util::sync::CancellationToken,
        event_sender: broadcast::Sender<SseEvent>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = Self::read_sse_stream(response, &cancel_token, &event_sender).await {
                if !cancel_token.is_cancelled() {
                    warn!("SSE 流读取Error: {}", e);
                }
            }
        })
    }

    /// 读取并解析 SSE 流
    ///
    /// 从 byte stream 中逐 chunk 读取，缓冲后按行分割，
    /// 根据 SSE 协议规则解析事件字段。
    async fn read_sse_stream(
        response: Response,
        cancel_token: &tokio_util::sync::CancellationToken,
        event_sender: &broadcast::Sender<SseEvent>,
    ) -> Result<(), String> {
        let mut stream = response.bytes_stream();

        // 行缓冲区（SSE 按行解析）
        let mut line_buffer = String::new();
        // 当前事件累积器
        let mut current_event_type: Option<String> = None;
        let mut current_id: Option<String> = None;
        let mut current_data_lines: Vec<String> = Vec::new();
        let mut current_retry: Option<u64> = None;

        while let Some(chunk_result) = stream.next().await {
            // 检查取消
            if cancel_token.is_cancelled() {
                info!("SSE 流读取被取消");
                return Ok(());
            }

            let chunk = chunk_result.map_err(|e| format!("SSE 流读取Failed: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            line_buffer.push_str(&text);

            // 逐行处理
            while let Some(newline_pos) = line_buffer.find('\n') {
                let line = line_buffer[..newline_pos]
                    .trim_end_matches('\r')
                    .to_string();
                line_buffer = line_buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    // 空行 = 事件结束，发送累积的事件
                    if !current_data_lines.is_empty() {
                        let event = SseEvent {
                            event: current_event_type.take(),
                            id: current_id.take(),
                            data: current_data_lines.join("\n"),
                            retry: current_retry.take(),
                        };

                        // 通过 broadcast 发送事件，无接收者时静默丢弃
                        let _ = event_sender.send(event);

                        current_data_lines.clear();
                    }
                    // 空行但没有数据 = 心跳/keep-alive，忽略
                    continue;
                }

                // 注释行（以 `:` 开头），忽略
                if line.starts_with(':') {
                    continue;
                }

                // 解析字段行
                if let Some(colon_pos) = line.find(':') {
                    let field = &line[..colon_pos];
                    let mut value = &line[colon_pos + 1..];
                    // 如果值以空格开头，去掉前导空格（SSE 规范）
                    if value.starts_with(' ') {
                        value = &value[1..];
                    }

                    match field {
                        "event" => {
                            current_event_type = Some(value.to_string());
                        }
                        "data" => {
                            current_data_lines.push(value.to_string());
                        }
                        "id" => {
                            // id 不能包含空字节
                            if !value.contains('\0') {
                                current_id = Some(value.to_string());
                            }
                        }
                        "retry" => {
                            if let Ok(ms) = value.parse::<u64>() {
                                current_retry = Some(ms);
                            }
                        }
                        _ => {
                            // 未知字段，忽略（SSE 规范要求）
                        }
                    }
                }
                // 没有冒号的行在 SSE 规范中被忽略
            }
        }

        // 流结束，发送最后一个未完成的事件（如果有）
        if !current_data_lines.is_empty() {
            let event = SseEvent {
                event: current_event_type,
                id: current_id,
                data: current_data_lines.join("\n"),
                retry: current_retry,
            };
            let _ = event_sender.send(event);
        }

        info!("SSE 流结束");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SseEvent 测试 ----

    #[test]
    fn test_sse_event_default_type() {
        assert_eq!(SseEvent::DEFAULT_EVENT_TYPE, "message");
    }

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent {
            event: Some("update".to_string()),
            id: Some("42".to_string()),
            data: r#"{"status":"ok"}"#.to_string(),
            retry: None,
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("\"event\":\"update\""));
        assert!(json.contains("\"id\":\"42\""));
        assert!(json.contains(r#""data":"{\"status\":\"ok\"}"#));
        // retry 为 None 时不应序列化
        assert!(!json.contains("retry"));
    }

    #[test]
    fn test_sse_event_deserialization() {
        let json = r#"{"event":"message","data":"hello","id":"1"}"#;
        let event: SseEvent = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(event.event, Some("message".to_string()));
        assert_eq!(event.data, "hello");
        assert_eq!(event.id, Some("1".to_string()));
        assert_eq!(event.retry, None);
    }

    // ---- 行解析逻辑测试（通过模拟字节流） ----

    /// 辅助：从 broadcast channel 收集所有事件（直到流结束或超时）
    async fn collect_events(
        mut rx: broadcast::Receiver<SseEvent>,
        timeout_ms: u64,
    ) -> Vec<SseEvent> {
        let timeout = if timeout_ms > 0 {
            Duration::from_millis(timeout_ms)
        } else {
            Duration::from_secs(5)
        };
        let mut events = Vec::new();
        loop {
            match tokio::time::timeout(timeout, rx.recv()).await {
                Ok(Ok(event)) => events.push(event),
                _ => break,
            }
        }
        events
    }

    #[tokio::test]
    async fn test_sse_stream_parse_single_event() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "event: message\ndata: hello world\nid: 1\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);

        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.event, Some("message".to_string()));
        assert_eq!(event.data, "hello world");
        assert_eq!(event.id, Some("1".to_string()));
    }

    #[tokio::test]
    async fn test_sse_stream_parse_multiline_data() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "data: line1\ndata: line2\ndata: line3\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.data, "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_sse_stream_parse_multiple_events() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "data: first\n\ndata: second\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert_eq!(events.len(), 2, "应该收到两个事件");
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[tokio::test]
    async fn test_sse_stream_parse_retry_field() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "retry: 5000\ndata: test\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.retry, Some(5000));
        assert_eq!(event.data, "test");
    }

    #[tokio::test]
    async fn test_sse_stream_ignore_comments() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = ": this is a comment\ndata: actual data\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.data, "actual data");
    }

    #[tokio::test]
    async fn test_sse_stream_cancellation() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "data: before_cancel\n\n";
        let response = mock_response(sse_text).await;

        cancel.cancel();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        // 取消后主要验证不会 panic 或死锁
    }

    #[tokio::test]
    async fn test_sse_stream_ignore_unknown_fields() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sse_text = "unknown: field\ndata: test\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.data, "test");
    }

    #[tokio::test]
    async fn test_sse_stream_id_no_null_byte() {
        let (tx, _) = broadcast::channel::<SseEvent>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        // id 包含空字节，应被忽略
        let sse_text = "id: bad\x00id\ndata: test\n\n";
        let response = mock_response(sse_text).await;

        let rx = tx.subscribe();
        let handle = SseClient::spawn_event_reader(response, cancel.clone(), tx);
        let _ = handle.await;

        let events = collect_events(rx, 1000).await;
        assert!(!events.is_empty(), "应该收到至少一个事件");
        let event = &events[0];
        assert_eq!(event.id, None);
        assert_eq!(event.data, "test");
    }

    // ---- 辅助函数：构造 mock Response ----

    /// 使用 wiremock 或 mockito 不太方便，这里用 reqwest Response 构造
    /// 由于无法直接构造 `reqwest::Response，使用` tokio channel 模拟
    ///
    /// 实际上 reqwest 的 `bytes_stream` 来自 Response，我们需要一个真正的 HTTP 服务
    /// 或者使用 mock。但为了避免复杂依赖，这里使用一个简单的内部 HTTP 服务器。
    async fn mock_response(body: &str) -> Response {
        // 初始化 rustls crypto provider（在测试中 reqwest 需要）
        let _ = rustls::crypto::ring::default_provider().install_default();

        // 使用 tokio::spawn 启动一个临时 HTTP 服务器
        let body_bytes = body.as_bytes().to_vec();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口Failed");
        let addr = listener.local_addr().expect("获取地址Failed");

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("接受connection failed");
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await;

            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/event-stream\r\n\
                 Connection: close\r\n\
                 Content-Length: {}\r\n\r\n",
                body_bytes.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(&body_bytes).await;
            let _ = stream.flush().await;
        });

        // 等待服务器就绪
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        reqwest::Client::new()
            .get(format!("http://{}", addr))
            .send()
            .await
            .expect("mock 请求Failed")
    }
}
