//! JSON-RPC 2.0 request type (host → plugin direction).

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request.
///
/// Batch arrays and notifications are forbidden **by construction**: `id` is
/// a plain `u64`, so it can never be `null` (notification) or an array
/// (batch element). A deserialized `Request` is therefore always a single
/// request with a caller-supplied numeric id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC protocol version; always `"2.0"`.
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    /// Client-generated identifier echoed in the matching response.
    pub id: u64,
    /// Method name to invoke on the peer.
    pub method: String,
    /// Positional or named parameters; opaque to the transport.
    pub params: serde_json::Value,
}

impl Request {
    /// Creates a new request with the protocol version pinned to `"2.0"`.
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let request = Request::new(7, "describe", serde_json::json!({"dir": "x"}));
        let json = serde_json::to_string(&request)?;
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","id":7,"method":"describe","params":{"dir":"x"}}"#
        );
        let back: Request = serde_json::from_str(&json)?;
        assert_eq!(back, request);
        Ok(())
    }

    #[test]
    fn request_id_is_plain_u64() {
        // A request carrying `id: null` or an array must be rejected — this
        // is the batch/notification forbidden-by-construction guarantee.
        let notification = r#"{"jsonrpc":"2.0","id":null,"method":"log","params":{}}"#;
        assert!(serde_json::from_str::<Request>(notification).is_err());
        let batch = r#"{"jsonrpc":"2.0","id":[1,2],"method":"describe","params":{}}"#;
        assert!(serde_json::from_str::<Request>(batch).is_err());
    }
}
