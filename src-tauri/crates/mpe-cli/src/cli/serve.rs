//! `mpe serve` subcommand — long-running HTTP Ingress mock server runtime
//!
//! Hosts multi-protocol flows via shared gateway routes and custom port listeners,
//! executes lifecycle Setup on boot, processes concurrent requests, and performs
//! Teardown during graceful shutdown.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use flow_engine::ingress::{IngressManager, IngressRouter};
use flow_engine::node_types::HTTP_INGRESS;
use flow_engine::registry::NodeRegistry;
use flow_engine::report::ExecutionStatus;

use super::config::AppConfig;
use super::output::CliOutput;
use super::variables::{parse_vars, resolve_execution_variables};
use super::CliError;

/// Discover flow files in directory recursively
fn find_flow_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(find_flow_files(&p));
            } else if p.is_file() {
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "mpf"
                        || ext_lower == "json"
                        || ext_lower == "yaml"
                        || ext_lower == "yml"
                    {
                        files.push(p);
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// Execute long-running serve runtime
pub async fn execute(
    format: &str,
    flow_path: &str,
    port_override: Option<u16>,
    env: Option<String>,
    vars: &[String],
    graceful_timeout: u64,
    registry: Arc<NodeRegistry>,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), CliError> {
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(CliError::InvalidArgs(format!(
            "Specified flow path does not exist: {}",
            flow_path
        )));
    }

    // Load config (to ensure directories are set)
    let app_config = AppConfig::load().map_err(CliError::Config)?;
    flow_engine::storage::set_flows_dir_override(Some(app_config.flows_dir.clone()));
    flow_engine::storage::set_environments_dir_override(Some(app_config.environments_dir.clone()));

    // 1. 发现并载入 Flow
    let mut target_flow_paths = Vec::new();
    if path.is_file() {
        target_flow_paths.push(path.clone());
    } else if path.is_dir() {
        target_flow_paths = find_flow_files(&path);
    }

    let mut loaded_flows = Vec::new();
    for p in &target_flow_paths {
        let p_str = p.to_string_lossy();
        match flow_engine::storage::load_flow(&p_str) {
            Ok(flow) => {
                let has_ingress = flow
                    .nodes
                    .iter()
                    .any(|n| n.node_type == "http:ingress" || n.node_type == HTTP_INGRESS);
                if has_ingress {
                    loaded_flows.push(flow);
                } else if path.is_file() {
                    return Err(CliError::InvalidArgs(format!(
                        "Flow '{}' does not contain any http:ingress node",
                        flow_path
                    )));
                }
            }
            Err(e) => {
                if path.is_file() {
                    return Err(CliError::InvalidArgs(format!("Failed to load flow: {}", e)));
                } else {
                    log::warn!("[Serve] Failed to load flow file '{}': {}", p_str, e);
                }
            }
        }
    }

    if loaded_flows.is_empty() {
        return Err(CliError::InvalidArgs(format!(
            "No valid flows containing http:ingress nodes found in '{}'",
            flow_path
        )));
    }

    // 2. 变量解析与 Setup 阶段单次执行
    let cli_vars = if vars.is_empty() {
        None
    } else {
        Some(parse_vars(vars)?)
    };

    for flow in &mut loaded_flows {
        let initial_val = resolve_execution_variables(flow, env.as_deref(), cli_vars.as_ref())?;
        let initial_vars: HashMap<String, serde_json::Value> = match initial_val {
            Some(serde_json::Value::Object(map)) => map.into_iter().collect(),
            _ => flow.get_active_variables().unwrap_or_default(),
        };

        if let Some(setup_subgraph) = &flow.setup {
            if !setup_subgraph.is_empty() {
                log::info!(
                    "[Serve] Executing Setup stage for flow '{}' ({})",
                    flow.name,
                    flow.uuid
                );
                let executor =
                    flow_engine::executor::FlowExecutor::new().with_registry(registry.clone());
                let (updated_vars, report) = flow_engine::subgraph_runner::execute_subgraph(
                    setup_subgraph,
                    initial_vars,
                    &executor,
                    None,
                )
                .await
                .map_err(|e| {
                    CliError::execution(format!(
                        "Setup stage execution error for flow '{}': {}",
                        flow.name, e
                    ))
                })?;

                if report.status == ExecutionStatus::Failed {
                    let err_msg = report
                        .error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "unknown error".to_string());
                    return Err(CliError::execution(format!(
                        "Setup stage failed for flow '{}': {}",
                        flow.name, err_msg
                    )));
                }

                flow.initial_variables = Some(updated_vars);
            } else {
                flow.initial_variables = Some(initial_vars);
            }
        } else {
            flow.initial_variables = Some(initial_vars);
        }
    }

    // 3. 构建路由挂载计划（IngressRouter）
    let effective_port_override = port_override.or(Some(app_config.ingress_port));
    let plan = IngressRouter::plan_from_flows(&loaded_flows, effective_port_override)
        .map_err(CliError::InvalidArgs)?;

    if plan.port_routers.is_empty() {
        return Err(CliError::InvalidArgs(
            "No ingress routes could be planned from specified flow(s)".to_string(),
        ));
    }

    // 4. 启动监听器
    let ingress_manager = Arc::new(IngressManager::new());
    for (port, router) in plan.port_routers {
        ingress_manager
            .start_router_listener(
                port,
                router,
                registry.clone(),
                None,
                Duration::from_secs(60),
            )
            .await
            .map_err(CliError::execution)?;
    }

    #[derive(serde::Serialize)]
    struct RouteInfo {
        flow: String,
        port: u16,
        prefix: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct ServeStartedInfo {
        status: String,
        default_port: u16,
        routes: Vec<RouteInfo>,
    }

    let route_infos: Vec<RouteInfo> = plan
        .mounted_routes
        .iter()
        .map(|(flow_name, port, prefix)| RouteInfo {
            flow: flow_name.clone(),
            port: *port,
            prefix: prefix.clone(),
        })
        .collect();

    let started_info = ServeStartedInfo {
        status: "running".to_string(),
        default_port: port_override.unwrap_or(app_config.ingress_port),
        routes: route_infos,
    };

    if format == "json" {
        let output = CliOutput::success(started_info);
        super::output::output_json(&output);
    } else {
        println!("🚀 Ingress mock server started successfully!");
        for (flow_name, port, prefix) in &plan.mounted_routes {
            if let Some(pfx) = prefix {
                println!(
                    "  • http://0.0.0.0:{}/{}/* -> Flow '{}'",
                    port, pfx, flow_name
                );
            } else {
                println!("  • http://0.0.0.0:{}/* -> Flow '{}'", port, flow_name);
            }
        }
        println!("\nPress Ctrl+C to stop.");
    }

    // 5. 监听退出信号
    tokio::select! {
        res = tokio::signal::ctrl_c() => {
            if let Err(e) = res {
                log::warn!("[Serve] Failed to listen for Ctrl+C signal: {}", e);
            } else {
                log::info!("[Serve] Received Ctrl+C shutdown signal");
            }
        }
        _ = cancel_rx.changed() => {
            log::info!("[Serve] Received cancel notification");
        }
    }

    // 6. 优雅停机与 Teardown
    log::info!("[Serve] Stopping ingress listeners...");
    ingress_manager.stop_all().await;

    log::info!(
        "[Serve] Waiting up to {}s for in-flight requests...",
        graceful_timeout
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 执行各流程的 Teardown 子图
    for flow in &loaded_flows {
        if let Some(teardown_subgraph) = &flow.teardown {
            if !teardown_subgraph.is_empty() {
                log::info!(
                    "[Serve] Executing Teardown stage for flow '{}' ({})",
                    flow.name,
                    flow.uuid
                );
                let executor =
                    flow_engine::executor::FlowExecutor::new().with_registry(registry.clone());
                let vars = flow.get_active_variables().unwrap_or_default();
                match flow_engine::subgraph_runner::execute_subgraph(
                    teardown_subgraph,
                    vars,
                    &executor,
                    None,
                )
                .await
                {
                    Ok((_, report)) => {
                        log::info!(
                            "[Serve] Teardown completed for flow '{}' with status: {:?}",
                            flow.name,
                            report.status
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "[Serve] Teardown execution error for flow '{}': {}",
                            flow.name,
                            e
                        );
                    }
                }
            }
        }
    }

    log::info!("[Serve] All listeners and background stages finished. Graceful shutdown complete.");
    Ok(())
}
