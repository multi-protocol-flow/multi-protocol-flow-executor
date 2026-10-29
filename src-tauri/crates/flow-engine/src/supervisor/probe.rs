//! Generic health and connectivity probes with grace period protection.
#![allow(missing_docs)]

use std::time::Duration;
use tokio::net::TcpStream;

/// Default timeout for HTTP health and TCP connectivity probes
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Default grace period after process start during which probes cannot trigger zombie self-healing
pub const DEFAULT_GRACE_PERIOD_MS: u64 = 5000;

/// Check if a process is still within its startup grace period.
pub fn is_in_grace_period(started_at_ms: u64, grace_period_ms: u64) -> bool {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    now.saturating_sub(started_at_ms) < grace_period_ms
}

/// Probe an HTTP GET endpoint with timeout, returning parsed JSON response if successful.
pub async fn probe_http_health(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder().timeout(timeout).build().ok()?;
    let clean_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    let url = format!("http://{host}:{port}{clean_path}");
    let resp = client.get(&url).send().await.ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

/// Probe TCP connectivity with timeout.
pub async fn probe_tcp_connect(host: &str, port: u16, timeout: Duration) -> bool {
    let addr = format!("{host}:{port}");
    tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map(|res| res.is_ok())
        .unwrap_or(false)
}
