//! MCP (Model Context Protocol) domain service adapter.
#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::mcp_server::tools::{build_tool_definition, parse_flow_content, sanitize_tool_name, slug_for_tool};
use crate::mcp_server::{
    register_flow_on_gateway, unregister_flow_from_gateway, DEFAULT_MCP_PORT,
};
use crate::supervisor::daemon::{check_early_crash, spawn_daemon};
use crate::supervisor::probe::{probe_http_health, DEFAULT_PROBE_TIMEOUT};
pub use crate::supervisor::process::{is_process_alive, terminate_process};
use crate::supervisor::store::{
    delete_instance, get_services_dir, list_instances, load_instance, save_instance,
};
use crate::supervisor::{ServiceInstance, ServiceKind};

/// Installation and discovery status of the MCP plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPluginStatus {
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Configuration for starting an MCP server instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStartConfig {
    pub name: String,
    #[serde(default)]
    pub flow_target: Option<String>,
    #[serde(default)]
    pub dir_target: Option<String>,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_managed_by")]
    pub managed_by: String,
    /// Join the shared gateway on `port` instead of binding a dedicated one.
    ///
    /// Shared instances are routed by slug (`/{slug}/mcp`) so many flows can
    /// live on one well-known port.
    #[serde(default)]
    pub shared: bool,
}

fn default_transport() -> String {
    "stdio".to_string()
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_mcp_port() -> u16 {
    DEFAULT_MCP_PORT
}
fn default_timeout() -> u64 {
    28000
}
fn default_managed_by() -> String {
    "gui".to_string()
}

/// Metadata representation of a running or managed MCP server instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInstance {
    pub id: String,
    pub name: String,
    pub pid: u32,
    pub transport: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub flow_target: Option<String>,
    #[serde(default)]
    pub dir_target: Option<String>,
    #[serde(default)]
    pub tools_count: usize,
    pub started_at: u64,
    pub managed_by: String,
    pub status: String,
    /// MCP tool name registered on the server (also the route slug source).
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Route slug when hosted on the shared gateway (`/{slug}/mcp`).
    #[serde(default)]
    pub slug: Option<String>,
    /// Whether this instance lives on the shared gateway port.
    #[serde(default)]
    pub shared: bool,
}

impl McpServerInstance {
    /// Client-facing MCP endpoint.
    ///
    /// The shared gateway aggregates every published flow behind one endpoint
    /// (`/mcp`), so a single client registration discovers all tools. Per-flow
    /// isolation stays available at `/{slug}/mcp` (see [`Self::scoped_endpoint`]).
    pub fn endpoint(&self) -> String {
        if self.transport != "streamable" {
            return self
                .flow_target
                .clone()
                .or_else(|| self.dir_target.clone())
                .unwrap_or_else(|| "-".to_string());
        }
        format!("http://{}:{}/mcp", self.host, self.port)
    }

    /// Slug-scoped endpoint exposing only this flow's tool.
    pub fn scoped_endpoint(&self) -> Option<String> {
        if self.transport != "streamable" {
            return None;
        }
        let slug = self.slug.as_deref().filter(|s| !s.is_empty())?;
        Some(format!("http://{}:{}/{}/mcp", self.host, self.port, slug))
    }
}

impl From<ServiceInstance> for McpServerInstance {
    fn from(s: ServiceInstance) -> Self {
        let tool_name = s
            .metadata
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let slug = s
            .metadata
            .get("slug")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let shared = s
            .metadata
            .get("shared")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Self {
            id: s.id,
            name: s.name,
            pid: s.pid,
            transport: s.transport,
            host: s.host,
            port: s.port,
            flow_target: s.flow_target,
            dir_target: s.dir_target,
            tools_count: s.tools_count,
            started_at: s.started_at,
            managed_by: s.managed_by,
            status: s.status,
            tool_name,
            slug,
            shared,
        }
    }
}

