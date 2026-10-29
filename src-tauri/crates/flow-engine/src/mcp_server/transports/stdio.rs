//! Stdio Transport for MCP Server (Claude Desktop / Cursor).
//!
//! Exclusively communicates JSON-RPC over stdin/stdout.
//! All diagnostics and logs MUST go to stderr to prevent pipeline pollution.

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;

use crate::mcp_server::executor::execute_flow_tool;
use crate::mcp_server::protocol::*;
use crate::mcp_server::tools::ToolRegistry;

/// Run MCP Server over stdio
pub async fn run_stdio(registry: Arc<RwLock<ToolRegistry>>, timeout_ms: u64) -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    eprintln!("[mpe-mcp-server] MCP Stdio Server started. Ready for JSON-RPC messages.");

    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    JsonRpcError::parse_error(format!("Invalid JSON-RPC request: {e}")),
                );
                send_response(&mut stdout, &err_resp).await?;
                continue;
            }
        };

        let response = dispatch_request(&request, &registry, timeout_ms, None).await;

        if let Some(resp) = response {
            send_response(&mut stdout, &resp).await?;
        }
    }

    eprintln!("[mpe-mcp-server] Stdio EOF received. Exiting cleanly.");
    Ok(())
}

async fn send_response<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    response: &JsonRpcResponse,
) -> Result<(), String> {
    let mut json_str = serde_json::to_string(response)
        .map_err(|e| format!("Serialization error: {e}"))?;
    json_str.push('\n');
    writer
        .write_all(json_str.as_bytes())
        .await
        .map_err(|e| format!("Failed to write to stdout: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;
    Ok(())
}

/// Send JSON-RPC Notification over an async writer
pub async fn send_notification<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    notification: &JsonRpcNotification,
) -> Result<(), String> {
    let mut json_str = serde_json::to_string(notification)
        .map_err(|e| format!("Serialization error: {e}"))?;
    json_str.push('\n');
    writer
        .write_all(json_str.as_bytes())
        .await
        .map_err(|e| format!("Failed to write notification: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;
    Ok(())
}

/// Dispatch JSON-RPC request against registered flow tools.
///
/// `scope` restricts the registry view to a single route slug
/// (`/{slug}/mcp` shared-gateway mode); `None` exposes every loaded flow.
pub async fn dispatch_request(
    request: &JsonRpcRequest,
    registry: &Arc<RwLock<ToolRegistry>>,
    timeout_ms: u64,
    scope: Option<&str>,
) -> Option<JsonRpcResponse> {
    let id = request.id.clone().unwrap_or(serde_json::Value::Null);

    // If it's a notification (id is None or Null), and method is notifications/*, handle it
    if request.method == "notifications/initialized" {
        eprintln!("[mpe-mcp-server] Client confirmed initialization.");
        return None;
    }

    match request.method.as_str() {
        "initialize" => {
            let res = InitializeResult::standard();
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(res).unwrap_or(serde_json::Value::Null),
            ))
        }
        "ping" => Some(JsonRpcResponse::success(id, serde_json::json!({}))),
        "tools/list" => {
            let guard = registry.read().await;
            let tools = guard.list_tools_in_scope(scope);
            Some(JsonRpcResponse::success(
                id,
                serde_json::json!({ "tools": tools }),
            ))
        }
        "resources/list" => Some(JsonRpcResponse::success(
            id,
            serde_json::json!({ "resources": [] }),
        )),
        "prompts/list" => Some(JsonRpcResponse::success(
            id,
            serde_json::json!({ "prompts": [] }),
        )),
        "tools/call" => {
            let params = request.params.as_ref();
            let tool_name = params
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");

            if tool_name.is_empty() {
                return Some(JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing or invalid 'name' in tools/call"),
                ));
            }

            let arguments = params.and_then(|p| p.get("arguments")).cloned();

            let target_tool = {
                let guard = registry.read().await;
                guard.get_tool(tool_name).cloned().filter(|tool| {
                    scope.is_none_or(|slug| {
                        crate::mcp_server::tools::slug_for_tool(&tool.tool_def.name) == slug
                    })
                })
            };

            match target_tool {
                Some(tool) => {
                    let call_res = execute_flow_tool(&tool, arguments, timeout_ms).await;
                    Some(JsonRpcResponse::success(
                        id,
                        serde_json::to_value(call_res).unwrap_or(serde_json::Value::Null),
                    ))
                }
                None => Some(JsonRpcResponse::success(
                    id,
                    serde_json::to_value(CallToolResult::error(format!(
                        "Tool '{tool_name}' not found"
                    )))
                    .unwrap_or(serde_json::Value::Null),
                )),
            }
        }
        _ => Some(JsonRpcResponse::error(
            id,
            JsonRpcError::method_not_found(&request.method),
        )),
    }
}
