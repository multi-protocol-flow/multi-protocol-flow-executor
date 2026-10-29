//! Streamable HTTP Transport for MCP Server.
//!
//! Two deployment shapes share one implementation:
//!
//! * **Dedicated** (`--port <custom>`): the process owns the port and serves
//!   MCP at the root paths `/mcp`, `/health`, `/reload`.
//! * **Shared gateway** (default port): one long-lived process hosts every
//!   registered flow, each addressed at `/{slug}/mcp` (slug-prefixed, the same
//!   convention as the Ingress gateway). Flows are added and removed at
//!   runtime through `/register` and `/unregister`.

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::mcp_server::protocol::*;
use crate::mcp_server::tools::{slug_for_tool, ToolRegistry};
use super::stdio::dispatch_request;

/// Run MCP Server over Streamable HTTP
pub async fn run_streamable(
    registry: Arc<RwLock<ToolRegistry>>,
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> Result<(), String> {
    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| format!("Failed to bind {bind_addr}: {e}"))?;

    let actual_port = listener
        .local_addr()
        .map(|a| a.port())
        .unwrap_or(port);

    eprintln!("[mpe-mcp-server] Streamable HTTP server listening on http://{host}:{actual_port}");
    eprintln!("MPE_MCP_SERVER_PORT={actual_port}");

    loop {
        let (socket, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[mpe-mcp-server] Accept error: {e}");
                continue;
            }
        };

        let reg = Arc::clone(&registry);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, reg, timeout_ms).await {
                eprintln!("[mpe-mcp-server] Connection error: {e}");
            }
        });
    }
}

/// Route resolved from the request path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Route {
    /// `/health` — global gateway health, reports total tool count.
    Health,
    /// `/reload` — re-read every flow from disk.
    Reload,
    /// `/register` — attach a flow file to this server.
    Register,
    /// `/unregister` — detach a flow from this server.
    Unregister,
    /// `/mcp` or `/{slug}/mcp` — JSON-RPC endpoint, optionally slug-scoped.
    Mcp(Option<String>),
    /// `/{slug}/health` — slug-scoped health.
    ScopedHealth(String),
    /// `/{slug}/reload` — slug-scoped reload.
    ScopedReload(String),
}

