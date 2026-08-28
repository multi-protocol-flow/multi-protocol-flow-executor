//! Execute RPC mapping helpers for [`crate::sidecar::SidecarPlugin`].
//!
//! Kept out of `sidecar.rs` so both files stay under the workspace 250-pure-
//! LOC ceiling. Everything here is pure with respect to the transport: given
//! an `execute` RPC response (or a `PluginError`), it produces the host
//! `NodePlugin` return contract and mutates the context (back-channel, A10
//! flat-key `variable_updates`, `provider_updates`).

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::plugin::PortDef;
use serde_json::Value;

use crate::error::PluginError;

/// Default wrapper-owned execution timeout when the node config sets none.
///
/// Mirrors the executor's `FALLBACK_TIMEOUT_MS` (30 s): plugin nodes have no
/// `execution_timeout_ms()` so this wrapper is the only timeout safety net.
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Derives the host default success port from a plugin's describe ports.
///
/// Fulfils the SDK's empty-`next_ports` contract: "true" when the describe
/// declares it (two-state nodes), else the sole non-"in" port (single-output
/// nodes like `db:close`), else `None` (no derivable default).
pub(crate) fn default_success_port(ports: &[PortDef]) -> Option<String> {
    if ports.iter().any(|p| p.id == "true") {
        Some("true".to_string())
    } else {
        let outs: Vec<&str> = ports
            .iter()
            .filter(|p| p.id != "in")
            .map(|p| p.id.as_str())
            .collect();
        match outs.as_slice() {
            [only] => Some((*only).to_string()),
            _ => None,
        }
    }
}

/// Maps the plugin's `next_ports` (port ids) to next node UUIDs.
///
/// A port with a matching outgoing connection resolves to its target node
/// UUIDs; a port with no connection contributes nothing (the executor treats
/// an empty result for a port-routing node as "no matching port connection",
/// terminating the branch cleanly).
pub(crate) fn resolve_next_node_ids(
    next_ports: &[String],
    node: &Node,
    connections: &[Connection],
) -> Vec<String> {
    let mut next_node_ids = Vec::new();
    for port in next_ports {
        next_node_ids.extend(
            connections
                .iter()
                .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == *port)
                .map(|c| c.target_node_uuid.clone()),
        );
    }
    next_node_ids
}

