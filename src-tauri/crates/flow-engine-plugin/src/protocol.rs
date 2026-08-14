//! JSON-RPC wire types shared with plugins.
//!
//! Re-exports the `mpe-plugin-sdk::protocol` types consumed by the transport so the rest
//! of the host crate never imports `mpe-plugin-sdk::protocol` directly.

pub use mpe_plugin_sdk::protocol::error::JsonRpcError;
pub use mpe_plugin_sdk::protocol::notification::{Notification, NotificationCorrelation};
pub use mpe_plugin_sdk::protocol::request::Request;
pub use mpe_plugin_sdk::protocol::response::Response;
