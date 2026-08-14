//! Protocol conformance suite against the `echo_mock` binary.
//!
//! Each test spawns the real echo mock via `process::spawn_plugin` and drives
//! it through a real `PluginConnection`, so the whole stack (framing, writer
//! task, reader demux, pending map, spawn hygiene) is exercised end to end.
//!
//! M5b adds one conformance test per non-Rust runtime — the Python template,
//! the Node template, and the compiled Rust SDK demo plugin (helpers in
//! `common::template_harness`). Those tests skip themselves when their
//! interpreter or binary is unavailable; the echo-mock tests are the
//! always-run baseline.

mod common;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use flow_engine_core::registry::NodeRegistry;
use flow_engine_plugin::describe::perform_describe;
use flow_engine_plugin::error::PluginError;
use flow_engine_plugin::manager::PluginManager;
use flow_engine_plugin::transport::{ConnectionConfig, CorrelationFilter, PluginConnection};
use mpe_plugin_sdk::protocol::notification::Notification;

/// Config tuned for conformance (short timeout so failures surface fast).
fn conformance_config() -> ConnectionConfig {
    ConnectionConfig {
        request_timeout: Duration::from_secs(10),
        ..ConnectionConfig::default()
    }
}

/// Unique scratch directory under the OS temp dir (mirrors the unit-test
/// convention; no tempfile dev-dependency).
fn unique_temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "mpe-plugin-conformance-{}-{}-{tag}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create temp dir {dir:?}: {e}"));
    dir
}

fn connect_echo() -> Result<PluginConnection, Box<dyn std::error::Error + Send + Sync>> {
    let process = common::echo_mock::spawn_echo_mock()?;
    Ok(PluginConnection::spawn(process, conformance_config())?)
}

/// Fires `count` echo requests pipelined (without awaiting between them) and
/// collects the results.
async fn fire_pipelined(
    connection: &PluginConnection,
    count: u64,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    for i in 0..count {
        let conn = connection.clone();
        handles.push(tokio::spawn(async move {
            conn.request("echo", serde_json::json!({"seq": i})).await
        }));
    }
    let mut ok = 0u64;
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await?;
        match result {
            Ok(value) if value["seq"] == serde_json::json!(i as u64) => ok += 1,
            Ok(_) => {}
            Err(_) => {}
        }
    }
    Ok(ok)
}

#[tokio::test]
async fn conformance_echo_roundtrip() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = connect_echo()?;
    let completed = fire_pipelined(&connection, 200).await?;
    // Every pipelined request round-trips through the mock and back.
    eprintln!(
        "burst exit: {:?} metrics: {:?}",
        connection.exit_reason(),
        connection.metrics()
    );
    assert_eq!(
        completed, 200,
        "expected all pipelined requests to complete"
    );
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

#[tokio::test]
async fn conformance_error_object_mapping() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let mut connection = connect_echo()?;
    let result = connection.request("fail", serde_json::json!({})).await;
    match result {
        Err(PluginError::JsonRpc { code, message, .. }) => {
            assert_eq!(code, -32000);
            assert_eq!(message, "mock failure");
        }
        other => panic!("expected typed JSON-RPC error, got {other:?}"),
    }
    // The connection survives an error response.
    let ok = connection
        .request("echo", serde_json::json!({"alive": true}))
        .await?;
    assert_eq!(ok, serde_json::json!({"alive": true}));
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

#[tokio::test]
async fn conformance_notification_demux() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = connect_echo()?;
    let (delivered_tx, delivered_rx) = std::sync::mpsc::channel();
    let _subscription = connection.register_notification_handler(
        "log",
        CorrelationFilter::default(),
        move |n: &Notification| {
            let _ = delivered_tx.send(n.clone());
        },
    );
    // The mock's "log" method emits a `log` notification (with correlation
    // fields) while the request is still pending, then echoes.
    let result = connection
        .request("log", serde_json::json!({"n": 1}))
        .await?;
    assert_eq!(result, serde_json::json!({"n": 1}));
    let delivered = delivered_rx
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "log notification was not delivered".to_string())?;
    assert_eq!(delivered.method, "log");
    let correlation = delivered
        .correlation()
        .ok_or_else(|| "notification missing execution_id/node_instance_id".to_string())?;
    assert_eq!(correlation.execution_id, "exec-1");
    assert_eq!(correlation.node_instance_id, "node-1");
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

