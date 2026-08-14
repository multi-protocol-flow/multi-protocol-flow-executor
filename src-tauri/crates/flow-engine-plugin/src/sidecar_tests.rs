//! `SidecarPlugin` tests: in-memory duplex fakes via
//! `PluginConnection::from_io` + a fake `PluginManager` (mirrors
//! `lifecycle_tests.rs`).
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorKind};
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::NodeData;
use mpe_plugin_sdk::protocol::describe::{NodeDescription, PortDescription};
use mpe_plugin_sdk::protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};
use mpe_plugin_sdk::protocol::request::Request;

use super::*;
use crate::error::PluginError;
use crate::framing::{FrameReader, FrameWriter};
use crate::lifecycle::LifecycleConfig;
use crate::manager::PluginRegistration;
use crate::transport::{ConnectionConfig, ExitReason, PluginConnection};

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

/// Builds a plugin `Node` whose config is `config` (must carry `type`).
fn plugin_node(config: serde_json::Value) -> Node {
    Node {
        uuid: "node-1".to_string(),
        node_type: config
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("demo:ping")
            .to_string(),
        name: "Ping Node".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Plugin(config),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    }
}

/// The plugin side of an in-memory stdio pair (mirrors transport tests).
struct PluginSide {
    reader: FrameReader<tokio::io::DuplexStream>,
    tx: tokio::io::DuplexStream,
}

impl PluginSide {
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
        let response = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
        let mut writer = FrameWriter::new(&mut self.tx);
        writer.write_frame(&serde_json::to_vec(&response)?).await?;
        Ok(())
    }
}

/// A live in-memory connection plus the plugin-side handle.
fn host_and_plugin_side() -> (PluginConnection, PluginSide) {
    let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
    let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
    let conn = PluginConnection::from_io(host_tx, host_rx, ConnectionConfig::default());
    (
        conn,
        PluginSide {
            reader: FrameReader::new(plugin_rx),
            tx: plugin_tx,
        },
    )
}

/// Builds a manager registered for the fake "demo" plugin plus a
/// `SidecarPlugin` for `demo:ping`. The connector hands every produced
/// connection's plugin-side handle back through `side_tx` (async — the
/// test awaits it without blocking the tokio runtime).
fn harness() -> (
    Arc<dyn NodePlugin>,
    Arc<PluginManager>,
    tokio::sync::mpsc::Receiver<PluginSide>,
) {
    let (side_tx, side_rx) = tokio::sync::mpsc::channel(8);
    let connector: crate::lifecycle::Connector = Arc::new(move |_: &PluginManifest| {
        let (conn, side) = host_and_plugin_side();
        let _ = side_tx.try_send(side);
        Ok(conn)
    });
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
    let manager = Arc::new(manager);
    (
        SidecarPlugin::new(test_node_description(), Arc::clone(&manager)),
        manager,
        side_rx,
    )
}

/// The canonical success execute result for fakes.
fn success_result(output_data: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "success": true,
        "next_ports": ["out"],
        "output_data": output_data,
        "errors": [],
        "report_data": null,
        "variable_updates": {},
        "provider_updates": {},
    })
}

/// Spawns `execute` for `plugin`/`node`/`context` and returns the task,
/// which hands back the result together with the mutated context.
fn spawn_execute(
    plugin: Arc<dyn NodePlugin>,
    node: Node,
    mut context: ExecutionContext,
) -> tokio::task::JoinHandle<(
    Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail>,
    ExecutionContext,
)> {
    tokio::spawn(async move {
        let connections: Vec<flow_engine_core::connection::Connection> = Vec::new();
        let node_map: HashMap<String, &Node> = HashMap::new();
        let result = plugin
            .execute(&node, &mut context, &connections, &node_map)
            .await;
        (result, context)
    })
}

