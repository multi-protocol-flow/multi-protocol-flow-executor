//! HTTP / SSE / GraphQL 统一测试服务器
//!
//! 为 flow-engine 的 HTTP 节点提供集成测试目标服务。
//! 包含 REST API、SSE 流式推送、GraphQL 端点。
//! 所有依赖均在 Cargo.toml 中声明，无需额外依赖。

use std::convert::Infallible;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::server::conn::http1;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

// ============================================================================
// 数据模型
// ============================================================================

/// Todo 数据项
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    title: String,
    completed: bool,
}

/// 创建 Todo 请求
#[derive(Debug, Deserialize)]
struct CreateTodo {
    title: String,
    #[serde(default)]
    completed: bool,
}

/// 更新 Todo 请求
#[derive(Debug, Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    completed: Option<bool>,
}

/// 应用共享状态
struct AppState {
    todos: Mutex<Vec<Todo>>,
    next_id: AtomicU64,
    start_time: Instant,
}

impl AppState {
    fn new() -> Self {
        // 预置一些测试数据
        let initial_todos = vec![
            Todo { id: 1, title: "学习 HTTP 协议".into(), completed: false },
            Todo { id: 2, title: "测试 REST API".into(), completed: true },
            Todo { id: 3, title: "验证 GraphQL".into(), completed: false },
        ];
        Self {
            todos: Mutex::new(initial_todos),
            next_id: AtomicU64::new(4),
            start_time: Instant::now(),
        }
    }
}

// ============================================================================
// 公共入口
// ============================================================================

/// 启动 HTTP/SSE/GraphQL 测试服务器并阻塞
///
/// 端口可通过 `HTTP_PORT` 环境变量配置，默认 8080。
pub async fn run(port: u16) -> Result<()> {
    let actual_port = env::var("HTTP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(port);

    let state = Arc::new(AppState::new());
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", actual_port))
        .await
        .with_context(|| format!("HTTP: 绑定端口 {} 失败", actual_port))?;

    tracing::info!("HTTP/SSE/GraphQL 服务启动 → 0.0.0.0:{}", actual_port);

    loop {
        let (stream, _addr) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { handle_request(req, &state).await }
            });

            if let Err(e) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(io, service)
                .await
            {
                tracing::debug!("HTTP 连接错误: {}", e);
            }
        });
    }
}

// ============================================================================
// 请求路由
// ============================================================================

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: &Arc<AppState>,
) -> std::result::Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // CORS 预检
    if method == Method::OPTIONS {
        return Ok(cors_response(StatusCode::NO_CONTENT));
    }

    tracing::info!(method = %method, path = %path, "HTTP 请求");

    let response = match (method, path.as_str()) {
        // --- REST API ---
        (Method::GET, "/api/status") => handle_status(state).await,
        (Method::GET, "/api/todos") => handle_list_todos(state).await,
        (Method::GET, path) if path.starts_with("/api/todos/") => {
            handle_get_todo(state, &path["/api/todos/".len()..]).await
        }
        (Method::POST, "/api/todos") => handle_create_todo(state, req).await,
        (Method::PUT, path) if path.starts_with("/api/todos/") => {
            handle_update_todo(state, &path["/api/todos/".len()..], req).await
        }
        (Method::DELETE, path) if path.starts_with("/api/todos/") => {
            handle_delete_todo(state, &path["/api/todos/".len()..]).await
        }
        (Method::POST, "/api/echo") => handle_echo(req).await,
        (Method::GET, path) if path.starts_with("/api/delay/") => {
            handle_delay(&path["/api/delay/".len()..]).await
        }
        (Method::GET, path) if path.starts_with("/api/status/") => {
            handle_status_code(&path["/api/status/".len()..]).await
        }
        (Method::GET, "/api/headers") => handle_headers(req).await,

        // --- SSE ---
        (Method::GET, "/sse") => handle_sse().await,

        // --- GraphQL ---
        (Method::POST, "/graphql") => handle_graphql(state, req).await,
        (Method::GET, "/graphql") => handle_graphql_playground(),

        // --- 404 ---
        _ => error_response(StatusCode::NOT_FOUND, "not found"),
    };

    Ok(add_cors_headers(response))
}

// ============================================================================
// REST API 处理
// ============================================================================

async fn handle_status(state: &Arc<AppState>) -> Response<BoxBody<Bytes, Infallible>> {
    let uptime = state.start_time.elapsed().as_secs();
    json_response(&json!({
        "status": "ok",
        "uptime_secs": uptime,
    }))
}

async fn handle_list_todos(state: &Arc<AppState>) -> Response<BoxBody<Bytes, Infallible>> {
    let todos = state.todos.lock().await;
    json_response(&json!({ "todos": *todos }))
}

async fn handle_get_todo(
    state: &Arc<AppState>,
    id_str: &str,
) -> Response<BoxBody<Bytes, Infallible>> {
    let id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid id"),
    };
    let todos = state.todos.lock().await;
    match todos.iter().find(|t| t.id == id) {
        Some(todo) => json_response(&json!({ "todo": todo })),
        None => error_response(StatusCode::NOT_FOUND, "todo not found"),
    }
}

