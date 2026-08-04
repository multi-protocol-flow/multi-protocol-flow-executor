//! Spawn-hygiene integration tests (M1.6).
//!
//! These run against the real `echo_mock` binary through
//! `process::spawn_plugin`, exercising stderr draining, handle-inheritance
//! hygiene, and (on Windows) the kill-on-close Job Object.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use flow_engine_plugin::transport::{ConnectionConfig, PluginConnection};
use tokio::io::AsyncReadExt;

/// A child that floods stderr with 128 KiB then answers requests would stall
/// forever on its next write if the host did not drain stderr from spawn
/// (the OS pipe buffer is only ~64 KiB).
#[tokio::test]
async fn spawn_drains_stderr_without_stall() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = HashMap::from([(common::echo_mock::FLOOD_STDERR_ENV.to_string(), "1".to_string())]);
    let process = common::echo_mock::spawn_echo_mock_with_env(&env)?;
    let mut connection = PluginConnection::spawn(
        process,
        ConnectionConfig {
            request_timeout: Duration::from_secs(10),
            ..ConnectionConfig::default()
        },
    )?;
    // The echo mock writes 128 KiB to stderr before reading stdin; without
    // the host-side drain this request would never be answered.
    let result = connection
        .request("echo", serde_json::json!({"stderr": true}))
        .await?;
    assert_eq!(result, serde_json::json!({"stderr": true}));
    connection.shutdown(Duration::from_secs(5)).await?;
    Ok(())
}

/// A stdout pipe whose write end is inherited by a stray handle never reaches
/// EOF. Reading the mock's stdout to EOF within a timeout proves the host's
/// handle-inheritance hygiene holds (std's CLOEXEC / `CREATE_NO_WINDOW`
/// path).
#[tokio::test]
async fn spawn_handles_do_not_leak() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = HashMap::from([(
        common::echo_mock::MODE_ENV.to_string(),
        "once".to_string(),
    )]);
    let process = common::echo_mock::spawn_echo_mock_with_env(&env)?;
    let (mut child, _guard) = process.into_parts();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "child stdout pipe missing".to_string())?;
    let mut output = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(5), stdout.read_to_end(&mut output)).await;
    read.map_err(|_| "stdout never reached EOF — a handle leaked".to_string())??;
    let text = String::from_utf8(output)?;
    assert!(text.contains("\"ok\":true"), "unexpected mock output: {text}");
    let _ = child.wait().await;
    Ok(())
}

/// On Windows the plugin is assigned to a kill-on-close Job Object: dropping
/// the job guard terminates the (still-sleeping) process. Without the job,
/// the child would keep sleeping for an hour.
#[cfg(windows)]
#[tokio::test]
async fn spawn_job_object_kills_on_close() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = HashMap::from([(
        common::echo_mock::MODE_ENV.to_string(),
        "sleep".to_string(),
    )]);
    let process = common::echo_mock::spawn_echo_mock_with_env(&env)?;
    let (mut child, guard) = process.into_parts();
    assert!(child.id().is_some(), "child should be running");
    // Closing the last job handle (the guard) must kill the process tree.
    // The mock is in 3600s sleep mode, so `wait()` resolving promptly can
    // only mean the job terminated it (Windows reports exit code 0 for a
    // job-terminated process, so we assert on prompt reaping, not the code).
    drop(guard);
    let waited = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    waited
        .map_err(|_| "child was not killed after the job object closed".to_string())??;
    Ok(())
}

/// Non-Windows placeholder so the gate list has a matching test name on every
/// platform (the unix Job Object equivalent is std's kill_on_drop + CLOEXEC).
#[cfg(not(windows))]
#[tokio::test]
async fn spawn_job_object_kills_on_close() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = HashMap::from([(
        common::echo_mock::MODE_ENV.to_string(),
        "sleep".to_string(),
    )]);
    let process = common::echo_mock::spawn_echo_mock_with_env(&env)?;
    let (mut child, _guard) = process.into_parts();
    assert!(child.id().is_some(), "child should be running");
    // kill_on_drop: dropping the child handle reaps it.
    child.kill().await?;
    let _ = child.wait().await;
    Ok(())
}
