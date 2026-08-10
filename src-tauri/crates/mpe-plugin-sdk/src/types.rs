//! Shared plugin-facing model types.
//!
//! These describe what a plugin node IS (`NodeDescription`) and what an
//! `execute` call RETURNS (`ExecuteResult`). They are deliberately
//! SDK-local: `mpe-protocol` does not yet carry a `NodeDescription` (the
//! host-side describe contract lands in M2a'/M5b), so this crate defines a
//! serde-compatible shape now and reconciles it against the host during the
//! M5b conformance milestone. All optional fields serialize only when
//! present, so minimal plugin code stays clean on the wire.

use serde::{Deserialize, Serialize};

/// Static description of one node type the plugin provides.
///
/// Returned by [`Plugin::describe`](crate::runtime::Plugin::describe) and
/// serialized as the `describe` response result (a JSON array of these).
/// Field names mirror the host-side describe contract (`type_id`,
/// `display_name`, …) so the M5b conformance suite can parse them as-is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDescription {
    /// Unique node type identifier, e.g. `"vendor:name"`. Must not collide
    /// with a built-in host type id.
    pub type_id: String,
    /// Human-readable node name shown in the host node library.
    pub display_name: String,
    /// Optional category grouping in the host node library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional icon identifier (host-dependent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional accent color (host-dependent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Node ports; `None` lets the host apply its defaults (`in`/`out`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<PortDescription>>,
    /// Default config injected into a freshly added node of this type.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub default_config: serde_json::Value,
    /// Optional JSON-Schema for the node config (drives host config forms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// Optional frontend description (shared wire shape from `mpe-protocol`).
    /// All kinds pass through to the host; the host generates the iframe
    /// `url` during normalization when a frontend is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend: Option<mpe_protocol::describe::FrontendDescription>,
    /// Optional runtime capabilities (e.g. streaming residency).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<mpe_protocol::manifest::PluginCapabilities>,
}

impl NodeDescription {
    /// Creates a node description with only the mandatory fields; everything
    /// else defaults to absent/empty.
    pub fn new(type_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            type_id: type_id.into(),
            display_name: display_name.into(),
            category: None,
            icon: None,
            color: None,
            ports: None,
            default_config: serde_json::json!({}),
            config_schema: None,
            frontend: None,
            capabilities: None,
        }
    }
}

/// Port direction constant: input port (`"in"`).
pub const PORT_KIND_IN: &str = "in";
/// Port direction constant: output port (`"out"`).
pub const PORT_KIND_OUT: &str = "out";

/// A single node port: an id, a display label and a direction.
///
/// Mirrors the wire contract (`mpe_protocol::describe::PortDescription`):
/// `{ id, label, kind }` with all three fields mandatory — the host
/// `registration.rs` derives the port position from `kind`
/// (`"in"` → `left`, anything else → `right`), so omitting `kind` would
/// break host-side describe deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortDescription {
    /// Port identifier, e.g. `"in"` / `"out"` / `"true"` / `"false"`.
    pub id: String,
    /// Human-readable display label for the port, e.g. `"Success"`.
    pub label: String,
    /// Port direction: `"in"` (input) or `"out"` (output).
    pub kind: String,
}

impl PortDescription {
    /// Creates a port with an id, a display label and a direction
    /// (see [`PORT_KIND_IN`] / [`PORT_KIND_OUT`]).
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: kind.into(),
        }
    }
}

/// The outcome of one `execute` call, serialized as the `execute` response
/// result. The runtime merges the context's `variable_updates` into the
/// response under a `variable_updates` member (flat top-level keys only,
/// per the locked A10 contract).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteResult {
    /// Whether the node execution succeeded; `false` routes to the `false`
    /// port (or aborts) per the node's `on_error` strategy.
    pub success: bool,
    /// Explicitly chosen next ports; empty lets the host apply its default
    /// routing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_ports: Vec<String>,
    /// Structured result data surfaced to the flow as `output_data`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_data: Option<serde_json::Value>,
    /// Human-readable error messages (included when `success` is `false`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    /// Optional ≤ 1 MiB report payload surfaced to the host report as
    /// `plugin_data`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_data: Option<serde_json::Value>,
}

impl Default for ExecuteResult {
    fn default() -> Self {
        Self {
            success: true,
            next_ports: Vec::new(),
            output_data: None,
            errors: Vec::new(),
            report_data: None,
        }
    }
}

impl ExecuteResult {
    /// A successful result carrying `output_data`.
    pub fn ok(output_data: serde_json::Value) -> Self {
        Self {
            output_data: Some(output_data),
            ..Self::default()
        }
    }

    /// A failed result carrying an error message.
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            errors: vec![message.into()],
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_result_defaults_success_true() {
        let result = ExecuteResult::default();
        assert!(result.success);
        assert!(result.next_ports.is_empty());
        assert!(result.output_data.is_none());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn execute_result_serializes_minimally() -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&ExecuteResult::default())?;
        assert_eq!(json, r#"{"success":true}"#);
        let json = serde_json::to_string(&ExecuteResult::fail("boom"))?;
        assert_eq!(json, r#"{"success":false,"errors":["boom"]}"#);
        Ok(())
    }

    #[test]
    fn node_description_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let node = NodeDescription::new("demo:echo", "Demo Echo");
        let json = serde_json::to_string(&node)?;
        let back: NodeDescription = serde_json::from_str(&json)?;
        assert_eq!(back, node);
        assert_eq!(back.type_id, "demo:echo");
        Ok(())
    }

    #[test]
    fn node_description_frontend_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = NodeDescription::new("demo:echo", "Demo Echo");
        node.frontend = Some(mpe_protocol::describe::FrontendDescription {
            kind: "iframe".to_string(),
            content: None,
            url: Some("http://demo.localhost/config-panel".to_string()),
        });
        let json = serde_json::to_string(&node)?;
        assert!(json.contains("\"frontend\""));
        let back: NodeDescription = serde_json::from_str(&json)?;
        let frontend = back
            .frontend
            .ok_or_else(|| "frontend missing after round trip".to_string())?;
        assert_eq!(frontend.kind, "iframe");
        assert_eq!(frontend.content, None);
        assert_eq!(
            frontend.url.as_deref(),
            Some("http://demo.localhost/config-panel")
        );
        Ok(())
    }
}