/// Resolve a cleaned request path into a [`Route`].
fn resolve_route(path: &str) -> Option<Route> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Some(Route::Mcp(None));
    }

    let segments: Vec<&str> = trimmed.split('/').collect();

    match segments.as_slice() {
        ["health"] => Some(Route::Health),
        ["reload"] => Some(Route::Reload),
        ["register"] => Some(Route::Register),
        ["unregister"] => Some(Route::Unregister),
        ["mcp"] | ["rpc"] | ["sse"] | ["messages"] | ["message"] => Some(Route::Mcp(None)),
        [slug, "mcp"] | [slug, "rpc"] | [slug, "sse"] => Some(Route::Mcp(Some(slug.to_string()))),
        [slug, "health"] => Some(Route::ScopedHealth(slug.to_string())),
        [slug, "reload"] => Some(Route::ScopedReload(slug.to_string())),
        _ => None,
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    registry: Arc<RwLock<ToolRegistry>>,
    timeout_ms: u64,
) -> Result<(), String> {
    let mut buf = vec![0u8; 16384];
    let mut total_read = 0;

    // Read HTTP Request Headers
    let (method, path, headers_end, content_length) = loop {
        let n = socket
            .read(&mut buf[total_read..])
            .await
            .map_err(|e| format!("Read failed: {e}"))?;
        if n == 0 {
            return Ok(());
        }
        total_read += n;

        if let Some(pos) = find_subsequence(&buf[..total_read], b"\r\n\r\n") {
            let header_str = String::from_utf8_lossy(&buf[..pos]);
            let mut lines = header_str.lines();
            let req_line = lines.next().unwrap_or("");
            let mut parts = req_line.split_whitespace();
            let m = parts.next().unwrap_or("").to_uppercase();
            let p = parts.next().unwrap_or("").to_string();

            let mut len = 0usize;
            for line in lines {
                if line.to_ascii_lowercase().starts_with("content-length:") {
                    if let Some(val) = line.split(':').nth(1) {
                        len = val.trim().parse::<usize>().unwrap_or(0);
                    }
                }
            }

            break (m, p, pos + 4, len);
        }

        if total_read >= buf.len() {
            let resp = [
                "HTTP/1.1 413 Payload Too Large",
                "Content-Length: 0",
                "Connection: close",
                "",
                "",
            ]
            .join("\r\n");
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.flush().await;
            return Ok(());
        }
    };

    // CORS preflight
    if method == "OPTIONS" {
        respond(
            &mut socket,
            "204 No Content",
            "text/plain",
            "",
            &[("Access-Control-Allow-Origin", "*")],
        )
        .await?;
        return Ok(());
    }

    // Read remaining body if Content-Length is larger than buffered
    let body_read = total_read - headers_end;
    let mut body = buf[headers_end..total_read].to_vec();
    if body_read < content_length {
        let remaining = content_length - body_read;
        let mut rest = vec![0u8; remaining];
        socket
            .read_exact(&mut rest)
            .await
            .map_err(|e| format!("Body read error: {e}"))?;
        body.extend_from_slice(&rest);
    }

    let clean_path = path.split('?').next().unwrap_or(&path).trim_end_matches('/');

    match (method.as_str(), resolve_route(clean_path)) {
        ("GET", Some(Route::Health)) => {
            let count = registry.read().await.tools.len();
            let json_body = serde_json::json!({ "status": "ok", "tools_count": count }).to_string();
            respond_json(&mut socket, "200 OK", &json_body).await?;
        }
        ("GET", Some(Route::ScopedHealth(slug))) => {
            let count = registry.read().await.list_tools_in_scope(Some(&slug)).len();
            if count == 0 {
                let json_body = serde_json::json!({
                    "status": "error",
                    "error": format!("No flow registered at slug '{slug}'")
                })
                .to_string();
                respond_json(&mut socket, "404 Not Found", &json_body).await?;
            } else {
                let json_body =
                    serde_json::json!({ "status": "ok", "slug": slug, "tools_count": count })
                        .to_string();
                respond_json(&mut socket, "200 OK", &json_body).await?;
            }
        }
        ("POST", Some(Route::Reload)) => {
            let mut guard = registry.write().await;
            let changed = guard.reload().unwrap_or(false);
            let count = guard.tools.len();
            drop(guard);
            let json_body = serde_json::json!({
                "status": "ok",
                "changed": changed,
                "tools_count": count
            })
            .to_string();
            respond_json(&mut socket, "200 OK", &json_body).await?;
        }
        ("POST", Some(Route::ScopedReload(slug))) => {
            let mut guard = registry.write().await;
            let changed = guard.reload().unwrap_or(false);
            let count = guard.list_tools_in_scope(Some(&slug)).len();
            drop(guard);
            if count == 0 {
                respond_json(&mut socket, "404 Not Found", &unknown_slug_body(Some(&slug))).await?;
            } else {
                let json_body = serde_json::json!({
                    "status": "ok",
                    "slug": slug,
                    "changed": changed,
                    "tools_count": count
                })
                .to_string();
                respond_json(&mut socket, "200 OK", &json_body).await?;
            }
        }
        ("POST", Some(Route::Register)) => {
            let flow_path = serde_json::from_slice::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("flow").and_then(|f| f.as_str()).map(str::to_string));

            let Some(flow_path) = flow_path else {
                respond_json(
                    &mut socket,
                    "400 Bad Request",
                    &serde_json::json!({ "status": "error", "error": "Missing 'flow' path in body" })
                        .to_string(),
                )
                .await?;
                return Ok(());
            };

            let mut guard = registry.write().await;
            let loaded = guard.load_file(std::path::Path::new(&flow_path));
            let count = guard.tools.len();
            drop(guard);

            match loaded {
                Ok(tool_name) => {
                    let slug = slug_for_tool(&tool_name);
                    let json_body = serde_json::json!({
                        "status": "ok",
                        "tool_name": tool_name,
                        "slug": slug,
                        "tools_count": count
                    })
                    .to_string();
                    respond_json(&mut socket, "200 OK", &json_body).await?;
                }
                Err(e) => {
                    let json_body = serde_json::json!({
                        "status": "error",
                        "error": e
                    })
                    .to_string();
                    respond_json(&mut socket, "400 Bad Request", &json_body).await?;
                }
            }
        }
        ("POST", Some(Route::Unregister)) => {
            // `flow` (source path) is exact; `tool_name` / `slug` are fallbacks.
            let payload = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or_default();
            let flow_path = payload
                .get("flow")
                .and_then(|f| f.as_str())
                .map(str::to_string);
            let target = payload
                .get("tool_name")
                .or_else(|| payload.get("slug"))
                .and_then(|f| f.as_str())
                .map(str::to_string);

            if flow_path.is_none() && target.is_none() {
                respond_json(
                    &mut socket,
                    "400 Bad Request",
                    &serde_json::json!({
                        "status": "error",
                        "error": "Missing 'flow' or 'tool_name' in body"
                    })
                    .to_string(),
                )
                .await?;
                return Ok(());
            }

            let mut guard = registry.write().await;
            let removed = if let Some(ref path) = flow_path {
                guard.remove_by_source(std::path::Path::new(path))
            } else {
                let target = target.unwrap_or_default();
                if guard.remove_tool(&target) {
                    true
                } else {
                    let matches: Vec<String> = guard
                        .tools
                        .keys()
                        .filter(|n| slug_for_tool(n) == target)
                        .cloned()
                        .collect();
                    !matches.is_empty() && matches.into_iter().all(|n| guard.remove_tool(&n))
                }
            };
            let count = guard.tools.len();
            drop(guard);

            let json_body = serde_json::json!({
                "status": "ok",
                "removed": removed,
                "tools_count": count
            })
            .to_string();
            respond_json(&mut socket, "200 OK", &json_body).await?;
        }
        ("GET", Some(Route::Mcp(scope))) => {
            if !scope_is_known(&registry, scope.as_deref()).await {
                respond_json(&mut socket, "404 Not Found", &unknown_slug_body(scope.as_deref()))
                    .await?;
                return Ok(());
            }
            // SSE endpoint for clients opening an event stream
            let resp = [
                "HTTP/1.1 200 OK",
                "Content-Type: text/event-stream",
                "Cache-Control: no-cache",
                "Mcp-Session-Id: default",
                "Access-Control-Allow-Origin: *",
                "Access-Control-Expose-Headers: Mcp-Session-Id, *",
                "Connection: keep-alive",
                "",
                ": ping\r\n\r\n",
            ]
            .join("\r\n");
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.flush().await;
            let mut dummy = [0u8; 1];
            let _ = socket.read(&mut dummy).await;
        }
        ("DELETE", Some(Route::Mcp(_))) => {
            respond(&mut socket, "204 No Content", "text/plain", "", &[]).await?;
        }
        ("POST", Some(Route::Mcp(scope))) => {
            if !scope_is_known(&registry, scope.as_deref()).await {
                respond_json(&mut socket, "404 Not Found", &unknown_slug_body(scope.as_deref()))
                    .await?;
                return Ok(());
            }
            let request: JsonRpcRequest = match serde_json::from_slice(&body) {
                Ok(req) => req,
                Err(e) => {
                    let err_resp = JsonRpcResponse::error(
                        serde_json::Value::Null,
                        JsonRpcError::parse_error(format!("JSON-RPC parse error: {e}")),
                    );
                    let body_str = serde_json::to_string(&err_resp).unwrap_or_default();
                    respond_json(&mut socket, "400 Bad Request", &body_str).await?;
                    return Ok(());
                }
            };

            let response = dispatch_request(&request, &registry, timeout_ms, scope.as_deref()).await;
            match response {
                Some(r) => {
                    let body_str = serde_json::to_string(&r).unwrap_or_default();
                    respond_json(&mut socket, "200 OK", &body_str).await?;
                }
                None => {
                    // JSON-RPC Notification: HTTP 202 Accepted with empty body
                    respond(&mut socket, "202 Accepted", "text/plain", "", &[]).await?;
                }
            }
        }
        _ => {
            respond(&mut socket, "404 Not Found", "text/plain", "", &[]).await?;
        }
    }

    Ok(())
}

