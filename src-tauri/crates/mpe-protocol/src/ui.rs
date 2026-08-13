//! The `uiCall` JSON-RPC method — host → plugin UI queries.
//!
//! Config panels rendered inside a plugin iframe (`mpe-plugin://` frames)
//! need runtime discovery data that static `config_schema` cannot express:
//! which MCP tools/resources/prompts a server exposes, what a tool's
//! `input_schema` looks like, and so on. Instead of routing those queries
//! through the host, the panel asks the host to relay a `uiCall` request to
//! the plugin process that owns the node type; the plugin answers with
//! arbitrary JSON (or a JSON-RPC error).
//!
//! Wire shape (request, host → plugin):
//! ```json
//! {"jsonrpc":"2.0","id":7,"method":"uiCall","params":{"method":"mcp.discover","params":{...}}}
//! ```
//! The response is a standard JSON-RPC response whose `result` is any JSON
//! value; errors use the standard JSON-RPC error object.

use serde::{Deserialize, Serialize};

/// The JSON-RPC method name for UI queries.
pub const UI_CALL_METHOD: &str = "uiCall";

/// Parameters of a `uiCall` request.
///
/// `method` is the plugin-defined sub-method (e.g. `"mcp.discover"`);
/// `params` is an opaque payload whose shape is owned by the plugin's
/// sub-method. `params` defaults to `Value::Null` when the caller has no
/// payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiCallParams {
    /// Plugin-defined sub-method name.
    pub method: String,
    /// Opaque sub-method payload.
    pub params: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_call_params_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let params = UiCallParams {
            method: "mcp.discover".to_string(),
            params: serde_json::json!({
                "transport": "stdio",
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-everything"],
                "env": {},
                "protocol_version": "2025-11-25"
            }),
        };
        let json = serde_json::to_string(&params)?;
        assert_eq!(
            json,
            r#"{"method":"mcp.discover","params":{"transport":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-everything"],"env":{},"protocol_version":"2025-11-25"}}"#
        );
        let back: UiCallParams = serde_json::from_str(&json)?;
        assert_eq!(back, params);
        Ok(())
    }

    #[test]
    fn ui_call_params_null_payload() -> Result<(), Box<dyn std::error::Error>> {
        let params = UiCallParams {
            method: "mcp.discover".to_string(),
            params: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&params)?;
        assert_eq!(json, r#"{"method":"mcp.discover","params":null}"#);
        let back: UiCallParams = serde_json::from_str(&json)?;
        assert_eq!(back, params);
        Ok(())
    }

    #[test]
    fn ui_call_wire_shape_inside_request() -> Result<(), Box<dyn std::error::Error>> {
        let request = crate::request::Request::new(
            7,
            UI_CALL_METHOD,
            serde_json::to_value(UiCallParams {
                method: "mcp.discover".to_string(),
                params: serde_json::json!({"url": "https://example.com/mcp"}),
            })?,
        );
        let json = serde_json::to_string(&request)?;
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","id":7,"method":"uiCall","params":{"method":"mcp.discover","params":{"url":"https://example.com/mcp"}}}"#
        );
        Ok(())
    }
}
