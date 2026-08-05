//! Unit tests for the describe handshake (host side).

use super::*;
use crate::framing::{FrameReader, FrameWriter};
use mpe_protocol::request::Request;
use mpe_protocol::response::Response;

/// Plugin side of an in-memory stdio pair (mirrors transport tests).
struct FakePlugin {
    reader: FrameReader<tokio::io::DuplexStream>,
    tx: tokio::io::DuplexStream,
}

impl FakePlugin {
    async fn read_request(&mut self) -> Result<Request, Box<dyn std::error::Error + Send + Sync>> {
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
        result: serde_json::Value,
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
            error: mpe_protocol::error::JsonRpcError::internal_error(),
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

/// The canned describe result the fake plugin answers with: one node with
/// an inline frontend, one with an iframe frontend (P0-unsupported).
fn canned_result() -> serde_json::Value {
    serde_json::json!({
        "plugin": {"name": "demo", "version": "1.0.0", "entry": {"command": "demo"}, "capabilities": {"streaming": true}},
        "nodes": [
            {
                "type_id": "demo:ping",
                "display_name": "Ping",
                "category": "demo",
                "icon": "wave",
                "color": "#ff0000",
                "ports": [
                    {"id": "in", "label": "In", "kind": "in"},
                    {"id": "out", "label": "Out", "kind": "out"}
                ],
                "default_config": {"target": "localhost"},
                "config_schema": {"type": "object"},
                "frontend": {"type": "inline", "content": "<p>ping</p>"},
                "capabilities": {"streaming": false}
            },
            {
                "type_id": "demo:legacy",
                "display_name": "Legacy",
                "frontend": {"type": "iframe", "url": "https://x"}
            }
        ]
    })
}

#[tokio::test]
async fn describe_handshake_parses_nodes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (conn, mut plugin) = host_and_plugin(64 * 1024);
    let plugin_task = tokio::spawn(async move {
        let request = plugin.read_request().await?;
        assert_eq!(request.method, "describe");
        assert_eq!(request.params, serde_json::json!({}));
        plugin.respond(request.id, canned_result()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    let result = perform_describe(&conn, None).await?;
    plugin_task.await??;

    // Echo manifest parsed; its capabilities override the on-disk one.
    assert_eq!(result.plugin.name, "demo");
    assert_eq!(result.plugin.version, "1.0.0");
    assert!(result.plugin.capabilities.streaming);

    assert_eq!(result.nodes.len(), 2);
    let ping = &result.nodes[0];
    assert_eq!(ping.type_id, "demo:ping");
    assert_eq!(ping.display_name, "Ping");
    assert_eq!(ping.category.as_deref(), Some("demo"));
    assert_eq!(ping.icon.as_deref(), Some("wave"));
    assert_eq!(ping.color.as_deref(), Some("#ff0000"));
    assert_eq!(ping.ports.len(), 2);
    assert_eq!(ping.ports[0].kind, "in");
    assert_eq!(ping.ports[1].kind, "out");
    assert_eq!(
        ping.default_config,
        serde_json::json!({"target": "localhost"})
    );
    assert_eq!(
        ping.config_schema.as_ref(),
        Some(&serde_json::json!({"type": "object"}))
    );
    let frontend = ping
        .frontend
        .as_ref()
        .ok_or_else(|| "inline frontend missing".to_string())?;
    assert_eq!(frontend.kind, "inline");
    assert_eq!(frontend.content.as_deref(), Some("<p>ping</p>"));
    assert!(!ping.capabilities.streaming);

    // The iframe frontend survives wire parsing but P0 normalization
    // drops it (with a warning) — never a crash.
    let legacy = &result.nodes[1];
    assert_eq!(
        legacy.frontend.as_ref().map(|f| f.kind.as_str()),
        Some("iframe")
    );
    let normalized = normalize_nodes(result.nodes, "demo");
    assert_eq!(
        normalized[0].frontend.as_ref().map(|f| f.kind.as_str()),
        Some("inline")
    );
    assert_eq!(normalized[1].frontend, None);
    Ok(())
}

#[tokio::test]
async fn describe_handshake_error_response_fails(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (conn, mut plugin) = host_and_plugin(64 * 1024);
    let plugin_task = tokio::spawn(async move {
        let request = plugin.read_request().await?;
        plugin.respond_error(request.id).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    let result = perform_describe(&conn, None).await;
    plugin_task.await??;
    assert!(
        matches!(result, Err(PluginError::JsonRpc { code: -32603, .. })),
        "expected internal-error JsonRpc failure, got {result:?}"
    );
    Ok(())
}

#[tokio::test]
async fn describe_handshake_invalid_result_fails(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (conn, mut plugin) = host_and_plugin(64 * 1024);
    let plugin_task = tokio::spawn(async move {
        let request = plugin.read_request().await?;
        // A non-object result cannot be a DescribeResult.
        plugin
            .respond(request.id, serde_json::json!([1, 2, 3]))
            .await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    let result = perform_describe(&conn, None).await;
    plugin_task.await??;
    assert!(
        matches!(result, Err(PluginError::ProtocolError(_))),
        "expected protocol error, got {result:?}"
    );
    Ok(())
}

#[tokio::test]
async fn describe_handshake_accepts_sdk_array_shape(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (conn, mut plugin) = host_and_plugin(64 * 1024);
    let plugin_task = tokio::spawn(async move {
        let request = plugin.read_request().await?;
        // Pre-M5b `mpe-plugin-sdk` shape: a bare array of node
        // descriptions with no manifest echo.
        plugin
            .respond(
                request.id,
                serde_json::json!([
                    {"type_id": "demo:echo", "display_name": "Demo Echo",
                     "ports": [{"id": "in", "label": "In", "kind": "in"},
                               {"id": "out", "label": "Out", "kind": "out"}]}
                ]),
            )
            .await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    let fallback = PluginManifest {
        name: "demo".to_string(),
        version: "1.2.3".to_string(),
        author: None,
        min_host_version: None,
        description: None,
        entry: mpe_protocol::manifest::PluginEntry {
            command: "demo".to_string(),
            args: Vec::new(),
        },
        env: std::collections::HashMap::new(),
        permissions: Vec::new(),
        capabilities: mpe_protocol::manifest::PluginCapabilities::default(),
    };
    let result = perform_describe(&conn, Some(fallback)).await?;
    plugin_task.await??;

    // The on-disk manifest substitutes for the missing echo…
    assert_eq!(result.plugin.name, "demo");
    assert_eq!(result.plugin.version, "1.2.3");
    // …and the node array parses with defaults for absent fields.
    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].type_id, "demo:echo");
    assert_eq!(result.nodes[0].ports.len(), 2);
    Ok(())
}

#[test]
fn normalize_keeps_inline_drops_other_frontend_kinds() {
    let json = r#"[
            {"type_id": "demo:a", "display_name": "A", "frontend": {"type": "inline", "content": "<p>a</p>"}},
            {"type_id": "demo:b", "display_name": "B", "frontend": {"type": "config-panel-url", "url": "https://x"}}
        ]"#;
    let nodes: Vec<NodeDescription> =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("test nodes should parse: {e}"));
    let normalized = normalize_nodes(nodes, "demo");
    assert_eq!(
        normalized[0].frontend.as_ref().map(|f| f.kind.as_str()),
        Some("inline")
    );
    assert_eq!(normalized[1].frontend, None);
}