/// Maps the `execute` RPC response into the `NodePlugin` return value.
///
/// `report_data` is back-channelled into
/// [`ExecutionContext::current_node_plugin_data`] (consumed at report
/// construction by M3.3). `variable_updates` are flat-key validated (A10:
/// dotted keys warn + skip) then merged into the variable storage.
/// `provider_updates` are absorbed as `serde_json::Value` providers (typed
/// providers are P1; JSON values round-trip as-is). On `success: true` the
/// resolved next node ids + `output_data` are returned; on `success: false`
/// the first `errors[]` entry (or a generic message) becomes the
/// `Err(ErrorDetail)`.
///
/// `default_success_port` fulfils the SDK's empty-`next_ports` contract: when
/// the plugin returns no explicit ports on success, the host routes via the
/// describe-derived default port (see [`crate::sidecar`]) instead of silently
/// terminating the branch.
///
/// `ErrorDetail` is 136 bytes by design (the read-only core type used
/// everywhere); this sync fn would otherwise trip `result_large_err`.
#[allow(clippy::result_large_err)]
pub(crate) fn map_execute_response(
    type_id: &str,
    response: &Value,
    node: &Node,
    context: &mut ExecutionContext,
    connections: &[Connection],
    default_success_port: Option<String>,
) -> Result<(Vec<String>, Option<Value>), ErrorDetail> {
    let obj = response.as_object().ok_or_else(|| {
        ErrorDetail::execution_error(format!(
            "plugin {type_id} node {} returned a non-object execute result",
            node.uuid
        ))
    })?;

    // report_data back-channel (M3.1 slot; consumed at report build).
    if let Some(report_data) = obj.get("report_data").filter(|v| !v.is_null()) {
        context.current_node_plugin_data = Some(report_data.clone());
    }

    // variable_updates: A10 flat-key validation + merge.
    if let Some(updates) = obj.get("variable_updates") {
        merge_variable_updates(updates, context);
    }

    // provider_updates: absorb as JSON-typed providers (P1 = typed).
    if let Some(updates) = obj.get("provider_updates") {
        merge_provider_updates(updates, context);
    }

    let success = obj.get("success").and_then(Value::as_bool).unwrap_or(false);

    if success {
        // Non-fatal errors reported alongside a success are warnings only.
        if let Some(errors) = obj.get("errors").and_then(Value::as_array) {
            for err in errors {
                log::warn!(
                    "plugin {type_id} node {} reported non-fatal error: {err}",
                    node.uuid
                );
            }
        }
        let mut next_ports: Vec<String> = obj
            .get("next_ports")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        // SDK contract (mpe-plugin-sdk types.rs): an empty `next_ports` lets
        // the HOST apply its default routing. Fulfil it: for two-state /
        // single-output nodes the describe ports derive the success port
        // ("true" if present, else the sole non-"in" port). Explicit plugin
        // `next_ports` still override the default.
        if next_ports.is_empty() {
            if let Some(port) = default_success_port {
                next_ports.push(port);
            }
        }
        let output_data = obj.get("output_data").cloned().filter(|v| !v.is_null());
        Ok((
            resolve_next_node_ids(&next_ports, node, connections),
            output_data,
        ))
    } else {
        let errors = obj.get("errors").and_then(Value::as_array);
        // Per docs/plugin-protocol.md each entry is a `{"message": ...}`
        // object; tolerate legacy plain-string entries from plugins built
        // against pre-M5b SDK versions.
        let message = errors
            .and_then(|arr| arr.first())
            .and_then(|e| match e {
                Value::Object(map) => map
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "plugin reported execution failure".to_string());
        let code = errors
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str);
        let mut detail = ErrorDetail::new(ErrorKind::Execution, message);
        if let Some(code) = code {
            detail = detail.with_i18n_arg("code", code.to_string());
        }
        if let Some(outputs) = obj
            .get("output_data")
            .or_else(|| obj.get("report_data"))
            .or_else(|| obj.get("outputs"))
            .or_else(|| obj.get("data"))
        {
            detail = detail.with_context(outputs.clone());
        }
        Err(detail)
    }
}

/// Extracts the set of `{{path}}` variable references from a JSON tree
/// (strategy A — only config-referenced paths, for the report's
/// `variable_changes`).
pub(crate) fn extract_referenced_variables(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_referenced_variables(value, &mut paths);
    paths
}

/// Recursive helper for [`extract_referenced_variables`].
///
/// Scans string leaves for `{{ ... }}` spans (a tiny manual parser — no
/// regex dependency), stripping whitespace inside the braces.
fn collect_referenced_variables(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            let mut rest = s.as_str();
            while let Some(start) = rest.find("{{") {
                let after = &rest[start + 2..];
                if let Some(end) = after.find("}}") {
                    let path = after[..end].trim();
                    if !path.is_empty() && !out.iter().any(|p| p == path) {
                        out.push(path.to_string());
                    }
                    rest = &after[end + 2..];
                } else {
                    break;
                }
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                collect_referenced_variables(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_referenced_variables(v, out);
            }
        }
        _ => {}
    }
}