#[tokio::test]
async fn sidecar_execute_resolves_variables() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({
        "type": "demo:ping",
        "target": "{{initial_variables.host}}",
        "timeout_ms": 5000,
    }));
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    context.current_node_uuid = Some("node-1".to_string());
    context.variables.set_variable(
        "initial_variables".to_string(),
        serde_json::json!({"host": "resolved.example"}),
        String::new(),
    );

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let request = side.read_request().await?;
    assert_eq!(request.method, "execute");
    // The plugin must receive the FINAL value, not the placeholder.
    assert_eq!(
        request.params["config"]["target"],
        serde_json::json!("resolved.example"),
        "host-side variable resolution must deliver resolved values on the wire"
    );
    assert_eq!(
        request.params["referenced_variables"],
        serde_json::json!(["initial_variables.host"])
    );
    side.respond(request.id, success_result(serde_json::json!({"ok": true})))
        .await?;

    let (result, _context) = handle.await?;
    let (next_node_ids, output_data) = result.map_err(|e| format!("execute failed: {e}"))?;
    assert!(next_node_ids.is_empty());
    assert_eq!(output_data, Some(serde_json::json!({"ok": true})));
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_returns_plugin_data(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    context.current_node_uuid = Some("node-1".to_string());

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let request = side.read_request().await?;
    let result = serde_json::json!({
        "success": true,
        "next_ports": ["out"],
        "output_data": null,
        "errors": [],
        "report_data": {"plugin_name": "demo", "metric": 42},
        "variable_updates": {},
        "provider_updates": {},
    });
    side.respond(request.id, result).await?;

    let (execute_result, context) = handle.await?;
    execute_result.map_err(|e| format!("execute failed: {e}"))?;
    assert_eq!(
        context.current_node_plugin_data,
        Some(serde_json::json!({"plugin_name": "demo", "metric": 42})),
        "report_data must land in the context back-channel"
    );
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_merges_variable_updates_flat_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    context.current_node_uuid = Some("node-1".to_string());

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let request = side.read_request().await?;
    let result = serde_json::json!({
        "success": true,
        "next_ports": ["out"],
        "output_data": null,
        "errors": [],
        "report_data": null,
        "variable_updates": {"token": "abc", "count": 3},
        "provider_updates": {},
    });
    side.respond(request.id, result).await?;

    let (execute_result, context) = handle.await?;
    execute_result.map_err(|e| format!("execute failed: {e}"))?;
    assert_eq!(
        context.variables.get_variable("token"),
        Some(&serde_json::json!("abc"))
    );
    assert_eq!(
        context.variables.get_variable("count"),
        Some(&serde_json::json!(3))
    );
    assert_eq!(
        context
            .variables
            .get_variable_full("token")
            .map(|v| v.source_node_uuid.as_str()),
        Some("node-1")
    );
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_flat_dotted_key_rejected(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    context.current_node_uuid = Some("node-1".to_string());

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let request = side.read_request().await?;
    let result = serde_json::json!({
        "success": true,
        "next_ports": ["out"],
        "output_data": null,
        "errors": [],
        "report_data": null,
        "variable_updates": {"runtime.total": 42, "clean": 1},
        "provider_updates": {},
    });
    side.respond(request.id, result).await?;

    let (execute_result, context) = handle.await?;
    execute_result.map_err(|e| format!("execute failed: {e}"))?;
    // The flat key is merged; the dotted key is skipped entirely.
    assert_eq!(
        context.variables.get_variable("clean"),
        Some(&serde_json::json!(1))
    );
    assert!(
        context.variables.get_variable("runtime.total").is_none(),
        "dotted key must never be stored"
    );
    assert!(
        context.variables.get_variable("runtime").is_none(),
        "dotted key must not be split into a partial object"
    );
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_provider_updates() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    context.current_node_uuid = Some("node-1".to_string());

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let request = side.read_request().await?;
    let result = serde_json::json!({
        "success": true,
        "next_ports": ["out"],
        "output_data": null,
        "errors": [],
        "report_data": null,
        "variable_updates": {},
        "provider_updates": {"demo_svc": {"base_url": "http://x"}},
    });
    side.respond(request.id, result).await?;

    let (execute_result, context) = handle.await?;
    execute_result.map_err(|e| format!("execute failed: {e}"))?;
    let provider: Option<Arc<serde_json::Value>> = context.get_provider("demo_svc");
    assert_eq!(
        provider.as_deref(),
        Some(&serde_json::json!({"base_url": "http://x"})),
        "provider_updates must be absorbed as JSON providers"
    );
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_timeout_kills() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (plugin, _manager, mut side_rx) = harness();
    // A tiny wrapper-owned timeout so the test completes fast.
    let node = plugin_node(serde_json::json!({"type": "demo:ping", "timeout_ms": 100}));
    let context = ExecutionContext::new("flow-1".to_string(), 0);

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    // Read the request but never respond: the wrapper timeout must fire.
    let _request = side.read_request().await?;

    let (result, _context) = handle.await?;
    let err = result
        .err()
        .ok_or_else(|| "expected execute Err, got Ok".to_string())?;
    assert_eq!(
        err.error_kind,
        ErrorKind::Timeout,
        "wrapper-owned timeout must surface as Timeout"
    );
    assert_eq!(err.error_code, Some(ErrorCode::E_TIMEOUT));
    Ok(())
}

#[tokio::test]
async fn sidecar_execute_cancel_receiver() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let context = ExecutionContext::new("flow-1".to_string(), 0).with_cancel_receiver(cancel_rx);

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    let _request = side.read_request().await?;
    cancel_tx
        .send(true)
        .map_err(|_| "cancel signal send failed")?;

    let (result, _context) = handle.await?;
    let err = result
        .err()
        .ok_or_else(|| "expected execute Err, got Ok".to_string())?;
    assert_eq!(
        err.error_kind,
        ErrorKind::Cancelled,
        "cancel_receiver must abort the in-flight execute"
    );
    assert_eq!(err.error_code, Some(ErrorCode::E_CANCELLED));
    Ok(())
}

