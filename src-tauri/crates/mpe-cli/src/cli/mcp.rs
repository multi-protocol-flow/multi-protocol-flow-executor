//! mpe mcp subcommand — Model Context Protocol sidecar lifecycle and flow governance

use std::path::{Path, PathBuf};

use flow_engine::mcp_server::{run_mcp_server, McpServerArgs, DEFAULT_MCP_PORT};
use flow_engine::mcp_supervisor::{
    get_client_config, inspect_flow_tools, list_servers, reload_server,
    start_server, stop_server, McpServerInstance, McpStartConfig,
};

use super::{plugins_dir, CliError, McpCommands};

/// Execute MCP command
pub async fn execute(command: McpCommands) -> Result<(), CliError> {
    match command {
        McpCommands::Serve {
            flow,
            dir,
            gateway,
            port,
            host,
            transport,
            timeout_ms,
        } => execute_serve(flow, dir, gateway, port, host, transport, timeout_ms).await,
        McpCommands::Start {
            flow,
            dir,
            port,
            shared,
            host,
            transport,
            timeout_ms,
            name,
            detach,
        } => {
            execute_start(
                flow, dir, port, shared, host, transport, timeout_ms, name, detach,
            )
            .await
        }
        McpCommands::List { all } => execute_list(all).await,
        McpCommands::Stop { target, all, port } => execute_stop(target, all, port).await,
        McpCommands::Restore { target } => execute_restore(target).await,
        McpCommands::Remove { target, all } => execute_remove(target, all).await,
        McpCommands::Restart { target } => execute_restart(&target).await,
        McpCommands::Reload { target } => execute_reload(&target).await,
        McpCommands::ConfigSnippet { target } => execute_config_snippet(&target).await,
    }
}

async fn execute_serve(
    flow: Option<String>,
    dir: Option<String>,
    gateway: bool,
    port: u16,
    host: String,
    transport: String,
    timeout_ms: u64,
) -> Result<(), CliError> {
    if !gateway && flow.is_none() && dir.is_none() {
        return Err(CliError::InvalidArgs(
            "Please specify --flow <path>, --dir <path>, or --gateway".to_string(),
        ));
    }

    let args = McpServerArgs {
        flow: flow.map(PathBuf::from),
        dir: dir.map(PathBuf::from),
        transport,
        host,
        port,
        timeout_ms,
    };

    run_mcp_server(args)
        .await
        .map_err(CliError::execution)
}

#[allow(clippy::too_many_arguments)]
async fn execute_start(
    flow: Option<String>,
    dir: Option<String>,
    port: Option<u16>,
    shared: bool,
    host: String,
    transport: String,
    timeout_ms: u64,
    name: Option<String>,
    detach: bool,
) -> Result<(), CliError> {
    if flow.is_none() && dir.is_none() {
        return Err(CliError::InvalidArgs(
            "Please specify either --flow <path> or --dir <path>".to_string(),
        ));
    }

    // No explicit port (or --shared) → join the shared gateway on the default port.
    let shared_mode = shared || port.is_none();
    let effective_port = port.unwrap_or(DEFAULT_MCP_PORT);

    // If running in foreground (e.g. stdio or foreground streamable), run directly
    if !detach {
        return execute_serve(
            flow,
            dir,
            false,
            effective_port,
            host,
            transport,
            timeout_ms,
        )
        .await;
    }

    let default_name = if let Some(ref f) = flow {
        Path::new(f)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("flow-mcp")
            .to_string()
    } else {
        "mpe-mcp-cluster".to_string()
    };

    let server_name = name.unwrap_or(default_name);

    let start_config = McpStartConfig {
        name: server_name,
        flow_target: flow,
        dir_target: dir,
        transport,
        host,
        port: effective_port,
        timeout_ms,
        managed_by: "cli".to_string(),
        shared: shared_mode,
    };

    println!("Starting MCP server '{}'...", start_config.name);
    let p_dir = plugins_dir();
    let instance = start_server(start_config, &p_dir)
        .await
        .map_err(CliError::execution)?;

    println!("\x1b[32m✔ MCP Server started successfully!\x1b[0m");
    println!("  ID:        {}", instance.id);
    println!("  Name:      {}", instance.name);
    println!("  PID:       {}", instance.pid);
    println!("  Mode:      {}", if instance.shared { "shared gateway" } else { "dedicated" });
    println!("  Transport: {}", instance.transport);
    if instance.transport == "streamable" {
        println!("  Endpoint:  {}", instance.endpoint());
        if let Some(scoped) = instance.scoped_endpoint() {
            println!("  Scoped:    {scoped}");
        }
        println!("  Health:    http://{}:{}/health", instance.host, instance.port);
    }
    println!("  Tools:     {} exposed", instance.tools_count);
    println!("\nView running servers with: \x1b[36mmpe mcp list\x1b[0m");
    println!("Stop this server with:     \x1b[36mmpe mcp stop {}\x1b[0m", instance.id);

    Ok(())
}

