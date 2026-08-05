//! Plugin error types.
//!
//! Host-side error taxonomy for plugin operations: spawn failure, process
//! crash, request timeout, framing/protocol violations, and JSON-RPC error
//! responses mapped into typed variants.

use std::time::Duration;

use mpe_protocol::error::JsonRpcError;

/// Host-side plugin error taxonomy.
///
/// These errors are produced by the transport (spawn/read/write/timeout) and
/// by error responses the plugin itself returns; the latter are carried in
/// [`PluginError::JsonRpc`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PluginError {
    /// The plugin process could not be spawned.
    #[error("failed to spawn plugin process: {0}")]
    SpawnFailed(String),
    /// A request did not receive a response within the timeout.
    ///
    /// The pending entry is removed and a best-effort `cancel` RPC is fired;
    /// this is the only mechanism that covers a stuck-but-alive plugin.
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    /// The plugin process crashed or its stdio stream was lost while
    /// requests were still outstanding.
    #[error("plugin process crashed: {0}")]
    PluginCrashed(String),
    /// The plugin exited with a non-zero status.
    #[error("plugin process exited with status {0}")]
    PluginExited(String),
    /// A line from the plugin was not a valid framed message.
    #[error("malformed line from plugin: {0}")]
    MalformedLine(String),
    /// A line exceeded the `2 MiB` framing cap.
    #[error("line from plugin exceeds maximum length")]
    LineTooLong,
    /// The peer violated the JSON-RPC protocol.
    #[error("protocol error: {0}")]
    ProtocolError(String),
    /// The plugin returned a JSON-RPC error object for a request.
    #[error("plugin returned JSON-RPC error {code}: {message}")]
    JsonRpc {
        /// JSON-RPC error code (see JSON-RPC 2.0 spec).
        code: i64,
        /// Human-readable error message.
        message: String,
        /// Optional structured error payload.
        data: Option<serde_json::Value>,
    },
    /// I/O failure on the stdio transport.
    #[error("plugin transport io error: {0}")]
    Io(String),
    /// The connection is shut down or shutting down; no new requests.
    #[error("plugin connection is shutting down")]
    Shutdown,
    /// A response arrived for an id with no pending request (late/duplicate).
    #[error("response for unknown request id {0}")]
    UnknownId(u64),
}

impl PluginError {
    /// Maps a JSON-RPC error object from the plugin into the typed variant.
    ///
    /// Standard codes keep their code+message verbatim; the mapping is
    /// canonical so downstream code can switch on [`PluginError::JsonRpc`]
    /// without re-parsing the wire form.
    pub fn from_jsonrpc_error(error: &JsonRpcError) -> Self {
        PluginError::JsonRpc {
            code: error.code,
            message: error.message.clone(),
            data: error.data.clone(),
        }
    }

    /// True when this error corresponds to a JSON-RPC standard error code.
    pub fn is_standard_jsonrpc_code(&self) -> bool {
        matches!(
            self,
            PluginError::JsonRpc { code, .. } if matches!(*code, -32700 | -32600 | -32601 | -32602 | -32603 | -32099..=-32000)
        )
    }
}

impl From<&JsonRpcError> for PluginError {
    fn from(error: &JsonRpcError) -> Self {
        Self::from_jsonrpc_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonrpc_error_maps_to_typed_variant() {
        let wire = JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(serde_json::json!({"field": "url"})),
        };
        let err = PluginError::from_jsonrpc_error(&wire);
        assert_eq!(
            err,
            PluginError::JsonRpc {
                code: -32602,
                message: "Invalid params".to_string(),
                data: Some(serde_json::json!({"field": "url"})),
            }
        );
        assert!(err.is_standard_jsonrpc_code());
    }

    #[test]
    fn non_standard_code_not_standard() {
        let err = PluginError::from_jsonrpc_error(&JsonRpcError {
            code: -1,
            message: "custom".to_string(),
            data: None,
        });
        assert!(!err.is_standard_jsonrpc_code());
    }

    #[test]
    fn from_reference_impl_matches_constructor() {
        let wire = JsonRpcError::internal_error();
        assert_eq!(
            PluginError::from(&wire),
            PluginError::from_jsonrpc_error(&wire)
        );
    }

    #[test]
    fn display_covers_variants() {
        assert!(PluginError::Timeout(Duration::from_secs(1))
            .to_string()
            .contains("timed out"));
        assert!(PluginError::PluginCrashed("boom".to_string())
            .to_string()
            .contains("crashed"));
        assert!(PluginError::MalformedLine("x".to_string())
            .to_string()
            .contains("malformed"));
        assert!(PluginError::UnknownId(3).to_string().contains("3"));
    }
}
