//! Lifecycle tests: in-memory duplex fakes injected through the connector
//! (no real plugin processes except the shared `process.rs` spawn path).

use std::sync::Arc;
use std::time::Duration;

use mpe_protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};
use mpe_protocol::request::Request;

use super::*;
use crate::framing::{FrameReader, FrameWriter};

/// A minimal, valid manifest for lifecycle fixtures.
fn test_manifest() -> PluginManifest {
    PluginManifest {
        name: "test-plugin".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        min_host_version: None,
        description: None,
        entry: PluginEntry {
            command: "unused".to_string(),
            args: Vec::new(),
        },
        env: std::collections::HashMap::new(),
        permissions: Vec::new(),
        capabilities: PluginCapabilities::default(),
    }
}

/// A connector that produces a live echo connection: the plugin side reads
/// requests, responds with `{id, result: params}`, and closes stdout on stdin
/// EOF (the clean-shutdown path).
fn live_connector() -> Connector {
    Arc::new(|_: &PluginManifest| Ok(live_connection(ConnectionConfig::default())))
}

fn live_connection(config: ConnectionConfig) -> PluginConnection {
    let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
    let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        let mut reader = FrameReader::new(plugin_rx);
        let mut writer = FrameWriter::new(plugin_tx);
        loop {
            match reader.next_frame().await {
                Ok(Some(frame)) => {
                    let request = match serde_json::from_slice::<Request>(&frame) {
                        Ok(request) => request,
                        Err(_) => continue,
                    };
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request.id,
                        "result": request.params,
                    });
                    let bytes = match serde_json::to_vec(&response) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    };
                    if writer.write_frame(&bytes).await.is_err() {
                        break;
                    }
                }
                // Host half-closed stdin: close stdout for a clean EOF.
                Ok(None) => break,
                Err(_) => break,
            }
        }
    });
    PluginConnection::from_io(host_tx, host_rx, config)
}

/// A connector that produces a connection whose peer halves are dropped
/// immediately: the reader observes EOF and marks the connection crashed.
fn dead_connector() -> Connector {
    Arc::new(|_: &PluginManifest| Ok(dead_connection(ConnectionConfig::default())))
}

fn dead_connection(config: ConnectionConfig) -> PluginConnection {
    let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
    let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
    drop(plugin_rx);
    drop(plugin_tx);
    PluginConnection::from_io(host_tx, host_rx, config)
}

fn test_config(idle: Duration, restart: Duration, connector: Connector) -> LifecycleConfig {
    LifecycleConfig {
        idle_recycle: idle,
        restart_min_interval: restart,
        shutdown_grace: Duration::from_millis(50),
        connector,
    }
}

fn entry_from(config: &LifecycleConfig, mode: ResidencyMode) -> Arc<LifecycleEntry> {
    LifecycleEntry::new(
        "test-plugin".to_string(),
        test_manifest(),
        mode,
        config.idle_recycle,
        config.restart_min_interval,
        config.shutdown_grace,
        Arc::clone(&config.connector),
    )
}

/// Waits until the fake connection's reader has observed EOF and marked it
/// crashed, so the next `connection()` hits the restart path deterministically.
async fn wait_dead(conn: &PluginConnection) {
    for _ in 0..50 {
        if !matches!(conn.exit_reason(), ExitReason::Running) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn lifecycle_on_demand_start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        live_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);
    assert_eq!(entry.epoch(), 0);

    let conn = entry.connection().await?;
    assert_eq!(entry.epoch(), 1);
    assert!(matches!(conn.exit_reason(), ExitReason::Running));
    // The fake echo connection answers requests.
    let result = conn.request("echo", serde_json::json!({"n": 1})).await?;
    assert_eq!(result, serde_json::json!({"n": 1}));

    let metrics = entry.metrics();
    assert_eq!(metrics.epoch, 1);
    assert_eq!(metrics.mode, ResidencyMode::OnDemand);
    assert!(metrics.connected);
    assert_eq!(metrics.restart_count, 0);
    Ok(())
}

