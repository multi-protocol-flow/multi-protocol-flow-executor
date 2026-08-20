//! Host-side datasource registry + RPC relay.
//!
//! Datasource plugins expose read-only platform resources (projects, APIs,
//! …) browsable in the host's left sidebar. [`PluginManager`] registers
//! their [`DatasourceDescription`]s from the describe handshake (id
//! conflicts are first-wins + warn, mirroring the node collision policy)
//! and relays the frontend's `browse`/`get_item`/`get_templates`/
//! `validate_config` RPCs to the owning plugin process — the plugin stays
//! stateless because every request carries the connection `config`.
//!
//! [`perform_datasource_call`] is a free function over a live
//! [`PluginConnection`] (mirroring [`crate::ui_call::perform_ui_call`]) so
//! tests drive it with `PluginConnection::from_io` in-memory fakes;
//! [`PluginManager::datasource_call`] adds the datasource-id → plugin
//! resolution.

use std::sync::Arc;

use serde_json::Value;

// Re-export the shared wire types and method-name constants so the app layer
// can build params and parse results through the facade without a direct
// mpe-plugin-sdk dependency.
pub use mpe_plugin_sdk::protocol::datasource::{
    BrowseParams, DataItem, DataItemProtocol, DatasourceAuth, DatasourceDescription,
    DatasourceResource, GetItemParams, GetTemplatesParams, KeyValue, NodeTemplate, SourceRef,
    ValidateConfigParams, ValidateConfigResult, BROWSE_METHOD, GET_ITEM_METHOD,
    GET_TEMPLATES_METHOD, VALIDATE_CONFIG_METHOD,
};

use crate::error::PluginError;
use crate::manager::PluginManager;
use crate::transport::PluginConnection;

/// Issues one datasource RPC over `conn` and returns the plugin's result.
///
/// The request payload is the caller-built params object (e.g.
/// [`mpe_plugin_sdk::protocol::datasource::BrowseParams`]); the host relays
/// it verbatim. Timeout uses the connection's own per-request timeout (30s
/// default, identical to execute RPCs); JSON-RPC error responses surface as
/// [`PluginError::JsonRpc`] with the code and message verbatim.
pub async fn perform_datasource_call(
    conn: &PluginConnection,
    method: &str,
    params: Value,
) -> Result<Value, PluginError> {
    conn.request(method, params).await
}

impl PluginManager {
    /// Registers datasource descriptions from a plugin's describe result.
    ///
    /// Id conflicts are first-wins + warn (aligned with the node collision
    /// policy): the first plugin to claim an id keeps it, later claims are
    /// skipped.
    pub fn register_datasources(
        &mut self,
        plugin_name: &str,
        descriptions: Vec<DatasourceDescription>,
    ) {
        for description in descriptions {
            if let Some(existing) = self.datasource_index.get(&description.id) {
                log::warn!(
                    "datasource {} from plugin {} conflicts with plugin {}; first wins",
                    description.id,
                    plugin_name,
                    existing
                );
                continue;
            }
            self.datasource_index
                .insert(description.id.clone(), plugin_name.to_string());
            self.datasource_descriptions
                .insert(description.id.clone(), description);
        }
    }

    /// Lists every registered datasource description, id-sorted for stable
    /// UI ordering.
    pub fn list_datasources(&self) -> Vec<&DatasourceDescription> {
        let mut descriptions: Vec<&DatasourceDescription> =
            self.datasource_descriptions.values().collect();
        descriptions.sort_by(|a, b| a.id.cmp(&b.id));
        descriptions
    }

    /// Resolves a datasource id to the live connection of its owning plugin
    /// (auto-starting the process on demand via the lifecycle entry).
    pub async fn connection_for_datasource(
        &self,
        datasource_id: &str,
    ) -> Result<Arc<PluginConnection>, PluginError> {
        let name = self.datasource_index.get(datasource_id).ok_or_else(|| {
            PluginError::ProtocolError(format!("unknown datasource {datasource_id}"))
        })?;
        self.connection_for(name).await
    }

    /// Relays a datasource RPC to the plugin owning `datasource_id`.
    ///
    /// An unknown datasource id is a [`PluginError::ProtocolError`] (same as
    /// [`PluginManager::ui_call`]'s unknown node type), which also prevents
    /// one plugin's datasource from reaching another plugin's process.
    pub async fn datasource_call(
        &self,
        datasource_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, PluginError> {
        let conn = self.connection_for_datasource(datasource_id).await?;
        perform_datasource_call(&conn, method, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpe_plugin_sdk::protocol::datasource::{DatasourceAuth, DatasourceResource};

    fn sample_description(id: &str) -> DatasourceDescription {
        DatasourceDescription {
            id: id.to_string(),
            display_name: id.to_string(),
            auth: DatasourceAuth {
                r#type: "api_key".to_string(),
            },
            config_schema: None,
            resources: vec![DatasourceResource {
                id: "project".to_string(),
                label: "项目".to_string(),
                children: None,
                leaf: true,
            }],
        }
    }

    /// Duplicate datasource ids across plugins: first wins + warn; the
    /// second claim is skipped (never shadows).
    #[test]
    fn register_datasources_first_wins_on_conflict() {
        let mut manager = PluginManager::new("/nonexistent".into());
        manager.register_datasources("plugin-a", vec![sample_description("eolink")]);
        manager.register_datasources("plugin-b", vec![sample_description("eolink")]);

        let datasources = manager.list_datasources();
        assert_eq!(datasources.len(), 1);
        assert_eq!(datasources[0].id, "eolink");
        // The index keeps the first claimant.
        assert_eq!(
            manager.datasource_index.get("eolink").map(String::as_str),
            Some("plugin-a")
        );
    }

    /// An unknown datasource id is rejected with a protocol error naming the
    /// id (before any connection is touched).
    #[tokio::test]
    async fn datasource_call_unknown_id_is_protocol_error() {
        let manager = PluginManager::new("/nonexistent".into());
        let err = match manager
            .datasource_call("nope", "browse", serde_json::json!({}))
            .await
        {
            Ok(_) => panic!("unknown datasource must fail"),
            Err(e) => e,
        };
        assert_eq!(
            err.to_string(),
            "protocol error: unknown datasource nope"
        );
    }

    /// `perform_datasource_call` round-trips a params object over an
    /// in-memory fake connection.
    #[tokio::test]
    async fn perform_datasource_call_round_trips_in_memory() -> Result<(), Box<dyn std::error::Error>>
    {
        let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
        let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
        let conn = PluginConnection::from_io(
            host_tx,
            host_rx,
            crate::transport::ConnectionConfig::default(),
        );
        let params = serde_json::json!({
            "config": {"api_key": "k"},
            "resource_id": "project"
        });

        // Fake plugin peer: read one framed request, echo its params back as
        // the result (the host's request carries the id to correlate).
        tokio::spawn(async move {
            let mut reader = crate::framing::FrameReader::new(plugin_rx);
            if let Ok(Some(line)) = reader.next_frame().await {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) {
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": value["id"],
                        "result": value["params"]
                    });
                    let mut writer = plugin_tx;
                    let _ = tokio::io::AsyncWriteExt::write_all(
                        &mut writer,
                        response.to_string().as_bytes(),
                    )
                    .await;
                    let _ = tokio::io::AsyncWriteExt::write_all(&mut writer, b"\n").await;
                    let _ = tokio::io::AsyncWriteExt::flush(&mut writer).await;
                }
            }
        });

        let result = perform_datasource_call(&conn, "browse", params.clone()).await?;
        assert_eq!(result, params);
        Ok(())
    }
}