impl From<McpServerInstance> for ServiceInstance {
    fn from(s: McpServerInstance) -> Self {
        let mut metadata = serde_json::Map::new();
        if let Some(ref tool) = s.tool_name {
            metadata.insert("tool_name".to_string(), serde_json::json!(tool));
        }
        if let Some(ref slug) = s.slug {
            metadata.insert("slug".to_string(), serde_json::json!(slug));
            metadata.insert("health_path".to_string(), serde_json::json!(format!("/{slug}/health")));
        }
        metadata.insert("shared".to_string(), serde_json::json!(s.shared));

        Self {
            id: s.id,
            name: s.name,
            kind: ServiceKind::Mcp,
            pid: s.pid,
            transport: s.transport,
            host: s.host,
            port: s.port,
            flow_target: s.flow_target,
            dir_target: s.dir_target,
            tools_count: s.tools_count,
            started_at: s.started_at,
            managed_by: s.managed_by,
            status: s.status,
            metadata,
        }
    }
}

/// Inspection outcome of a flow file's tools and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInspection {
    pub tool_name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub variables: Vec<String>,
}

/// Client configuration snippets for Claude Desktop and Cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientSnippets {
    pub claude_desktop: serde_json::Value,
    pub cursor: serde_json::Value,
}

/// Default port used when a caller does not pick one explicitly.
pub fn get_default_mcp_port() -> u16 {
    DEFAULT_MCP_PORT
}

/// Get primary directory where MCP server instances metadata is saved (`~/.mpe/services/`)
pub fn get_mcp_servers_dir() -> PathBuf {
    get_services_dir()
}

/// Check if the MCP plugin is installed and retrieve binary path (for client nodes like mcp:connect)
pub fn check_plugin_status(plugins_dir: &Path) -> McpPluginStatus {
    let mcp_dir = plugins_dir.join("mcp");
    let manifest_path = mcp_dir.join("plugin.json");

    if manifest_path.is_file() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                let version = manifest
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let mut bin_candidates = vec![
                    mcp_dir.join("mpe_plugin_mcp"),
                    mcp_dir.join("mpe_plugin_mcp.exe"),
                ];

                if let Some(bin_name) = manifest.get("binary").and_then(|b| b.as_str()) {
                    bin_candidates.insert(0, mcp_dir.join(bin_name));
                    bin_candidates.insert(1, mcp_dir.join(format!("{bin_name}.exe")));
                }

                for candidate in bin_candidates {
                    if candidate.is_file() {
                        return McpPluginStatus {
                            installed: true,
                            plugin_path: Some(mcp_dir.to_string_lossy().to_string()),
                            binary_path: Some(candidate.to_string_lossy().to_string()),
                            version,
                            error: None,
                        };
                    }
                }
            }
        }
    }

    let version = None;
    McpPluginStatus {
        installed: false,
        plugin_path: if mcp_dir.exists() {
            Some(mcp_dir.to_string_lossy().to_string())
        } else {
            None
        },
        binary_path: None,
        version,
        error: Some("MCP plugin binary not found. Please install the plugin from marketplace.".to_string()),
    }
}

/// Resolve a flow target into a concrete path on disk
pub fn resolve_flow_path(raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() && p.exists() {
        return p.to_path_buf();
    }
    if p.exists() {
        if let Ok(abs) = fs::canonicalize(p) {
            return abs;
        }
    }
    // Check in flows storage directory
    if let Ok(data_dir) = flow_engine_executor::storage::get_data_dir() {
        let in_flows = data_dir.join("flows").join(raw);
        if in_flows.exists() {
            return in_flows;
        }
        let in_flows_mpf = data_dir.join("flows").join(format!("{raw}.mpf"));
        if in_flows_mpf.exists() {
            return in_flows_mpf;
        }
    }
    p.to_path_buf()
}

/// Probe Streamable HTTP /health endpoint
pub async fn probe_server_health(host: &str, port: u16) -> Option<usize> {
    let resp = probe_http_health(host, port, "/health", DEFAULT_PROBE_TIMEOUT).await?;
    let tools_count = resp.get("tools_count").and_then(|c| c.as_u64()).unwrap_or(0);
    Some(tools_count as usize)
}

