//! Metadata persistence, directory layout, and zombie self-healing store.
#![allow(missing_docs)]

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::process::{is_process_alive, terminate_process};
use super::probe::{is_in_grace_period, probe_http_health, DEFAULT_GRACE_PERIOD_MS, DEFAULT_PROBE_TIMEOUT};
use super::{ServiceInstance, ServiceKind};

/// Get primary services storage directory (`~/.mpe/services/`).
pub fn get_services_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MPE_HOME") {
        return PathBuf::from(dir).join("services");
    }
    if cfg!(target_os = "windows") {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data).join("mpe").join("services");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".mpe").join("services");
    }
    PathBuf::from(".mpe").join("services")
}

/// Get legacy MCP servers storage directory (`~/.mpe/mcp-servers/`) for soft backwards-compatibility.
pub fn get_legacy_mcp_servers_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MPE_HOME") {
        return PathBuf::from(dir).join("mcp-servers");
    }
    if cfg!(target_os = "windows") {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data).join("mpe").join("mcp-servers");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".mpe").join("mcp-servers");
    }
    PathBuf::from(".mpe").join("mcp-servers")
}

/// Save a service instance metadata to disk.
pub fn save_instance(instance: &ServiceInstance) -> Result<(), String> {
    let dir = get_services_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create services dir: {e}"))?;

    let path = dir.join(format!("{}.json", instance.id));
    let bytes = serde_json::to_vec_pretty(instance)
        .map_err(|e| format!("Serialization error for instance {}: {e}", instance.id))?;
    fs::write(path, bytes).map_err(|e| format!("Failed to write instance metadata: {e}"))
}

/// Load a service instance metadata by ID.
pub fn load_instance(id: &str) -> Option<ServiceInstance> {
    let services_dir = get_services_dir();
    let path = services_dir.join(format!("{id}.json"));
    if path.is_file() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(inst) = serde_json::from_str::<ServiceInstance>(&content) {
                return Some(inst);
            }
        }
    }

    // Try legacy directory
    let legacy_dir = get_legacy_mcp_servers_dir();
    let legacy_path = legacy_dir.join(format!("{id}.json"));
    if legacy_path.is_file() {
        if let Ok(content) = fs::read_to_string(&legacy_path) {
            if let Ok(inst) = serde_json::from_str::<ServiceInstance>(&content) {
                return Some(inst);
            }
        }
    }

    None
}

/// Delete service instance metadata and its log file from both current and legacy directories.
pub fn delete_instance(id: &str) -> bool {
    let mut deleted = false;
    let dirs = [get_services_dir(), get_legacy_mcp_servers_dir()];

    for dir in &dirs {
        let json_path = dir.join(format!("{id}.json"));
        let log_path = dir.join(format!("{id}.log"));

        if json_path.exists() {
            let _ = fs::remove_file(json_path);
            deleted = true;
        }
        if log_path.exists() {
            let _ = fs::remove_file(log_path);
        }
    }

    deleted
}

/// List all managed service instances with automatic zombie self-healing.
pub async fn list_instances(
    filter_kind: Option<ServiceKind>,
    heal: bool,
) -> Vec<ServiceInstance> {
    let mut instances = Vec::new();
    let mut seen_ids = HashSet::new();

    let dirs = [get_services_dir(), get_legacy_mcp_servers_dir()];

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut instance: ServiceInstance = match serde_json::from_str(&content) {
                Ok(i) => i,
                Err(_) => {
                    if heal {
                        let _ = fs::remove_file(&path);
                    }
                    continue;
                }
            };

            if !seen_ids.insert(instance.id.clone()) {
                // Already loaded from primary directory
                continue;
            }

            let alive = is_process_alive(instance.pid);
            if !alive {
                // The record is the *published intent*: it survives app restarts,
                // crashes and reboots so `mcp restore` can bring the flow back.
                // Only an explicit stop deletes it.
                if instance.status != "stopped" || instance.pid != 0 {
                    instance.status = "stopped".to_string();
                    instance.pid = 0;
                    let _ = save_instance(&instance);
                }
            } else if instance.transport == "streamable" || instance.transport == "http" {
                let health_path = instance
                    .metadata
                    .get("health_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/health")
                    .to_string();

                if let Some(resp) = probe_http_health(
                    &instance.host,
                    instance.port,
                    &health_path,
                    DEFAULT_PROBE_TIMEOUT,
                )
                .await
                {
                    if let Some(count) = resp.get("tools_count").and_then(|c| c.as_u64()) {
                        instance.tools_count = count as usize;
                    }
                    instance.status = "running".to_string();
                } else {
                    let is_fresh = is_in_grace_period(instance.started_at, DEFAULT_GRACE_PERIOD_MS);
                    if heal && !is_fresh {
                        log::warn!(
                            "[Supervisor] Unresponsive port {} for PID {} ({}). Terminating; record kept for restore.",
                            instance.port,
                            instance.pid,
                            instance.id
                        );
                        terminate_process(instance.pid);
                        instance.status = "stopped".to_string();
                        instance.pid = 0;
                        let _ = save_instance(&instance);
                    } else {
                        instance.status = if is_fresh {
                            "running".to_string()
                        } else {
                            "error".to_string()
                        };
                    }
                }
            } else {
                instance.status = "running".to_string();
            }

            if let Some(kind) = filter_kind {
                if instance.kind != kind {
                    continue;
                }
            }

            instances.push(instance);
        }
    }

    instances.sort_by_key(|a| std::cmp::Reverse(a.started_at));
    instances
}
