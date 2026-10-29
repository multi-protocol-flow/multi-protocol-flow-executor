//! Opportunistic Loopback Telemetry for MPE GUI integration.
//!
//! When MPE GUI is running, sends execution events via local loopback HTTP.
//! If GUI is not running, silently does nothing without any performance penalty.

use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Basic information about a running MPE GUI instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiInstanceInfo {
    pub pid: u32,
    pub port: u16,
    pub auth_token: String,
    pub version: String,
}

/// Telemetry event dispatched upon MCP tool call completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTelemetryEvent {
    pub event: String,
    pub tool_name: String,
    pub execution_id: String,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: u64,
}

/// Locate `gui-instance.json` across platforms
pub fn get_gui_instance_path() -> PathBuf {
    if let Ok(dir) = std::env::var("MPE_HOME") {
        return PathBuf::from(dir).join("gui-instance.json");
    }
    if cfg!(target_os = "windows") {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data)
                .join("mpe")
                .join("gui-instance.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".mpe").join("gui-instance.json");
    }
    PathBuf::from(".mpe").join("gui-instance.json")
}

/// Read GUI instance info if running
pub fn find_running_gui() -> Option<GuiInstanceInfo> {
    let path = get_gui_instance_path();
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<GuiInstanceInfo>(&content).ok()
}

/// Send telemetry event asynchronously without blocking
pub fn emit_opportunistic_telemetry(event: McpTelemetryEvent) {
    tokio::spawn(async move {
        if let Some(gui) = find_running_gui() {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_millis(500))
                .build()
            {
                Ok(c) => c,
                Err(_) => return,
            };

            let url = format!("http://127.0.0.1:{}/telemetry", gui.port);
            let _ = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", gui.auth_token))
                .json(&event)
                .send()
                .await;
        }
    });
}
