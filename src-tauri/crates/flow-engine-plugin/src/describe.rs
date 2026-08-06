//! Host-side describe RPC handshake (M2a').
//!
//! [`perform_describe`] issues the `describe` RPC over a live
//! [`PluginConnection`] and parses the response into a
//! [`DescribeResult`](mpe_protocol::describe::DescribeResult). Every frontend
//! kind passes through to the host (PR-0); the host generates the iframe
//! `url` (per-plugin origin, D5/F3) because only the host knows the
//! platform-usable scheme. The function is pure with respect to the
//! transport, so tests drive it with `PluginConnection::from_io` in-memory
//! fakes; [`crate::manager`] runs it against real spawned processes.

use flow_engine_core::plugin::FrontendMetadata;
use mpe_protocol::describe::{DescribeResult, FrontendDescription, NodeDescription};
use mpe_protocol::manifest::PluginManifest;

use crate::error::PluginError;
use crate::transport::PluginConnection;

/// Wire method name of the describe RPC (re-exported for host callers).
pub use mpe_protocol::describe::DESCRIBE_METHOD;

/// Issues `describe` over `conn` and parses the plugin's response.
///
/// Sends `params: {}` per the protocol spec (§Describe in
/// `docs/plugin-protocol.md`); the plugin echoes its manifest and node
/// descriptions.
///
/// Two wire shapes are accepted (reconciliation, M5b):
/// 1. **Object shape** (normative, `docs/plugin-protocol.md` §9): a
///    `DescribeResult` with `plugin` + `nodes`. The echo manifest's
///    `capabilities` override the on-disk manifest.
/// 2. **Array shape** (pre-M5b `mpe-plugin-sdk`): a bare JSON array of
///    `NodeDescription`s with no manifest echo. The host falls back to
///    `fallback_manifest` (the on-disk manifest from discovery) so SDK
///    plugins load without recompiling. A response that is neither shape is
///    a protocol error — the caller decides whether it counts toward the
///    quarantine policy.
pub async fn perform_describe(
    conn: &PluginConnection,
    fallback_manifest: Option<PluginManifest>,
) -> Result<DescribeResult, PluginError> {
    let response = conn.request(DESCRIBE_METHOD, serde_json::json!({})).await?;
    if let Ok(result) = serde_json::from_value::<DescribeResult>(response.clone()) {
        return Ok(result);
    }
    if let Ok(nodes) = serde_json::from_value::<Vec<NodeDescription>>(response) {
        let manifest = fallback_manifest.ok_or_else(|| {
            PluginError::ProtocolError(
                "array-shape describe response but no fallback manifest".to_string(),
            )
        })?;
        return Ok(DescribeResult {
            plugin: manifest,
            nodes,
        });
    }
    Err(PluginError::ProtocolError(
        "invalid describe response (neither DescribeResult object nor node array)".to_string(),
    ))
}

/// Builds the host-side iframe URL for a plugin's `page` under its
/// per-plugin origin (`<plugin-id>.localhost`, locked D5/F3).
///
/// macOS/Linux use the custom `mpe-plugin://` scheme; Windows/Android use
/// plain `http://` because custom schemes are not usable in iframe `src`
/// there. The scheme is decided at runtime with `cfg!(windows)` so the same
/// binary ships the correct URL on every platform — the frontend needs zero
/// platform branching.
fn plugin_iframe_url(plugin_id: &str, page: &str) -> String {
    let scheme = if cfg!(windows) { "http" } else { "mpe-plugin" };
    format!("{scheme}://{plugin_id}.localhost/{page}")
}

/// Normalizes parsed node descriptions for host consumption (PR-0/PR-2).
///
/// Every frontend kind passes through (no more dropping of non-inline
/// kinds). When a node declares a frontend, the host generates the iframe
/// URL for the plugin's `config-panel` page into `frontend.url` — the
/// plugin cannot know the platform-usable scheme, so the host is the
/// authority (D5/F3). Reserved P1 fields (`config_panel` / `viewer` /
/// `help`) are not modeled by the wire type and are dropped by parsing
/// itself.
pub fn normalize_nodes(nodes: Vec<NodeDescription>, plugin_name: &str) -> Vec<NodeDescription> {
    nodes
        .into_iter()
        .map(|node| normalize_node(node, plugin_name))
        .collect()
}

/// Normalizes one node's frontend: all kinds pass through and the host
/// generates the iframe URL into `frontend.url`.
fn normalize_node(mut node: NodeDescription, plugin_name: &str) -> NodeDescription {
    if let Some(frontend) = &mut node.frontend {
        frontend.url = Some(plugin_iframe_url(plugin_name, "config-panel"));
    }
    node
}

/// Maps a wire `FrontendDescription` onto the host-side `FrontendMetadata`
/// (same shape: `kind` / `content` / `url`) so the plugin wrappers
/// (`SidecarPlugin` / `PendingPlugin`) can hand the host registry the
/// plugin's frontend metadata.
///
/// A free function rather than `From` because both types are defined outside
/// this crate (orphan rule: `mpe_protocol` cannot depend on
/// `flow-engine-core`).
pub fn frontend_metadata_from(frontend: &FrontendDescription) -> FrontendMetadata {
    FrontendMetadata {
        kind: frontend.kind.clone(),
        content: frontend.content.clone(),
        url: frontend.url.clone(),
    }
}

#[cfg(test)]
#[path = "describe_tests.rs"]
mod tests;