/// List all managed MCP servers with automatic zombie self-healing.
///
/// Shared-gateway bookkeeping rows (the gateway process itself) are filtered
/// out; callers see one row per published flow, carrying its slug route.
pub async fn list_servers(heal: bool) -> Vec<McpServerInstance> {
    let instances = list_instances(Some(ServiceKind::Mcp), heal).await;
    instances
        .into_iter()
        .filter(|s| {
            !s.metadata
                .get("gateway")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .map(McpServerInstance::from)
        .collect()
}

/// Identity of the shared gateway process for a given port.
fn gateway_id(port: u16) -> String {
    format!("mcp-gateway-{port}")
}

/// Ensure a shared gateway is listening on `host:port`, reusing a live one.
async fn ensure_gateway(
    host: &str,
    port: u16,
    timeout_ms: u64,
    managed_by: &str,
) -> Result<McpServerInstance, String> {
    let gateway = gateway_id(port);

    if let Some(existing) = load_instance(&gateway) {
        if is_process_alive(existing.pid)
            && probe_server_health(host, port).await.is_some()
        {
            return Ok(McpServerInstance::from(existing));
        }
        // Stale gateway bookkeeping: drop it and start over.
        let _ = delete_instance(&gateway);
    }

    // Fail fast when another process already owns the port without our metadata.
    if probe_server_health(host, port).await.is_some() {
        return Err(format!(
            "Port {port} is already served by an unmanaged process. Stop it or pick another port."
        ));
    }

    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to determine current executable: {e}"))?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("mcp")
        .arg("serve")
        .arg("--gateway")
        .arg("--transport")
        .arg("streamable")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .arg("--timeout-ms")
        .arg(timeout_ms.to_string());

    let services_dir = get_services_dir();
    let (mut child, log_path) = spawn_daemon(&mut cmd, &gateway, &services_dir)?;
    let pid = child.id();

    let mut healthy = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        check_early_crash(&mut child, &log_path)?;
        if probe_server_health(host, port).await.is_some() {
            healthy = true;
            break;
        }
    }

    if !healthy {
        terminate_process(pid);
        return Err(format!(
            "MCP gateway failed to answer health probe on {host}:{port} within 4s"
        ));
    }

    let instance = McpServerInstance {
        id: gateway.clone(),
        name: gateway.clone(),
        pid,
        transport: "streamable".to_string(),
        host: host.to_string(),
        port,
        flow_target: None,
        dir_target: None,
        tools_count: 0,
        started_at: chrono::Utc::now().timestamp_millis() as u64,
        managed_by: managed_by.to_string(),
        status: "running".to_string(),
        tool_name: None,
        slug: None,
        shared: true,
    };

    let mut service_inst = ServiceInstance::from(instance.clone());
    service_inst
        .metadata
        .insert("gateway".to_string(), serde_json::json!(true));
    save_instance(&service_inst)?;

    Ok(instance)
}

/// Start an MCP server instance using the native host runner.
///
/// * `shared = true` → join the shared gateway on `config.port`; the flow's
///   tools appear on `http://{host}:{port}/mcp` and are also reachable at the
///   slug-scoped `http://{host}:{port}/{slug}/mcp`.
/// * `shared = false` → spawn a dedicated process owning the port; the flow is
///   reachable at `http://{host}:{port}/mcp`.
///
/// Publishing the same flow twice is idempotent: an already running instance
/// for the same flow / port / mode is returned as-is.
pub async fn start_server(
    config: McpStartConfig,
    _plugins_dir: &Path,
) -> Result<McpServerInstance, String> {
    let resolved_flow = if let Some(ref flow) = config.flow_target {
        let p = resolve_flow_path(flow);
        if !p.exists() {
            return Err(format!("Flow file does not exist: {}", p.display()));
        }
        Some(p.to_string_lossy().to_string())
    } else {
        None
    };

    let existing = find_matching_instance(&config, resolved_flow.as_deref()).await;
    if let Some(instance) = existing {
        if instance.status == "running" {
            return Ok(instance);
        }
        // Stale record for the same flow: drop it before republishing.
        delete_instance(&instance.id);
    }

    if config.transport == "streamable" && config.shared {
        return start_shared_instance(config, resolved_flow).await;
    }

    start_dedicated_instance(config, resolved_flow).await
}

/// Find a published record matching this flow / port / mode, running or not.
async fn find_matching_instance(
    config: &McpStartConfig,
    resolved_flow: Option<&str>,
) -> Option<McpServerInstance> {
    let instances = list_instances(Some(ServiceKind::Mcp), false).await;
    instances
        .into_iter()
        .filter(|i| {
            !i.metadata
                .get("gateway")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .map(McpServerInstance::from)
        .find(|instance| {
            instance.transport == config.transport
                && instance.port == config.port
                && instance.shared == config.shared
                && instance.flow_target.as_deref() == resolved_flow
                && instance.dir_target == config.dir_target
        })
}

/// Restart every published-but-stopped instance, e.g. after an app restart,
/// a crash, or a machine reboot.
///
/// Returns the instances that came back up plus the failures, so callers can
/// report partial restores. Explicitly stopped instances are gone for good:
/// `stop_server` deletes their record.
pub async fn restore_published(
    only_ids: Option<&[String]>,
) -> (Vec<McpServerInstance>, Vec<String>) {
    let mut restored = Vec::new();
    let mut failures = Vec::new();

    let candidates = list_servers(false).await;
    for instance in candidates {
        if instance.status == "running" {
            continue;
        }
        if let Some(ids) = only_ids {
            if !ids.iter().any(|id| id == &instance.id) {
                continue;
            }
        }

        let config = McpStartConfig {
            name: instance.name.clone(),
            flow_target: instance.flow_target.clone(),
            dir_target: instance.dir_target.clone(),
            transport: instance.transport.clone(),
            host: instance.host.clone(),
            port: instance.port,
            timeout_ms: 28000,
            managed_by: instance.managed_by.clone(),
            shared: instance.shared,
        };

        match start_server(config, Path::new("")).await {
            Ok(restarted) => {
                if restarted.id != instance.id {
                    delete_instance(&instance.id);
                }
                restored.push(restarted);
            }
            Err(e) => failures.push(format!("{}: {e}", instance.name)),
        }
    }

    (restored, failures)
}

/// Join the shared gateway; the flow becomes a slug route on the shared port.
async fn start_shared_instance(
    config: McpStartConfig,
    resolved_flow: Option<String>,
) -> Result<McpServerInstance, String> {
    let flow_path = resolved_flow.ok_or_else(|| {
        "Shared gateway mode requires --flow <path> (directory targets need a dedicated port)"
            .to_string()
    })?;

    let gateway = ensure_gateway(&config.host, config.port, config.timeout_ms, &config.managed_by).await?;

    let registration =
        register_flow_on_gateway(&config.host, config.port, Path::new(&flow_path)).await?;

    let id = format!("{}-{}", config.name, &uuid::Uuid::new_v4().to_string()[..8]);
    let instance = McpServerInstance {
        id: id.clone(),
        name: config.name,
        pid: gateway.pid,
        transport: config.transport,
        host: config.host,
        port: config.port,
        flow_target: Some(flow_path),
        dir_target: None,
        tools_count: 1,
        started_at: chrono::Utc::now().timestamp_millis() as u64,
        managed_by: config.managed_by,
        status: "running".to_string(),
        tool_name: Some(registration.tool_name),
        slug: Some(registration.slug),
        shared: true,
    };

    save_instance(&ServiceInstance::from(instance.clone()))?;
    Ok(instance)
}

/// Spawn a dedicated MCP server process owning its own port.
async fn start_dedicated_instance(
    config: McpStartConfig,
    resolved_flow: Option<String>,
) -> Result<McpServerInstance, String> {
    let id = format!("{}-{}", config.name, &uuid::Uuid::new_v4().to_string()[..8]);

    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to determine current executable: {e}"))?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("mcp").arg("serve");
    if let Some(ref flow) = resolved_flow {
        cmd.arg("--flow").arg(flow);
    }
    if let Some(ref dir) = config.dir_target {
        cmd.arg("--dir").arg(dir);
    }
    cmd.arg("--transport").arg(&config.transport);
    cmd.arg("--host").arg(&config.host);
    cmd.arg("--port").arg(config.port.to_string());
    cmd.arg("--timeout-ms").arg(config.timeout_ms.to_string());

    let services_dir = get_services_dir();
    let (mut child, log_path) = spawn_daemon(&mut cmd, &id, &services_dir)?;
    let pid = child.id();

    let started_at = chrono::Utc::now().timestamp_millis() as u64;
    let mut tools_count = 0;
    let mut tool_name = None;

    if config.transport == "streamable" {
        // Wait up to 3 seconds for health probe
        let mut healthy = false;
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Immediately catch early exit/crash with error log output
            check_early_crash(&mut child, &log_path)?;

            if let Some(count) = probe_server_health(&config.host, config.port).await {
                tools_count = count;
                healthy = true;
                break;
            }
        }
        if !healthy {
            terminate_process(pid);
            return Err(format!(
                "MCP server failed to answer health probe on {}:{} within 3s",
                config.host, config.port
            ));
        }
    } else if let Some(ref flow) = resolved_flow {
        if let Ok(inspection) = inspect_flow_tools(flow) {
            tools_count = 1;
            tool_name = Some(inspection.tool_name);
        }
    }

    let instance = McpServerInstance {
        id: id.clone(),
        name: config.name,
        pid,
        transport: config.transport,
        host: config.host,
        port: config.port,
        flow_target: resolved_flow,
        dir_target: config.dir_target,
        tools_count,
        started_at,
        managed_by: config.managed_by,
        status: "running".to_string(),
        slug: tool_name.as_deref().map(slug_for_tool),
        tool_name,
        shared: false,
    };

    save_instance(&ServiceInstance::from(instance.clone()))?;
    Ok(instance)
}

/// Find a published instance by ID, PID, port, or name.
fn find_instance<'a>(
    servers: &'a [McpServerInstance],
    id_or_name: &str,
) -> Option<&'a McpServerInstance> {
    servers.iter().find(|s| {
        s.id == id_or_name
            || s.name == id_or_name
            || (s.pid != 0 && s.pid.to_string() == id_or_name)
            || s.port.to_string() == id_or_name
    })
}

