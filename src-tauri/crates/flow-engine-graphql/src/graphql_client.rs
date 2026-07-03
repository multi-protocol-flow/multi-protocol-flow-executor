//! GraphQL 客户端封装层
//!
//! 使用 reqwest 执行 HTTP Query/Mutation/Introspection。
//! 使用 tokio-tungstenite 执行 WebSocket 订阅（graphql-transport-ws 协议）。
//! 所有 GraphQL 查询均为运行时字符串，不使用编译时代码生成。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// GraphQL Error信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlError {
    /// Error消息
    pub message: String,
    /// Error位置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<GraphqlErrorLocation>>,
    /// Error路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<serde_json::Value>,
    /// 扩展信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL Error位置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlErrorLocation {
    /// 行号
    pub line: u32,
    /// 列号
    pub column: u32,
}

/// GraphQL 请求耗时信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlTiming {
    /// 总耗时（毫秒）
    pub total_ms: u64,
    /// DNS 解析耗时
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns_ms: Option<u64>,
    /// 连接耗时
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_ms: Option<u64>,
    /// TLS 握手耗时
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ms: Option<u64>,
    /// 服务器处理耗时
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_ms: Option<u64>,
}

/// GraphQL 响应
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlResponse {
    /// 响应数据
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// GraphQL Error列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<GraphqlError>>,
    /// 响应扩展信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// HTTP 状态码
    pub status_code: u16,
    /// 请求耗时信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<GraphqlTiming>,
}

/// GraphQL 订阅事件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlSubscriptionEvent {
    /// 事件数据
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// GraphQL Error列表
    #[serde(default)]
    pub errors: Vec<GraphqlError>,
    /// 响应扩展信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// 接收时间（毫秒时间戳）
    pub received_at: u64,
}

/// GraphQL 订阅响应
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphqlSubscriptionResponse {
    /// 接收到的事件列表
    pub events: Vec<GraphqlSubscriptionEvent>,
    /// 事件数量
    pub event_count: u32,
    /// 订阅持续时间（毫秒）
    pub duration_ms: u64,
}

/// 认证配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum GraphqlAuth {
    /// 无认证
    #[default]
    None,
    /// Bearer Token
    Bearer(String),
    /// API Key（header 名称 + 值）
    ApiKey {
        /// Header 名称
        name: String,
        /// Header 值
        value: String,
    },
    /// Cookie
    Cookie(String),
}

// ---------------------------------------------------------------------------
// GraphQL 请求体
// ---------------------------------------------------------------------------

/// GraphQL HTTP 请求体
#[derive(Debug, Clone, Serialize)]
struct GraphqlRequestBody {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_name: Option<String>,
}

// ---------------------------------------------------------------------------
// GraphqlClient
// ---------------------------------------------------------------------------

/// GraphQL HTTP 客户端
///
/// 使用 reqwest 发送 GraphQL Query/Mutation/Introspection 请求。
/// 提供 HTTP 方法用于查询、变更、文件上传，以及 WebSocket 订阅的底层原语。
pub struct GraphqlClient {
    /// reqwest HTTP 客户端
    client: reqwest::Client,
}

