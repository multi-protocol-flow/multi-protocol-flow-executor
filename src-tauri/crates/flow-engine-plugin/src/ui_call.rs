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

use mpe_protocol::ui::{UiCallParams, UI_CALL_METHOD};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::framing::{FrameReader, FrameWriter};
    use crate::lifecycle::LifecycleConfig;
    use crate::manager::PluginRegistration;
    use mpe_protocol::describe::{NodeDescription, PortDescription};
    use mpe_protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};
    use mpe_protocol::request::Request;
    use mpe_protocol::response::Response;

    /// A minimal valid manifest for the fake "demo" plugin.
    fn test_manifest() -> PluginManifest {
        PluginManifest {
            name: "demo".to_string(),
            version: "0.1.0".to_string(),
            author: None,
            min_host_version: None,
            description: None,
            entry: PluginEntry {
                command: "unused".to_string(),
                args: Vec::new(),
            },
            env: HashMap::new(),
            permissions: Vec::new(),
            capabilities: PluginCapabilities::default(),
            locales: Vec::new(),
            default_locale: None,
        }
    }

    /// The node description registered for the fake "demo" plugin.
    fn test_node_description() -> NodeDescription {
        NodeDescription {
            type_id: "demo:ping".to_string(),
            display_name: "Ping".to_string(),
            category: Some("demo".to_string()),
            icon: Some("wave".to_string()),
            color: Some("#ff0000".to_string()),
            ports: vec![
                PortDescription {
                    id: "in".to_string(),
                    label: "In".to_string(),
                    kind: "in".to_string(),
                },
                PortDescription {
                    id: "out".to_string(),
                    label: "Out".to_string(),
                    kind: "out".to_string(),
                },
            ],
            default_config: serde_json::json!({"target": "localhost"}),
            config_schema: None,
            frontend: None,
            capabilities: PluginCapabilities::default(),
        }
    }

    /// Plugin side of an in-memory stdio pair (mirrors describe_tests).
    struct FakePlugin {
        reader: FrameReader<tokio::io::DuplexStream>,
        tx: tokio::io::DuplexStream,
    }

    impl FakePlugin {
        async fn read_request(&mut self) -> Result<Request, Box<dyn std::error::Error + Send + Sync>>
        {
            let frame = self
                .reader
                .next_frame()
                .await?
                .ok_or_else(|| "unexpected eof while reading request".to_string())?;
            Ok(serde_json::from_slice(&frame)?)
        }

        async fn respond(
            &mut self,
            id: u64,
            result: Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let response = Response::Success { id, result };
            let bytes = serde_json::to_vec(&response)?;
            let mut writer = FrameWriter::new(&mut self.tx);
            writer.write_frame(&bytes).await?;
            Ok(())
        }

        async fn respond_error(
            &mut self,
            id: u64,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let response = Response::Error {
                id,
                error: mpe_protocol::error::JsonRpcError {
                    code: -32601,
                    message: "unknown uiCall sub-method".to_string(),
                    data: None,
                },
            };
            let bytes = serde_json::to_vec(&response)?;
            let mut writer = FrameWriter::new(&mut self.tx);
            writer.write_frame(&bytes).await?;
            Ok(())
        }
    }

    /// Wires an in-memory duplex pair into a host connection + plugin side.
    fn host_and_plugin(cap: usize) -> (PluginConnection, FakePlugin) {
        let (host_tx, plugin_rx) = tokio::io::duplex(cap);
        let (plugin_tx, host_rx) = tokio::io::duplex(cap);
        let conn = PluginConnection::from_io(
            host_tx,
            host_rx,
            crate::transport::ConnectionConfig::default(),
        );
        (
            conn,
            FakePlugin {
                reader: FrameReader::new(plugin_rx),
                tx: plugin_tx,
            },
        )
    }

    /// A successful `uiCall` round-trips the method + params and returns the
    /// plugin's result verbatim.
    #[tokio::test]
    async fn ui_call_round_trips_method_and_result(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (conn, mut fake) = host_and_plugin(64 * 1024);
        let request_task = tokio::spawn(async move {
            perform_ui_call(
                &conn,
                "mcp.discover",
                serde_json::json!({ "transport": "stdio", "command": "npx" }),
            )
            .await
        });
        let request = fake.read_request().await?;
        assert_eq!(request.method, mpe_protocol::ui::UI_CALL_METHOD);
        let params: UiCallParams = serde_json::from_value(request.params)?;
        assert_eq!(params.method, "mcp.discover");
        assert_eq!(params.params["command"], "npx");
        fake.respond(request.id, serde_json::json!({ "tools": [{ "name": "echo" }] }))
            .await?;
        let result = request_task.await??;
        assert_eq!(result["tools"][0]["name"], "echo");
        Ok(())
    }

    /// A plugin JSON-RPC error response surfaces as `PluginError::JsonRpc`
    /// with the code and message verbatim.
    #[tokio::test]
    async fn ui_call_error_response_surfaces_as_jsonrpc(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (conn, mut fake) = host_and_plugin(64 * 1024);
        let request_task = tokio::spawn(async move {
            perform_ui_call(&conn, "mcp.discover", serde_json::json!({})).await
        });
        let request = fake.read_request().await?;
        fake.respond_error(request.id).await?;
        let result: Result<Value, PluginError> = request_task.await?;
        let err = result.unwrap_err();
        match err {
            PluginError::JsonRpc { code, message, .. } => {
                assert_eq!(code, -32601);
                assert_eq!(message, "unknown uiCall sub-method");
            }
            other => panic!("expected PluginError::JsonRpc, got {other:?}"),
        }
        Ok(())
    }

    /// Unknown node types fail at resolution (`connection_for_node`), which
    /// also prevents cross-plugin panel access: the request never reaches
    /// any plugin process.
    #[tokio::test]
    async fn ui_call_unknown_node_type_is_protocol_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_side_tx, _side_rx) = tokio::sync::mpsc::channel::<FakePlugin>(8);
        let connector: crate::lifecycle::Connector = Arc::new(|_: &PluginManifest| {
            Err(PluginError::ProtocolError(
                "connector must not be invoked for unknown node types".to_string(),
            ))
        });
        let mut manager = PluginManager::new(PathBuf::from("/nonexistent")).with_lifecycle_config(
            LifecycleConfig {
                idle_recycle: Duration::from_secs(60),
                restart_min_interval: Duration::from_secs(1),
                shutdown_grace: Duration::from_millis(50),
                host_locale: None,
                connector,
            },
        );
        let err = manager
            .ui_call("demo:missing", "mcp.discover", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, PluginError::ProtocolError(_)),
            "unknown node type must be a protocol error, got {err:?}"
        );
        Ok(())
    }

    /// The manager routes a `ui_call` through the node type's plugin
    /// connection (auto-started on demand) and returns the plugin's result.
    #[tokio::test]
    async fn ui_call_routes_through_plugin_connection(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (side_tx, mut side_rx) = tokio::sync::mpsc::channel::<FakePlugin>(8);
        let connector: crate::lifecycle::Connector = Arc::new(move |_: &PluginManifest| {
            let (conn, fake) = host_and_plugin(64 * 1024);
            let _ = side_tx.try_send(fake);
            Ok(conn)
        });
        let mut manager = PluginManager::new(PathBuf::from("/nonexistent")).with_lifecycle_config(
            LifecycleConfig {
                idle_recycle: Duration::from_secs(60),
                restart_min_interval: Duration::from_secs(1),
                shutdown_grace: Duration::from_millis(50),
                host_locale: None,
                connector,
            },
        );
        manager.register_plugin("demo".to_string(), test_manifest());
        manager.add_registration(PluginRegistration {
            name: "demo".to_string(),
            version: "0.1.0".to_string(),
            dir: PathBuf::from("/nonexistent"),
            manifest: test_manifest(),
            nodes: vec![test_node_description()],
        });

        let request_task = tokio::spawn(async move {
            manager
                .ui_call("demo:ping", "mcp.discover", serde_json::json!({ "url": "u" }))
                .await
        });
        let mut fake = side_rx
            .recv()
            .await
            .ok_or_else(|| "fake plugin never connected".to_string())?;
        let request = fake.read_request().await?;
        assert_eq!(request.method, mpe_protocol::ui::UI_CALL_METHOD);
        let params: UiCallParams = serde_json::from_value(request.params)?;
        assert_eq!(params.method, "mcp.discover");
        assert_eq!(params.params["url"], "u");
        fake.respond(request.id, serde_json::json!({ "tools": ["echo"] }))
            .await?;
        let result = request_task.await??;
        assert_eq!(result["tools"][0], "echo");
        Ok(())
    }
}
