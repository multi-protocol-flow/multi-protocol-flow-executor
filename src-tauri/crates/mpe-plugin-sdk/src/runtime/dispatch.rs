//! JSON-RPC method dispatch (private to the runtime).
//!
//! Decodes framed messages into requests/notifications and routes them:
//! `describe`/`validate`/`shutdown` are answered inline, `execute` is
//! spawned per request (pipelining), unknown methods get a `-32601` error,
//! and `cancel`/`shutdown` notifications are handled without a response.

use std::sync::Arc;

use mpe_protocol::error::JsonRpcError;
use mpe_protocol::notification::Notification;
use mpe_protocol::request::Request;
use mpe_protocol::response::Response;
use tokio::sync::mpsc;

use super::{Plugin, RuntimeError};
use crate::context::ExecuteContext;
use crate::types::{CANONICAL_ICON_NAMES, NodeDescription};

/// A decoded inbound message.
pub(crate) enum Message {
    /// A request expecting a response.
    Request(Request),
    /// A fire-and-forget notification.
    Notification(Notification),
}

/// Decodes a frame: JSON → `Request`, else `Notification`, else `None`.
pub(crate) fn demux(line: &[u8]) -> Option<Message> {
    let value: serde_json::Value = match serde_json::from_slice(line) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("mpe-plugin-sdk: skipping malformed JSON frame: {err}");
            return None;
        }
    };
    if let Ok(request) = serde_json::from_value::<Request>(value.clone()) {
        return Some(Message::Request(request));
    }
    match serde_json::from_value::<Notification>(value) {
        Ok(notification) => Some(Message::Notification(notification)),
        Err(err) => {
            eprintln!("mpe-plugin-sdk: frame is neither request nor notification: {err}");
            None
        }
    }
}

/// What the event loop should do after handling a message.
#[derive(PartialEq, Eq)]
pub(crate) enum Action {
    /// Keep reading.
    Continue,
    /// Stop the loop (shutdown requested).
    Shutdown,
}

/// Handles one request; `execute` is spawned, everything else is answered
/// inline. Returns the next loop action.
pub(crate) async fn handle_request<P: Plugin>(
    plugin: &Arc<P>,
    request: Request,
    frame_tx: mpsc::Sender<Vec<u8>>,
) -> Result<Action, RuntimeError> {
    match request.method.as_str() {
        "describe" => {
            let nodes = plugin.describe();
            warn_unknown_icons(&nodes);
            let result = serde_json::to_value(nodes).map_err(RuntimeError::Serialize)?;
            send_response(
                &frame_tx,
                &Response::Success {
                    id: request.id,
                    result,
                },
            )
            .await?;
            Ok(Action::Continue)
        }
        "validate" => {
            let config = request
                .params
                .get("config")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let result = match plugin.validate(&config) {
                Ok(()) => serde_json::json!({ "valid": true }),
                Err(message) => serde_json::json!({ "valid": false, "message": message }),
            };
            send_response(
                &frame_tx,
                &Response::Success {
                    id: request.id,
                    result,
                },
            )
            .await?;
            Ok(Action::Continue)
        }
        "execute" => {
            tokio::spawn(execute_task(Arc::clone(plugin), request, frame_tx));
            Ok(Action::Continue)
        }
        "shutdown" => {
            // M2b owns the exact shutdown shape (request vs notification);
            // answer politely either way, then exit.
            send_response(
                &frame_tx,
                &Response::Success {
                    id: request.id,
                    result: serde_json::Value::Null,
                },
            )
            .await?;
            Ok(Action::Shutdown)
        }
        _ => {
            send_response(
                &frame_tx,
                &Response::Error {
                    id: request.id,
                    error: JsonRpcError::method_not_found(),
                },
            )
            .await?;
            Ok(Action::Continue)
        }
    }
}

/// Executes one request in its own task: build the context, run the plugin
/// handler, and reply with the result plus the context's `variable_updates`.
async fn execute_task<P: Plugin>(
    plugin: Arc<P>,
    request: Request,
    frame_tx: mpsc::Sender<Vec<u8>>,
) {
    let mut ctx = ExecuteContext::from_params(&request.params, frame_tx.clone());
    let result = plugin.execute(&mut ctx).await;
    let mut payload = match serde_json::to_value(result) {
        Ok(payload) => payload,
        Err(err) => {
            eprintln!("mpe-plugin-sdk: execute result serialization failed: {err}");
            return;
        }
    };
    payload["variable_updates"] = match serde_json::to_value(ctx.variable_updates()) {
        Ok(updates) => updates,
        Err(err) => {
            eprintln!("mpe-plugin-sdk: variable_updates serialization failed: {err}");
            serde_json::Value::Null
        }
    };
    let response = Response::Success {
        id: request.id,
        result: payload,
    };
    if let Err(err) = send_response(&frame_tx, &response).await {
        eprintln!("mpe-plugin-sdk: failed to send execute response: {err}");
    }
}

