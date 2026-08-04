//! Async JSON-RPC event loop for plugins.
//!
//! Implements the plugin side of the `mpe-protocol` wire contract over
//! framed stdio: reads `describe`/`execute`/`validate`/`shutdown` requests
//! and `cancel`/`shutdown` notifications from stdin, dispatches them to the
//! plugin's [`Plugin`] implementation, and writes framed responses to
//! stdout.
//!
//! Design notes:
//! - **Single writer task.** Responses and log notifications are pushed as
//!   serialized frames into an `mpsc` channel; one writer task owns stdout
//!   and writes whole frames (payload + `LF`) atomically. Frames can never
//!   interleave, mirroring the host-side single-writer pattern.
//! - **Spawn-per-request pipelining.** `execute` is dispatched by spawning
//!   one task per request (the plugin's `execute` future must be `Send`).
//!   Responses are correlated by request id and may arrive in any order.
//!   `describe`/`validate` are handled inline — they are fast and order
//!   independent.
//! - **Malformed frames are skipped, never fatal.** A bad JSON line or a
//!   frame over the line cap is logged to stderr and skipped; the loop only
//!   exits on a clean EOF, an I/O error, or a `shutdown` message.
//! - **Backpressure.** The frame channel is bounded; `execute` response
//!   sends await for space (the host is expected to keep reading), while
//!   log notifications are fire-and-forget (`try_send`, dropped when full)
//!   so a log flood can never starve responses.

mod dispatch;
mod frame;

use std::fmt;
use std::future::Future;
use std::io;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

use crate::context::ExecuteContext;
use crate::types::{ExecuteResult, NodeDescription};

/// Bound on the frame channel between dispatch tasks and the writer task.
const FRAME_CHANNEL_CAPACITY: usize = 256;

/// The plugin contract: describe what nodes this plugin provides and execute
/// them when the host asks. The runtime owns all I/O; implementations are
/// pure handlers.
///
/// `execute` returns `impl Future + Send`: the runtime awaits it inside a
/// spawned task (spawn-per-request pipelining), so the future must be
/// `Send`. A plain `async fn` cannot express a `Send` bound on stable (the
/// compiler would reject `tokio::spawn`), hence the RPITIT shape.
pub trait Plugin: Send + Sync + 'static {
    /// Returns the node types this plugin provides.
    fn describe(&self) -> Vec<NodeDescription>;

    /// Validates a node config; `Ok(())` accepts it, `Err(message)` rejects
    /// it with a human-readable reason. The default accepts everything.
    fn validate(&self, _config: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }

    /// Executes one node instance with the host-resolved context.
    fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send;

    /// Cancellation hook, fired when the host sends a best-effort `cancel`
    /// notification for an in-flight request id (see the M1 transport's
    /// `fire_cancel`). The default does nothing; in-flight work can only be
    /// interrupted cooperatively from the plugin's own state.
    fn cancel(&self, _request_id: u64) {}
}

/// Fatal runtime errors. Framing violations are handled by skipping frames
/// and never surface here.
#[derive(Debug)]
pub enum RuntimeError {
    /// Reading stdin (or the underlying test stream) failed.
    Io(io::Error),
    /// A value failed to serialize for the wire.
    Serialize(serde_json::Error),
    /// The writer task is gone (stdout closed), so no further frames can be
    /// delivered to the host.
    StdoutClosed,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Io(err) => write!(f, "stdio io error: {err}"),
            RuntimeError::Serialize(err) => write!(f, "serialization error: {err}"),
            RuntimeError::StdoutClosed => write!(f, "stdout writer task exited"),
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeError::Io(err) => Some(err),
            RuntimeError::Serialize(err) => Some(err),
            RuntimeError::StdoutClosed => None,
        }
    }
}

/// Runs the plugin event loop against the real process stdio until a clean
/// EOF or a `shutdown` message. The plugin must implement [`Plugin`].
pub async fn run<P: Plugin>(plugin: P) -> Result<(), RuntimeError> {
    run_with_streams(plugin, tokio::io::stdin(), tokio::io::stdout()).await
}