/// Reads the wrapper-owned execution timeout from the node config and referenced upstream connection/provider nodes.
///
/// Honors:
/// 1. `override_timeout_ms` (explicit override on this node);
/// 2. Referenced upstream provider/connection node timeout from `node_map` (via `provider_uuid` or `connection_id`);
/// 3. `timeout_ms` or `execution_timeout_ms` on this node;
/// 4. Inline `provider.timeout_ms` (only when `provider_uuid` is absent/empty);
/// 5. `fallback_timeout`.
pub(crate) fn execution_timeout_with_node_map(
    config: &Value,
    node_map: &HashMap<String, &Node>,
    fallback_timeout: Duration,
) -> Duration {
    // 1. Explicit override on this node always takes precedence
    if let Some(ms) = config
        .get("override_timeout_ms")
        .and_then(Value::as_u64)
        .filter(|&t| t > 0)
    {
        return Duration::from_millis(ms);
    }

    // 2. Inherit from referenced upstream provider or connect node in node_map
    let referenced_id = config
        .get("provider_uuid")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            config
                .get("connection_id")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        });

    if let Some(target_uuid) = referenced_id {
        if let Some(target_node) = node_map.get(target_uuid) {
            if let NodeData::Plugin(prov_cfg) = &target_node.data {
                if let Some(ms) = prov_cfg
                    .get("timeout_ms")
                    .or_else(|| prov_cfg.get("execution_timeout_ms"))
                    .and_then(Value::as_u64)
                    .filter(|&t| t > 0)
                {
                    return Duration::from_millis(ms);
                }
            }
        }
    }

    // 3. Node's direct timeout_ms or execution_timeout_ms
    if let Some(ms) = config
        .get("timeout_ms")
        .or_else(|| config.get("execution_timeout_ms"))
        .and_then(Value::as_u64)
        .filter(|&t| t > 0)
    {
        return Duration::from_millis(ms);
    }

    // 4. Inline provider.timeout_ms (only if provider_uuid was not referenced)
    if referenced_id.is_none() {
        if let Some(ms) = config
            .get("provider")
            .and_then(|p| p.get("timeout_ms"))
            .and_then(Value::as_u64)
            .filter(|&t| t > 0)
        {
            return Duration::from_millis(ms);
        }
    }

    // 5. Fallback
    fallback_timeout
}

/// Reads the wrapper-owned execution timeout from the node config without node map context (used in tests).
#[cfg(test)]
pub(crate) fn execution_timeout(config: &Value, fallback_timeout: Duration) -> Duration {
    execution_timeout_with_node_map(config, &HashMap::new(), fallback_timeout)
}

/// Runs `future` under the wrapper-owned timeout, honouring the context's
/// cancel receiver (if any). Cancellation wins when the two race; a watchdog
/// timeout yields [`ErrorKind::Timeout`].
pub(crate) async fn run_with_timeout_and_cancel<F, T>(
    future: F,
    context: &mut ExecutionContext,
    timeout: Duration,
) -> Result<T, ErrorDetail>
where
    F: std::future::Future<Output = T>,
{
    if let Some(mut rx) = context.cancel_receiver.clone() {
        if *rx.borrow() {
            return Err(cancelled_error());
        }
        tokio::select! {
            _ = rx.changed() => Err(cancelled_error()),
            result = tokio::time::timeout(timeout, future) => {
                match result {
                    Ok(inner) => Ok(inner),
                    Err(_) => Err(timeout_error("plugin execution timed out")),
                }
            }
        }
    } else {
        match tokio::time::timeout(timeout, future).await {
            Ok(inner) => Ok(inner),
            Err(_) => Err(timeout_error("plugin execution timed out")),
        }
    }
}

/// Builds the `ErrorDetail` for a wrapper/transport failure.
///
/// Timeouts map to [`ErrorKind::Timeout`] (+ `E_TIMEOUT`); a crashed plugin
/// maps to [`ErrorKind::Execution`] with `ErrorCode::PluginCrashed` (the
/// manager's restart-once policy handles the process — we only report);
/// anything else maps to [`ErrorKind::Execution`] with the display form.
pub(crate) fn map_plugin_error(error: PluginError) -> ErrorDetail {
    match error {
        PluginError::Timeout(_) => timeout_error("plugin request timed out"),
        PluginError::PluginCrashed(detail) => {
            ErrorDetail::new(ErrorKind::Execution, format!("plugin crashed: {detail}"))
                .with_error_code_typed(ErrorCode::PluginCrashed)
        }
        other => ErrorDetail::new(ErrorKind::Execution, format!("{other}")),
    }
}

