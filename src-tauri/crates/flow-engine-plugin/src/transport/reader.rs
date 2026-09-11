//! Reader task owning the child's stdout.
//!
//! The reader is the only task that parses inbound frames. It dispatches on
//! the wire shape:
//! - a frame carrying `result`/`error` is a **response** — resolved against
//!   the pending map (unknown/duplicate ids are logged, never fatal);
//! - a frame carrying `method` (and no id) is a **notification** — demuxed
//!   to registered handlers *before* any pending-request lookup;
//! - anything else (including batch arrays) is **malformed** — skipped,
//!   logged, and counted; after `max_malformed_lines` consecutive malformed
//!   lines the plugin is killed.
//!
//! The reader never exits on a single malformed or error line.

use std::sync::Arc;

use tokio::io::AsyncRead;

use mpe_plugin_sdk::protocol::notification::Notification;
use mpe_plugin_sdk::protocol::response::Response;

use crate::error::PluginError;
use crate::framing::{FrameError, FrameReader};
use crate::transport::connection::{ConnectionShared, ExitReason};

/// Outcome of processing one inbound frame.
pub(crate) enum Dispatch {
    /// A well-formed response was resolved against the pending map.
    Response,
    /// A well-formed notification was demuxed to handlers.
    Notification,
    /// The frame violated the protocol; carries a human-readable reason.
    Malformed(String),
}

/// Runs the reader loop until EOF or a fatal stream error.
pub(crate) async fn reader_task(
    stdout: Box<dyn AsyncRead + Send + Unpin>,
    shared: Arc<ConnectionShared>,
) {
    let mut reader = FrameReader::new(stdout);
    let mut consecutive_malformed: u32 = 0;
    loop {
        let malformed_reason: Option<String> = match reader.next_frame().await {
            Ok(Some(frame)) => match dispatch_frame(&frame, &shared) {
                Dispatch::Response | Dispatch::Notification => {
                    consecutive_malformed = 0;
                    None
                }
                Dispatch::Malformed(reason) => Some(reason),
            },
            Ok(None) => {
                // Clean EOF between frames.
                shared.on_eof().await;
                break;
            }
            Err(FrameError::UnexpectedEof) => {
                // EOF in the middle of a line: the peer died mid-frame.
                shared.on_eof().await;
                break;
            }
            Err(FrameError::Io(e)) => {
                shared.mark_exit(ExitReason::Crashed(PluginError::Io(e.to_string())));
                break;
            }
            Err(FrameError::LineTooLong) => Some("line exceeds maximum length".to_string()),
            Err(FrameError::InvalidUtf8) => Some("line is not valid UTF-8".to_string()),
        };

        if let Some(reason) = malformed_reason {
            consecutive_malformed += 1;
            shared.note_malformed_line();
            log::warn!(
                "malformed line from plugin ({reason}); consecutive={consecutive_malformed}"
            );
            if consecutive_malformed >= shared.config.max_malformed_lines {
                log::error!(
                    "killing plugin after {consecutive_malformed} consecutive malformed lines"
                );
                shared.kill_child().await;
                shared.mark_exit(ExitReason::Crashed(PluginError::MalformedLine(reason)));
                break;
            }
        }
    }
}

/// Parses one frame and routes it (response → pending map, notification →
/// handlers). Batch arrays and mixed forms are rejected as malformed by
/// construction of the `mpe_plugin_sdk::protocol` types.
fn dispatch_frame(frame: &[u8], shared: &ConnectionShared) -> Dispatch {
    let value: serde_json::Value = match serde_json::from_slice(frame) {
        Ok(value) => value,
        Err(e) => return Dispatch::Malformed(format!("not valid JSON: {e}")),
    };
    let Some(object) = value.as_object() else {
        return Dispatch::Malformed("frame is not a JSON object".to_string());
    };

    if object.contains_key("result") || object.contains_key("error") {
        match serde_json::from_value::<Response>(value) {
            Ok(Response::Success { id, result }) => {
                shared.resolve_response(id, Ok(result));
                Dispatch::Response
            }
            Ok(Response::Error { id, error }) => {
                shared.resolve_response(id, Err(PluginError::from_jsonrpc_error(&error)));
                Dispatch::Response
            }
            Err(e) => Dispatch::Malformed(format!("invalid response object: {e}")),
        }
    } else if object.contains_key("method") {
        // Notification path: demuxed BEFORE any pending-request lookup.
        match serde_json::from_value::<Notification>(value) {
            Ok(notification) => {
                shared.dispatch_notification(&notification);
                Dispatch::Notification
            }
            Err(e) => Dispatch::Malformed(format!("invalid notification object: {e}")),
        }
    } else {
        Dispatch::Malformed(
            "frame is neither a response (result/error) nor a notification (method)".to_string(),
        )
    }
}
