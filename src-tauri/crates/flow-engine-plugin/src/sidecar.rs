//! `SidecarPlugin`: the `NodePlugin` wrapper around a plugin connection.
//!
//! Presents one describe-produced plugin node type as a host `NodePlugin`:
//! host-side variable resolution (the plugin receives FINAL values), the
//! `execute` RPC with a **wrapper-owned** timeout + cancellation (the host
//! executor's `execution_timeout_ms()` returns `None` for plugin nodes, so
//! there is no tokio timeout safety net below us), result mapping
//! (`report_data` back-channel, A10 flat-key `variable_updates`,
//! `provider_updates`), and crash mapping into a single `ErrorDetail`.
//!
//! The mapping/error helpers live in [`crate::wrapper`] so this file stays
//! under the workspace 250-pure-LOC ceiling.
//!
//! The plugin process is owned by the `PluginManager`'s lifecycle entries
//! (M2b); this wrapper only acquires a connection via
//! [`PluginManager::connection_for_node`] and reports crashes — the manager's
//! restart-once policy handles the process.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::resolve_variables_in_json;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::plugin::{FrontendMetadata, NodePlugin, PortDef, ValidationError};
use mpe_plugin_sdk::protocol::describe::NodeDescription;
use serde_json::Value;

use crate::describe::frontend_metadata_from;
use crate::manager::PluginManager;
// (grpc.stream 通知 handler 在 wrapper.rs)
use crate::wrapper::{
    execution_timeout, extract_referenced_variables, map_execute_response, map_plugin_error,
    run_with_timeout_and_cancel,
};

/// Wire method name of the execute RPC.
const EXECUTE_METHOD: &str = "execute";

/// Wire method name of the validate RPC (documented for M4, which owns the
/// async `validate_with_registry` path).
pub const VALIDATE_METHOD: &str = "validate";

/// The `NodePlugin` facade for one describe-produced plugin node type.
pub struct SidecarPlugin {
    node: NodeDescription,
    frontend: Option<FrontendMetadata>,
    viewer: Option<FrontendMetadata>,
    manager: Arc<PluginManager>,
}

impl SidecarPlugin {
    /// Wraps one describe-produced node description with its owning manager.
    ///
    /// M3.3's registration factory (`register_nodes_with`) supplies this
    /// constructor; the manager provides the lifecycle connection for the
    /// node's plugin. The wire frontend description is converted to the
    /// host-side [`FrontendMetadata`] once here so [`NodePlugin::frontend`]
    /// can hand it back by reference.
    pub fn new(node: NodeDescription, manager: Arc<PluginManager>) -> Arc<Self> {
        let frontend = node.frontend.as_ref().map(frontend_metadata_from);
        let viewer = node.viewer.as_ref().map(frontend_metadata_from);
        Arc::new(Self {
            node,
            frontend,
            viewer,
            manager,
        })
    }
}

#[async_trait]
impl NodePlugin for SidecarPlugin {
    fn type_id(&self) -> &str {
        &self.node.type_id
    }

    fn display_name(&self) -> &str {
        &self.node.display_name
    }

    fn category(&self) -> &str {
        self.node.category.as_deref().unwrap_or("plugin")
    }

    fn icon(&self) -> &str {
        self.node.icon.as_deref().unwrap_or("plugin")
    }

    fn color(&self) -> &str {
        self.node.color.as_deref().unwrap_or("#64748b")
    }

    fn ports(&self) -> Vec<PortDef> {
        self.node
            .ports
            .iter()
            .map(|p| PortDef {
                id: p.id.clone(),
                label: p.label.clone(),
                position: if p.kind == "in" { "left" } else { "right" }.to_string(),
            })
            .collect()
    }

    fn default_config(&self) -> Value {
        self.node.default_config.clone()
    }

    /// Structural validation only — the `NodePlugin` trait's `validate` is
    /// synchronous and cannot await the plugin's `validate` RPC. Forwarding
    /// that RPC is deferred to M4's async `validate_with_registry`, which owns
    /// a registry handle it can call [`PluginManager::connection_for_node`]
    /// from.
    fn validate(&self, config: &Value) -> Vec<ValidationError> {
        if config.is_object() || config.is_null() {
            Vec::new()
        } else {
            vec![ValidationError {
                field: "config".to_string(),
                message: "plugin node config must be a JSON object".to_string(),
            }]
        }
    }

    fn config_schema(&self) -> Option<Value> {
        self.node.config_schema.clone()
    }