/// Merges `variable_updates` into the context's variable storage.
///
/// A10 (locked): keys MUST be flat top-level names. A dotted key is warned
/// about and skipped (non-fatal) — never silently stored under a name the
/// flat `VariableStorage` cannot resolve.
fn merge_variable_updates(updates: &Value, context: &mut ExecutionContext) {
    let Some(obj) = updates.as_object() else {
        log::warn!("plugin variable_updates is not an object; ignored");
        return;
    };
    let source = context.current_node_uuid.clone().unwrap_or_default();
    for (key, value) in obj {
        if key.contains('.') {
            log::warn!(
                "plugin variable_updates key {key:?} contains a dot; skipped (A10 flat-key contract)"
            );
            continue;
        }
        context
            .variables
            .set_variable(key.clone(), value.clone(), source.clone());
    }
}

/// Absorbs `provider_updates` into the context's provider registry.
///
/// Providers are `Arc<dyn Any + Send + Sync>`; the values here are JSON, so
/// each is stored as an `Arc<serde_json::Value>` retrievable via
/// [`ExecutionContext::get_provider::<serde_json::Value>`]. Typed providers
/// (protocol handles) are P1 — plugins must not assume a plugin-provided key
/// is downcastable to a host protocol provider.
fn merge_provider_updates(updates: &Value, context: &mut ExecutionContext) {
    let Some(obj) = updates.as_object() else {
        log::warn!("plugin provider_updates is not an object; ignored");
        return;
    };
    for (key, value) in obj {
        let wrapped: Arc<dyn Any + Send + Sync> = Arc::new(value.clone());
        context.set_provider(key.clone(), wrapped);
    }
}

/// `ErrorDetail` for a wrapper- or transport-level timeout.
fn timeout_error(message: &str) -> ErrorDetail {
    ErrorDetail::new(ErrorKind::Timeout, message.to_string())
        .with_error_code_typed(ErrorCode::E_TIMEOUT)
}

/// `ErrorDetail` for a cancelled execution.
fn cancelled_error() -> ErrorDetail {
    ErrorDetail::new(
        ErrorKind::Cancelled,
        "plugin execution cancelled".to_string(),
    )
    .with_error_code_typed(ErrorCode::E_CANCELLED)
}