async fn execute_list(all: bool) -> Result<(), CliError> {
    let mut servers = list_servers(true).await;
    if !all {
        servers.retain(|s| s.status == "running");
    }

    if servers.is_empty() {
        if all {
            println!("No MCP servers found.");
        } else {
            println!("No active MCP servers found (use --all to show stopped).");
        }
        println!("Start one with: \x1b[36mmpe mcp start --flow <flow_file>\x1b[0m");
        return Ok(());
    }

    println!(
        "\x1b[1m{:<20} {:<16} {:<8} {:<12} {:<24} {:<8} {:<10}\x1b[0m",
        "ID", "NAME", "PID", "TRANSPORT", "ENDPOINT / TARGET", "TOOLS", "STATUS"
    );
    println!("{}", "-".repeat(98));

    for s in servers {
        let endpoint = if s.transport == "streamable" {
            truncate_str(&s.endpoint(), 24)
        } else if let Some(ref f) = s.flow_target {
            truncate_str(f, 24)
        } else if let Some(ref d) = s.dir_target {
            truncate_str(d, 24)
        } else {
            "-".to_string()
        };

        let status_colored = if s.status == "running" {
            "\x1b[32mrunning\x1b[0m"
        } else {
            "\x1b[31mstopped\x1b[0m"
        };

        println!(
            "{:<20} {:<16} {:<8} {:<12} {:<24} {:<8} {}",
            truncate_str(&s.id, 19),
            truncate_str(&s.name, 15),
            s.pid,
            s.transport,
            endpoint,
            s.tools_count,
            status_colored
        );
    }

    Ok(())
}

async fn execute_stop(
    target: Option<String>,
    all: bool,
    port: Option<u16>,
) -> Result<(), CliError> {
    let servers = list_servers(false).await;

    if all {
        println!("Stopping all published MCP server(s) (records kept for restore)...");
        for s in servers {
            if s.status != "running" {
                continue;
            }
            let _ = stop_server(&s.id).await;
            println!("  \x1b[32m✔\x1b[0m Stopped {} (PID {})", s.name, s.pid);
        }
        return Ok(());
    }

    if let Some(p) = port {
        let matching: Vec<_> = servers.into_iter().filter(|s| s.port == p).collect();
        if matching.is_empty() {
            return Err(CliError::execution(format!("No server found listening on port {p}")));
        }
        for s in matching {
            let _ = stop_server(&s.id).await;
            println!("  \x1b[32m✔\x1b[0m Stopped {} on port {p} (PID {})", s.name, s.pid);
        }
        return Ok(());
    }

    let tgt = target.ok_or_else(|| {
        CliError::InvalidArgs("Please provide a server NAME/ID, --all, or --port".to_string())
    })?;

    let stopped = stop_server(&tgt)
        .await
        .map_err(CliError::execution)?;

    if stopped {
        println!("\x1b[32m✔ Stopped MCP server '{}'\x1b[0m", tgt);
        println!("  Published record kept — restore with: \x1b[36mmpe mcp restore {tgt}\x1b[0m");
    } else {
        println!("Server '{tgt}' is already stopped (record kept, restorable).");
    }

    Ok(())
}

async fn execute_remove(target: Option<String>, all: bool) -> Result<(), CliError> {
    let servers = list_servers(false).await;

    if all {
        if servers.is_empty() {
            println!("No published MCP servers to remove.");
            return Ok(());
        }
        for s in servers {
            let _ = flow_engine::mcp_supervisor::remove_server(&s.id).await;
            println!("  \x1b[32m✔\x1b[0m Unpublished {}", s.name);
        }
        return Ok(());
    }

    let tgt = target.ok_or_else(|| {
        CliError::InvalidArgs("Please provide a server NAME/ID or --all".to_string())
    })?;

    flow_engine::mcp_supervisor::remove_server(&tgt)
        .await
        .map_err(CliError::execution)?;
    println!("\x1b[32m✔ Unpublished MCP server '{tgt}' (record deleted)\x1b[0m");
    Ok(())
}

