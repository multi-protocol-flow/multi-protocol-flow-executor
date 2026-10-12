//! Host-side Loopback HTTP Tool Bridge for dynamic tool scheduling.
//!
//! Provides an authenticated HTTP service listening on `127.0.0.1:0` (ephemeral port),
//! enabling sidecar plugins (such as `agent:react`) to dispatch real node executions
//! (HTTP, DB, Redis, scripts, etc.) back into the host engine with access to the flow's
//! execution context, variables, and connection providers.

use flow_engine_core::context::ExecutionContext;
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::NodeData;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex, RwLock};
use uuid::Uuid;

use crate::executor::FlowExecutor;

/// Header name for bridge authentication token.
pub const BRIDGE_TOKEN_HEADER: &str = "x-mpe-bridge-token";

/// Maximum request body size for tool execution (16 MB).
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Request payload for `POST /tool/execute`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ToolExecuteRequest {
    /// Associated execution instance ID.
    pub execution_id: String,
    /// Target node type in MPE (e.g. "http:request", "db:query", "script").
    pub node_type: String,
    /// Node configuration payload.
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Response payload from `POST /tool/execute`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ToolExecuteResponse {
    /// Whether the tool execution completed successfully.
    pub success: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Output data produced by the node (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error message (if execution failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Raw response or error string representation for Agent reasoning.
    pub raw_response: String,
}

/// Handle to a running Tool Bridge HTTP server.
#[derive(Clone)]
pub struct ToolBridgeHandle {
    /// Full base URL (e.g. `<http://127.0.0.1:45678>`).
    pub url: String,
    /// Port number assigned to this server.
    pub port: u16,
    /// Authentication token required in request headers.
    pub token: String,
    /// Shared execution contexts map keyed by `execution_id`.
    pub contexts: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    server_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ToolBridgeHandle {
    /// Registers or updates an execution context for an active flow execution.
    pub async fn register_context(&self, execution_id: String, context: ExecutionContext) {
        self.contexts.write().await.insert(execution_id, context);
    }

    /// Removes an execution context when a flow execution finishes.
    pub async fn unregister_context(&self, execution_id: &str) {
        self.contexts.write().await.remove(execution_id);
    }

    /// Retrieves a clone of the execution context for a given execution ID.
    pub async fn get_context(&self, execution_id: &str) -> Option<ExecutionContext> {
        self.contexts.read().await.get(execution_id).cloned()
    }

    /// Shuts down the Tool Bridge server.
    pub async fn shutdown(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.server_task.lock().await.take() {
            let _ = task.await;
        }
    }
}

/// Tool Bridge HTTP server launcher.
pub struct ToolBridgeServer;

impl ToolBridgeServer {
    /// Starts a new Tool Bridge HTTP server on `127.0.0.1:0` with an auto-generated token.
    pub async fn start(executor: Arc<FlowExecutor>) -> Result<ToolBridgeHandle, String> {
        let token = Uuid::new_v4().to_string();
        Self::start_with_token(executor, token).await
    }

    /// Starts a new Tool Bridge HTTP server with a specified authentication token.
    pub async fn start_with_token(
        executor: Arc<FlowExecutor>,
        token: String,
    ) -> Result<ToolBridgeHandle, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind Tool Bridge on 127.0.0.1:0: {e}"))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| format!("Failed to get Tool Bridge local address: {e}"))?;

        let port = local_addr.port();
        let url = format!("http://127.0.0.1:{}", port);
        let contexts = Arc::new(RwLock::new(HashMap::new()));
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        let server_contexts = contexts.clone();
        let server_token = token.clone();
        let server_executor = executor;

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        log::info!("[ToolBridge] Shutdown signal received for port {}", port);
                        break;
                    }
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, _client_addr)) => {
                                let exec = server_executor.clone();
                                let ctxs = server_contexts.clone();
                                let tok = server_token.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, exec, ctxs, tok).await {
                                        log::debug!("[ToolBridge] Connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::warn!("[ToolBridge] Accept error on port {}: {}", port, e);
                            }
                        }
                    }
                }
            }
        });

        Ok(ToolBridgeHandle {
            url,
            port,
            token,
            contexts,
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
            server_task: Arc::new(Mutex::new(Some(task))),
        })
    }
}