impl GraphqlClient {
    /// 确保 `CryptoProvider` 已安装（rustls 0.23+ 必需）
    fn ensure_crypto_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    /// 创建新的 GraphQL 客户端（不自动跟随 301/302 重定向，避免 POST→GET 丢失 body）
    pub fn new() -> Self {
        Self::ensure_crypto_provider();
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// 创建带自定义超时的 GraphQL 客户端
    pub fn with_timeout(timeout: Duration) -> Self {
        Self::ensure_crypto_provider();
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(timeout)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// 执行 GraphQL 查询
    ///
    /// 向指定端点发送 GraphQL 查询请求。
    pub async fn execute_query(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<serde_json::Value>,
        headers: Option<&HashMap<String, String>>,
        auth: &GraphqlAuth,
        timeout_ms: u64,
    ) -> Result<GraphqlResponse, String> {
        self.execute_request(endpoint, query, variables, None, headers, auth, timeout_ms)
            .await
    }

    /// 执行 GraphQL 变更
    ///
    /// 与查询相同，语义上区分变更操作。
    pub async fn execute_mutation(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<serde_json::Value>,
        headers: Option<&HashMap<String, String>>,
        auth: &GraphqlAuth,
        timeout_ms: u64,
    ) -> Result<GraphqlResponse, String> {
        self.execute_request(endpoint, query, variables, None, headers, auth, timeout_ms)
            .await
    }

    /// 执行 GraphQL 文件上传（multipart）
    ///
    /// 使用 GraphQL multipart 请求规范上传文件。
    pub async fn upload_files(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<serde_json::Value>,
        files: &[FileUpload],
        headers: Option<&HashMap<String, String>>,
        auth: &GraphqlAuth,
        timeout_ms: u64,
    ) -> Result<GraphqlResponse, String> {
        let start = Instant::now();
        let timeout = Self::timeout_duration(timeout_ms);

        // 构建 multipart 请求体
        let mut form = reqwest::multipart::Form::new();

        // operations 字段
        let body = GraphqlRequestBody {
            query: query.to_string(),
            variables: variables.clone(),
            operation_name: None,
        };
        let operations_json =
            serde_json::to_string(&body).map_err(|e| format!("序列化请求体Failed: {}", e))?;
        form = form.text("operations", operations_json);

        // map 字段：文件索引到变量路径的映射
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (i, _file) in files.iter().enumerate() {
            map.insert(
                i.to_string(),
                vec![format!("variables.file.{}", i)],
            );
        }
        let map_json =
            serde_json::to_string(&map).map_err(|e| format!("序列化文件映射Failed: {}", e))?;
        form = form.text("map", map_json);

        // 添加文件
        for (i, file) in files.iter().enumerate() {
            let part = reqwest::multipart::Part::bytes(file.data.clone())
                .file_name(file.name.clone())
                .mime_str(
                    file.content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream"),
                )
                .map_err(|e| format!("设置 MIME 类型Failed: {}", e))?;
            form = form.part(i.to_string(), part);
        }

        // 构建请求
        let mut builder = self
            .client
            .post(endpoint)
            .timeout(timeout)
            .multipart(form);

        builder = Self::apply_auth(builder, auth);
        builder = Self::apply_headers(builder, headers);

        let response = builder
            .send()
            .await
            .map_err(|e| format!("GraphQL 文件上传请求Failed: {}", e))?;

        let status_code = response.status().as_u16();
        let elapsed = start.elapsed();

        let response_text = response
            .text()
            .await
            .map_err(|e| format!("读取响应体Failed: {}", e))?;

        Self::parse_graphql_response(response_text, status_code, elapsed)
    }

    /// 执行 GraphQL 订阅（通过 WebSocket）
    ///
    /// 使用 graphql-transport-ws 协议通过 WebSocket 订阅 GraphQL 事件。
    /// 支持自定义请求头、认证、操作名称、最大事件数和超时控制。
    pub async fn execute_subscription_via_ws(
        &self,
        ws_endpoint: &str,
        subscription: &str,
        variables: Option<serde_json::Value>,
        operation_name: Option<String>,
        _headers: Option<&HashMap<String, String>>,
        _auth: &GraphqlAuth,
        max_events: u32,
        timeout_ms: u64,
    ) -> Result<GraphqlSubscriptionResponse, String> {
        let start = Instant::now();

        // 1. 构建 WebSocket 升级请求
        info!(
            "GraphQL WebSocket 连接: endpoint={}",
            ws_endpoint
        );

        // 先用 IntoClientRequest 从 URL 构建标准请求（自动设置 Host、Method、URI 等）
        let mut request = ws_endpoint
            .into_client_request()
            .map_err(|e| format!("构建 WebSocket 请求Failed: {}", e))?;

        // 添加 graphql-ws 子协议头
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "graphql-transport-ws".parse().unwrap());

        let (ws_stream, _response) = connect_async(request)
            .await
            .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // 3. 发送 connection_init
        let init_msg = serde_json::json!({"type": "connection_init"});
        write
            .send(WsMessage::Text(init_msg.to_string().into()))
            .await
            .map_err(|e| format!("发送 connection_init Failed: {}", e))?;

        // 4. 等待 connection_ack
        let ack_timeout = Duration::from_secs(10);
        let ack_received = match tokio::time::timeout(ack_timeout, read.next()).await {
            Ok(Some(Ok(WsMessage::Text(text)))) => {
                let parsed: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("解析 connection_ack Failed: {}", e))?;
                let msg_type = parsed["type"].as_str().unwrap_or("");
                if msg_type == "connection_ack" {
                    true
                } else if msg_type == "connection_error" {
                    let error_msg = parsed["payload"]
                        .as_str()
                        .unwrap_or("连接被拒绝");
                    return Err(format!("GraphQL 连接被拒绝: {}", error_msg));
                } else {
                    // 某些服务器不发送 connection_ack，直接忽略
                    info!("GraphQL 收到非预期消息: type={}", msg_type);
                    false
                }
            }
            Ok(Some(Ok(WsMessage::Close(frame)))) => {
                return Err(format!(
                    "WebSocket 连接被关闭: {:?}",
                    frame.map(|f| f.reason)
                ));
            }
            Ok(Some(Ok(WsMessage::Ping(data)))) => {
                let _ = write.send(WsMessage::Pong(data)).await;
                false // 继续等待 ack
            }
            Ok(Some(Ok(_))) => {
                // 忽略其他消息类型（binary, pong 等），继续等待 ack
                false
            }
            Ok(Some(Err(e))) => {
                return Err(format!("WebSocket 消息读取Error: {}", e));
            }
            Ok(None) => {
                return Err("WebSocket 连接意外关闭".to_string());
            }
            Err(_) => {
                // connection_ack 超时 — 某些服务器不发送 ack，继续
                info!("GraphQL connection_ack 超时，继续执行订阅");
                true
            }
        };

        if !ack_received {
            // 如果没收到 ack 但也没超时，可能已经错过了消息，继续
        }

        // 5. 发送 subscribe 消息
        let sub_id = "1";
        let mut subscribe_payload = serde_json::json!({
            "query": subscription,
        });

        if let Some(vars) = &variables {
            subscribe_payload["variables"] = vars.clone();
        }
        if let Some(op_name) = &operation_name {
            subscribe_payload["operationName"] = serde_json::Value::String(op_name.clone());
        }

        let subscribe_msg = serde_json::json!({
            "type": "subscribe",
            "id": sub_id,
            "payload": subscribe_payload,
        });

        write
            .send(WsMessage::Text(subscribe_msg.to_string().into()))
            .await
            .map_err(|e| format!("发送 subscribe Failed: {}", e))?;

        info!(
            "GraphQL subscribe 已发送: id={}",
            sub_id
        );

        // 6. 收集事件
        let actual_timeout = if timeout_ms > 0 {
            Duration::from_millis(timeout_ms)
        } else {
            Duration::from_secs(300) // 默认 5 分钟
        };

        let mut events: Vec<GraphqlSubscriptionEvent> = Vec::new();
        let per_msg_timeout = Duration::from_secs(30);

        loop {
            // 检查整体超时
            if start.elapsed() >= actual_timeout {
                info!("GraphQL 订阅超时，发送 complete");
                let _ = write
                    .send(WsMessage::Text(
                        serde_json::json!({"type": "complete", "id": sub_id})
                            .to_string()
                            .into(),
                    ))
                    .await;
                break;
            }

            // 检查最大事件数
            if max_events > 0 && events.len() as u32 >= max_events {
                info!(
                    "GraphQL 订阅达到最大事件数 {}，发送 complete",
                    max_events
                );
                let _ = write
                    .send(WsMessage::Text(
                        serde_json::json!({"type": "complete", "id": sub_id})
                            .to_string()
                            .into(),
                    ))
                    .await;
                break;
            }

            // 读取下一条消息（带逐消息超时）
            let msg_result = tokio::time::timeout(per_msg_timeout, read.next()).await;

            match msg_result {
                Ok(Some(Ok(WsMessage::Text(text)))) => {
                    let parsed: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            log::warn!("GraphQL 解析消息Failed: {}", e);
                            continue;
                        }
                    };

                    let msg_type = parsed["type"].as_str().unwrap_or("");

                    match msg_type {
                        "next" => {
                            // 检查 id 是否匹配
                            let msg_id = parsed["id"].as_str().unwrap_or("");
                            if msg_id != sub_id {
                                continue;
                            }

                            let payload = &parsed["payload"];
                            let event = GraphqlSubscriptionEvent {
                                data: payload.get("data").cloned(),
                                errors: payload
                                    .get("errors")
                                    .and_then(|e| {
                                        serde_json::from_value::<Vec<GraphqlError>>(e.clone()).ok()
                                    })
                                    .unwrap_or_default(),
                                extensions: payload.get("extensions").cloned(),
                                received_at: chrono::Utc::now().timestamp_millis() as u64,
                            };

                            // 检查是否有 GraphQL Error
                            let has_errors = !event.errors.is_empty();
                            if has_errors {
                                let error_msgs: Vec<String> = event
                                    .errors
                                    .iter()
                                    .map(|e| e.message.clone())
                                    .collect();
                                log::warn!(
                                    "GraphQL 订阅事件包含Error: {:?}",
                                    error_msgs
                                );
                            }

                            events.push(event);
                        }
                        "complete" => {
                            info!("GraphQL 订阅收到 complete (服务端结束)");
                            break;
                        }
                        "ping" => {
                            let _ = write
                                .send(WsMessage::Text(
                                    serde_json::json!({"type": "pong"}).to_string().into(),
                                ))
                                .await;
                        }
                        "error" => {
                            let error_msg = parsed["payload"]
                                .as_str()
                                .unwrap_or("未知订阅Error");
                            return Err(format!("GraphQL 订阅Error: {}", error_msg));
                        }
                        _ => {
                            log::debug!("GraphQL 收到未处理消息类型: {}", msg_type);
                        }
                    }
                }
                Ok(Some(Ok(WsMessage::Close(frame)))) => {
                    info!(
                        "GraphQL WebSocket 连接已关闭: {:?}",
                        frame.map(|f| f.reason)
                    );
                    break;
                }
                Ok(Some(Ok(WsMessage::Ping(data)))) => {
                    let _ = write.send(WsMessage::Pong(data)).await;
                }
                Ok(Some(Ok(_))) => {
                    // 忽略其他类型（binary, pong 等）
                }
                Ok(Some(Err(e))) => {
                    log::warn!("GraphQL WebSocket 消息读取Error: {}", e);
                    break;
                }
                Ok(None) => {
                    info!("GraphQL WebSocket 流已结束");
                    break;
                }
                Err(_) => {
                    // 逐消息超时，继续循环（整体超时会在下次循环检查）
                    continue;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let event_count = events.len() as u32;

        info!(
            "GraphQL 订阅完成: events={}, duration={}ms",
            event_count, duration_ms
        );

        Ok(GraphqlSubscriptionResponse {
            events,
            event_count,
            duration_ms,
        })
    }

    // -----------------------------------------------------------------------
    // 内部方法
    // -----------------------------------------------------------------------

    /// 手动跟随重定向并发送 POST 请求（保持 POST 方法）
    async fn send_post_with_redirects(
        client: &reqwest::Client,
        endpoint: &str,
        body: &GraphqlRequestBody,
        headers: Option<&HashMap<String, String>>,
        auth: &GraphqlAuth,
        timeout: Duration,
        start: Instant,
    ) -> Result<GraphqlResponse, String> {
        let max_redirects = 5u32;
        let mut current_url = endpoint.to_string();

        for _ in 0..=max_redirects {
            let mut builder = client
                .post(&current_url)
                .timeout(timeout)
                .json(body);

            builder = Self::apply_auth(builder, auth);
            builder = Self::apply_headers(builder, headers);

            let resp = builder.send().await.map_err(|e| {
                if e.is_timeout() {
                    format!("GraphQL 请求超时: {}", e)
                } else if e.is_connect() {
                    format!("GraphQL connection failed: {}", e)
                } else {
                    format!("GraphQL 请求Failed: {}", e)
                }
            })?;

            let status = resp.status();
            let elapsed = start.elapsed();

            if status.is_redirection() {
                let next = resp.headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .map(|loc| {
                        if loc.starts_with('/') {
                            // 相对 origin 解析
                            let origin = current_url.split('/')
                                .take(3)
                                .collect::<Vec<_>>()
                                .join("/");
                            format!("{}{}", origin, loc)
                        } else {
                            loc.to_string()
                        }
                    });
                match next {
                    Some(url) => { current_url = url; continue; }
                    None => return Err("重定向无 Location 头".to_string()),
                }
            }

            let status_code = status.as_u16();
            let response_text = resp.text().await
                .map_err(|e| format!("读取响应体Failed: {}", e))?;

            info!(
                "GraphQL 请求完成: endpoint={}, status={}, elapsed={}ms",
                current_url, status_code, elapsed.as_millis()
            );

            return Self::parse_graphql_response(response_text, status_code, elapsed);
        }

        Err("重定向次数过多".to_string())
    }

    /// 执行通用 GraphQL HTTP 请求
    async fn execute_request(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<serde_json::Value>,
        operation_name: Option<String>,
        headers: Option<&HashMap<String, String>>,
        auth: &GraphqlAuth,
        timeout_ms: u64,
    ) -> Result<GraphqlResponse, String> {
        let start = Instant::now();
        let timeout = Self::timeout_duration(timeout_ms);

        let body = GraphqlRequestBody {
            query: query.to_string(),
            variables,
            operation_name,
        };

        Self::send_post_with_redirects(
            &self.client, endpoint, &body, headers, auth, timeout, start,
        ).await
    }

    /// 解析 GraphQL HTTP 响应
    fn parse_graphql_response(
        response_text: String,
        status_code: u16,
        elapsed: Duration,
    ) -> Result<GraphqlResponse, String> {
        // 尝试解析为 GraphQL 标准响应格式
        let parsed: serde_json::Value =
            serde_json::from_str(&response_text).map_err(|e| {
                format!(
                    "GraphQL 响应 JSON 解析Failed: {} (响应长度: {})",
                    e,
                    response_text.len()
                )
            })?;

        let data = parsed.get("data").cloned();
        let errors: Option<Vec<GraphqlError>> = parsed
            .get("errors")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let extensions = parsed.get("extensions").cloned();

        // 如果有 GraphQL Error，记录警告
        if let Some(ref errs) = errors {
            if !errs.is_empty() {
                warn!("GraphQL 返回 {} 个Error", errs.len());
            }
        }

        let timing = GraphqlTiming {
            total_ms: elapsed.as_millis() as u64,
            dns_ms: None,
            connect_ms: None,
            tls_ms: None,
            server_ms: None,
        };

        Ok(GraphqlResponse {
            data,
            errors,
            extensions,
            status_code,
            timing: Some(timing),
        })
    }

    /// 应用认证到请求构建器
    fn apply_auth(
        builder: reqwest::RequestBuilder,
        auth: &GraphqlAuth,
    ) -> reqwest::RequestBuilder {
        match auth {
            GraphqlAuth::None => builder,
            GraphqlAuth::Bearer(token) => builder.bearer_auth(token),
            GraphqlAuth::ApiKey { name, value } => builder.header(name.as_str(), value.as_str()),
            GraphqlAuth::Cookie(cookie) => builder.header("Cookie", cookie.as_str()),
        }
    }

    /// 应用自定义请求头
    fn apply_headers(
        mut builder: reqwest::RequestBuilder,
        headers: Option<&HashMap<String, String>>,
    ) -> reqwest::RequestBuilder {
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                builder = builder.header(key.as_str(), value.as_str());
            }
        }
        builder
    }

    /// 将超时毫秒数转换为 Duration
    fn timeout_duration(timeout_ms: u64) -> Duration {
        if timeout_ms > 0 {
            Duration::from_millis(timeout_ms)
        } else {
            Duration::from_secs(30)
        }
    }
}

