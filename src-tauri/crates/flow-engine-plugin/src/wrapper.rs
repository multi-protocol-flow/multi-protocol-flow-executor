//! Execute RPC mapping helpers for [`crate::sidecar::SidecarPlugin`].
//!
//! Kept out of `sidecar.rs` so both files stay under the workspace 250-pure-
//! LOC ceiling. Everything here is pure with respect to the transport: given
//! an `execute` RPC response (or a `PluginError`), it produces the host
//! `NodePlugin` return contract and mutates the context (back-channel, A10
//! flat-key `variable_updates`, `provider_updates`).

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::node::Node;
use serde_json::Value;

use crate::error::PluginError;

/// Default wrapper-owned execution timeout when the node config sets none.
///
/// Mirrors the executor's `FALLBACK_TIMEOUT_MS` (30 s): plugin nodes have no
/// `execution_timeout_ms()` so this wrapper is the only timeout safety net.
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

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
/// `ErrorDetail` is 136 bytes by design (the read-only core type used
/// everywhere); this sync fn would otherwise trip `result_large_err`.
#[allow(clippy::result_large_err)]
pub(crate) fn map_execute_response(
    type_id: &str,
    response: &Value,
    node: &Node,
    context: &mut ExecutionContext,
    connections: &[Connection],
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
        let next_ports: Vec<String> = obj
            .get("next_ports")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
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

/// Reads the wrapper-owned execution timeout from the node config.
///
/// Honors a `timeout_ms` or `execution_timeout_ms` number field; anything
/// missing / zero / non-numeric falls back to [`DEFAULT_EXECUTION_TIMEOUT`].
pub(crate) fn execution_timeout(config: &Value) -> Duration {
    let ms = config
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .or_else(|| config.get("execution_timeout_ms").and_then(Value::as_u64))
        .unwrap_or(0);
    if ms == 0 {
        DEFAULT_EXECUTION_TIMEOUT
    } else {
        Duration::from_millis(ms)
    }
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
