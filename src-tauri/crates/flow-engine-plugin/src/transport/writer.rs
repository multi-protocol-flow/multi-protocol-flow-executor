//! Single-writer task owning the child's stdin.
//!
//! The writer task is the **only** task that writes to the child's stdin. It
//! consumes pre-serialized frames from a bounded mpsc channel and writes each
//! one atomically (payload + delimiter + flush). Because no other task can
//! interleave, a frame is never torn — this is the property that makes a
//! write-mutex insufficient (cancellation mid-write can still drop half a
//! frame; the single-writer task cannot be cancelled mid-frame).

use std::sync::Arc;

use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

use crate::error::PluginError;
use crate::framing::FrameWriter;
use crate::transport::connection::{ConnectionShared, ExitReason};

/// A pre-serialized outbound frame destined for the plugin's stdin.
///
/// `id` is `0` for notifications (which carry no id); otherwise the request
/// id the frame answers or originates.
#[derive(Debug)]
pub(crate) struct OutboundFrame {
    /// Request id (0 for fire-and-forget notifications).
    pub id: u64,
    /// Fully serialized JSON frame (without the trailing LF).
    pub bytes: Vec<u8>,
}

/// Runs the writer loop until the channel closes, then half-closes stdin.
///
/// On a write failure the plugin process is effectively gone: the exit state
/// is marked (draining all pending requests with `PluginCrashed`) and the
/// remaining channel is discarded.
pub(crate) async fn writer_task(
    mut stdin: Box<dyn AsyncWrite + Send + Unpin>,
    mut rx: mpsc::Receiver<OutboundFrame>,
    shared: Arc<ConnectionShared>,
) {
    let mut writer = FrameWriter::new(&mut stdin);
    while let Some(frame) = rx.recv().await {
        if let Err(e) = writer.write_frame(&frame.bytes).await {
            log::warn!(
                "plugin writer failed writing frame id={}: {}; process is likely gone",
                frame.id,
                e
            );
            shared.mark_exit(ExitReason::Crashed(PluginError::Io(e.to_string())));
            // Discard anything currently queued non-blockingly — nobody can write it.
            while rx.try_recv().is_ok() {}
            break;
        }
    }
    // Channel closed: half-close stdin so the plugin sees EOF and can exit.
    if let Err(e) = writer.shutdown().await {
        log::debug!("plugin stdin shutdown failed: {e}");
    }
}