#[tokio::test]
async fn sidecar_plugin_crashed_error_code() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (plugin, _manager, mut side_rx) = harness();
    let node = plugin_node(serde_json::json!({"type": "demo:ping"}));
    let context = ExecutionContext::new("flow-1".to_string(), 0);

    let handle = spawn_execute(plugin, node, context);
    let mut side = side_rx
        .recv()
        .await
        .ok_or_else(|| "fake plugin never connected".to_string())?;
    // Read the request, then drop the peer halves: the host reader sees
    // EOF with a request pending → PluginCrashed → single final Err.
    let _request = side.read_request().await?;
    drop(side);

    let (result, _context) = handle.await?;
    let err = result
        .err()
        .ok_or_else(|| "expected execute Err, got Ok".to_string())?;
    assert_eq!(err.error_kind, ErrorKind::Execution);
    assert_eq!(
        err.error_code,
        Some(ErrorCode::PluginCrashed),
        "a dead plugin connection must surface as PluginCrashed"
    );
    assert!(err.message.contains("crashed"));
    Ok(())
}

/// Registration metadata accessors mirror the describe metadata (a smoke
/// check that `SidecarPlugin` reads through to `self.node`).
#[tokio::test]
async fn sidecar_metadata_mirrors_description() {
    let (plugin, _manager, _side_rx) = harness();
    // UFCS: `type_id` is ambiguous with `std::any::Any::type_id` on the
    // trait object, so call the `NodePlugin` method explicitly.
    assert_eq!(NodePlugin::type_id(&*plugin), "demo:ping");
    assert_eq!(NodePlugin::display_name(&*plugin), "Ping");
    assert_eq!(NodePlugin::category(&*plugin), "demo");
    assert_eq!(NodePlugin::icon(&*plugin), "wave");
    assert_eq!(NodePlugin::color(&*plugin), "#ff0000");
    assert_eq!(
        plugin.default_config(),
        serde_json::json!({"target": "localhost"})
    );
    let ports = plugin.ports();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].id, "in");
    assert_eq!(ports[1].id, "out");
}

/// `SidecarPlugin::frontend()` mirrors the describe-produced frontend
/// description (converted to the host-side `FrontendMetadata`).
#[test]
fn sidecar_frontend_mirrors_description() {
    let mut node = test_node_description();
    node.frontend = Some(mpe_plugin_sdk::protocol::describe::FrontendDescription {
        kind: "iframe".to_string(),
        content: None,
        url: Some("http://demo.localhost/config-panel".to_string()),
    });
    let manager = Arc::new(PluginManager::new(PathBuf::from("/nonexistent")));
    let plugin = SidecarPlugin::new(node, manager);

    let frontend = plugin
        .frontend()
        .unwrap_or_else(|| panic!("frontend must be present"));
    assert_eq!(frontend.kind, "iframe");
    assert_eq!(frontend.content, None);
    assert_eq!(
        frontend.url.as_deref(),
        Some("http://demo.localhost/config-panel")
    );
}

/// `connection_for_node` maps `type_id` → plugin name → lifecycle entry;
/// unknown type ids are a protocol error.
#[tokio::test]
async fn sidecar_connection_for_node_resolves_plugin(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_plugin, manager, _side_rx) = harness();
    let conn = manager.connection_for_node("demo:ping").await?;
    assert!(matches!(conn.exit_reason(), ExitReason::Running));

    let err = manager.connection_for_node("demo:missing").await.err();
    assert!(
        matches!(err, Some(PluginError::ProtocolError(_))),
        "unknown node type must be a protocol error, got {err:?}"
    );
    Ok(())
}