/// Registers a `grpc.stream` notification subscription on `conn` that
/// forwards plugin-side streaming messages/errors into the host debug store
/// as `DebugEvent::GrpcStreamMessage/Error` (the existing grpcStreamMessages
/// data path — the report viewer iframe consumes it in real time).
///
/// The `CorrelationFilter` restricts delivery to notifications from
/// `execution_id`; notifications from other executions are dropped. The
/// returned `SubscriptionHandle` unsubscribes when dropped (execute done).
pub(crate) fn register_grpc_stream_handler(
    conn: &crate::transport::PluginConnection,
    execution_id: String,
    node_uuid: String,
    connection_id: String,
    sender: Arc<dyn flow_engine_core::traits::DebugEventSender>,
) -> crate::transport::SubscriptionHandle {
    use flow_engine_core::debug_types::DebugEvent;
    use mpe_plugin_sdk::protocol::notification::Notification;
    use std::sync::Mutex;

    let counters: Arc<Mutex<std::collections::HashMap<String, u32>>> = Arc::default();
    conn.register_notification_handler(
        "grpc.stream",
        crate::transport::CorrelationFilter {
            execution_id: Some(execution_id),
            node_instance_id: None,
        },
        move |notification: &Notification| {
            let params = &notification.params;
            let Some(call_id) = params.get("call_id").and_then(Value::as_str) else {
                return;
            };
            let kind = params
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("message");
            // 归属到发出该流的 call 节点（emit 自动注入 correlation）
            let node_instance_id = params
                .get("node_instance_id")
                .and_then(Value::as_str)
                .unwrap_or(&node_uuid);
            if kind == "error" {
                let message = params
                    .get("data")
                    .and_then(|d| d.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("grpc stream error")
                    .to_string();
                let _ = sender.send_event(DebugEvent::GrpcStreamError {
                    node_uuid: node_instance_id.to_string(),
                    connection_id: connection_id.clone(),
                    call_id: call_id.to_string(),
                    message,
                });
            } else {
                let data = params.get("data").cloned().unwrap_or(Value::Null);
                let data_str = serde_json::to_string(&data).unwrap_or_default();
                let size_bytes = data_str.len() as u64;
                let index = {
                    let mut counters = counters
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let counter = counters.entry(call_id.to_string()).or_insert(0);
                    *counter += 1;
                    *counter
                };
                let _ = sender.send_event(DebugEvent::GrpcStreamMessage {
                    node_uuid: node_instance_id.to_string(),
                    connection_id: connection_id.clone(),
                    call_id: call_id.to_string(),
                    index,
                    data: data_str,
                    received_at_ms: 0,
                    size_bytes,
                    is_last: false,
                });
            }
        },
    )
}

/// gRPC 流式实时链路入口（sidecar execute 调用）：`grpc:*` 节点执行时把
/// 插件的 `grpc.stream` 通知（backpressure emit，自动带 correlation）
/// 转成 `DebugEvent::GrpcStreamMessage/Error` 进宿主 debug store
/// （grpcStreamMessages 按 `call_id` 累积，报告 viewer iframe 实时消费）。
///
/// 非 grpc 节点返回 None（零开销）；宿主未注入 debug sender 也返回 None。
/// 返回的 handle 在 execute 返回后 drop 自动退订。
pub(crate) fn register_grpc_stream_if_needed(
    node_type_id: &str,
    conn: &crate::transport::PluginConnection,
    context: &ExecutionContext,
    node: &Node,
    resolved_config: &Value,
) -> Option<crate::transport::SubscriptionHandle> {
    if !node_type_id.starts_with("grpc:") {
        return None;
    }
    let sender = context.debug_event_sender.clone()?;
    let connection_id = resolved_config
        .get("connection_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some(register_grpc_stream_handler(
        conn,
        context.execution_id.clone(),
        node.uuid.clone(),
        connection_id,
        sender,
    ))
}

