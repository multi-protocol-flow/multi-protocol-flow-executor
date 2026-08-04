//! JSON-RPC 2.0 error objects and standard error-code constructors.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Machine-readable error code (see the JSON-RPC 2.0 spec).
    pub code: i64,
    /// Short human-readable description of the error.
    pub message: String,
    /// Optional structured error payload; omitted from the wire when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Standard error: invalid JSON was received (`-32700`).
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Standard error: the JSON sent is not a valid request object (`-32600`).
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    /// Standard error: the method does not exist or is not implemented (`-32601`).
    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    /// Standard error: invalid method parameter(s) (`-32602`).
    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }

    /// Standard error: internal JSON-RPC error (`-32603`).
    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }

    /// Server error in the implementation-defined range `-32000..=-32099`.
    ///
    /// `start` is the specific code to use (callers must pick a value within
    /// the range; the constructor does not re-anchor it).
    pub fn server_error(start: i64) -> Self {
        Self::new(start, "Server error")
    }

    fn new(code: i64, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let error = JsonRpcError {
            code: -32000,
            message: "Plugin exploded".to_string(),
            data: Some(serde_json::json!({"exit_code": 42})),
        };
        let json = serde_json::to_string(&error)?;
        assert_eq!(
            json,
            r#"{"code":-32000,"message":"Plugin exploded","data":{"exit_code":42}}"#
        );
        let back: JsonRpcError = serde_json::from_str(&json)?;
        assert_eq!(back, error);
        Ok(())
    }

    #[test]
    fn error_omits_none_data() -> Result<(), Box<dyn std::error::Error>> {
        let error = JsonRpcError::internal_error();
        let json = serde_json::to_string(&error)?;
        assert_eq!(json, r#"{"code":-32603,"message":"Internal error"}"#);
        Ok(())
    }

    #[test]
    fn standard_code_constructors() {
        assert_eq!(JsonRpcError::parse_error().code, -32700);
        assert_eq!(JsonRpcError::invalid_request().code, -32600);
        assert_eq!(JsonRpcError::method_not_found().code, -32601);
        assert_eq!(JsonRpcError::invalid_params().code, -32602);
        assert_eq!(JsonRpcError::internal_error().code, -32603);
        assert_eq!(JsonRpcError::server_error(-32099).code, -32099);
    }
}
