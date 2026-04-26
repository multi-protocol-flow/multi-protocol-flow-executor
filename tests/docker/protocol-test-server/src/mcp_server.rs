//! MCP Streamable HTTP 测试服务器
//!
//! 为 flow-engine 的 MCP 节点提供集成测试目标服务。
//! 实现 MCP 协议 (JSON-RPC 2.0) 的 Streamable HTTP 传输。

use std::convert::Infallible;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};

// ============================================================================
// MCP 协议处理
// ============================================================================

/// 处理单个 MCP JSON-RPC 请求
fn handle_mcp_request(request: Value, session_id: &str) -> Value {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 通知类型（没有 id）不需要响应
    if id.is_none() {
        tracing::debug!(method, "MCP 通知，无需响应");
        return json!({});
    }

    let result = match method {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(request.get("params")),
        "resources/list" => handle_resources_list(),
        "resources/read" => handle_resources_read(request.get("params")),
        "prompts/list" => handle_prompts_list(),
        "prompts/get" => handle_prompts_get(request.get("params")),
        "ping" => json!({}),
        _ => {
            return json_rpc_error(id.unwrap(), -32601, &format!("Method not found: {}", method));
        }
    };

    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap(),
        "result": result,
        "_meta": { "sessionId": session_id }
    })
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {},
            "prompts": {}
        },
        "serverInfo": {
            "name": "test-mcp-server",
            "version": "0.1.0"
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "echo",
                "description": "Echo back the input text",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Text to echo back" }
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "add",
                "description": "Add two numbers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "a": { "type": "number", "description": "First number" },
                        "b": { "type": "number", "description": "Second number" }
                    },
                    "required": ["a", "b"]
                }
            },
            {
                "name": "current_time",
                "description": "Get current server time in ISO 8601 format",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

fn handle_tools_call(params: Option<&Value>) -> Value {
    let params = match params {
        Some(p) => p,
        None => {
            return json!({
                "content": [{ "type": "text", "text": "error: missing params" }],
                "isError": true
            });
        }
    };

    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let (text, is_error) = match tool_name {
        "echo" => {
            let text = arguments
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            (text.to_string(), false)
        }
        "add" => {
            let a = arguments.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = arguments.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            (format!("{}", a + b), false)
        }
        "current_time" => {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            (format!("{}ms since epoch", ts), false)
        }
        _ => (format!("Unknown tool: {}", tool_name), true),
    };

    let mut result = json!({
        "content": [{ "type": "text", "text": text }]
    });
    if is_error {
        result["isError"] = json!(true);
    }
    result
}

fn handle_resources_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "test://info",
                "name": "Server Info",
                "description": "Test server information",
                "mimeType": "application/json"
            },
            {
                "uri": "test://config",
                "name": "Server Config",
                "description": "Test server configuration",
                "mimeType": "application/json"
            }
        ]
    })
}

fn handle_resources_read(params: Option<&Value>) -> Value {
    let uri = params
        .and_then(|p| p.get("uri"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (text, mime) = match uri {
        "test://info" => (
            json!({
                "name": "test-mcp-server",
                "version": "0.1.0",
                "protocols": ["tools", "resources", "prompts"]
            })
            .to_string(),
            "application/json",
        ),
        "test://config" => (
            json!({
                "maxTools": 100,
                "maxResources": 50,
                "debug": true
            })
            .to_string(),
            "application/json",
        ),
        _ => (
            json!({"error": "resource not found", "uri": uri}).to_string(),
            "application/json",
        ),
    };

    json!({
        "contents": [{
            "uri": uri,
            "mimeType": mime,
            "text": text
        }]
    })
}

fn handle_prompts_list() -> Value {
    json!({
        "prompts": [
            {
                "name": "greeting",
                "description": "Generate a greeting prompt",
                "arguments": [
                    {
                        "name": "name",
                        "description": "Person's name",
                        "required": true
                    }
                ]
            },
            {
                "name": "summarize",
                "description": "Generate a summarization prompt",
                "arguments": [
                    {
                        "name": "topic",
                        "description": "Topic to summarize",
                        "required": true
                    },
                    {
                        "name": "length",
                        "description": "Desired length (short/medium/long)",
                        "required": false
                    }
                ]
            }
        ]
    })
}

fn handle_prompts_get(params: Option<&Value>) -> Value {
    let params = match params {
        Some(p) => p,
        None => {
            return json!({
                "description": "error",
                "messages": [{ "role": "user", "content": { "type": "text", "text": "missing params" } }]
            });
        }
    };

    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "greeting" => {
            let person_name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            json!({
                "description": "Generate a greeting",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": format!("Please greet {} warmly and creatively.", person_name)
                    }
                }]
            })
        }
        "summarize" => {
            let topic = arguments
                .get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let length = arguments
                .get("length")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");
            json!({
                "description": "Generate a summarization prompt",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": format!("Please provide a {} summary of: {}", length, topic)
                    }
                }]
            })
        }
        _ => {
            json!({
                "description": "error",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": format!("Unknown prompt: {}", name)
                    }
                }]
            })
        }
    }
}