/// Registers a generic plugin `stream` (and `llm.stream` for backward compatibility)
/// notification subscription on `conn` that forwards plugin-side streaming events into
/// the host execution event sender as `ExecutionEvent::PluginEvent`.
///
/// The `CorrelationFilter` restricts delivery to notifications from `execution_id`.
/// The returned `SubscriptionHandle`s unsubscribe when dropped (execute done).
pub(crate) fn register_plugin_stream_handler(
    conn: &crate::transport::PluginConnection,
    execution_id: String,
    flow_uuid: String,
    node_uuid: String,
    node_type: String,
    sender: Arc<dyn flow_engine_core::traits::ExecutionEventSender>,
) -> Vec<crate::transport::SubscriptionHandle> {
    use flow_engine_core::execution_event::ExecutionEvent;
    use mpe_plugin_sdk::protocol::notification::Notification;

    let methods = ["stream", "llm.stream"];
    let mut handles = Vec::with_capacity(methods.len());

    for method in methods {
        let sender = Arc::clone(&sender);
        let flow_uuid = flow_uuid.clone();
        let node_uuid = node_uuid.clone();
        let node_type = node_type.clone();

        let handle = conn.register_notification_handler(
            method,
            crate::transport::CorrelationFilter {
                execution_id: Some(execution_id.clone()),
                node_instance_id: None,
            },
            move |notification: &Notification| {
                let _ = sender.send_event(ExecutionEvent::PluginEvent {
                    flow_uuid: flow_uuid.clone(),
                    node_uuid: node_uuid.clone(),
                    node_type: node_type.clone(),
                    event_type: "stream".to_string(),
                    data: notification.params.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            },
        );
        handles.push(handle);
    }

    handles
}

/// Generic plugin stream real-time event subscription entry point.
///
/// Subscribes to `stream` notifications on `conn` if an `execution_event_sender` is
/// provided in the context, forwarding them as `ExecutionEvent::PluginEvent`.
/// The returned subscription handles unsubscribe automatically when dropped after `execute`.
pub(crate) fn register_plugin_stream_if_needed(
    node_type_id: &str,
    conn: &crate::transport::PluginConnection,
    context: &ExecutionContext,
    node: &Node,
) -> Option<Vec<crate::transport::SubscriptionHandle>> {
    let sender = context.execution_event_sender.clone()?;
    Some(register_plugin_stream_handler(
        conn,
        context.execution_id.clone(),
        context.flow_uuid.clone(),
        node.uuid.clone(),
        node_type_id.to_string(),
        sender,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_execution_timeout_precedence() {
        // Default fallback
        assert_eq!(execution_timeout(&json!({}), DEFAULT_EXECUTION_TIMEOUT), DEFAULT_EXECUTION_TIMEOUT);
        assert_eq!(execution_timeout(&json!({}), Duration::from_secs(60)), Duration::from_secs(60));

        // timeout_ms
        assert_eq!(execution_timeout(&json!({ "timeout_ms": 45000 }), DEFAULT_EXECUTION_TIMEOUT), Duration::from_millis(45000));

        // execution_timeout_ms
        assert_eq!(execution_timeout(&json!({ "execution_timeout_ms": 50000 }), DEFAULT_EXECUTION_TIMEOUT), Duration::from_millis(50000));

        // provider.timeout_ms
        assert_eq!(execution_timeout(&json!({ "provider": { "timeout_ms": 60000 } }), DEFAULT_EXECUTION_TIMEOUT), Duration::from_millis(60000));

        // override_timeout_ms takes precedence
        assert_eq!(
            execution_timeout(&json!({
                "override_timeout_ms": 90000,
                "timeout_ms": 45000,
                "provider": { "timeout_ms": 60000 }
            }), DEFAULT_EXECUTION_TIMEOUT),
            Duration::from_millis(90000)
        );
    }

    #[test]
    fn test_execution_timeout_with_provider_uuid_inheritance() {
        let provider_node = Node {
            uuid: "prov-uuid-1".to_string(),
            node_type: "llm:provider".to_string(),
            name: "My Provider".to_string(),
            data: NodeData::Plugin(json!({
                "base_url": "http://api.example.com",
                "model": "gpt-5",
                "timeout_ms": 300000
            })),
            ..Default::default()
        };

        let mut node_map = HashMap::new();
        node_map.insert("prov-uuid-1".to_string(), &provider_node);

        // Node references provider_uuid, has stale inline provider.timeout_ms = 60000.
        // It MUST inherit 300000ms from the referenced provider node!
        let chat_config = json!({
            "provider_uuid": "prov-uuid-1",
            "provider": {
                "timeout_ms": 60000
            },
            "messages": []
        });

        assert_eq!(
            execution_timeout_with_node_map(&chat_config, &node_map, DEFAULT_EXECUTION_TIMEOUT),
            Duration::from_millis(300000)
        );

        // If override_timeout_ms is explicitly specified on the chat node, it overrides the provider
        let chat_with_override = json!({
            "provider_uuid": "prov-uuid-1",
            "override_timeout_ms": 120000,
            "provider": {
                "timeout_ms": 60000
            }
        });
        assert_eq!(
            execution_timeout_with_node_map(&chat_with_override, &node_map, DEFAULT_EXECUTION_TIMEOUT),
            Duration::from_millis(120000)
        );
    }

    #[test]
    fn test_execution_timeout_with_connection_id_inheritance() {
        let db_connect_node = Node {
            uuid: "db-conn-1".to_string(),
            node_type: "db:connect".to_string(),
            name: "MySQL Conn".to_string(),
            data: NodeData::Plugin(json!({
                "timeout_ms": 150000
            })),
            ..Default::default()
        };

        let mut node_map = HashMap::new();
        node_map.insert("db-conn-1".to_string(), &db_connect_node);

        let query_config = json!({
            "connection_id": "db-conn-1",
            "sql": "SELECT 1"
        });

        assert_eq!(
            execution_timeout_with_node_map(&query_config, &node_map, DEFAULT_EXECUTION_TIMEOUT),
            Duration::from_millis(150000)
        );
    }
}
