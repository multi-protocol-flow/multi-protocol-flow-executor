//! JSON-RPC 2.0 notification type (host → plugin and plugin → host).
//!
//! A notification is a fire-and-forget message: it carries a `method` and
//! `params` but **no `id`** — by construction the struct has no `id` field,
//! so an id can never appear in serialized output and a `Notification` can
//! never be mistaken for a `Request` (which requires a numeric `id`).

use serde::{Deserialize, Serialize};

/// Wire method name of the `flowEnded` host → plugin notification.
///
/// The host emits this fire-and-forget notification when a flow execution
/// finishes (success, failure, or cancellation), carrying
/// `params: { "execution_id": "<per-execution id>" }`. Plugins use it to
/// release per-execution resources (e.g. a connection pool keyed by
/// `execution_id`). Plugins that do not understand it must ignore it.
pub const FLOW_ENDED_METHOD: &str = "flowEnded";

/// A JSON-RPC 2.0 notification.
///
/// The absence of an `id` field is the wire contract: serializing a
/// `Notification` never emits an `id` key, and deserializing a notification
/// as a [`Request`] always fails (the `id` member is required there).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    /// JSON-RPC protocol version; always `"2.0"`.
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    /// Method name to invoke on the peer.
    pub method: String,
    /// Positional or named parameters; opaque to the transport.
    pub params: serde_json::Value,
}

/// Correlation fields carried by every event/log notification.
///
/// Frozen in the schema day one so plugin event streams can be correlated
/// to a single node execution (see `docs/plugin-protocol.md` §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCorrelation {
    /// The flow execution this notification belongs to.
    pub execution_id: String,
    /// The node instance within the execution that produced it.
    pub node_instance_id: String,
}

impl Notification {
    /// Creates a new notification with the protocol version pinned to `"2.0"`.
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }

    /// Extracts the correlation fields (`execution_id` + `node_instance_id`)
    /// from `params` when present.
    ///
    /// Returns `None` when either field is missing or not a string — such a
    /// notification is still delivered to un-filtered handlers, but cannot be
    /// correlated to an execution.
    pub fn correlation(&self) -> Option<NotificationCorrelation> {
        let execution_id = self.params.get("execution_id")?.as_str()?;
        let node_instance_id = self.params.get("node_instance_id")?.as_str()?;
        Some(NotificationCorrelation {
            execution_id: execution_id.to_string(),
            node_instance_id: node_instance_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Request;

    #[test]
    fn notification_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let notification = Notification::new(
            "log",
            serde_json::json!({
                "execution_id": "flow-1",
                "node_instance_id": "node-7",
                "level": "info",
                "message": "hello"
            }),
        );
        let json = serde_json::to_string(&notification)?;
        let back: Notification = serde_json::from_str(&json)?;
        assert_eq!(back, notification);
        // `jsonrpc`/`method` are leading scalar keys; the params map ordering
        // is not part of the contract.
        assert!(json.starts_with(r#"{"jsonrpc":"2.0","method":"log","params":"#));
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["params"]["execution_id"], "flow-1");
        assert_eq!(parsed["params"]["node_instance_id"], "node-7");
        Ok(())
    }

    #[test]
    fn test_notification_id_null() -> Result<(), Box<dyn std::error::Error>> {
        // A notification must never serialize an `id` member — the absence of
        // an id is what distinguishes it from a request on the wire.
        let notification = Notification::new("event", serde_json::json!({"n": 1}));
        let json = serde_json::to_string(&notification)?;
        assert!(
            !json.contains("\"id\""),
            "notification must not carry an id, got: {json}"
        );
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","method":"event","params":{"n":1}}"#
        );

        // A `Notification` never parses as a `Request` (Request requires a
        // numeric id; notifications have none by construction).
        assert!(serde_json::from_str::<Request>(&json).is_err());

        // And an explicit `"id": null` form is rejected as a Request too,
        // while still parsing as a Notification (serde ignores it as an
        // unknown member — this is the lenient read path of the demuxer).
        let explicit_null = r#"{"jsonrpc":"2.0","id":null,"method":"event","params":{"n":1}}"#;
        assert!(serde_json::from_str::<Request>(explicit_null).is_err());
        let notif: Notification = serde_json::from_str(explicit_null)?;
        assert_eq!(notif.method, "event");
        Ok(())
    }

    #[test]
    fn notification_correlation_extracts_fields() -> Result<(), Box<dyn std::error::Error>> {
        let notification = Notification::new(
            "log",
            serde_json::json!({"execution_id": "e-1", "node_instance_id": "n-2", "msg": "x"}),
        );
        let correlation = notification.correlation();
        assert!(correlation.is_some());
        let correlation = correlation.unwrap_or_else(|| panic!("correlation should parse"));
        assert_eq!(correlation.execution_id, "e-1");
        assert_eq!(correlation.node_instance_id, "n-2");
        Ok(())
    }

    #[test]
    fn notification_without_correlation_fields_yields_none() {
        let notification = Notification::new("log", serde_json::json!({"msg": "x"}));
        assert!(notification.correlation().is_none());
        let wrong_types = Notification::new(
            "log",
            serde_json::json!({"execution_id": 1, "node_instance_id": "n"}),
        );
        assert!(wrong_types.correlation().is_none());
    }
}