fn json_rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

// ============================================================================
// 公共入口
// ============================================================================

/// 启动 MCP Streamable HTTP 测试服务器并阻塞
///
/// 端口可通过 `MCP_PORT` 环境变量配置，默认 8085。
pub async fn run(port: u16) -> Result<()> {
    let actual_port = env::var("MCP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(port);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", actual_port))
        .await
        .with_context(|| format!("MCP: 绑定端口 {} 失败", actual_port))?;

    let session_counter = Arc::new(AtomicU64::new(1));

    tracing::info!("MCP Streamable HTTP 服务启动 → 0.0.0.0:{}", actual_port);

    loop {
        let (stream, _addr) = listener.accept().await?;
        let session_counter = Arc::clone(&session_counter);

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                let session_counter = Arc::clone(&session_counter);
                async move { handle_mcp_http(req, &session_counter).await }
            });

            if let Err(e) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(io, service)
                .await
            {
                tracing::debug!("MCP 连接错误: {}", e);
            }
        });
    }
}

async fn handle_mcp_http(
    req: Request<hyper::body::Incoming>,
    session_counter: &Arc<AtomicU64>,
) -> std::result::Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // CORS 预检
    if method == Method::OPTIONS {
        return Ok(mcp_cors_response(StatusCode::NO_CONTENT));
    }

    // 仅处理 /mcp 路径
    if !path.starts_with("/mcp") {
        return Ok(mcp_error_response(StatusCode::NOT_FOUND, "not found"));
    }

    match method {
        Method::POST => {
            // 读取请求体
            let body_bytes = match req.into_body().collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => {
                    return Ok(mcp_error_response(StatusCode::BAD_REQUEST, "failed to read body"));
                }
            };

            // 解析 JSON-RPC
            let rpc_request: Value = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(mcp_error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("invalid json: {}", e),
                    ));
                }
            };

            // 验证或创建 session
            let session_id = format!("session-{}", session_counter.fetch_add(1, Ordering::Relaxed));

            tracing::info!(
                method = %rpc_request.get("method").and_then(|v| v.as_str()).unwrap_or("?"),
                session_id = %session_id,
                "MCP 请求"
            );

            let response = handle_mcp_request(rpc_request, &session_id);
            let body = serde_json::to_string(&response).unwrap_or_default();

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .header("mcp-session-id", &session_id)
                .header("access-control-allow-origin", "*")
                .header("access-control-expose-headers", "mcp-session-id")
                .body(Full::new(Bytes::from(body)).boxed())
                .unwrap())
        }

        Method::GET => {
            // GET /mcp — 返回 SSE 空流（用于测试 SSE 连接）
            // 简单响应表示服务在线
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("access-control-allow-origin", "*")
                .body(Full::new(Bytes::from("event: ping\ndata: {}\n\n")).boxed())
                .unwrap())
        }

        _ => Ok(mcp_error_response(StatusCode::METHOD_NOT_ALLOWED, "method not allowed")),
    }
}

fn mcp_error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, Infallible>> {
    let body = serde_json::to_string(&json!({ "error": message })).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .body(Full::new(Bytes::from(body)).boxed())
        .unwrap()
}

fn mcp_cors_response(status: StatusCode) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(status)
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, OPTIONS")
        .header("access-control-allow-headers", "content-type, authorization, mcp-session-id, accept")
        .header("access-control-expose-headers", "mcp-session-id")
        .header("access-control-max-age", "86400")
        .body(Full::new(Bytes::new()).boxed())
        .unwrap()
}