/// Whether the route scope resolves to at least one registered flow.
async fn scope_is_known(
    registry: &Arc<RwLock<ToolRegistry>>,
    scope: Option<&str>,
) -> bool {
    match scope {
        None => true,
        Some(slug) => !registry
            .read()
            .await
            .list_tools_in_scope(Some(slug))
            .is_empty(),
    }
}

/// JSON body for an unknown slug route.
fn unknown_slug_body(scope: Option<&str>) -> String {
    serde_json::json!({
        "status": "error",
        "error": format!(
            "No flow registered at slug '{}'",
            scope.unwrap_or_default()
        )
    })
    .to_string()
}

/// Write a JSON HTTP response with the standard MCP headers.
async fn respond_json(
    socket: &mut TcpStream,
    status: &str,
    body: &str,
) -> Result<(), String> {
    respond(
        socket,
        status,
        "application/json",
        body,
        &[
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Expose-Headers", "Mcp-Session-Id, *"),
            ("Mcp-Session-Id", "default"),
        ],
    )
    .await
}

/// Write a raw HTTP/1.1 response and close the connection.
async fn respond(
    socket: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
    extra_headers: &[(&str, &str)],
) -> Result<(), String> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in extra_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    head.push_str(body);

    socket
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let _ = socket.flush().await;
    let _ = socket.shutdown().await;
    Ok(())
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}