async fn handle_create_todo(
    state: &Arc<AppState>,
    req: Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, Infallible>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "failed to read body"),
    };

    let create: CreateTodo = match serde_json::from_slice(&body) {
        Ok(c) => c,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid json: {}", e),
            )
        }
    };

    let id = state.next_id.fetch_add(1, Ordering::Relaxed);
    let todo = Todo {
        id,
        title: create.title,
        completed: create.completed,
    };

    state.todos.lock().await.push(todo.clone());
    json_response_with_status(StatusCode::CREATED, &json!({ "todo": todo }))
}

async fn handle_update_todo(
    state: &Arc<AppState>,
    id_str: &str,
    req: Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, Infallible>> {
    let id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid id"),
    };

    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "failed to read body"),
    };

    let update: UpdateTodo = match serde_json::from_slice(&body) {
        Ok(u) => u,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid json: {}", e),
            )
        }
    };

    let mut todos = state.todos.lock().await;
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        if let Some(title) = update.title {
            todo.title = title;
        }
        if let Some(completed) = update.completed {
            todo.completed = completed;
        }
        json_response(&json!({ "todo": *todo }))
    } else {
        error_response(StatusCode::NOT_FOUND, "todo not found")
    }
}

async fn handle_delete_todo(
    state: &Arc<AppState>,
    id_str: &str,
) -> Response<BoxBody<Bytes, Infallible>> {
    let id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid id"),
    };

    let mut todos = state.todos.lock().await;
    let len_before = todos.len();
    todos.retain(|t| t.id != id);

    if todos.len() < len_before {
        json_response(&json!({ "deleted": true, "id": id }))
    } else {
        error_response(StatusCode::NOT_FOUND, "todo not found")
    }
}

async fn handle_echo(
    req: Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, Infallible>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "failed to read body"),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("access-control-allow-origin", "*")
        .body(Full::new(Bytes::from(body)).boxed())
        .unwrap()
}

async fn handle_delay(ms_str: &str) -> Response<BoxBody<Bytes, Infallible>> {
    let ms: u64 = ms_str.parse().unwrap_or(100);
    let ms = ms.min(30000); // 上限 30 秒
    tokio::time::sleep(Duration::from_millis(ms)).await;
    json_response(&json!({ "delayed_ms": ms }))
}

async fn handle_status_code(code_str: &str) -> Response<BoxBody<Bytes, Infallible>> {
    let code: u16 = code_str.parse().unwrap_or(200);
    let code = if (100..600).contains(&code) { code } else { 200 };
    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    json_response_with_status(status, &json!({ "status": code }))
}

async fn handle_headers(
    req: Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, Infallible>> {
    let headers: serde_json::Map<String, serde_json::Value> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), json!(v.to_str().unwrap_or("<binary>"))))
        .collect();
    json_response(&json!({ "headers": headers }))
}

// ============================================================================
// SSE 处理
// ============================================================================

/// 将 tokio mpsc::Receiver 包装为 futures_util::Stream
struct ReceiverStream {
    inner: tokio::sync::mpsc::Receiver<Bytes>,
}

impl ReceiverStream {
    fn new(rx: tokio::sync::mpsc::Receiver<Bytes>) -> Self {
        Self { inner: rx }
    }
}

impl Unpin for ReceiverStream {}

impl futures_util::Stream for ReceiverStream {
    type Item = Bytes;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Bytes>> {
        self.inner.poll_recv(cx)
    }
}

async fn handle_sse() -> Response<BoxBody<Bytes, Infallible>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(32);

    // 启动 SSE 事件生成任务
    tokio::spawn(async move {
        // 欢迎事件
        let _ = tx.send(Bytes::from(sse_event("welcome", &json!({"message": "SSE connected"})))).await;

        // 心跳 + 时间戳 循环
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(5));
        let mut timestamp_interval = tokio::time::interval(Duration::from_secs(2));
        heartbeat_interval.tick().await; // 跳过第一个
        timestamp_interval.tick().await;

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let data = sse_event("heartbeat", &json!({"timestamp": now_ms()}));
                    if tx.send(Bytes::from(data)).await.is_err() {
                        break;
                    }
                }
                _ = timestamp_interval.tick() => {
                    let data = sse_event("timestamp", &json!({
                        "timestamp": now_ms(),
                        "iso": now_iso(),
                    }));
                    if tx.send(Bytes::from(data)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // 将 mpsc Receiver 转为 HTTP Body Stream
    let stream = ReceiverStream::new(rx).map(|b| {
        Ok::<_, Infallible>(Frame::data(b))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .header("access-control-allow-origin", "*")
        .body(BodyExt::boxed(StreamBody::new(stream)))
        .unwrap()
}

fn sse_event(event: &str, data: &serde_json::Value) -> String {
    format!("event: {}\ndata: {}\n\n", event, data)
}

// ============================================================================
// GraphQL 处理 (简易实现，不依赖 async-graphql 运行时)
// ============================================================================

async fn handle_graphql(
    state: &Arc<AppState>,
    req: Request<hyper::body::Incoming>,
) -> Response<BoxBody<Bytes, Infallible>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "failed to read body"),
    };

    let gql_req: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid json: {}", e))
        }
    };

    let query = gql_req
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let variables = gql_req.get("variables").cloned().unwrap_or(json!({}));

    // 简易 GraphQL 解析（仅支持查询字符串匹配）
    let result = resolve_graphql(query, &variables, state).await;
    json_response(&json!({ "data": result }))
}