/// Stop a published MCP server **without dropping its record**, so it can be
/// restored later (`restore_published`, GUI startup reconcile, `mcp restore`).
///
/// Shared-gateway flows are detached from the gateway; the gateway process is
/// only torn down once it hosts no flows anymore. Use [`remove_server`] to
/// unpublish and delete the record.
pub async fn stop_server(id_or_name: &str) -> Result<bool, String> {
    let servers = list_servers(false).await;
    let found = find_instance(&servers, id_or_name)
        .ok_or_else(|| format!("No published MCP server found matching '{id_or_name}'"))?;

    if found.status != "running" {
        // Already stopped: nothing to terminate, record stays published.
        return Ok(false);
    }

    let mut stopped = found.clone();

    if stopped.shared {
        let flow_path = stopped.flow_target.as_deref().map(Path::new);
        let handle = stopped
            .tool_name
            .clone()
            .or_else(|| stopped.slug.clone())
            .unwrap_or_else(|| stopped.name.clone());

        let remaining = unregister_flow_from_gateway(
            &stopped.host,
            stopped.port,
            flow_path,
            Some(&handle),
        )
        .await
        .unwrap_or(0);

        if remaining == 0 {
            let gateway = gateway_id(stopped.port);
            if let Some(gw) = load_instance(&gateway) {
                terminate_process(gw.pid);
                delete_instance(&gateway);
            }
        }
    } else {
        terminate_process(stopped.pid);
    }

    stopped.status = "stopped".to_string();
    stopped.pid = 0;
    save_instance(&ServiceInstance::from(stopped))?;
    Ok(true)
}