/// Handles a single incoming HTTP connection to the Tool Bridge.
async fn handle_connection(
    mut stream: TcpStream,
    executor: Arc<FlowExecutor>,
    contexts: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    expected_token: String,
) -> Result<(), String> {
    let mut buffer = Vec::with_capacity(4096);
    let mut temp_buf = [0u8; 4096];

    // 1. Read headers until \r\n\r\n
    let header_end_pos = loop {
        let n = stream
            .read(&mut temp_buf)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp_buf[..n]);

        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }

        if buffer.len() > 65536 {
            write_http_response(
                &mut stream,
                431,
                &[("Content-Type", "application/json")],
                b"{\"error\":\"Request Header Fields Too Large\"}",
            )
            .await;
            return Ok(());
        }
    };

    let header_bytes = &buffer[..header_end_pos];
    let header_str = String::from_utf8_lossy(header_bytes);
    let mut lines = header_str.lines();

    // 2. Parse request line (METHOD PATH HTTP/1.1)
    let request_line = lines.next().ok_or("Empty request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_uppercase();
    let path = parts.next().unwrap_or("/").to_string();

    // 3. Parse headers
    let mut headers_map = HashMap::new();
    let mut content_length: usize = 0;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers_map.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    if let Some(cl_val) = headers_map.get("content-length") {
        content_length = cl_val.parse::<usize>().unwrap_or(0);
    }

    // Check token authentication
    let provided_token = headers_map
        .get(BRIDGE_TOKEN_HEADER)
        .cloned()
        .unwrap_or_default();
    if provided_token != expected_token {
        write_http_response(
            &mut stream,
            401,
            &[("Content-Type", "application/json")],
            b"{\"error\":\"Unauthorized: invalid bridge token\"}",
        )
        .await;
        return Ok(());
    }

    // Route checking
    let route_path = path.split('?').next().unwrap_or("/");
    if route_path != "/tool/execute" {
        write_http_response(
            &mut stream,
            404,
            &[("Content-Type", "application/json")],
            b"{\"error\":\"Not Found\"}",
        )
        .await;
        return Ok(());
    }

    if method != "POST" {
        write_http_response(
            &mut stream,
            405,
            &[("Content-Type", "application/json")],
            b"{\"error\":\"Method Not Allowed\"}",
        )
        .await;
        return Ok(());
    }

    if content_length > MAX_REQUEST_BODY_BYTES {
        write_http_response(
            &mut stream,
            413,
            &[("Content-Type", "application/json")],
            b"{\"error\":\"Payload Too Large\"}",
        )
        .await;
        return Ok(());
    }

    // 4. Read body
    let body_start = header_end_pos + 4;
    let mut body_bytes = if buffer.len() > body_start {
        buffer[body_start..].to_vec()
    } else {
        Vec::new()
    };

    while body_bytes.len() < content_length {
        let needed = content_length - body_bytes.len();
        let read_size = needed.min(temp_buf.len());
        let n = stream
            .read(&mut temp_buf[..read_size])
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            write_http_response(
                &mut stream,
                400,
                &[("Content-Type", "application/json")],
                b"{\"error\":\"Incomplete request body\"}",
            )
            .await;
            return Ok(());
        }
        body_bytes.extend_from_slice(&temp_buf[..n]);
    }

    // 5. Parse JSON request
    let req: ToolExecuteRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            let err_json = serde_json::json!({
                "error": format!("Invalid JSON request payload: {e}")
            });
            write_http_response(
                &mut stream,
                400,
                &[("Content-Type", "application/json")],
                err_json.to_string().as_bytes(),
            )
            .await;
            return Ok(());
        }
    };

    // 6. Look up context and execute
    let start_instant = std::time::Instant::now();
    let ctx_opt = contexts.read().await.get(&req.execution_id).cloned();
    let (variables, extra_providers) = match &ctx_opt {
        Some(ctx) => (
            Some(serde_json::json!(ctx.variables.get_merged_variables())),
            Some(ctx.providers.clone()),
        ),
        None => (None, None),
    };

    // Construct temporary Node
    let uuid = format!("tool_call_{}", Uuid::new_v4());
    let mut config_val = req.config.clone();
    if !config_val.is_object() {
        config_val = serde_json::json!({});
    }
    if config_val.get("type").is_none() {
        config_val["type"] = serde_json::Value::String(req.node_type.clone());
    }

    // Normalize http:request body if it is a JSON object or array
    if req.node_type == "http:request" || req.node_type == "http" {
        if let Some(body) = config_val.get_mut("body") {
            if !body.is_string() && !body.is_null() {
                *body = serde_json::Value::String(body.to_string());
            }
        }
        if config_val.get("method").is_none() {
            config_val["method"] = serde_json::Value::String("get".to_string());
        }
    }

    let node_data = serde_json::from_value::<NodeData>(config_val.clone())
        .unwrap_or(NodeData::Plugin(config_val));

    let node = Node {
        uuid,
        node_type: req.node_type.clone(),
        name: format!("Tool {}", req.node_type),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: node_data,
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let result = executor
        .execute_tool_node(
            &node,
            variables,
            extra_providers.as_ref(),
            Some(req.execution_id.clone()),
        )
        .await;

    let duration_ms = start_instant.elapsed().as_millis() as u64;

    let (success, output, error_message, raw_response) = if result.success {
        let first_report = result.node_reports.into_iter().next();
        let output_data = first_report
            .as_ref()
            .and_then(|r| r.output_data.clone())
            .or(result.data);

        let is_ok = match &output_data {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(b)) = map.get("success") {
                    *b
                } else {
                    true
                }
            }
            _ => true,
        };

        if is_ok {
            let raw = output_data
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "{}".to_string());
            (true, output_data, None, raw)
        } else {
            let err_msg = output_data
                .as_ref()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Tool returned failure".to_string());
            let raw = output_data
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| serde_json::json!({ "error": &err_msg }).to_string());
            (false, output_data, Some(err_msg), raw)
        }
    } else {
        let err_msg = result
            .error
            .unwrap_or_else(|| "Tool execution failed".to_string());
        let raw = serde_json::json!({ "error": &err_msg }).to_string();
        (false, None, Some(err_msg), raw)
    };

    let resp_payload = ToolExecuteResponse {
        success,
        duration_ms,
        output,
        error_message,
        raw_response,
    };

    let resp_bytes = serde_json::to_vec(&resp_payload).unwrap_or_default();
    write_http_response(
        &mut stream,
        200,
        &[("Content-Type", "application/json")],
        &resp_bytes,
    )
    .await;

    Ok(())
}

/// Helper function to find `\r\n\r\n` boundary in byte slice.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Helper function to write an HTTP response.
async fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let reason = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let mut header_str = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status_code,
        reason,
        body.len()
    );
    for (k, v) in headers {
        header_str.push_str(&format!("{}: {}\r\n", k, v));
    }
    header_str.push_str("\r\n");
    let _ = stream.write_all(header_str.as_bytes()).await;
    let _ = stream.write_all(body).await;
    let _ = stream.flush().await;
}
