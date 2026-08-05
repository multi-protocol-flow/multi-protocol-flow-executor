#![allow(missing_docs)]

//! CLI e2e tests for the plugin system (M5b).
//!
//! `cli_run_plugin_flow` runs `mpe-cli run` against a flow whose node type
//! comes from a fixture plugin (Python template, `pyecho:add`), installed in
//! a plugins directory selected via the `MPE_PLUGIN_DIR` env override (OD1).
//! This exercises the M3.3b CLI path end to end: `run_cli` →
//! `build_shared_registry` (scan + describe + register) → `SidecarPlugin`
//! execute over the real transport.

use std::path::{Path, PathBuf};

use assert_cmd::cargo;
use serde::Deserialize;
use serde_json::Value;

/// The checked-in fixtures root: `tests/plugin-fixtures` (contains one
/// plugin subdir, `pyecho/`).
fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("plugin-fixtures")
}

/// Recursively finds the first object whose `node_type` is `pyecho:add`.
fn find_plugin_node_report(value: &Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            if map.get("node_type").and_then(Value::as_str) == Some("pyecho:add") {
                return Some(value.clone());
            }
            map.values().find_map(find_plugin_node_report)
        }
        Value::Array(items) => items.iter().find_map(find_plugin_node_report),
        _ => None,
    }
}

/// A flow whose middle node is the fixture plugin's `pyecho:add` type:
/// entry → pyecho:add (config `{a: 2, b: 3}`) → end.
fn plugin_flow_json() -> Value {
    serde_json::json!({
        "_version": 1,
        "flow": {
            "uuid": "plugin-flow-001",
            "name": "Plugin Flow",
            "nodes": [
                {
                    "uuid": "entry-1",
                    "type": "entry",
                    "name": "Entry",
                    "position": {"x": 0.0, "y": 0.0},
                    "data": {"type": "entry"},
                    "ports": null
                },
                {
                    "uuid": "add-1",
                    "type": "pyecho:add",
                    "name": "Add",
                    "position": {"x": 100.0, "y": 0.0},
                    "data": {"type": "pyecho:add", "a": 2, "b": 3},
                    "ports": null
                },
                {
                    "uuid": "end-1",
                    "type": "end",
                    "name": "End",
                    "position": {"x": 200.0, "y": 0.0},
                    "data": {"type": "end"},
                    "ports": null
                }
            ],
            "connections": [
                {
                    "id": "conn-1",
                    "source_node_uuid": "entry-1",
                    "target_node_uuid": "add-1",
                    "source_port_id": "out",
                    "target_port_id": "in"
                },
                {
                    "id": "conn-2",
                    "source_node_uuid": "add-1",
                    "target_node_uuid": "end-1",
                    "source_port_id": "out",
                    "target_port_id": "in"
                }
            ]
        },
        "_meta": null
    })
}

#[test]
fn cli_run_plugin_flow() -> Result<(), Box<dyn std::error::Error>> {
    // The CLI is spawned with its cwd inside the plugin dir so the
    // manifest's relative entry (`python plugin.py`) resolves — the host
    // spawns plugin processes with the caller's cwd.
    let flow_path = std::env::temp_dir().join(format!(
        "mpe-plugin-flow-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&flow_path, serde_json::to_vec(&plugin_flow_json())?)?;

    let output = cargo::cargo_bin_cmd!("mpe-cli")
        .current_dir(fixtures_root().join("pyecho"))
        .env("MPE_PLUGIN_DIR", fixtures_root())
        .arg("run")
        .arg("-f")
        .arg(&flow_path)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&flow_path);

    assert!(
        output.status.success(),
        "mpe run failed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout is pretty-printed JSON; parse the first value (the CLI may
    // emit a second error object after a failed run — not on success).
    let mut de = serde_json::Deserializer::from_str(&stdout);
    let value = Value::deserialize(&mut de)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true, "flow must succeed: {stdout}");

    // The plugin node report carries the Python template's output_data.
    let report = find_plugin_node_report(&value)
        .ok_or_else(|| format!("pyecho:add node report missing: {stdout}"))?;
    assert_eq!(report["output"]["sum"], 5, "plugin output_data: {stdout}");
    assert_eq!(report["status"], "success");
    Ok(())
}