/// Unpublish an MCP server: stop it (if running) and delete its record.
pub async fn remove_server(id_or_name: &str) -> Result<bool, String> {
    let servers = list_servers(false).await;
    let found = find_instance(&servers, id_or_name)
        .ok_or_else(|| format!("No published MCP server found matching '{id_or_name}'"))?;

    let id = found.id.clone();
    if found.status == "running" {
        let _ = stop_server(&id).await;
    }
    delete_instance(&id);
    Ok(true)
}

/// Stop every published MCP server, keeping their records restorable.
pub async fn stop_all(keep_records: bool) -> usize {
    let servers = list_servers(false).await;
    let mut count = 0;
    for server in servers {
        if server.status != "running" {
            continue;
        }
        let outcome = if keep_records {
            stop_server(&server.id).await
        } else {
            remove_server(&server.id).await
        };
        if matches!(outcome, Ok(true)) {
            count += 1;
        }
    }
    count
}

/// Reload an MCP server instance
pub async fn reload_server(id_or_name: &str) -> Result<bool, String> {
    let servers = list_servers(false).await;
    let target = servers
        .iter()
        .find(|s| s.id == id_or_name || s.name == id_or_name)
        .ok_or_else(|| format!("Server '{id_or_name}' not found"))?;

    if target.transport == "streamable" {
        let path = match target.slug.as_deref() {
            Some(slug) if !slug.is_empty() => format!("/{slug}/reload"),
            _ => "/reload".to_string(),
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(2000))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let url = format!("http://{}:{}{}", target.host, target.port, path);
        let resp = client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to send reload request: {e}"))?;

        if resp.status().is_success() {
            let val: serde_json::Value = resp.json().await.unwrap_or_default();
            let new_count = val
                .get("tools_count")
                .and_then(|c| c.as_u64())
                .unwrap_or(target.tools_count as u64) as usize;
            let mut updated = target.clone();
            updated.tools_count = new_count;

            let _ = save_instance(&ServiceInstance::from(updated));
            Ok(true)
        } else {
            Err(format!("Reload endpoint answered status {}", resp.status()))
        }
    } else {
        Ok(true)
    }
}

