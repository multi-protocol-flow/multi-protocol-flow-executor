//! JSON-RPC wire types shared with plugins.
//!
//! Re-exports the `mpe-protocol` types consumed by the transport so the rest
//! of the host crate never imports `mpe-protocol` directly.

pub use mpe_protocol::error::JsonRpcError;
pub use mpe_protocol::notification::{Notification, NotificationCorrelation};
pub use mpe_protocol::request::Request;
pub use mpe_protocol::response::Response;
