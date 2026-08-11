//! Validator + plugin registry wiring tests (M4).
//!
//! Verifies that validation sees the shared registry (plugin node types are
//! accepted, unknown types rejected with `E_VAL_UNKNOWN_NODE_TYPE`) and that
//! the async validator runs the per-plugin `validate` RPC. Uses the same
//! in-memory duplex fake connector as `registry_threading.rs` — no real plugin
//! processes.

#![cfg(feature = "plugin-manager")]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorCode;
use flow_engine_core::flow::Flow;
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::NodeData;
use flow_engine_executor::default_registry::{default_registry, default_registry_with_plugins};
use flow_engine_executor::validator::FlowValidator;
use flow_engine_plugin::framing::{FrameReader, FrameWriter};
use flow_engine_plugin::lifecycle::{Connector, LifecycleConfig};
use flow_engine_plugin::manager::{PluginManager, PluginRegistration};
use flow_engine_plugin::transport::{ConnectionConfig, PluginConnection};
use mpe_protocol::describe::{NodeDescription, PortDescription};
use mpe_protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};
use mpe_protocol::request::Request;

/// Minimal legal manifest (the connector is an in-process fake, so
/// `entry.command` is never executed).
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
        env: std::collections::HashMap::new(),
        permissions: Vec::new(),
        capabilities: PluginCapabilities::default(),
        locales: Vec::new(),
        default_locale: None,
    }
}

/// Node description registered for the fake "demo" plugin.
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

/// Builds a manager with an in-memory duplex connector and one registered
/// plugin node (`demo:ping`).
fn test_manager(connector: Connector) -> Arc<PluginManager> {
    let mut manager =
        PluginManager::new(PathBuf::from("/nonexistent")).with_lifecycle_config(LifecycleConfig {
            idle_recycle: Duration::from_secs(60),
            restart_min_interval: Duration::from_secs(1),
            shutdown_grace: Duration::from_millis(50),
            host_locale: None,
            connector,
        });
    manager.register_plugin("demo".to_string(), test_manifest());
    manager.add_registration(PluginRegistration {
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        dir: PathBuf::from("/nonexistent"),
        manifest: test_manifest(),
        nodes: vec![test_node_description()],
    });
    Arc::new(manager)
}

/// In-memory connector: every method echoes its params back as the result,
/// except `validate`, which returns the given JSON value.
fn connector_with_validate(validate_result: serde_json::Value) -> Connector {
    Arc::new(move |_: &PluginManifest| {
        let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
        let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
        let validate_result = validate_result.clone();
        tokio::spawn(async move {
            let mut reader = FrameReader::new(plugin_rx);
            let mut writer = FrameWriter::new(plugin_tx);
            while let Ok(Some(frame)) = reader.next_frame().await {
                let request = match serde_json::from_slice::<Request>(&frame) {
                    Ok(request) => request,
                    Err(_) => continue,
                };
                let result = if request.method == "validate" {
                    validate_result.clone()
                } else {
                    request.params
                };
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "result": result,
                });
                let bytes = match serde_json::to_vec(&response) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if writer.write_frame(&bytes).await.is_err() {
                    break;
                }
            }
        });
        Ok(PluginConnection::from_io(
            host_tx,
            host_rx,
            ConnectionConfig::default(),
        ))
    })
}

/// A flow with one `demo:ping` plugin node connected to an entry node.
fn plugin_flow() -> Flow {
    let entry_node = Node {
        uuid: "entry-1".to_string(),
        node_type: "entry".to_string(),
        name: "Entry".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };
    let plugin_node = Node {
        uuid: "plugin-1".to_string(),
        node_type: "demo:ping".to_string(),
        name: "Ping".to_string(),
        position: NodePosition { x: 200.0, y: 0.0 },
        data: NodeData::Plugin(serde_json::json!({"type": "demo:ping"})),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };
    Flow {
        uuid: "flow-1".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Plugin Flow".to_string(),
        description: None,
        nodes: vec![entry_node, plugin_node],
        connections: vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "entry-1".to_string(),
            target_node_uuid: "plugin-1".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    }
}

/// A registry containing a plugin node type accepts a flow using it — no
/// unknown-type error. (Runs under a tokio runtime: the fake manager's
/// lifecycle reaper is spawned at construction.)
#[tokio::test]
async fn validate_with_registry_accepts_plugin_type() {
    let manager = test_manager(connector_with_validate(serde_json::json!({"valid": true})));
    let registry = default_registry_with_plugins(&manager);
    let flow = plugin_flow();

    let result = FlowValidator::new().validate_with_registry(&flow, &registry);

    assert!(
        result.valid,
        "flow with a registered plugin node should validate: {:?}",
        result.errors
    );
    assert!(
        result
            .errors
            .iter()
            .all(|e| e.code != ErrorCode::E_VAL_UNKNOWN_NODE_TYPE),
        "registered plugin type must not be reported as unknown"
    );
}

/// A flow with a type absent from the registry is rejected with
/// `E_VAL_UNKNOWN_NODE_TYPE`.
#[test]
fn validate_with_registry_rejects_unknown() {
    let registry = default_registry();
    let mut flow = plugin_flow();
    flow.nodes[1].node_type = "no:such:type".to_string();

    let result = FlowValidator::new().validate_with_registry(&flow, &registry);

    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.code == ErrorCode::E_VAL_UNKNOWN_NODE_TYPE),
        "unknown node type must be reported: {:?}",
        result.errors
    );
}

/// The async validator runs the plugin's `validate` RPC; a `valid: false`
/// answer surfaces as a validation error carrying the plugin's message.
#[tokio::test]
async fn validate_async_plugin_rpc_called() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let manager = test_manager(connector_with_validate(serde_json::json!({
        "valid": false,
        "message": "bad config",
    })));
    let registry = default_registry_with_plugins(&manager);
    let flow = plugin_flow();

    let result = FlowValidator::new()
        .validate_with_registry_async(&flow, &registry, Some(manager.as_ref()))
        .await;

    assert!(
        !result.valid,
        "plugin validate rejection must fail the flow"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.code == ErrorCode::E_CONFIG_ERROR && e.message.contains("bad config")),
        "plugin validate message must surface: {:?}",
        result.errors
    );
    Ok(())
}