    fn frontend(&self) -> Option<&FrontendMetadata> {
        self.frontend.as_ref()
    }

    fn viewer(&self) -> Option<&FrontendMetadata> {
        self.viewer.as_ref()
    }

    /// Single-node execution capability comes from the plugin's describe
    /// `capabilities.single_node` declaration — the host's gate consults
    /// this instead of a hardcoded whitelist, so third-party connect nodes
    /// (e.g. `redis:connect`) are testable via `execute_single_node` /
    /// `mpe run node` without a host-side list entry.
    fn supports_single_node(&self) -> bool {
        self.node.capabilities.single_node
    }

    /// Report status fidelity: a plugin that returned `Ok` succeeded; only an
    /// explicit `success: false` in its output data maps to the `false` port.
    fn resolve_port(&self, output: &Option<Value>) -> String {
        match output {
            Some(Value::Object(map)) => {
                if map.get("success").and_then(Value::as_bool) == Some(false) {
                    "false".to_string()
                } else {
                    "true".to_string()
                }
            }
            _ => "true".to_string(),
        }
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), ErrorDetail> {
        // 1. Extract the plugin config from the node data.
        let raw_config = match &node.data {
            NodeData::Plugin(config) => config.clone(),
            other => {
                return Err(ErrorDetail::execution_error(format!(
                    "plugin node {} has non-plugin data: {other:?}",
                    node.uuid
                )))
            }
        };

        // 2. Referenced variable paths (strategy A: only config-referenced).
        let referenced_variables = extract_referenced_variables(&raw_config);

        // 3. Host-side variable resolution — the plugin receives FINAL values.
        let resolved_config = resolve_variables_in_json(&raw_config, context);

        // 4. Wrapper-owned timeout using the manager's configured fallback
        let timeout = execution_timeout(&resolved_config, self.manager.default_execution_timeout());

        // 5. JSON-serializable providers (typed providers cannot be encoded).
        let mut providers = serde_json::Map::new();
        for (key, provider) in &context.providers {
            if let Some(value) = provider.downcast_ref::<Value>() {
                providers.insert(key.clone(), value.clone());
            }
        }

        // 6. Acquire the plugin connection (type_id → plugin → lifecycle).
        let conn = self
            .manager
            .connection_for_node(&self.node.type_id)
            .await
            .map_err(map_plugin_error)?;

        // 6.5 gRPC 流式实时链路（helper 在 wrapper.rs）：grpc:* 节点执行
        // 时把插件的 `grpc.stream` 通知转成 DebugEvent 进 debug store，
        // 报告 viewer 实时消费。非 grpc 节点不注册（零开销）；subscription
        // handle 在 execute 返回后 drop 自动退订。
        let _grpc_stream_subscription = crate::wrapper::register_grpc_stream_if_needed(
            self.node.type_id.as_str(),
            conn.as_ref(),
            context,
            node,
            &resolved_config,
        );

        // 6.6 通用插件流式实时链路：当 context 提供了 execution_event_sender 时，
        // 订阅插件的 stream 通知并实时转发为 ExecutionEvent::PluginEvent。
        let _plugin_stream_subscription = crate::wrapper::register_plugin_stream_if_needed(
            self.node.type_id.as_str(),
            conn.as_ref(),
            context,
            node,
        );

        let params = serde_json::json!({
            "config": resolved_config,
            "providers": providers,
            "referenced_variables": referenced_variables,
            // per-execution 唯一实例 ID（每次 execute_flow 生成 fresh uuid），
            // 供插件按"单次执行"隔离 per-execution 资源（如 Mongo 连接池）。
            "execution_id": context.execution_id,
            "node_instance_id": node.uuid,
        });

        // 7. Execute RPC under the wrapper-owned timeout + cancellation.
        let response = match run_with_timeout_and_cancel(
            conn.request_with_timeout(EXECUTE_METHOD, params, timeout),
            context,
            timeout,
        )
        .await
        {
            Ok(Ok(value)) => value,
            Ok(Err(plugin_error)) => return Err(map_plugin_error(plugin_error)),
            Err(detail) => return Err(detail),
        };

        // 8. Map the response into the NodePlugin return contract.
        // Default success port (SDK empty-`next_ports` contract) derived from
        // the describe ports: "true" when declared, else the sole non-"in"
        // port (single-output nodes like `db:close`), else None.
        let default_success_port = crate::wrapper::default_success_port(&self.ports());
        map_execute_response(
            &self.node.type_id,
            &response,
            node,
            context,
            connections,
            default_success_port,
        )
    }
}