async fn execute_restore(target: Option<String>) -> Result<(), CliError> {
    let ids = match target {
        Some(t) => {
            let servers = list_servers(false).await;
            let found = servers
                .into_iter()
                .find(|s| s.id == t || s.name == t)
                .ok_or_else(|| CliError::execution(format!("No published MCP server found matching '{t}'")))?;
            Some(vec![found.id])
        }
        None => None,
    };

    let (restored, failures) = flow_engine::mcp_supervisor::restore_published(ids.as_deref()).await;

    if restored.is_empty() {
        println!("Nothing to restore: every published MCP server is already running.");
    }
    for instance in &restored {
        println!(
            "\x1b[32m✔\x1b[0m Restored {} -> {}",
            instance.name,
            instance.endpoint()
        );
    }
    for failure in &failures {
        println!("\x1b[31m✘\x1b[0m Failed to restore {failure}");
    }
    if restored.is_empty() && !failures.is_empty() {
        return Err(CliError::execution("Failed to restore published MCP servers".to_string()));
    }
    Ok(())
}

async fn execute_restart(target: &str) -> Result<(), CliError> {
    let servers = list_servers(false).await;
    let s = servers
        .into_iter()
        .find(|srv| srv.id == target || srv.name == target || srv.pid.to_string() == target)
        .ok_or_else(|| CliError::execution(format!("Server '{target}' not found")))?;

    println!("Restarting server '{}' (PID {})...", s.name, s.pid);
    let config = McpStartConfig {
        name: s.name,
        flow_target: s.flow_target,
        dir_target: s.dir_target,
        transport: s.transport,
        host: s.host,
        port: s.port,
        timeout_ms: 28000,
        managed_by: s.managed_by,
        shared: s.shared,
    };

    let _ = stop_server(&s.id).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let p_dir = plugins_dir();
    let restarted = start_server(config, &p_dir)
        .await
        .map_err(CliError::execution)?;

    println!("\x1b[32m✔ Server restarted with new PID {}\x1b[0m", restarted.pid);
    Ok(())
}

async fn execute_reload(target: &str) -> Result<(), CliError> {
    println!("Reloading flows for server '{target}'...");
    let reloaded = reload_server(target)
        .await
        .map_err(CliError::execution)?;

    if reloaded {
        println!("\x1b[32m✔ Server '{target}' successfully reloaded tools.\x1b[0m");
    } else {
        println!("Server '{target}' reload had no changes.");
    }
    Ok(())
}

async fn execute_config_snippet(target: &str) -> Result<(), CliError> {
    let host_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "mpe".to_string());
    let bin_path = host_bin.as_str();

    // Check if target matches a running server
    let servers = list_servers(false).await;
    let instance = servers.into_iter().find(|s| s.id == target || s.name == target);

    let snippets = if let Some(inst) = instance {
        get_client_config(&inst, Some(bin_path))
    } else if Path::new(target).exists() {
        // Target is a flow file path
        let insp = inspect_flow_tools(target).map_err(CliError::execution)?;
        let pseudo_server = McpServerInstance {
            id: insp.tool_name.clone(),
            name: insp.tool_name.clone(),
            pid: 0,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: DEFAULT_MCP_PORT,
            flow_target: Some(target.to_string()),
            dir_target: None,
            tools_count: 1,
            started_at: 0,
            managed_by: "cli".to_string(),
            status: "configured".to_string(),
            tool_name: Some(insp.tool_name),
            slug: None,
            shared: false,
        };
        get_client_config(&pseudo_server, Some(bin_path))
    } else {
        return Err(CliError::execution(format!(
            "Target '{target}' is neither a running server nor a valid flow file path"
        )));
    };

    println!("\n\x1b[1m=== Claude Desktop Configuration (claude_desktop_config.json) ===\x1b[0m");
    let claude_str = serde_json::to_string_pretty(&snippets.claude_desktop)
        .map_err(|e| CliError::execution(format!("Failed to serialize Claude config: {e}")))?;
    println!("{claude_str}");

    println!("\n\x1b[1m=== Cursor Configuration (.cursor/mcp.json) ===\x1b[0m");
    let cursor_str = serde_json::to_string_pretty(&snippets.cursor)
        .map_err(|e| CliError::execution(format!("Failed to serialize Cursor config: {e}")))?;
    println!("{cursor_str}");

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