fn handle_graphql_playground() -> Response<BoxBody<Bytes, Infallible>> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>GraphQL Playground</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphql-playground-middleware-express@1.1.4/static/style.css"/>
</head>
<body>
<div id="root"/>
<script src="https://cdn.jsdelivr.net/npm/graphql-playground-middleware-express@1.1.4/static/bundle.js"></script>
<script>window.addEventListener('load', function(e) { GraphQLPlayground.init(document.getElementById('root'), { endpoint: '/graphql' }) })</script>
</body>
</html>"#;

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .header("access-control-allow-origin", "*")
        .body(Full::new(Bytes::from(html)).boxed())
        .unwrap()
}

/// 简易 GraphQL 解析器 — 通过关键词匹配支持基本查询
async fn resolve_graphql(
    query: &str,
    variables: &serde_json::Value,
    state: &Arc<AppState>,
) -> serde_json::Value {
    let query_lower = query.to_lowercase();

    if query_lower.contains("todos") && !query_lower.contains("todo(") && !query_lower.contains("todo (") {
        let todos = state.todos.lock().await;
        json!({ "todos": *todos })
    } else if query_lower.contains("todo") {
        // 尝试提取 id 参数
        let id = extract_graphql_id(query, variables);
        if let Some(id) = id {
            let todos = state.todos.lock().await;
            match todos.iter().find(|t| t.id == id) {
                Some(todo) => json!({ "todo": *todo }),
                None => json!({ "todo": null }),
            }
        } else {
            let todos = state.todos.lock().await;
            json!({ "todos": *todos })
        }
    } else if query_lower.contains("status") {
        let uptime = state.start_time.elapsed().as_secs();
        json!({ "status": { "ok": true, "uptime_secs": uptime } })
    } else if query_lower.contains("createtodo") || query_lower.contains("create_todo") {
        let title = variables
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled");
        let id = state.next_id.fetch_add(1, Ordering::Relaxed);
        let todo = Todo {
            id,
            title: title.to_string(),
            completed: false,
        };
        state.todos.lock().await.push(todo.clone());
        json!({ "createTodo": todo })
    } else if query_lower.contains("deletetodo") || query_lower.contains("delete_todo") {
        let id = extract_graphql_id(query, variables);
        if let Some(id) = id {
            let mut todos = state.todos.lock().await;
            let before = todos.len();
            todos.retain(|t| t.id != id);
            json!({ "deleteTodo": todos.len() < before })
        } else {
            json!({ "deleteTodo": false })
        }
    } else {
        // 默认返回状态
        let uptime = state.start_time.elapsed().as_secs();
        json!({ "status": { "ok": true, "uptime_secs": uptime } })
    }
}

fn extract_graphql_id(query: &str, variables: &serde_json::Value) -> Option<u64> {
    // 优先从 variables 取
    if let Some(id) = variables.get("id").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    // 尝试从查询字符串提取 id:N 或 id: "N" 模式
    let lower = query.to_lowercase();
    if let Some(pos) = lower.find("id") {
        let rest = &query[pos + 2..];
        let rest = rest.trim_start_matches(|c: char| c == ' ' || c == ':' || c == '=');
        let rest = rest.trim_start_matches('"').trim_start_matches('\'');
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            if let Ok(id) = digits.parse::<u64>() {
                return Some(id);
            }
        }
    }
    None
}

// ============================================================================
// HTTP 辅助
// ============================================================================

async fn read_body(req: Request<hyper::body::Incoming>) -> std::result::Result<Vec<u8>, hyper::Error> {
    let body = req.into_body();
    let bytes = body.collect().await?.to_bytes();
    Ok(bytes.to_vec())
}

fn json_response(data: &serde_json::Value) -> Response<BoxBody<Bytes, Infallible>> {
    json_response_with_status(StatusCode::OK, data)
}

fn json_response_with_status(
    status: StatusCode,
    data: &serde_json::Value,
) -> Response<BoxBody<Bytes, Infallible>> {
    let body = serde_json::to_string(data).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("access-control-allow-origin", "*")
        .body(Full::new(Bytes::from(body)).boxed())
        .unwrap()
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, Infallible>> {
    json_response_with_status(status, &json!({ "error": message }))
}

fn cors_response(status: StatusCode) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(status)
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("access-control-allow-headers", "content-type, authorization, mcp-session-id")
        .header("access-control-max-age", "86400")
        .body(Full::new(Bytes::new()).boxed())
        .unwrap()
}

fn add_cors_headers(mut resp: Response<BoxBody<Bytes, Infallible>>) -> Response<BoxBody<Bytes, Infallible>> {
    resp.headers_mut().insert(
        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );
    resp
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简易 ISO 格式
    format!("{}s since epoch", secs)
}