#[tokio::test]
async fn lifecycle_idle_recycle_after_60s() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        live_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);
    let conn = entry.connection().await?;
    assert!(matches!(conn.exit_reason(), ExitReason::Running));
    drop(conn); // release the lease: in_flight drops to 0

    entry.spawn_reaper();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let metrics = entry.metrics();
    assert_eq!(metrics.recycle_count, 1);
    assert!(!metrics.connected);
    assert_eq!(metrics.epoch, 1);
    Ok(())
}

#[tokio::test]
async fn lifecycle_streaming_stays_resident() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        live_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::Resident);
    let conn = entry.connection().await?;
    drop(conn);

    entry.spawn_reaper(); // no-op for Resident
    tokio::time::sleep(Duration::from_millis(300)).await;

    let metrics = entry.metrics();
    assert_eq!(metrics.recycle_count, 0);
    assert!(metrics.connected);
    assert_eq!(metrics.mode, ResidencyMode::Resident);
    Ok(())
}

#[tokio::test]
async fn lifecycle_restart_once_then_on_error(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        dead_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);

    // First start spawns a connection that immediately dies.
    let conn = entry.connection().await?;
    assert_eq!(entry.epoch(), 1);
    wait_dead(&conn).await;
    drop(conn);

    // Exactly one restart is allowed; the new generation is also dead.
    let conn = entry.connection().await?;
    let metrics = entry.metrics();
    assert_eq!(metrics.epoch, 2);
    assert_eq!(metrics.restart_count, 1);
    wait_dead(&conn).await;
    drop(conn);

    // A second restart attempt inside `restart_min_interval` → rate-limited.
    let err = entry.connection().await.err();
    assert!(
        matches!(err, Some(PluginError::ProtocolError(_))),
        "expected rate-limit error, got {err:?}"
    );
    assert_eq!(entry.metrics().epoch, 2);
    Ok(())
}

#[tokio::test]
async fn lifecycle_epoch_rejects_stale() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        dead_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);

    let conn = entry.connection().await?; // epoch 1
    let stale_epoch = entry.epoch();
    wait_dead(&conn).await;
    drop(conn);

    // Restart advances the epoch and clears the old generation.
    let conn = entry.connection().await?; // epoch 2
    assert_eq!(entry.metrics().epoch, 2);
    assert_eq!(entry.metrics().restart_count, 1);
    wait_dead(&conn).await;
    drop(conn);

    // A caller pinned to the pre-restart epoch is rejected.
    let err = entry.connection_stamped(stale_epoch).await.err();
    assert!(
        matches!(err, Some(PluginError::ProtocolError(_))),
        "expected stale-epoch error, got {err:?}"
    );
    assert_eq!(entry.metrics().epoch, 2);
    Ok(())
}

#[tokio::test]
async fn lifecycle_shutdown_grace_then_kill() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        live_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);
    let conn = entry.connection().await?;
    assert!(matches!(conn.exit_reason(), ExitReason::Running));
    drop(conn); // release the lease so Arc::get_mut succeeds

    let reason = entry.shutdown(Duration::from_millis(50)).await;
    assert_eq!(reason, ExitReason::CleanEof);
    assert!(!entry.metrics().connected);

    // Post-shutdown acquires fail fast.
    let result = entry.connection().await;
    assert!(matches!(result, Err(PluginError::Shutdown)));
    Ok(())
}

#[tokio::test]
async fn lifecycle_rate_limits_restart() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = test_config(
        Duration::from_millis(80),
        Duration::from_secs(1),
        dead_connector(),
    );
    let entry = entry_from(&config, ResidencyMode::OnDemand);

    // First start (allowed).
    let conn = entry.connection().await?;
    assert_eq!(entry.epoch(), 1);
    wait_dead(&conn).await;
    drop(conn);

    // First restart (allowed: no previous restart to rate-limit against).
    let conn = entry.connection().await?;
    assert_eq!(entry.metrics().epoch, 2);
    assert_eq!(entry.metrics().restart_count, 1);
    wait_dead(&conn).await;
    drop(conn);

    // A second rapid restart attempt is rate-limited, not re-spawned.
    let err = entry.connection().await.err();
    assert!(
        matches!(err, Some(PluginError::ProtocolError(_))),
        "expected rate-limit error, got {err:?}"
    );
    assert_eq!(entry.metrics().epoch, 2);
    Ok(())
}
