//! JSON-RPC 2.0 response type (plugin → host direction).
//!
//! The response is a **discriminated union** with exactly two variants: a
//! `result` payload or an `error` object. The mixed form (both keys present)
//! is rejected at the wire boundary by the custom `Deserialize`
//! implementation, and the enum shape itself makes a mixed-form value
//! unrepresentable in memory.

use serde::{Deserialize, Serialize};

use crate::error::JsonRpcError;

/// A JSON-RPC 2.0 response to a request with the given `id`.
///
/// Exactly one of `result` or `error` is carried — enforced by the enum
/// shape and by the custom `Deserialize` impl, which rejects objects that
/// contain both keys (mixed form).
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Successful response carrying the result of the invoked method.
    Success {
        /// The id of the request this response answers.
        id: u64,
        /// The method's result payload.
        result: serde_json::Value,
    },
    /// Error response carrying a structured JSON-RPC error object.
    Error {
        /// The id of the request this response answers.
        id: u64,
        /// The structured error object.
        error: JsonRpcError,
    },
}

/// Wire representation of a successful response: `{"jsonrpc":"2.0","id":1,"result":...}`.
#[derive(Serialize)]
struct SuccessRepr<'a> {
    jsonrpc: &'static str,
    id: u64,
    result: &'a serde_json::Value,
}

/// Wire representation of an error response: `{"jsonrpc":"2.0","id":1,"error":{...}}`.
#[derive(Serialize)]
struct ErrorRepr<'a> {
    jsonrpc: &'static str,
    id: u64,
    error: &'a JsonRpcError,
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Response::Success { id, result } => {
                SuccessRepr {
                    jsonrpc: "2.0",
                    id: *id,
                    result,
                }
                .serialize(serializer)
            }
            Response::Error { id, error } => {
                ErrorRepr {
                    jsonrpc: "2.0",
                    id: *id,
                    error,
                }
                .serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("a JSON-RPC response must be a JSON object"))?;

        let has_result = object.contains_key("result");
        let has_error = object.contains_key("error");

        match (has_result, has_error) {
            (true, true) => Err(D::Error::custom(
                "a JSON-RPC response cannot contain both `result` and `error`",
            )),
            (true, false) => {
                #[derive(Deserialize)]
                struct SuccessWire {
                    id: u64,
                    result: serde_json::Value,
                }
                let wire =
                    SuccessWire::deserialize(value).map_err(D::Error::custom)?;
                Ok(Response::Success {
                    id: wire.id,
                    result: wire.result,
                })
            }
            (false, true) => {
                #[derive(Deserialize)]
                struct ErrorWire {
                    id: u64,
                    error: JsonRpcError,
                }
                let wire = ErrorWire::deserialize(value).map_err(D::Error::custom)?;
                Ok(Response::Error {
                    id: wire.id,
                    error: wire.error,
                })
            }
            (false, false) => Err(D::Error::custom(
                "a JSON-RPC response must contain either `result` or `error`",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_success_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let response = Response::Success {
            id: 1,
            result: serde_json::json!({"ok": true}),
        };
        let json = serde_json::to_string(&response)?;
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#
        );
        let back: Response = serde_json::from_str(&json)?;
        assert_eq!(back, response);
        Ok(())
    }

    #[test]
    fn response_error_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let response = Response::Error {
            id: 1,
            error: JsonRpcError::parse_error(),
        };
        let json = serde_json::to_string(&response)?;
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#
        );
        let back: Response = serde_json::from_str(&json)?;
        assert_eq!(back, response);
        Ok(())
    }

    #[test]
    fn response_union_discriminates() -> Result<(), Box<dyn std::error::Error>> {
        let result_json = r#"{"jsonrpc":"2.0","id":1,"result":{"a":1}}"#;
        let error_json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;

        let success = serde_json::from_str::<Response>(result_json)?;
        assert!(matches!(success, Response::Success { id: 1, .. }));
        let error = serde_json::from_str::<Response>(error_json)?;
        assert!(matches!(error, Response::Error { id: 1, .. }));
        Ok(())
    }

    #[test]
    fn response_rejects_mixed_and_empty_forms() {
        let mixed = r#"{"jsonrpc":"2.0","id":1,"result":1,"error":{"code":-1,"message":"x"}}"#;
        assert!(serde_json::from_str::<Response>(mixed).is_err());
        let neither = r#"{"jsonrpc":"2.0","id":1}"#;
        assert!(serde_json::from_str::<Response>(neither).is_err());
        let not_object = r#"["jsonrpc","2.0"]"#;
        assert!(serde_json::from_str::<Response>(not_object).is_err());
    }

    #[test]
    fn response_tolerates_missing_jsonrpc_on_parse() -> Result<(), Box<dyn std::error::Error>> {
        // `jsonrpc` defaults to "2.0" by construction; parsing without it is
        // tolerated for robustness against minimalistic peers.
        let bare = r#"{"id":3,"result":null}"#;
        let response = serde_json::from_str::<Response>(bare)?;
        assert!(matches!(response, Response::Success { id: 3, .. }));
        Ok(())
    }
}
