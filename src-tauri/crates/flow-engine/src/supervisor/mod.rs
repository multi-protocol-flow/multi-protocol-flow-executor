//! Generic background service supervisor and process governance.
#![allow(missing_docs)]

pub mod daemon;
pub mod probe;
pub mod process;
pub mod services;
pub mod store;

use serde::{Deserialize, Serialize};

/// Type classification for supervised background processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    /// Model Context Protocol sidecar server
    #[default]
    Mcp,
    /// Ingress gateway listener
    Ingress,
    /// Protocol mock server
    Mock,
}

/// Metadata representation of a running or managed background service instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: ServiceKind,
    pub pid: u32,
    pub transport: String,
    pub host: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir_target: Option<String>,
    #[serde(default)]
    pub tools_count: usize,
    pub started_at: u64,
    pub managed_by: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
