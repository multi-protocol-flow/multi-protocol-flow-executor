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

use std::time::Duration;

use flow_engine_plugin::error::PluginError;
use flow_engine_plugin::transport::{ConnectionConfig, CorrelationFilter, PluginConnection};
use mpe_protocol::notification::Notification;

/// Config tuned for conformance (short timeout so failures surface fast).
fn conformance_config() -> ConnectionConfig {
    ConnectionConfig {
        request_timeout: Duration::from_secs(10),
        ..ConnectionConfig::default()
    }
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