/// Handles one notification; returns `true` when the loop must exit.
pub(crate) fn handle_notification<P: Plugin>(plugin: &Arc<P>, notification: &Notification) -> bool {
    match notification.method.as_str() {
        "cancel" => match notification
            .params
            .get("id")
            .and_then(serde_json::Value::as_u64)
        {
            Some(request_id) => plugin.cancel(request_id),
            None => eprintln!("mpe-plugin-sdk: cancel notification without numeric `id`; ignored"),
        },
        "flowEnded" => match notification
            .params
            .get("execution_id")
            .and_then(serde_json::Value::as_str)
        {
            Some(execution_id) => plugin.flow_ended(execution_id),
            None => eprintln!(
                "mpe-plugin-sdk: flowEnded notification without string `execution_id`; ignored"
            ),
        },
        "shutdown" => return true,
        method => eprintln!("mpe-plugin-sdk: ignoring unknown notification `{method}`"),
    }
    false
}

/// Serializes and enqueues one response frame, awaiting backpressure.
async fn send_response(
    frame_tx: &mpsc::Sender<Vec<u8>>,
    response: &Response,
) -> Result<(), RuntimeError> {
    let bytes = serde_json::to_vec(response).map_err(RuntimeError::Serialize)?;
    frame_tx
        .send(bytes)
        .await
        .map_err(|_| RuntimeError::StdoutClosed)
}

/// Collects the node icons that the host frontend cannot render
/// (not in [`CANONICAL_ICON_NAMES`]).
///
/// Advisory: describe still succeeds; the host just falls back to a
/// placeholder icon. The stderr warning in [`warn_unknown_icons`] gives the
/// plugin author immediate feedback instead of a silently "lost" icon.
pub(crate) fn unknown_icons(nodes: &[NodeDescription]) -> Vec<&str> {
    let mut unknown = Vec::new();
    for node in nodes {
        if let Some(icon) = node.icon.as_deref() {
            if !CANONICAL_ICON_NAMES.contains(&icon) && !unknown.contains(&icon) {
                unknown.push(icon);
            }
        }
    }
    unknown
}

/// Warns on stderr for every icon outside the host's renderable vocabulary.
pub(crate) fn warn_unknown_icons(nodes: &[NodeDescription]) {
    for icon in unknown_icons(nodes) {
        eprintln!(
            "mpe-plugin-sdk: a node declares icon `{icon}` which the host frontend cannot \
             render (missing from CANONICAL_ICON_NAMES); the host will show a fallback icon. \
             Pick a name from CANONICAL_ICON_NAMES or extend src/lib/nodeIcons.ts."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Node descriptions whose icons are all renderable produce no warnings.
    #[test]
    fn unknown_icons_empty_for_known_icons() {
        let nodes = vec![
            NodeDescription::new("demo:connect", "Connect"),
            NodeDescription::new("demo:command", "Command"),
            NodeDescription::new("demo:close", "Close"),
        ];
        let mut nodes = nodes;
        nodes[0].icon = Some("database".to_string());
        nodes[1].icon = Some("terminal".to_string());
        nodes[2].icon = Some("plug".to_string());
        assert!(unknown_icons(&nodes).is_empty());
    }

    /// Unknown icons are reported once each, deduplicated.
    #[test]
    fn unknown_icons_reported_deduplicated() {
        let mut node = NodeDescription::new("demo:rocket", "Rocket");
        node.icon = Some("rocket".to_string());
        let mut other = NodeDescription::new("demo:zap", "Zap");
        other.icon = Some("rocket".to_string());
        let nodes = vec![node, other];
        assert_eq!(unknown_icons(&nodes), vec!["rocket"]);
    }

    /// No icon at all is fine (host applies its default).
    #[test]
    fn unknown_icons_ignores_nodes_without_icon() {
        let nodes = vec![NodeDescription::new("demo:plain", "Plain")];
        assert!(unknown_icons(&nodes).is_empty());
    }
}