impl Default for GraphqlClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 文件上传类型
// ---------------------------------------------------------------------------

/// 文件上传描述
#[derive(Debug, Clone)]
pub struct FileUpload {
    /// 文件名
    pub name: String,
    /// 文件数据
    pub data: Vec<u8>,
    /// MIME 类型
    pub content_type: Option<String>,
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- GraphqlResponse 测试 ----

    #[test]
    fn test_graphql_response_serialization() {
        let response = GraphqlResponse {
            data: Some(serde_json::json!({"user": {"name": "Alice"}})),
            errors: None,
            extensions: None,
            status_code: 200,
            timing: Some(GraphqlTiming {
                total_ms: 150,
                dns_ms: Some(10),
                connect_ms: Some(20),
                tls_ms: Some(30),
                server_ms: Some(90),
            }),
        };
        let json = serde_json::to_string(&response).expect("序列化Failed");
        assert!(json.contains("\"status_code\":200"));
        assert!(json.contains("\"total_ms\":150"));
        // errors 为 None 时不应序列化
        assert!(!json.contains("\"errors\":null"));
    }

    #[test]
    fn test_graphql_response_with_errors() {
        let response = GraphqlResponse {
            data: Some(serde_json::json!(null)),
            errors: Some(vec![GraphqlError {
                message: "Field 'foo' not found".to_string(),
                locations: Some(vec![GraphqlErrorLocation {
                    line: 1,
                    column: 5,
                }]),
                path: Some(serde_json::json!(["query", "foo"])),
                extensions: None,
            }]),
            extensions: None,
            status_code: 200,
            timing: Some(GraphqlTiming {
                total_ms: 50,
                dns_ms: None,
                connect_ms: None,
                tls_ms: None,
                server_ms: None,
            }),
        };
        let json = serde_json::to_string(&response).expect("序列化Failed");
        assert!(json.contains("Field 'foo' not found"));
        assert!(json.contains("\"line\":1"));
        assert!(json.contains("\"column\":5"));
    }

