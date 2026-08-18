//! Host-side `uiCall` relay (config-panel discovery queries).
//!
//! Config panels rendered inside a plugin iframe need runtime discovery data
//! (MCP tools/resources/prompts, tool input schemas, …) that static
//! `config_schema` cannot express. The panel cannot talk to the plugin
//! process directly, so the host relays a `uiCall` request: resolve the node
//! type to its plugin connection ([`PluginManager::connection_for_node`],
//! auto-starting the process on demand), issue the JSON-RPC request, and
//! return the plugin's result.
//!
//! [`perform_ui_call`] is a free function over a live [`PluginConnection`]
//! (mirroring [`crate::describe::perform_describe`]) so tests drive it with
//! `PluginConnection::from_io` in-memory fakes; [`PluginManager::ui_call`]
//! adds the node-type → plugin resolution.

use mpe_plugin_sdk::protocol::ui::{UiCallParams, UI_CALL_METHOD};
use serde_json::Value;

use crate::error::PluginError;
use crate::manager::PluginManager;
use crate::transport::PluginConnection;

/// Issues one `uiCall` request over `conn` and returns the plugin's result.
///
/// The request payload is `UiCallParams { method, params }`. Timeout uses
/// the connection's own per-request timeout (30s default, identical to
/// execute RPCs) — no separate mechanism is introduced. JSON-RPC error
/// responses surface as [`PluginError::JsonRpc`] with the code and message
/// verbatim; transport failures surface as their usual variants.
pub async fn perform_ui_call(
    conn: &PluginConnection,
    method: &str,
    params: Value,
) -> Result<Value, PluginError> {
    let wire_params = serde_json::to_value(UiCallParams {
        method: method.to_string(),
        params,
    })
    .map_err(|e| PluginError::ProtocolError(format!("uiCall params serialization failed: {e}")))?;
    conn.request(UI_CALL_METHOD, wire_params).await
}

impl PluginManager {
    /// Relays a `uiCall` discovery query to the plugin owning `node_type`.
    ///
    /// Resolves the plugin via [`PluginManager::connection_for_node`] — an
    /// unknown node type is a [`PluginError::ProtocolError`], which also
    /// prevents one plugin's panel from reaching another plugin's process.
    pub async fn ui_call(
        &self,
        node_type: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, PluginError> {
        let conn = self.connection_for_node(node_type).await?;
        perform_ui_call(&conn, method, params).await
    }
}

