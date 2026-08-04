//! Per-execution plugin context.
//!
//! Handed to [`Plugin::execute`](crate::runtime::Plugin::execute). Carries
//! the host-resolved config and providers, plus two communication channels
//! back to the host:
//!
//! - `log(level, message)` emits a `log` notification frame (with
//!   `execution_id`/`node_instance_id` correlation when the host provided
//!   them).
//! - `set_variable(name, value)` collects variable write-backs; the runtime
//!   merges them into the `execute` response as a flat `variable_updates`
//!   map (locked A10 contract: keys must be flat top-level variable names,
//!   no dots — the host warns + skips dotted keys).

use std::collections::HashMap;

use mpe_protocol::notification::Notification;
use tokio::sync::mpsc;

/// Everything a plugin node needs to know about one `execute` invocation.
pub struct ExecuteContext {
    /// The host-resolved node config (variables already substituted).
    config: serde_json::Value,
    /// Providers exposed by the host, keyed by name (opaque JSON values;
    /// plugin-typed providers are a host-side concern).
    providers: HashMap<String, serde_json::Value>,
    /// Correlation id of the enclosing flow execution, when provided.
    execution_id: Option<String>,
    /// Correlation id of the node instance, when provided.
    node_instance_id: Option<String>,
    /// Flat variable write-backs accumulated by `set_variable`.
    variable_updates: HashMap<String, serde_json::Value>,
    /// Serialized-frame channel to the runtime's single writer task.
    frame_tx: mpsc::Sender<Vec<u8>>,
}

impl ExecuteContext {
    /// Builds the context from the `execute` request params. The exact host
    /// param shape is locked in M5b conformance; parsing is lenient and every
    /// missing member falls back to a sensible default.
    pub(crate) fn from_params(params: &serde_json::Value, frame_tx: mpsc::Sender<Vec<u8>>) -> Self {
        let config = params
            .get("config")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let providers = params
            .get("providers")
            .and_then(serde_json::Value::as_object)
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let execution_id = params
            .get("execution_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let node_instance_id = params
            .get("node_instance_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        Self {
            config,
            providers,
            execution_id,
            node_instance_id,
            variable_updates: HashMap::new(),
            frame_tx,
        }
    }

    /// The host-resolved node config (JSON value; plugin-typed access is up
    /// to the plugin).
    pub fn config(&self) -> &serde_json::Value {
        &self.config
    }

    /// One provider by name, if the host exposed it.
    pub fn provider(&self, key: &str) -> Option<&serde_json::Value> {
        self.providers.get(key)
    }

    /// All providers exposed by the host.
    pub fn providers(&self) -> &HashMap<String, serde_json::Value> {
        &self.providers
    }

    /// The enclosing flow execution id, when the host provided one.
    pub fn execution_id(&self) -> Option<&str> {
        self.execution_id.as_deref()
    }

    /// The node instance id, when the host provided one.
    pub fn node_instance_id(&self) -> Option<&str> {
        self.node_instance_id.as_deref()
    }

    /// Emits a `log` notification to the host (fire-and-forget; dropped when
    /// the frame channel is full). `level` follows the host convention, e.g.
    /// `"info"` or `"error"`.
    pub fn log(&self, level: &str, message: impl Into<String>) {
        let mut params = serde_json::json!({ "level": level, "message": message.into() });
        if let Some(id) = &self.execution_id {
            params["execution_id"] = serde_json::json!(id);
        }
        if let Some(id) = &self.node_instance_id {
            params["node_instance_id"] = serde_json::json!(id);
        }
        let notification = Notification::new("log", params);
        let bytes = match serde_json::to_vec(&notification) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("mpe-plugin-sdk: log serialization failed: {err}");
                return;
            }
        };
        if self.frame_tx.try_send(bytes).is_err() {
            eprintln!("mpe-plugin-sdk: log notification dropped (frame channel full)");
        }
    }

    /// Records a variable write-back. Per the locked A10 contract the key
    /// must be a flat top-level variable name (no dots); the host warns and
    /// skips dotted keys.
    pub fn set_variable(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.variable_updates.insert(name.into(), value);
    }

    /// The accumulated variable write-backs; the runtime merges this map
    /// into the `execute` response.
    pub fn variable_updates(&self) -> &HashMap<String, serde_json::Value> {
        &self.variable_updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context with a test channel (capacity 16; tests drop the sender's
    /// peer, so log sends always succeed).
    fn context_with_params(params: serde_json::Value) -> (ExecuteContext, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel(16);
        (ExecuteContext::from_params(&params, tx), rx)
    }

    #[test]
    fn from_params_extracts_config_and_providers() {
        let (ctx, _rx) = context_with_params(serde_json::json!({
            "config": {"url": "https://example.com"},
            "providers": {"http": {"timeout": 5}},
            "execution_id": "flow-1",
            "node_instance_id": "node-7",
        }));
        assert_eq!(ctx.config()["url"], "https://example.com");
        assert_eq!(
            ctx.provider("http").and_then(|v| v.get("timeout")),
            Some(&serde_json::json!(5))
        );
        assert_eq!(ctx.execution_id(), Some("flow-1"));
        assert_eq!(ctx.node_instance_id(), Some("node-7"));
    }

    #[test]
    fn from_params_defaults_missing_members() {
        let (ctx, _rx) = context_with_params(serde_json::json!({}));
        assert_eq!(ctx.config(), &serde_json::json!({}));
        assert!(ctx.providers().is_empty());
        assert_eq!(ctx.execution_id(), None);
        assert_eq!(ctx.node_instance_id(), None);
        assert!(ctx.variable_updates().is_empty());
    }

    #[test]
    fn set_variable_accumulates_flat_keys() {
        let (mut ctx, _rx) = context_with_params(serde_json::json!({}));
        ctx.set_variable("answer", serde_json::json!(42));
        ctx.set_variable("answer", serde_json::json!(43)); // last write wins
        ctx.set_variable("name", serde_json::json!("mpe"));
        assert_eq!(ctx.variable_updates().len(), 2);
        assert_eq!(ctx.variable_updates()["answer"], 43);
        assert_eq!(ctx.variable_updates()["name"], "mpe");
    }

    #[test]
    fn log_emits_correlation_fields_when_known() -> Result<(), Box<dyn std::error::Error>> {
        let (ctx, mut rx) = context_with_params(serde_json::json!({
            "execution_id": "flow-1",
            "node_instance_id": "node-7",
        }));
        ctx.log("info", "hello");
        let bytes = rx.try_recv()?;
        let notification: Notification = serde_json::from_slice(&bytes)?;
        assert_eq!(notification.method, "log");
        assert_eq!(notification.params["level"], "info");
        assert_eq!(notification.params["message"], "hello");
        assert_eq!(notification.params["execution_id"], "flow-1");
        assert_eq!(notification.params["node_instance_id"], "node-7");
        Ok(())
    }

    #[test]
    fn log_omits_correlation_fields_when_unknown() -> Result<(), Box<dyn std::error::Error>> {
        let (ctx, mut rx) = context_with_params(serde_json::json!({}));
        ctx.log("error", "no correlation");
        let bytes = rx.try_recv()?;
        let notification: Notification = serde_json::from_slice(&bytes)?;
        assert!(notification.params.get("execution_id").is_none());
        assert!(notification.params.get("node_instance_id").is_none());
        Ok(())
    }
}
