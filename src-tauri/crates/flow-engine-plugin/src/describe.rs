//! Host-side describe RPC handshake (M2a').
//!
//! [`perform_describe`] issues the `describe` RPC over a live
//! [`PluginConnection`] and parses the response into a
//! [`DescribeResult`](mpe_protocol::describe::DescribeResult), normalizing
//! P0-unsupported frontend kinds to no-frontend (warn, never crash). The
//! function is pure with respect to the transport, so tests drive it with
//! `PluginConnection::from_io` in-memory fakes; [`crate::manager`] runs it
//! against real spawned processes.

use mpe_protocol::describe::{DescribeResult, NodeDescription};
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

/// Normalizes parsed node descriptions for host consumption (P0).
///
/// P0 accepts only `frontend.kind == "inline"`. A node declaring any other
/// frontend kind is logged with a warning and treated as having no frontend;
/// its remaining fields are kept. The reserved P1 frontend fields
/// (`config_panel` / `viewer` / `help`) are not modeled by the wire type and
/// are dropped by parsing itself.
pub fn normalize_nodes(nodes: Vec<NodeDescription>, plugin_name: &str) -> Vec<NodeDescription> {
    nodes
        .into_iter()
        .map(|node| normalize_node(node, plugin_name))
        .collect()
}

/// Normalizes one node's frontend per the P0 inline-only policy.
fn normalize_node(mut node: NodeDescription, plugin_name: &str) -> NodeDescription {
    if let Some(frontend) = &node.frontend {
        if frontend.kind != "inline" {
            log::warn!(
                "plugin {plugin_name} node {} declares unsupported frontend kind {:?}; \
                 ignoring frontend (P0 supports inline only)",
                node.type_id,
                frontend.kind,
            );
            node.frontend = None;
        }
    }
    node
}

#[cfg(test)]
#[path = "describe_tests.rs"]
mod tests;
