//! MCP stdio 传输二进制
//!
//! 通过 stdin/stdout 传输的 MCP 服务器。
//! 每行一个 JSON-RPC 2.0 消息，适合本地进程通信。
//!
//! 用法:
//!   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./mcp-stdio-server

use std::io::{BufRead, Write, BufWriter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = BufWriter::new(stdout.lock());

    eprintln!("[mcp-stdio-server] 启动，等待 stdin 输入...");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[mcp-stdio-server] 读取 stdin 错误: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 解析 JSON-RPC
        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[mcp-stdio-server] JSON 解析错误: {}", e);
                let error_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                writeln!(stdout, "{}", error_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

        eprintln!("[mcp-stdio-server] 收到: method={}", method);

        // 通知类型（无 id）不需要响应
        if id.is_none() {
            eprintln!("[mcp-stdio-server] 通知，跳过响应");
            continue;
        }

        let result = match method {
            "initialize" => handle_initialize(),
            "notifications/initialized" => {
                // 通知，不应有 id，但如果有的话也不响应
                continue;
            }
            "tools/list" => handle_tools_list(),
            "tools/call" => handle_tools_call(request.get("params")),
            "resources/list" => handle_resources_list(),
            "resources/read" => handle_resources_read(request.get("params")),
            "prompts/list" => handle_prompts_list(),
            "prompts/get" => handle_prompts_get(request.get("params")),
            "ping" => json!({}),
            "notifications/cancelled" => {
                eprintln!("[mcp-stdio-server] 收到取消通知");
                continue;
            }
            _ => {
                let error_resp = json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap(),
                    "error": { "code": -32601, "message": format!("Method not found: {}", method) }
                });
                writeln!(stdout, "{}", error_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id.unwrap(),
            "result": result
        });

        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }

    eprintln!("[mcp-stdio-server] stdin 关闭，退出");
    Ok(())
}

// ============================================================================
// MCP 协议处理（与 mcp_server.rs 相同逻辑）
// ============================================================================

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {},
            "prompts": {}
        },
        "serverInfo": {
            "name": "test-mcp-stdio-server",
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
                "description": "Get current server time",
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
            let text = arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
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
            json!({ "name": "test-mcp-stdio-server", "version": "0.1.0" }).to_string(),
            "application/json",
        ),
        "test://config" => (
            json!({ "maxTools": 100, "debug": true }).to_string(),
            "application/json",
        ),
        _ => (
            json!({"error": "resource not found", "uri": uri}).to_string(),
            "application/json",
        ),
    };

    json!({
        "contents": [{ "uri": uri, "mimeType": mime, "text": text }]
    })
}

fn handle_prompts_list() -> Value {
    json!({
        "prompts": [
            {
                "name": "greeting",
                "description": "Generate a greeting prompt",
                "arguments": [
                    { "name": "name", "description": "Person's name", "required": true }
                ]
            },
            {
                "name": "summarize",
                "description": "Generate a summarization prompt",
                "arguments": [
                    { "name": "topic", "description": "Topic to summarize", "required": true },
                    { "name": "length", "description": "Desired length", "required": false }
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
            let person_name = arguments.get("name").and_then(|v| v.as_str()).unwrap_or("World");
            json!({
                "description": "Generate a greeting",
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": format!("Please greet {} warmly.", person_name) }
                }]
            })
        }
        "summarize" => {
            let topic = arguments.get("topic").and_then(|v| v.as_str()).unwrap_or("general");
            let length = arguments.get("length").and_then(|v| v.as_str()).unwrap_or("medium");
            json!({
                "description": "Generate a summarization prompt",
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": format!("Please provide a {} summary of: {}", length, topic) }
                }]
            })
        }
        _ => {
            json!({
                "description": "error",
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": format!("Unknown prompt: {}", name) }
                }]
            })
        }
    }
}
