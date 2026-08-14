//! Cross-runtime conformance helpers (M5b).
//!
//! Spawns the Python/Node templates and the compiled Rust SDK demo plugin
//! through the real `process::spawn_plugin` path and drives them over a real
//! `PluginConnection`. Each `*_conformance` test skips itself (with a log)
//! when its interpreter or binary is unavailable — the echo-mock tests stay
//! the always-run baseline.
//!
//! Each integration-test binary compiles this module separately and uses a
//! different subset of its helpers, so dead-code is expected per target.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use flow_engine_plugin::describe::perform_describe;
use flow_engine_plugin::process::{spawn_plugin, PluginProcess};
use flow_engine_plugin::transport::{ConnectionConfig, PluginConnection};
use mpe_plugin_sdk::protocol::describe::DescribeResult;
use mpe_plugin_sdk::protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};

/// Repository root (three levels above the crate dir).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// True when `command --version` exits 0, i.e. the interpreter is usable.
pub fn interpreter_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Path to an `sdk-templates/<kind>/<script>` file, if present.
pub fn template_script(kind: &str, script: &str) -> Option<PathBuf> {
    let path = repo_root().join("sdk-templates").join(kind).join(script);
    path.is_file().then_some(path)
}

/// Path to the compiled Rust SDK demo plugin binary, if available.
///
/// The SDK lives in its own repository now (not a workspace member), so its
/// binary no longer lands in the workspace target dir. Set
/// `MPE_DEMO_PLUGIN_BIN` to the built `mpe_demo_plugin` path (build it in
/// the SDK repo with `cargo build --bin mpe_demo_plugin`); without it the
/// legacy workspace-target scan always misses and the SDK conformance test
/// is skipped with a log rather than failed.
pub fn sdk_plugin_bin() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("MPE_DEMO_PLUGIN_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        eprintln!("MPE_DEMO_PLUGIN_BIN set but not a file: {}", path.display());
        return None;
    }
    let exe = if cfg!(windows) {
        "mpe_demo_plugin.exe"
    } else {
        "mpe_demo_plugin"
    };
    for profile in ["debug", "release"] {
        let path = repo_root()
            .join("src-tauri")
            .join("target")
            .join(profile)
            .join(exe);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Spawns a plugin by absolute command path with no extra env.
pub fn spawn_absolute(
    command: &Path,
) -> Result<PluginProcess, flow_engine_plugin::error::PluginError> {
    let entry = PluginEntry {
        command: command.to_string_lossy().to_string(),
        args: Vec::new(),
    };
    spawn_plugin(&entry, &HashMap::new())
}

/// Spawns an interpreter running a template script (e.g. `python plugin.py`).
pub fn spawn_interpreter(
    interpreter: &str,
    script: &Path,
) -> Result<PluginProcess, flow_engine_plugin::error::PluginError> {
    let entry = PluginEntry {
        command: interpreter.to_string(),
        args: vec![script.to_string_lossy().to_string()],
    };
    spawn_plugin(&entry, &HashMap::new())
}

/// Connection config tuned for conformance (short timeout, fast failures).
pub fn conformance_config() -> ConnectionConfig {
    ConnectionConfig {
        request_timeout: std::time::Duration::from_secs(10),
        ..ConnectionConfig::default()
    }
}

/// A fallback manifest for SDK plugins whose describe answers with the
/// array shape (no manifest echo).
pub fn fallback_manifest(name: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_string(),
        version: "0.0.0".to_string(),
        author: None,
        min_host_version: None,
        description: None,
        entry: PluginEntry {
            command: name.to_string(),
            args: Vec::new(),
        },
        env: HashMap::new(),
        permissions: Vec::new(),
        capabilities: PluginCapabilities::default(),
        locales: Vec::new(),
        default_locale: None,
    }
}

/// Runs the describe handshake, accepting both wire shapes.
pub async fn describe_from(
    connection: &PluginConnection,
    name: &str,
) -> Result<DescribeResult, Box<dyn std::error::Error + Send + Sync>> {
    Ok(perform_describe(connection, Some(fallback_manifest(name))).await?)
}

/// Fires `count` pipelined requests of `method`, correlating responses by
/// id, and returns how many responses matched their expected value.
pub async fn fire_pipelined(
    connection: &PluginConnection,
    method: &str,
    count: u64,
    params_for: impl Fn(u64) -> serde_json::Value + Send + Sync + 'static,
    expected_for: impl Fn(u64) -> serde_json::Value + Send + Sync + 'static,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let method = method.to_string();
    let params_for = std::sync::Arc::new(params_for);
    let expected_for = std::sync::Arc::new(expected_for);
    let mut handles = Vec::new();
    for i in 0..count {
        let conn = connection.clone();
        let method = method.clone();
        let params_for = std::sync::Arc::clone(&params_for);
        let expected_for = std::sync::Arc::clone(&expected_for);
        handles.push(tokio::spawn(async move {
            let params = params_for(i);
            let expected = expected_for(i);
            conn.request(&method, params).await.map(
                |value| {
                    if value == expected {
                        1u64
                    } else {
                        0u64
                    }
                },
            )
        }));
    }
    let mut ok = 0u64;
    for handle in handles {
        ok += handle.await?.unwrap_or(0);
    }
    Ok(ok)
}
