//! Native MCP (Model Context Protocol) Server implementation.
//!
//! Exposes flows as MCP tools over stdio or Streamable HTTP.
#![allow(missing_docs)]

pub mod executor;
pub mod protocol;
pub mod telemetry;
pub mod tools;
pub mod transports;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub use tools::ToolRegistry;

/// Default port for the shared MCP gateway (slug-prefixed routing).
///
/// Sits next to the Ingress gateway default (9527) so the two resident
/// services are easy to remember.
pub const DEFAULT_MCP_PORT: u16 = 9528;

/// Arguments to configure and launch an MCP server instance
#[derive(Debug, Clone)]
pub struct McpServerArgs {
    pub flow: Option<PathBuf>,
    pub dir: Option<PathBuf>,
    pub transport: String,
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Default for McpServerArgs {
    fn default() -> Self {
        Self {
            flow: None,
            dir: None,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: DEFAULT_MCP_PORT,
            timeout_ms: 28000,
        }
    }
}

/// Run native MCP server according to provided arguments.
///
/// With neither `flow` nor `dir` the server starts as an empty *shared
/// gateway*: flows are attached at runtime through `POST /register` and each
/// one is reachable at `/{slug}/mcp`.
pub async fn run_mcp_server(args: McpServerArgs) -> Result<(), String> {
    let registry = ToolRegistry::load(args.flow.as_deref(), args.dir.as_deref())?;
    let tools_count = registry.tools.len();
    eprintln!(
        "[mpe-mcp-server] Loaded {} tool(s) from flow targets",
        tools_count
    );

    let registry_arc = Arc::new(RwLock::new(registry));

    match args.transport.to_lowercase().as_str() {
        "stdio" => {
            transports::stdio::run_stdio(registry_arc, args.timeout_ms).await?;
        }
        "streamable" | "http" => {
            transports::streamable::run_streamable(
                registry_arc,
                &args.host,
                args.port,
                args.timeout_ms,
            )
            .await?;
        }
        other => {
            return Err(format!(
                "Unsupported transport: '{other}'. Expected 'stdio' or 'streamable'"
            ));
        }
    }

    Ok(())
}

/// Outcome of attaching a flow to a running shared gateway.
#[derive(Debug, Clone)]
pub struct GatewayRegistration {
    pub tool_name: String,
    pub slug: String,
    pub tools_count: usize,
}

fn gateway_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(5000))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

/// Attach a flow file to a running shared gateway (`POST /register`).
pub async fn register_flow_on_gateway(
    host: &str,
    port: u16,
    flow_path: &Path,
) -> Result<GatewayRegistration, String> {
    let client = gateway_client()?;
    let url = format!("http://{host}:{port}/register");
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "flow": flow_path.to_string_lossy() }))
        .send()
        .await
        .map_err(|e| format!("Failed to reach MCP gateway at {host}:{port}: {e}"))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();

    if !status.is_success() {
        let detail = body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown error");
        return Err(format!("MCP gateway rejected flow registration: {detail}"));
    }

    Ok(GatewayRegistration {
        tool_name: body
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        slug: body
            .get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        tools_count: body
            .get("tools_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
    })
}

/// Detach a flow from a running shared gateway (`POST /unregister`).
///
/// Prefers the exact source path; falls back to the tool name or slug when the
/// metadata predates source tracking. Returns the remaining tool count so the
/// caller can decide whether the gateway still has work to do.
pub async fn unregister_flow_from_gateway(
    host: &str,
    port: u16,
    flow_path: Option<&Path>,
    tool_or_slug: Option<&str>,
) -> Result<usize, String> {
    let payload = match (flow_path, tool_or_slug) {
        (Some(path), _) => serde_json::json!({ "flow": path.to_string_lossy() }),
        (None, Some(handle)) => serde_json::json!({ "tool_name": handle }),
        (None, None) => return Err("Missing flow path or tool name".to_string()),
    };

    let client = gateway_client()?;
    let url = format!("http://{host}:{port}/unregister");
    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to reach MCP gateway at {host}:{port}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "MCP gateway rejected flow unregistration: {}",
            resp.status()
        ));
    }

    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    Ok(body
        .get("tools_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize)
}