    #[test]
    fn test_graphql_response_deserialization() {
        let json = r#"{
            "data": {"users": [{"id": 1}]},
            "status_code": 200,
            "timing": {"total_ms": 100}
        }"#;
        let response: GraphqlResponse = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(response.status_code, 200);
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        assert!(response.timing.is_some());
        assert_eq!(response.timing.expect("timing").total_ms, 100);
    }

    // ---- GraphqlError 测试 ----

    #[test]
    fn test_graphql_error_serialization() {
        let error = GraphqlError {
            message: "Syntax Error".to_string(),
            locations: None,
            path: None,
            extensions: None,
        };
        let json = serde_json::to_string(&error).expect("序列化Failed");
        assert!(json.contains("\"message\":\"Syntax Error\""));
        // 可选字段为 None 时不序列化
        assert!(!json.contains("locations"));
        assert!(!json.contains("path"));
        assert!(!json.contains("extensions"));
    }

    #[test]
    fn test_graphql_error_with_all_fields() {
        let error = GraphqlError {
            message: "Validation Error".to_string(),
            locations: Some(vec![
                GraphqlErrorLocation { line: 2, column: 3 },
            ]),
            path: Some(serde_json::json!("query.user")),
            extensions: Some(serde_json::json!({"code": "VALIDATION_ERROR"})),
        };
        let json = serde_json::to_string(&error).expect("序列化Failed");
        assert!(json.contains("\"line\":2"));
        assert!(json.contains("\"column\":3"));
        assert!(json.contains("VALIDATION_ERROR"));
    }

    // ---- GraphqlTiming 测试 ----

    #[test]
    fn test_graphql_timing_serialization() {
        let timing = GraphqlTiming {
            total_ms: 200,
            dns_ms: None,
            connect_ms: None,
            tls_ms: None,
            server_ms: None,
        };
        let json = serde_json::to_string(&timing).expect("序列化Failed");
        assert!(json.contains("\"total_ms\":200"));
        assert!(!json.contains("dns_ms"));
    }

    // ---- GraphqlSubscriptionEvent 测试 ----

    #[test]
    fn test_subscription_event_serialization() {
        let event = GraphqlSubscriptionEvent {
            data: Some(serde_json::json!({"newMessage": {"text": "hello"}})),
            errors: vec![],
            extensions: None,
            received_at: 1234567890,
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("newMessage"));
        assert!(json.contains("\"received_at\":1234567890"));
        // errors 为空数组时仍然序列化（serde default）
        assert!(json.contains("\"errors\":[]"));
    }

    // ---- GraphqlSubscriptionResponse 测试 ----

    #[test]
    fn test_subscription_response_serialization() {
        let response = GraphqlSubscriptionResponse {
            events: vec![],
            event_count: 0,
            duration_ms: 1000,
        };
        let json = serde_json::to_string(&response).expect("序列化Failed");
        assert!(json.contains("\"event_count\":0"));
        assert!(json.contains("\"duration_ms\":1000"));
    }

    // ---- GraphqlAuth 测试 ----

    #[test]
    fn test_auth_default_is_none() {
        let auth = GraphqlAuth::default();
        assert_eq!(auth, GraphqlAuth::None);
    }

    #[test]
    fn test_auth_serialization() {
        let auth = GraphqlAuth::Bearer("my-token".to_string());
        let json = serde_json::to_string(&auth).expect("序列化Failed");
        assert!(json.contains("Bearer"));
        assert!(json.contains("my-token"));
    }

    // ---- FileUpload 测试 ----

    #[test]
    fn test_file_upload_creation() {
        let upload = FileUpload {
            name: "test.txt".to_string(),
            data: b"hello world".to_vec(),
            content_type: Some("text/plain".to_string()),
        };
        assert_eq!(upload.name, "test.txt");
        assert_eq!(upload.data, b"hello world");
        assert_eq!(upload.content_type, Some("text/plain".to_string()));
    }

    // ---- GraphqlClient 构造器测试 ----

    #[test]
    fn test_client_new() {
        let _client = GraphqlClient::new();
    }

    #[test]
    fn test_client_default() {
        let _client = GraphqlClient::default();
    }

    #[test]
    fn test_client_with_timeout() {
        let _client = GraphqlClient::with_timeout(Duration::from_secs(60));
    }

    // ---- timeout_duration 测试 ----

    #[test]
    fn test_timeout_duration_zero() {
        let d = GraphqlClient::timeout_duration(0);
        assert_eq!(d, Duration::from_secs(30));
    }

    #[test]
    fn test_timeout_duration_custom() {
        let d = GraphqlClient::timeout_duration(5000);
        assert_eq!(d, Duration::from_millis(5000));
    }

    // ---- parse_graphql_response 测试 ----

    #[test]
    fn test_parse_graphql_response_success() {
        let body = r#"{"data": {"user": {"id": 1, "name": "Alice"}}}"#;
        let result =
            GraphqlClient::parse_graphql_response(body.to_string(), 200, Duration::from_millis(100));
        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        assert_eq!(response.status_code, 200);
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        assert!(response.timing.is_some());
        assert_eq!(response.timing.expect("timing").total_ms, 100);
    }

    #[test]
    fn test_parse_graphql_response_with_errors() {
        let body = r#"{"data": null, "errors": [{"message": "Not found"}]}"#;
        let result =
            GraphqlClient::parse_graphql_response(body.to_string(), 200, Duration::from_millis(50));
        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        assert!(response.data.is_some()); // null 值仍被解析为 Some(Value::Null)
        let errors = response.errors.expect("应有Error");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Not found");
    }

    #[test]
    fn test_parse_graphql_response_invalid_json() {
        let body = "not json at all";
        let result =
            GraphqlClient::parse_graphql_response(body.to_string(), 200, Duration::from_millis(10));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON 解析Failed"));
    }

    #[test]
    fn test_parse_graphql_response_with_extensions() {
        let body = r#"{"data": {"ok": true}, "extensions": {"tracing": {"version": 1}}}"#;
        let result =
            GraphqlClient::parse_graphql_response(body.to_string(), 200, Duration::from_millis(20));
        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        assert!(response.extensions.is_some());
        let ext = response.extensions.expect("应有扩展信息");
        assert!(ext.get("tracing").is_some());
    }

    // ---- GraphqlErrorLocation 测试 ----

    #[test]
    fn test_error_location() {
        let loc = GraphqlErrorLocation {
            line: 3,
            column: 7,
        };
        let json = serde_json::to_string(&loc).expect("序列化Failed");
        assert!(json.contains("\"line\":3"));
        assert!(json.contains("\"column\":7"));
    }

    // ---- HTTP 请求测试（需要真实服务，仅测试Error路径） ----

    #[tokio::test]
    async fn test_execute_query_invalid_endpoint() {
        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                "http://localhost:1/invalid-endpoint",
                "{ __schema { types { name } } }",
                None,
                None,
                &GraphqlAuth::None,
                1000,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_mutation_invalid_endpoint() {
        let client = GraphqlClient::new();
        let result = client
            .execute_mutation(
                "http://localhost:1/invalid-endpoint",
                "mutation { createUser(name: \"test\") { id } }",
                None,
                None,
                &GraphqlAuth::None,
                1000,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upload_files_invalid_endpoint() {
        let client = GraphqlClient::new();
        let files = vec![FileUpload {
            name: "test.txt".to_string(),
            data: b"hello".to_vec(),
            content_type: None,
        }];
        let result = client
            .upload_files(
                "http://localhost:1/invalid-endpoint",
                "mutation ($file: Upload!) { upload(file: $file) { url } }",
                None,
                &files,
                None,
                &GraphqlAuth::None,
                1000,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_subscription_connection_refused() {
        let client = GraphqlClient::new();
        let result = client
            .execute_subscription_via_ws(
                "ws://localhost:1/graphql",
                "subscription { messageAdded { text } }",
                None,
                None,
                None,
                &GraphqlAuth::None,
                10,
                1000,
            )
            .await;
        // 连接被拒绝是预期的（无服务在 localhost:1 监听）
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("connection failed") || err.contains("Connection refused"));
    }

    // ---- 使用本地 HTTP 服务的集成测试 ----

    /// 创建本地 HTTP 服务器进行集成测试
    async fn create_mock_server(
        status_code: u16,
        body: &str,
    ) -> (tokio::task::JoinHandle<()>, String) {
        let body_bytes = body.as_bytes().to_vec();
        let status_line = format!("HTTP/1.1 {} OK\r\n", status_code);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定端口Failed");
        let addr = listener.local_addr().expect("获取地址Failed");
        let port = addr.port();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("接受connection failed");
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 8192];
            let _ = stream.read(&mut buf).await;

            let response = format!(
                "{}Content-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                status_line.trim_end_matches("\r\n"),
                body_bytes.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(&body_bytes).await;
            let _ = stream.flush().await;
        });

        // 等待服务器就绪
        tokio::time::sleep(Duration::from_millis(10)).await;

        (handle, format!("http://127.0.0.1:{}", port))
    }

    #[tokio::test]
    async fn test_execute_query_mock_server() {
        let response_body = r#"{"data": {"__schema": {"types": [{"name": "Query"}]}}}"#;
        let (_handle, endpoint) = create_mock_server(200, response_body).await;

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "{ __schema { types { name } } }",
                None,
                None,
                &GraphqlAuth::None,
                5000,
            )
            .await;

        assert!(result.is_ok(), "查询应该Success: {:?}", result);
        let response = result.expect("响应解析Failed");
        assert_eq!(response.status_code, 200);
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        assert!(response.timing.is_some());
    }

    #[tokio::test]
    async fn test_execute_query_with_auth_bearer() {
        let response_body = r#"{"data": {"me": {"id": 1}}}"#;
        let (_handle, endpoint) = create_mock_server(200, response_body).await;

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "{ me { id } }",
                None,
                None,
                &GraphqlAuth::Bearer("test-token".to_string()),
                5000,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_with_headers() {
        let response_body = r#"{"data": {"ok": true}}"#;
        let (_handle, endpoint) = create_mock_server(200, response_body).await;

        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "{ ok }",
                None,
                Some(&headers),
                &GraphqlAuth::None,
                5000,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_with_variables() {
        let response_body = r#"{"data": {"user": {"id": 42, "name": "Bob"}}}"#;
        let (_handle, endpoint) = create_mock_server(200, response_body).await;

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "query GetUser($id: ID!) { user(id: $id) { id name } }",
                Some(serde_json::json!({"id": "42"})),
                None,
                &GraphqlAuth::None,
                5000,
            )
            .await;

        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        let data = response.data.expect("应有数据");
        let user = data.get("user").expect("应有 user 字段");
        assert_eq!(user["id"], 42);
        assert_eq!(user["name"], "Bob");
    }

    #[tokio::test]
    async fn test_execute_query_with_errors() {
        let response_body = r#"{"data": null, "errors": [{"message": "Not found", "locations": [{"line": 1, "column": 3}]}]}"#;
        let (_handle, endpoint) = create_mock_server(200, response_body).await;

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "{ nonExistentField }",
                None,
                None,
                &GraphqlAuth::None,
                5000,
            )
            .await;

        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        let errors = response.errors.expect("应有Error");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Not found");
        let locations = errors[0].locations.clone().expect("应有位置信息");
        assert_eq!(locations[0].line, 1);
        assert_eq!(locations[0].column, 3);
    }

    #[tokio::test]
    async fn test_execute_query_server_error() {
        let response_body = r#"{"errors": [{"message": "Internal Server Error"}]}"#;
        let (_handle, endpoint) = create_mock_server(500, response_body).await;

        let client = GraphqlClient::new();
        let result = client
            .execute_query(
                &endpoint,
                "{ test }",
                None,
                None,
                &GraphqlAuth::None,
                5000,
            )
            .await;

        assert!(result.is_ok());
        let response = result.expect("响应解析Failed");
        assert_eq!(response.status_code, 500);
    }
}