/// Inspect a flow file to determine its tool name and parameters
pub fn inspect_flow_tools(flow_path: &str) -> Result<McpToolInspection, String> {
    let resolved = resolve_flow_path(flow_path);
    if !resolved.exists() {
        return Err(format!("File does not exist: {}", resolved.display()));
    }

    let content = fs::read_to_string(&resolved)
        .map_err(|e| format!("Failed to read {}: {e}", resolved.display()))?;

    let flow = parse_flow_content(&content)
        .map_err(|e| format!("Failed to parse flow from {}: {e}", resolved.display()))?;

    let name = flow.name.trim();
    let description = flow
        .description
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();

    let tool_name = if !name.is_empty() {
        sanitize_tool_name(name)
    } else if let Some(stem) = resolved.file_stem().and_then(|s| s.to_str()) {
        sanitize_tool_name(stem)
    } else {
        "mpe_flow".to_string()
    };

    let tool_def = build_tool_definition(&tool_name, &flow);
    let mut variables = Vec::new();

    if let Some(props) = tool_def.input_schema.get("properties").and_then(|p| p.as_object()) {
        for key in props.keys() {
            variables.push(key.clone());
        }
    }

    Ok(McpToolInspection {
        tool_name,
        description: if description.is_empty() {
            format!("Execute flow '{name}'")
        } else {
            description
        },
        input_schema: tool_def.input_schema,
        variables,
    })
}

/// Generate client config JSON for Claude Desktop and Cursor
pub fn get_client_config(
    server: &McpServerInstance,
    binary_path: Option<&str>,
) -> McpClientSnippets {
    let server_key = &server.name;

    let (claude_val, cursor_val) = if server.transport == "streamable" {
        let url = server.endpoint();
        let c = serde_json::json!({
            "mcpServers": {
                server_key: {
                    "url": url
                }
            }
        });
        (c.clone(), c)
    } else {
        let bin = binary_path.unwrap_or("mpe");
        let mut args = if bin.contains("mpe-plugin-mcp") {
            vec!["--server".to_string()]
        } else {
            vec!["mcp".to_string(), "serve".to_string()]
        };

        if let Some(ref flow) = server.flow_target {
            args.push("--flow".to_string());
            args.push(flow.clone());
        } else if let Some(ref dir) = server.dir_target {
            args.push("--dir".to_string());
            args.push(dir.clone());
        }
        args.push("--transport".to_string());
        args.push("stdio".to_string());

        let c = serde_json::json!({
            "mcpServers": {
                server_key: {
                    "command": bin,
                    "args": args
                }
            }
        });
        (c.clone(), c)
    };

    McpClientSnippets {
        claude_desktop: claude_val,
        cursor: cursor_val,
    }
}