#[tokio::test]
async fn conformance_crlf_tolerance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = connect_echo()?;
    // The mock answers "crlf" requests with a `\r\n` terminator (Python
    // print() style); the host framing must strip the trailing CR.
    let result = connection
        .request("crlf", serde_json::json!({"crlf": true}))
        .await?;
    assert_eq!(result, serde_json::json!({"crlf": true}));
    // The connection still works after a CRLF frame.
    let again = connection
        .request("echo", serde_json::json!({"ok": 1}))
        .await?;
    assert_eq!(again, serde_json::json!({"ok": 1}));
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

/// The config-panel iframe URL a host must generate for the echo mock's
/// describe (mirrors `describe_tests::plugin_iframe_url_uses_platform_scheme`):
/// `mpe-plugin://<id>.localhost/config-panel` on macOS/Linux,
/// `http://mpe-plugin.<id>.localhost/config-panel` on Windows/Android.
fn expected_iframe_url(plugin_id: &str) -> String {
    if cfg!(windows) {
        format!("http://mpe-plugin.{plugin_id}.localhost/config-panel")
    } else {
        format!("mpe-plugin://{plugin_id}.localhost/config-panel")
    }
}

/// PR-0 wire conformance: a real plugin process answers `describe` with an
/// `iframe`-kind frontend that omits `url`; the host parses it through
/// `perform_describe` WITHOUT dropping it (all kinds pass through) over a
/// real stdio round-trip.
#[tokio::test]
async fn conformance_describe_keeps_iframe_frontend(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = connect_echo()?;
    let result = perform_describe(&connection, None).await?;

    // Object-shape describe: the mock's manifest echo parses.
    assert_eq!(result.plugin.name, "demo");

    // The inline node keeps its content on the wire…
    let inline = result
        .nodes
        .iter()
        .find(|n| n.type_id == "echo:inline")
        .ok_or_else(|| "echo:inline missing from describe".to_string())?;
    assert_eq!(
        inline.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("inline")
    );
    assert_eq!(
        inline.frontend.as_ref().and_then(|f| f.content.as_deref()),
        Some("<p>echo</p>")
    );

    // …and the iframe node is NOT dropped at parse time (PR-0). It carries no
    // `url` because the plugin omitted it — the host must generate it.
    let iframe = result
        .nodes
        .iter()
        .find(|n| n.type_id == "echo:iframe")
        .ok_or_else(|| "echo:iframe missing from describe".to_string())?;
    assert_eq!(
        iframe.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("iframe")
    );
    assert_eq!(
        iframe.frontend.as_ref().and_then(|f| f.url.as_deref()),
        None
    );

    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

/// PR-0/PR-1/PR-2 end-to-end conformance: `PluginManager::scan_and_describe`
/// spawns the real echo mock, normalizes the describe response (host
/// generates the iframe url into `frontend.url`), and `register_nodes` wraps
/// the node in a plugin whose `NodePlugin::frontend()` returns the metadata.
#[tokio::test]
async fn conformance_scan_and_register_carries_iframe_frontend(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = unique_temp_dir("iframe-frontend");
    let plugin_dir = root.join("demo");
    std::fs::create_dir_all(&plugin_dir)?;
    // The manifest points at the real echo mock binary; the plugin name
    // (`demo`) becomes the iframe URL origin.
    let manifest = serde_json::json!({
        "name": "demo",
        "version": "0.0.0",
        "entry": {"command": env!("CARGO_BIN_EXE_echo_mock")}
    });
    std::fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    let mut manager = PluginManager::new(root.clone());
    let registrations = manager.scan_and_describe().await;

    // The scan spawned the real echo mock and completed the describe
    // handshake end to end.
    assert_eq!(registrations.len(), 1, "echo mock must register");
    assert_eq!(registrations[0].name, "demo");

    // Normalization ran inside scan_and_describe: the iframe kind passed
    // through and the host generated the config-panel url (locked contract).
    let expected_url = expected_iframe_url("demo");
    let iframe = registrations[0]
        .nodes
        .iter()
        .find(|n| n.type_id == "echo:iframe")
        .ok_or_else(|| "echo:iframe not in registration".to_string())?;
    assert_eq!(
        iframe.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("iframe")
    );
    assert_eq!(
        iframe.frontend.as_ref().and_then(|f| f.url.as_deref()),
        Some(expected_url.as_str())
    );
    // The inline node keeps its content and gets the same host-generated url.
    let inline = registrations[0]
        .nodes
        .iter()
        .find(|n| n.type_id == "echo:inline")
        .ok_or_else(|| "echo:inline not in registration".to_string())?;
    assert_eq!(
        inline.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("inline")
    );
    assert_eq!(
        inline.frontend.as_ref().and_then(|f| f.content.as_deref()),
        Some("<p>echo</p>")
    );

    // End-to-end registration: the registry's `NodePlugin::frontend()`
    // returns the metadata (describe → normalize → PendingPlugin).
    let mut registry = NodeRegistry::new();
    manager.register_nodes(&mut registry);
    let plugin = registry
        .get("echo:iframe")
        .ok_or_else(|| "echo:iframe missing from registry".to_string())?;
    let frontend = plugin
        .frontend()
        .ok_or_else(|| "registered node lost its frontend metadata".to_string())?;
    assert_eq!(frontend.kind, "iframe");
    assert_eq!(frontend.content, None);
    assert_eq!(frontend.url.as_deref(), Some(expected_url.as_str()));

    // `all_metadata()` (the batch the frontend consumes) carries the metadata.
    let meta = registry
        .all_metadata()
        .into_iter()
        .find(|m| m.type_id == "echo:iframe")
        .ok_or_else(|| "echo:iframe metadata missing".to_string())?;
    assert_eq!(
        meta.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("iframe")
    );
    assert_eq!(
        meta.frontend.as_ref().and_then(|f| f.url.as_deref()),
        Some(expected_url.as_str())
    );

    manager.shutdown_all(Duration::from_secs(5)).await;
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// `PluginManager::notify_flow_ended` broadcasts a fire-and-forget `flowEnded`
/// notification (carrying `execution_id`) to every **running** plugin, without
/// spawning idle on-demand plugins.
#[tokio::test]
async fn conformance_notify_flow_ended_reaches_running_plugin_only(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = unique_temp_dir("flow-ended");
    let plugin_dir = root.join("demo");
    std::fs::create_dir_all(&plugin_dir)?;
    let manifest = serde_json::json!({
        "name": "demo",
        "version": "0.0.0",
        "entry": {"command": env!("CARGO_BIN_EXE_echo_mock")}
    });
    std::fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    let mut manager = PluginManager::new(root.clone());
    let registrations = manager.scan_and_describe().await;
    assert_eq!(registrations.len(), 1, "echo mock must register");

    // Before any connection is forced, the entry is on-demand and idle:
    // the broadcast must be a no-op (no spawn, no error).
    manager.notify_flow_ended("exec-idle").await;

    // Force the process up through the real connection path…
    let conn = manager.connection_for("demo").await?;
    assert!(matches!(
        conn.exit_reason(),
        flow_engine_plugin::transport::ExitReason::Running
    ));

    // …then broadcast to a running plugin: must not error.
    manager.notify_flow_ended("exec-live").await;

    // The plugin connection must still be usable after the broadcast
    // (fire-and-forget never wedges the transport).
    let reply = conn
        .request("echo", serde_json::json!({"message": "still-alive"}))
        .await?;
    assert_eq!(reply.get("message"), Some(&serde_json::json!("still-alive")));

    manager.shutdown_all(Duration::from_secs(5)).await;
    std::fs::remove_dir_all(&root)?;
    Ok(())
}
