//! Pipelined JSON-RPC transport over a child process's stdin/stdout.
//!
//! Architecture:
//! - **Single-writer task** (`writer`) owns the child's stdin. Callers push
//!   serialized `(id, bytes)` frames into a bounded mpsc channel; the writer
//!   serializes full-frame `write_all` + `flush`, so whole frames are written
//!   atomically and a frame is never truncated by cancellation.
//! - **Reader task** (`reader`) owns the child's stdout. It parses the
//!   `mpe_protocol` discriminated union; notifications are demuxed to
//!   registered handlers before any pending-request lookup, malformed lines
//!   are skipped (kill after N consecutive), and error responses map to the
//!   pending oneshot as a typed [`PluginError`](crate::error::PluginError).
//! - **Pending map** tracks in-flight requests; each request carries a
//!   per-request timeout (`select!` on the oneshot + `sleep`) which is the
//!   only mechanism covering a stuck-but-alive plugin.
//! - **Backpressure** bounds in-flight requests via a semaphore and the writer
//!   queue via the bounded channel; overflow blocks the caller (never rejects).

mod backpressure;
mod connection;
mod reader;
mod writer;

pub use backpressure::Backpressure;
pub use connection::{
    ConnectionConfig, CorrelationFilter, ExitReason, PluginConnection, SubscriptionHandle,
    TransportMetrics,
};
