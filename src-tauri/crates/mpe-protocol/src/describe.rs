//! Describe handshake wire types (plugin → host direction).
//!
//! After spawning a plugin the host issues `describe` with `params: {}`;
//! the plugin answers with its authoritative [`PluginManifest`] (echo, whose
//! `capabilities` override the on-disk manifest) plus a
//! [`NodeDescription`] per node type it provides. This module is strictly
//! host-agnostic — it depends only on `serde`/`serde_json` and the manifest
//! schema — so the SDK and language templates share the exact wire contract.

use serde::{Deserialize, Serialize};

use crate::manifest::{PluginCapabilities, PluginManifest};

/// Wire method name of the describe RPC.
pub const DESCRIBE_METHOD: &str = "describe";

/// A node port as described by a plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortDescription {
    /// Port id, e.g. `"in"` / `"out"` / `"true"` / `"false"`.
    pub id: String,
    /// Human-readable port label, e.g. `"Success"`.
    pub label: String,
    /// Port direction: `"in"` (input) or `"out"` (output).
    pub kind: String,
}

/// Frontend description of a node.
///
/// All frontend kinds pass through to the host (PR-0); the host generates
/// the iframe `url` (per-plugin origin, D5/F3) during normalization when a
/// frontend is present. The reserved `config_panel` / `viewer` / `help`
/// fields are deferred to the P1 schema and deliberately not modeled yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontendDescription {
    /// Frontend kind, e.g. `"inline"`, `"iframe"`, `"config-panel-url"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Inline HTML content for `kind == "inline"` frontends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Host-generated iframe URL (`mpe-plugin://<plugin-id>.localhost/…` on
    /// macOS/Linux, `http://<plugin-id>.localhost/…` on Windows/Android).
    /// Plugins may omit it — the host fills it in during normalization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// One node type a plugin provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDescription {
    /// Unique node type id (vendor-prefixed, e.g. `"acme:ping"`).
    pub type_id: String,
    /// Display name shown in the node palette.
    pub display_name: String,
    /// Optional palette category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional icon identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional color (hex, e.g. `"#ff0000"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Ports of the node (empty for terminal nodes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortDescription>,
    /// Default config used when the node is dropped onto the canvas.
    #[serde(default)]
    pub default_config: serde_json::Value,
    /// JSON Schema describing the node's config (drives a dynamic config
    /// form on the host; `None` = host's hardcoded panel, if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// Frontend description (P0: `"inline"` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend: Option<FrontendDescription>,
    /// Runtime capabilities declared for this node type.
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

/// The full `describe` response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeResult {
    /// The plugin's authoritative manifest echo; its `capabilities` override
    /// the on-disk manifest.
    pub plugin: PluginManifest,
    /// Node types the plugin provides (may be empty).
    #[serde(default)]
    pub nodes: Vec<NodeDescription>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::manifest::PluginEntry;

    fn sample_manifest() -> PluginManifest {
        PluginManifest {
            name: "demo".to_string(),
            version: "1.2.3".to_string(),
            author: None,
            min_host_version: None,
            description: None,
            entry: PluginEntry {
                command: "demo".to_string(),
                args: Vec::new(),
            },
            env: HashMap::new(),
            permissions: Vec::new(),
            capabilities: PluginCapabilities { streaming: true },
        }
    }

    #[test]
    fn describe_result_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let result = DescribeResult {
            plugin: sample_manifest(),
            nodes: vec![NodeDescription {
                type_id: "demo:ping".to_string(),
                display_name: "Ping".to_string(),
                category: Some("demo".to_string()),
                icon: None,
                color: Some("#ff0000".to_string()),
                ports: vec![
                    PortDescription {
                        id: "in".to_string(),
                        label: "In".to_string(),
                        kind: "in".to_string(),
                    },
                    PortDescription {
                        id: "out".to_string(),
                        label: "Out".to_string(),
                        kind: "out".to_string(),
                    },
                ],
                default_config: serde_json::json!({"target": "localhost"}),
                config_schema: Some(serde_json::json!({"type": "object"})),
                frontend: Some(FrontendDescription {
                    kind: "inline".to_string(),
                    content: Some("<p>ping</p>".to_string()),
                    url: None,
                }),
                capabilities: PluginCapabilities::default(),
            }],
        };
        let json = serde_json::to_string(&result)?;
        let back: DescribeResult = serde_json::from_str(&json)?;
        assert_eq!(back, result);
        Ok(())
    }

    #[test]
    fn describe_result_parses_wire_sample() {
        let json = r#"{
            "plugin": {"name": "demo", "version": "1.0.0", "entry": {"command": "demo"}},
            "nodes": [
                {
                    "type_id": "demo:a",
                    "display_name": "A",
                    "ports": [{"id": "in", "label": "In", "kind": "in"}],
                    "frontend": {"type": "inline", "content": "<p>a</p>"}
                },
                {
                    "type_id": "demo:b",
                    "display_name": "B",
                    "frontend": {"type": "iframe", "url": "https://x"}
                }
            ]
        }"#;
        let result: DescribeResult =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("wire sample should parse: {e}"));
        assert_eq!(result.plugin.name, "demo");
        assert_eq!(result.nodes.len(), 2);
        let a = &result.nodes[0];
        assert_eq!(a.type_id, "demo:a");
        assert_eq!(a.category, None);
        assert_eq!(a.default_config, serde_json::Value::Null);
        assert_eq!(a.ports.len(), 1);
        assert_eq!(a.ports[0].kind, "in");
        assert_eq!(a.frontend.as_ref().map(|f| f.kind.as_str()), Some("inline"));
        assert_eq!(
            a.frontend.as_ref().and_then(|f| f.content.as_deref()),
            Some("<p>a</p>")
        );
        // The `url` field is absent for inline frontends (host fills it in).
        assert_eq!(a.frontend.as_ref().and_then(|f| f.url.as_deref()), None);
        // A non-inline frontend still parses on the wire with its `url`;
        // host-side normalization passes it through and (re)generates `url`.
        let b = &result.nodes[1];
        assert_eq!(b.frontend.as_ref().map(|f| f.kind.as_str()), Some("iframe"));
        assert_eq!(b.frontend.as_ref().and_then(|f| f.content.as_deref()), None);
        assert_eq!(
            b.frontend.as_ref().and_then(|f| f.url.as_deref()),
            Some("https://x")
        );
    }

    #[test]
    fn describe_result_requires_plugin_defaults_nodes() {
        // `plugin` is required by the wire shape…
        assert!(serde_json::from_str::<DescribeResult>(r#"{"nodes": []}"#).is_err());
        // …but `nodes` defaults to empty.
        let json = r#"{"plugin": {"name": "d", "version": "1.0.0", "entry": {"command": "d"}}}"#;
        let result: DescribeResult =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("plugin-only result: {e}"));
        assert!(result.nodes.is_empty());
    }
}
