//! 注册表线程化集成测试（M3.3a）。
//!
//! 验证 `FlowExecutor::with_registry` 注入的共享注册表能执行插件节点，
//! 且报告携带插件 `report_data` back-channel。使用进程内假连接
//! （in-memory duplex echo，镜像 `flow-engine-plugin` 的 `lifecycle_tests.rs`），
//! 不启动真实插件进程。

#![cfg(feature = "plugin-manager")]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use flow_engine_core::connection::Connection;
use flow_engine_core::flow::Flow;
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::NodeData;
use flow_engine_executor::default_registry::default_registry_with_plugins;
use flow_engine_executor::executor::FlowExecutor;
use flow_engine_plugin::framing::{FrameReader, FrameWriter};
use flow_engine_plugin::lifecycle::{Connector, LifecycleConfig};
use flow_engine_plugin::manager::{PluginManager, PluginRegistration};
use flow_engine_plugin::transport::{ConnectionConfig, PluginConnection};
use mpe_plugin_sdk::protocol::describe::{NodeDescription, PortDescription};
use mpe_plugin_sdk::protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};
use mpe_plugin_sdk::protocol::request::Request;

/// 最小合法 manifest（`entry.command` 不会被使用——连接器是进程内假实现）。
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

/// 注册给假 "demo" 插件的节点描述。
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

/// 进程内 echo 连接器：`execute` 返回成功结果（含 `report_data`），
/// 其余方法回显请求参数。
fn live_connector() -> Connector {
    Arc::new(|_: &PluginManifest| {
        let (host_tx, plugin_rx) = tokio::io::duplex(64 * 1024);
        let (plugin_tx, host_rx) = tokio::io::duplex(64 * 1024);
        tokio::spawn(async move {
            let mut reader = FrameReader::new(plugin_rx);
            let mut writer = FrameWriter::new(plugin_tx);
            while let Ok(Some(frame)) = reader.next_frame().await {
                let request = match serde_json::from_slice::<Request>(&frame) {
                    Ok(request) => request,
                    Err(_) => continue,
                };
                let result = if request.method == "execute" {
                    serde_json::json!({
                        "success": true,
                        "next_ports": ["out"],
                        "output_data": {"pong": true},
                        "errors": [],
                        "report_data": {"plugin_name": "demo", "metric": 42},
                        "variable_updates": {"plugin_var": "set"},
                        "provider_updates": {},
                    })
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

/// 注册表线程化：共享注册表执行插件流程，报告携带插件 `report_data`。
#[tokio::test]
async fn registry_threaded_executor_runs_plugin_flow(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 构建带假连接器的 PluginManager，登记 demo:ping 节点。
    let mut manager =
        PluginManager::new(PathBuf::from("/nonexistent")).with_lifecycle_config(LifecycleConfig {
            idle_recycle: Duration::from_secs(60),
            restart_min_interval: Duration::from_secs(1),
            shutdown_grace: Duration::from_millis(50),
            host_locale: None,
            connector: live_connector(),
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

    // 共享注册表：内置节点 + SidecarPlugin 包装的插件节点。
    let registry = Arc::new(default_registry_with_plugins(&manager));
    let executor = FlowExecutor::new().with_registry(registry);

    // 流程：entry -> demo:ping 插件节点。
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
    let connection = Connection {
        id: "conn-1".to_string(),
        source_node_uuid: entry_node.uuid.clone(),
        target_node_uuid: plugin_node.uuid.clone(),
        source_port_id: "out".to_string(),
        target_port_id: "in".to_string(),
    };
    let flow = Flow {
        uuid: "flow-1".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Plugin Flow".to_string(),
        description: None,
        nodes: vec![entry_node, plugin_node],
        connections: vec![connection],
        hooks: None,
    };

    let result = executor.execute_flow(&flow, None, None, None).await;

    // 流程应整体成功，执行了 entry + 插件节点。
    assert!(result.success, "flow should succeed: {:?}", result.error);
    assert_eq!(result.executed_count, 2);

    // 插件节点报告应携带 report_data。
    let plugin_report = result
        .flow_report
        .as_ref()
        .and_then(|r| r.node_reports.iter().find(|n| n.node_type == "demo:ping"))
        .ok_or_else(|| "plugin node report missing".to_string())?;
    assert_eq!(
        plugin_report.plugin_data,
        Some(serde_json::json!({"plugin_name": "demo", "metric": 42})),
        "report must carry the plugin report_data"
    );
    // 插件输出数据也应在报告中。
    assert_eq!(
        plugin_report.output_data,
        Some(serde_json::json!({"pong": true}))
    );
    // 变量更新（A10 flat-key）应进入变量池并被报告为 variable_changes。
    assert!(
        plugin_report
            .variable_changes
            .as_ref()
            .map(|changes| changes.iter().any(|c| c.key == "plugin_var"))
            .unwrap_or(false),
        "plugin variable_updates must be recorded as variable_changes"
    );

    Ok(())
}