/// The event loop, generic over the streams so tests can drive it with
/// in-memory duplex pairs. The output stream must be `Send` because the
/// writer task owns it.
pub(crate) async fn run_with_streams<P: Plugin>(
    plugin: P,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin + Send + 'static,
) -> Result<(), RuntimeError> {
    let plugin = Arc::new(plugin);
    let (frame_tx, frame_rx) = mpsc::channel(FRAME_CHANNEL_CAPACITY);
    let writer_task = tokio::spawn(writer_loop(output, frame_rx));
    let mut reader = frame::FrameReader::new(input);

    loop {
        let line = match reader.next_frame().await {
            Ok(Some(line)) => line,
            Ok(None) => break, // clean EOF: the host closed stdin.
            Err(frame::FrameError::UnexpectedEof) => break,
            Err(frame::FrameError::Io(err)) => return Err(RuntimeError::Io(err)),
            Err(err) => {
                eprintln!("mpe-plugin-sdk: skipping malformed frame: {err}");
                continue;
            }
        };
        match dispatch::demux(&line) {
            Some(dispatch::Message::Request(request)) => {
                let action = dispatch::handle_request(&plugin, request, frame_tx.clone()).await?;
                if action == dispatch::Action::Shutdown {
                    break;
                }
            }
            Some(dispatch::Message::Notification(notification)) => {
                if dispatch::handle_notification(&plugin, &notification) {
                    break;
                }
            }
            None => eprintln!("mpe-plugin-sdk: frame is neither request nor notification"),
        }
    }

    drop(frame_tx); // signal the writer to drain and exit.
    let _ = writer_task.await;
    Ok(())
}

/// The single writer task: serializes every frame write, so concurrent
/// senders can never interleave partial frames.
async fn writer_loop(
    mut output: impl AsyncWrite + Unpin + Send + 'static,
    mut frames: mpsc::Receiver<Vec<u8>>,
) {
    while let Some(frame) = frames.recv().await {
        if frame::write_frame(&mut output, &frame).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Default)]
    struct EchoPlugin;

    impl Plugin for EchoPlugin {
        fn describe(&self) -> Vec<NodeDescription> {
            vec![NodeDescription::new("test:echo", "Test Echo")]
        }

        // The runtime spawns execute futures, so the future must be Send; an
        // `async fn` cannot express that bound on stable (see `Plugin`).
        #[allow(clippy::manual_async_fn)]
        fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send {
            async move { ExecuteResult::ok(ctx.config().clone()) }
        }
    }

    /// Spawns the event loop over in-memory duplex pairs; returns the host's
    /// write half, read half, and the loop task.
    async fn spawn_in_memory() -> (
        tokio::io::DuplexStream,
        tokio::io::DuplexStream,
        tokio::task::JoinHandle<Result<(), RuntimeError>>,
    ) {
        let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
        let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
        let task = tokio::spawn(run_with_streams(EchoPlugin, plugin_rx, plugin_tx));
        (host_tx, host_rx, task)
    }

    /// Reads one `LF`-delimited frame from the host's read half.
    async fn read_response(
        rx: &mut tokio::io::DuplexStream,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut frame = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            let n = rx.read(&mut byte).await?;
            if n == 0 {
                return Err("stream closed before a complete frame".into());
            }
            frame.push(byte[0]);
            if byte[0] == b'\n' {
                return Ok(frame);
            }
        }
    }

    #[tokio::test]
    async fn run_roundtrips_describe_and_execute_in_memory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut host_tx, mut host_rx, task) = spawn_in_memory().await;
        host_tx
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"describe\",\"params\":{}}\n")
            .await?;
        host_tx
            .write_all(
                b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"execute\",\"params\":{\"config\":{\"k\":\"v\"}}}\n",
            )
            .await?;
        let frames = [
            read_response(&mut host_rx).await?,
            read_response(&mut host_rx).await?,
        ];
        let responses: Vec<serde_json::Value> = frames
            .iter()
            .map(|f| serde_json::from_slice::<serde_json::Value>(f))
            .collect::<Result<_, _>>()?;

        let describe = responses
            .iter()
            .find(|r| r["id"] == 1)
            .ok_or_else(|| "missing describe response".to_string())?;
        let nodes: Vec<NodeDescription> = serde_json::from_value(describe["result"].clone())?;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].type_id, "test:echo");

        let execute = responses
            .iter()
            .find(|r| r["id"] == 2)
            .ok_or_else(|| "missing execute response".to_string())?;
        assert_eq!(execute["result"]["success"], true);
        assert_eq!(execute["result"]["output_data"]["k"], "v");
        assert!(execute["result"]["variable_updates"].is_object());

        host_tx.shutdown().await?;
        task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn run_unknown_method_returns_error_in_memory() -> Result<(), Box<dyn std::error::Error>>
    {
        let (mut host_tx, mut host_rx, task) = spawn_in_memory().await;
        host_tx
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"nope\",\"params\":{}}\n")
            .await?;
        let frame = read_response(&mut host_rx).await?;
        let response: serde_json::Value = serde_json::from_slice(&frame)?;
        assert_eq!(response["id"], 5);
        assert_eq!(response["error"]["code"], -32601);
        host_tx.shutdown().await?;
        task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn run_exits_cleanly_on_eof() -> Result<(), Box<dyn std::error::Error>> {
        let (mut host_tx, _host_rx, task) = spawn_in_memory().await;
        host_tx.shutdown().await?;
        task.await??;
        Ok(())
    }
}
