//! Cross-runtime conformance (M5b): Python/Node templates + Rust SDK.
//!
//! Spawned through the real transport via `common::template_harness`; each
//! test skips itself (with a log) when its interpreter or binary is
//! unavailable.

mod common;

use common::template_harness;
use flow_engine_plugin::error::PluginError;
use flow_engine_plugin::transport::PluginConnection;
use std::time::Duration;

// ── Cross-runtime conformance (M5b) ───────────────────────────────────

/// Common surface for the Python template (`pyecho:add`): describe, 30
/// pipelined executes (`a=i, b=i` → `sum=2i`), validate, unknown method,
/// cancel notification, shutdown.
#[tokio::test]
async fn python_template_conformance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(script) = template_harness::template_script("python", "plugin.py") else {
        eprintln!("SKIP python_template_conformance: template file missing");
        return Ok(());
    };
    if !template_harness::interpreter_available("python") {
        eprintln!("SKIP python_template_conformance: no python interpreter");
        return Ok(());
    }
    let process = template_harness::spawn_interpreter("python", &script)?;
    let mut connection = PluginConnection::spawn(process, template_harness::conformance_config())?;

    let described = template_harness::describe_from(&connection, "pyecho").await?;
    assert_eq!(described.plugin.name, "pyecho");
    assert_eq!(described.nodes[0].type_id, "pyecho:add");

    let ok = template_harness::fire_pipelined(
        &connection,
        "execute",
        30,
        |i| serde_json::json!({"config": {"a": i, "b": i}}),
        |i| {
            serde_json::json!({
                "success": true,
                "next_ports": ["out"],
                "output_data": {"sum": 2 * i, "a": i, "b": i},
                "variable_updates": {"pyecho_last_sum": 2 * i},
            })
        },
    )
    .await?;
    assert_eq!(ok, 30, "python pipelined executes must all round-trip");

    let valid = connection
        .request("validate", serde_json::json!({"config": {"a": 1, "b": 2}}))
        .await?;
    assert_eq!(valid, serde_json::json!({"valid": true}));
    assert!(
        matches!(
            connection.request("nope", serde_json::json!({})).await,
            Err(PluginError::JsonRpc { code: -32601, .. })
        ),
        "unknown method must yield -32601"
    );
    // P0 cancel is a no-op notification: the connection stays usable.
    connection
        .send_notification("cancel", serde_json::json!({"id": 1}))
        .await?;
    let alive = connection
        .request("execute", serde_json::json!({"config": {"a": 7, "b": 8}}))
        .await?;
    assert_eq!(alive["output_data"]["sum"], 15);
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

/// Common surface for the Node template (`nodedemo:echo`): describe, 30
/// pipelined executes (config echoed back), validate, unknown method, cancel
/// notification, shutdown.
#[tokio::test]
async fn node_template_conformance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(script) = template_harness::template_script("node", "plugin.js") else {
        eprintln!("SKIP node_template_conformance: template file missing");
        return Ok(());
    };
    if !template_harness::interpreter_available("node") {
        eprintln!("SKIP node_template_conformance: no node interpreter");
        return Ok(());
    }
    let process = template_harness::spawn_interpreter("node", &script)?;
    let mut connection = PluginConnection::spawn(process, template_harness::conformance_config())?;

    let described = template_harness::describe_from(&connection, "nodedemo").await?;
    assert_eq!(described.plugin.name, "nodedemo");
    assert_eq!(described.nodes[0].type_id, "nodedemo:echo");

    let ok = template_harness::fire_pipelined(
        &connection,
        "execute",
        30,
        |i| serde_json::json!({"config": {"msg": format!("msg-{i}")}}),
        |i| {
            serde_json::json!({
                "success": true,
                "next_ports": ["out"],
                "output_data": {"msg": format!("msg-{i}")},
                "variable_updates": {
                    "nodedemo_echoed": format!("{{\"msg\":\"msg-{i}\"}}"),
                },
            })
        },
    )
    .await?;
    assert_eq!(ok, 30, "node pipelined executes must all round-trip");

    let valid = connection
        .request("validate", serde_json::json!({"config": {"msg": "ok"}}))
        .await?;
    assert_eq!(valid, serde_json::json!({"valid": true}));
    assert!(
        matches!(
            connection.request("nope", serde_json::json!({})).await,
            Err(PluginError::JsonRpc { code: -32601, .. })
        ),
        "unknown method must yield -32601"
    );
    connection
        .send_notification("cancel", serde_json::json!({"id": 1}))
        .await?;
    let alive = connection
        .request("execute", serde_json::json!({"config": {"msg": "hi"}}))
        .await?;
    assert_eq!(alive["output_data"]["msg"], "hi");
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

/// Conformance against the compiled Rust SDK demo plugin (`mpe_demo_plugin`).
///
/// The SDK answers `describe` with the array shape (no manifest echo), which
/// the host reconciles via `perform_describe` + the on-disk fallback
/// manifest. Requires `cargo build -p mpe-plugin-sdk --bin mpe_demo_plugin`.
#[tokio::test]
async fn sdk_plugin_conformance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(bin) = template_harness::sdk_plugin_bin() else {
        eprintln!(
            "SKIP sdk_plugin_conformance: mpe_demo_plugin not built \
             (run `cargo build -p mpe-plugin-sdk --bin mpe_demo_plugin`)"
        );
        return Ok(());
    };
    let process = template_harness::spawn_absolute(&bin)?;
    let mut connection = PluginConnection::spawn(process, template_harness::conformance_config())?;

    let described = template_harness::describe_from(&connection, "demo").await?;
    assert_eq!(described.nodes[0].type_id, "demo:echo");

    let ok = template_harness::fire_pipelined(
        &connection,
        "execute",
        30,
        |i| serde_json::json!({"config": {"k": i}}),
        |i| {
            serde_json::json!({
                "success": true,
                "output_data": {"k": i},
                "variable_updates": {},
            })
        },
    )
    .await?;
    assert_eq!(ok, 30, "sdk pipelined executes must all round-trip");

    assert!(
        matches!(
            connection.request("nope", serde_json::json!({})).await,
            Err(PluginError::JsonRpc { code: -32601, .. })
        ),
        "unknown method must yield -32601"
    );
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}