/// `map_execute_response` surfaces the plugin's real error message on
/// `success: false` — for BOTH the documented `[{"message": ...}]` shape
/// (post-M5b SDK) and the legacy `["msg"]` string shape (plugins built
/// against pre-M5b SDK binaries). Empty `errors` falls back to the generic
/// message.
#[test]
fn map_execute_response_failure_surfaces_plugin_message() {
    let node = plugin_node(serde_json::json!({ "type": "demo:ping" }));

    let check = |response: serde_json::Value, expected: &str| {
        let mut context = ExecutionContext::new("flow-1".to_string(), 0);
        let err = crate::wrapper::map_execute_response(
            "demo:ping",
            &response,
            &node,
            &mut context,
            &[],
            None,
        )
        .expect_err("failure response must map to Err");
        assert_eq!(err.message, expected);
    };

    check(
        serde_json::json!({ "success": false, "errors": [{ "message": "connection refused" }] }),
        "connection refused",
    );
    check(
        serde_json::json!({ "success": false, "errors": ["connection refused"] }),
        "connection refused",
    );
    check(
        serde_json::json!({ "success": false, "errors": [] }),
        "plugin reported execution failure",
    );
    check(
        serde_json::json!({ "success": false }),
        "plugin reported execution failure",
    );
}

/// SDK empty-`next_ports` contract: on `success: true` with no explicit
/// ports, the host routes via the describe-derived `default_success_port`
/// instead of terminating the branch. An explicit `next_ports` still
/// overrides the default.
#[test]
fn map_execute_response_applies_default_success_port() {
    use flow_engine_core::connection::Connection;
    let node = plugin_node(serde_json::json!({ "type": "demo:ping" }));
    let out_conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: node.uuid.clone(),
        target_node_uuid: "node-2".to_string(),
        source_port_id: "out".to_string(),
        target_port_id: "in".to_string(),
    };
    let custom_conn = Connection {
        id: "c2".to_string(),
        source_node_uuid: node.uuid.clone(),
        target_node_uuid: "node-3".to_string(),
        source_port_id: "custom".to_string(),
        target_port_id: "in".to_string(),
    };

    // No next_ports + default "out" → resolves through the out port to the
    // connected downstream node.
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    let ok = crate::wrapper::map_execute_response(
        "demo:ping",
        &serde_json::json!({ "success": true, "output_data": { "ok": true } }),
        &node,
        &mut context,
        &[out_conn.clone()],
        Some("out".to_string()),
    )
    .expect("success must map to Ok");
    assert_eq!(
        ok.0,
        vec!["node-2".to_string()],
        "default port must route to the connected downstream node"
    );

    // Explicit next_ports wins over the default: only the "custom" edge is
    // followed even though the default "out" edge also exists.
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    let ok = crate::wrapper::map_execute_response(
        "demo:ping",
        &serde_json::json!({ "success": true, "next_ports": ["custom"], "output_data": {} }),
        &node,
        &mut context,
        &[out_conn.clone(), custom_conn.clone()],
        Some("out".to_string()),
    )
    .expect("success must map to Ok");
    assert_eq!(
        ok.0,
        vec!["node-3".to_string()],
        "explicit next_ports must override the default"
    );

    // No default (None) + no next_ports → empty routing (legacy behavior).
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    let ok = crate::wrapper::map_execute_response(
        "demo:ping",
        &serde_json::json!({ "success": true, "output_data": {} }),
        &node,
        &mut context,
        &[out_conn],
        None,
    )
    .expect("success must map to Ok");
    assert_eq!(ok.0, Vec::<String>::new(), "no default → legacy empty routing");
}

/// `default_success_port` derivation rules from describe ports.
#[test]
fn default_success_port_derivation() {
    use flow_engine_core::plugin::PortDef;
    let port = |id: &str| PortDef {
        id: id.to_string(),
        label: String::new(),
        position: String::new(),
    };
    // Two-state node (in/true/false) → "true".
    assert_eq!(
        crate::wrapper::default_success_port(&[port("in"), port("true"), port("false")]),
        Some("true".to_string())
    );
    // Single-output node (in/out) → "out".
    assert_eq!(
        crate::wrapper::default_success_port(&[port("in"), port("out")]),
        Some("out".to_string())
    );
    // 2+ outputs → no derivable default.
    assert_eq!(
        crate::wrapper::default_success_port(&[port("in"), port("a"), port("b")]),
        None
    );
    // No ports → None.
    assert_eq!(crate::wrapper::default_success_port(&[]), None);
}
