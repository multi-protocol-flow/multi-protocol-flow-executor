//! `PluginManager`: scan + describe handshake + quarantine + lifecycle +
//! registration.
//!
//! Owns the plugins directory, the per-plugin describe connections, and the
//! quarantine policy: a plugin whose describe handshake fails
//! [`MAX_DESCRIBE_FAILURES`] times (spawn failure, RPC timeout, connection
//! crash, or error response) is disabled for the rest of this manager's
//! lifetime and skipped with a warning on every later scan. A successful
//! describe resets the failure counter.
//!
//! Every successful describe seeds a [`LifecycleEntry`] (from
//! [`crate::lifecycle`]) so later [`PluginManager::connection_for`] calls get
//! an on-demand connection, an idle reaper, and the restart policy for free.
//! Streaming plugins (`capabilities.streaming`) are registered as Resident.
//!
//! Successful describes also retain a [`PluginRegistration`]; the registration
//! path (collision policy + insertion into the host `NodeRegistry`) lives in
//! [`crate::registration`] as inherent impls on this type.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mpe_plugin_sdk::protocol::datasource::DatasourceDescription;
use mpe_plugin_sdk::protocol::describe::{DescribeResult, NodeDescription};
use mpe_plugin_sdk::protocol::manifest::PluginManifest;

use crate::describe::{normalize_nodes, perform_describe};
use crate::discovery::{DiscoveredPlugin, PluginScan};
use crate::error::PluginError;
use crate::lifecycle::{LifecycleConfig, LifecycleEntry, ResidencyMode};
use crate::process;
use crate::registration::CollisionReport;
use crate::transport::{ConnectionConfig, PluginConnection};

/// Describe attempts before a plugin is quarantined for this manager's
/// lifetime.
pub const MAX_DESCRIBE_FAILURES: u32 = 3;

/// Upper bound for one describe handshake.
///
/// Describe is a light request/response exchange; a healthy plugin answers
/// in milliseconds. Five seconds bounds a hung or malicious interpreter,
/// which the transport's 30s per-request timeout would otherwise let stall
/// every scan and validate for half a minute per plugin.
pub const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(5);

/// A plugin that survived the describe handshake.
#[derive(Debug, Clone)]
pub struct PluginRegistration {
    /// Plugin name (from its on-disk manifest).
    pub name: String,
    /// Plugin version (from its on-disk manifest).
    pub version: String,
    /// The plugin's install directory (contains `plugin.json`).
    pub dir: PathBuf,
    /// The plugin's authoritative manifest (the describe echo; its
    /// `capabilities` override the on-disk manifest).
    pub manifest: PluginManifest,
    /// Node types the plugin provides, P0-normalized (non-inline frontends
    /// dropped with a warning).
    pub nodes: Vec<NodeDescription>,
    /// Datasources the plugin provides (empty for protocol plugins).
    pub datasources: Vec<DatasourceDescription>,
}

/// `PluginManager`: plugins-directory scan, describe handshake, quarantine,
/// and process lifecycle.
pub struct PluginManager {
    plugins_dir: PathBuf,
    connection_config: ConnectionConfig,
    /// Describe-failure counts per plugin name (reset on success).
    failures: HashMap<String, u32>,
    /// Plugin names disabled for this manager's lifetime.
    quarantined: HashSet<String>,
    /// Registered lifecycle entries by plugin name (seeded by describe).
    entries: HashMap<String, Arc<LifecycleEntry>>,
    /// Lifecycle policy shared by all registered entries.
    lifecycle_config: LifecycleConfig,
    /// Registrations retained from every successful describe; their nodes are
    /// inserted into the host registry by `register_nodes` / `register_nodes_with`
    /// (see [`crate::registration`]).
    pub(crate) registrations: Vec<PluginRegistration>,
    /// Node type ids already claimed by plugin registrations (first-wins set;
    /// persisted for the manager's lifetime so later calls cannot shadow).
    pub(crate) claimed_node_ids: HashSet<String>,
    /// Every node the collision policy rejected (test-observable).
    pub(crate) collisions: Vec<CollisionReport>,
    /// Maps a registered node `type_id` to the plugin name that owns it
    /// (populated by [`PluginManager::add_registration`]; drives
    /// [`PluginManager::connection_for_node`]).
    pub(crate) type_id_to_plugin: HashMap<String, String>,
    /// Maps a registered datasource id to the plugin name that owns it
    /// (populated by [`PluginManager::register_datasources`]; drives
    /// [`PluginManager::datasource_call`]).
    pub(crate) datasource_index: HashMap<String, String>,
    /// Registered datasource descriptions by id (first-wins; listing source
    /// for `list_datasources`).
    pub(crate) datasource_descriptions: HashMap<String, DatasourceDescription>,
    /// Upper bound for one describe handshake; exceeded, the plugin process
    /// is killed and the failure counts toward the quarantine policy
    /// (defaults to [`DESCRIBE_TIMEOUT`]).
    describe_timeout: Duration,
    /// Default execution timeout when a plugin node config specifies none
    /// (defaults to [`crate::wrapper::DEFAULT_EXECUTION_TIMEOUT`]).
    default_execution_timeout: Duration,
    /// The host's current UI language (BCP-47, e.g. `en-US`), injected as
    /// `MPE_LOCALE` into plugin processes at spawn when it matches the
    /// plugin's declared `locales` (falling back to `default_locale`).
    /// `None` = no injection; plugins keep their hard-coded default.
    host_locale: Option<String>,
    /// Most recent describe failure reason per plugin name — surfaced to the
    /// UI (`list_installed_plugins` `error` field) and to the reload flow so
    /// a plugin that fails to load is diagnosable without digging through
    /// logs.
    last_errors: Mutex<HashMap<String, String>>,
}

impl PluginManager {
    /// Creates a manager that scans `plugins_dir` with default connection
    /// settings and default lifecycle policy.
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            connection_config: ConnectionConfig::default(),
            failures: HashMap::new(),
            quarantined: HashSet::new(),
            entries: HashMap::new(),
            lifecycle_config: LifecycleConfig::default(),
            registrations: Vec::new(),
            claimed_node_ids: HashSet::new(),
            collisions: Vec::new(),
            type_id_to_plugin: HashMap::new(),
            datasource_index: HashMap::new(),
            datasource_descriptions: HashMap::new(),
            describe_timeout: DESCRIBE_TIMEOUT,
            default_execution_timeout: crate::wrapper::DEFAULT_EXECUTION_TIMEOUT,
            host_locale: None,
            last_errors: Mutex::new(HashMap::new()),
        }
    }

    /// Sets the host UI language (BCP-47, e.g. `en-US`) injected as
    /// `MPE_LOCALE` into plugin processes at spawn.
    ///
    /// The language is fixed per plugin process for its lifetime: the host
    /// language is used when it matches the plugin's declared `locales`,
    /// otherwise the plugin's `default_locale`. Changing the host language
    /// requires restarting or reloading plugins to take effect.
    pub fn with_host_locale(mut self, locale: Option<String>) -> Self {
        self.host_locale = locale.clone();
        self.lifecycle_config = self.lifecycle_config.with_host_locale(locale);
        self
    }

    /// Overrides the describe handshake bound for subsequently scanned
    /// plugins.
    ///
    /// A healthy plugin answers `describe` in milliseconds; this cap turns a
    /// hung or malicious interpreter into a bounded [`PluginError::Timeout`]
    /// that counts toward the quarantine policy instead of blocking scans
    /// and validates indefinitely.
    pub fn with_describe_timeout(mut self, timeout: Duration) -> Self {
        self.describe_timeout = timeout;
        self
    }

    /// Overrides the lifecycle policy for subsequently registered plugins.
    pub fn with_lifecycle_config(mut self, config: LifecycleConfig) -> Self {
        self.lifecycle_config = config;
        self
    }
    /// Overrides the connection configuration for subsequently spawned plugins.
    pub fn with_connection_config(mut self, config: ConnectionConfig) -> Self {
        self.connection_config = config.clone();
        self.lifecycle_config = self.lifecycle_config.with_connection_config(config);
        self
    }
    /// Overrides the default execution timeout when a plugin node config specifies none.
    ///
    /// Also ensures the underlying connection's request timeout is at least this duration,
    /// preventing transport cutoff before the execution timeout fires.
    pub fn with_default_execution_timeout(mut self, timeout: Duration) -> Self {
        self.default_execution_timeout = timeout;
        if self.connection_config.request_timeout < timeout {
            self.connection_config.request_timeout = timeout;
        }
        self
    }

    /// Returns the default execution timeout for plugin nodes.
    pub fn default_execution_timeout(&self) -> Duration {
        self.default_execution_timeout
    }

    /// Registers a plugin with the lifecycle manager, creating an entry whose
    /// residency mode comes from `manifest.capabilities.streaming`
    /// (streaming → [`ResidencyMode::Resident`], otherwise on-demand).
    ///
    /// An on-demand entry gets an idle reaper immediately; the process itself
    /// is not spawned until the first [`PluginManager::connection_for`].
    pub fn register_plugin(&mut self, name: String, manifest: PluginManifest) {
        let mode = if manifest.capabilities.streaming {
            ResidencyMode::Resident
        } else {
            ResidencyMode::OnDemand
        };
        let config = &self.lifecycle_config;
        let entry = LifecycleEntry::new(
            name.clone(),
            manifest,
            mode,
            config.idle_recycle,
            config.restart_min_interval,
            config.shutdown_grace,
            Arc::clone(&config.connector),
        );
        entry.spawn_reaper();
        self.entries.insert(name, entry);
    }

    /// Acquires a live connection for a registered plugin, starting or
    /// restarting its process as the lifecycle policy dictates.
    pub async fn connection_for(&self, name: &str) -> Result<Arc<PluginConnection>, PluginError> {
        match self.entries.get(name) {
            Some(entry) => entry.connection().await,
            None => Err(PluginError::ProtocolError(format!("unknown plugin {name}"))),
        }
    }

    /// Shuts down every registered plugin's connection within `grace`.
    pub async fn shutdown_all(&self, grace: Duration) {
        for entry in self.entries.values() {
            entry.shutdown(grace).await;
        }
    }

    /// Notifies every **running** plugin that the flow execution `execution_id`
    /// has ended, so the plugin can release per-execution resources (e.g. a
    /// Mongo connection pool keyed by `execution_id`).
    ///
    /// Fire-and-forget: uses [`LifecycleEntry::try_connection`] so idle
    /// on-demand plugins are **not** spawned just to receive the notice, and
    /// per-plugin send failures are logged and skipped (never fatal).
    pub async fn notify_flow_ended(&self, execution_id: &str) {
        let params = serde_json::json!({ "execution_id": execution_id });
        for entry in self.entries.values() {
            let Some(conn) = entry.try_connection().await else {
                continue;
            };
            if let Err(e) = conn
                .send_notification(
                    mpe_plugin_sdk::protocol::notification::FLOW_ENDED_METHOD,
                    params.clone(),
                )
                .await
            {
                log::debug!("flowEnded notify failed for plugin {}: {e}", entry.name);
            }
        }
    }

    /// Scans the plugins directory and runs the describe handshake on every
    /// discovered, non-quarantined plugin.
    ///
    /// Returns one [`PluginRegistration`] per successful describe. Each
    /// handshake spawns the plugin process, issues `describe` over the M1
    /// transport, and drops the connection when done — the connection's
    /// `Drop` kills the child process. A failed handshake counts toward
    /// [`MAX_DESCRIBE_FAILURES`]; reaching it quarantines the plugin (warn),
    /// after which later scans skip it entirely.
    pub async fn scan_and_describe(&mut self) -> Vec<PluginRegistration> {
        let mut registrations = Vec::new();
        for plugin in PluginScan::scan(&self.plugins_dir) {
            if self.quarantined.contains(&plugin.name) {
                log::warn!("plugin {} is quarantined; skipping describe", plugin.name);
                continue;
            }
            match self.describe_one(&plugin).await {
                Ok(registration) => {
                    if let Ok(mut errors) = self.last_errors.lock() {
                        errors.remove(&plugin.name);
                    }
                    self.note_describe_success(&plugin.name);
                    self.register_plugin(registration.name.clone(), registration.manifest.clone());
                    self.add_registration(registration.clone());
                    if !registration.datasources.is_empty() {
                        self.register_datasources(
                            &registration.name,
                            registration.datasources.clone(),
                        );
                    }
                    registrations.push(registration);
                }
                Err(e) => {
                    log::warn!(
                        "describe failed for plugin {} v{}: {e}",
                        plugin.name,
                        plugin.version
                    );
                    if let Ok(mut errors) = self.last_errors.lock() {
                        errors.insert(plugin.name.clone(), e.to_string());
                    }
                    if self.note_describe_failure(&plugin.name) {
                        log::warn!(
                            "plugin {} quarantined after {MAX_DESCRIBE_FAILURES} describe failures",
                            plugin.name
                        );
                    }
                }
            }
        }
        registrations
    }

    /// Most recent describe failure reason for `name` (UI-facing; `None`
    /// when the plugin loaded or was never scanned).
    pub fn last_error(&self, name: &str) -> Option<String> {
        self.last_errors.lock().ok()?.get(name).cloned()
    }

    /// Names of plugins quarantined by the describe policy (sorted).
    pub fn quarantined(&self) -> Vec<String> {
        let mut names: Vec<String> = self.quarantined.iter().cloned().collect();
        names.sort();
        names
    }

    /// Records a describe failure for `name`, returning `true` when the
    /// plugin crossed [`MAX_DESCRIBE_FAILURES`] and became quarantined.
    ///
    /// Pure policy — testable without any process or connection.
    pub fn note_describe_failure(&mut self, name: &str) -> bool {
        let count = self.failures.entry(name.to_string()).or_insert(0);
        *count += 1;
        if *count >= MAX_DESCRIBE_FAILURES {
            self.failures.remove(name);
            self.quarantined.insert(name.to_string());
            true
        } else {
            false
        }
    }

    /// Resets the describe failure counter for `name` after a success.
    pub fn note_describe_success(&mut self, name: &str) {
        self.failures.remove(name);
    }

    /// One full handshake: spawn → connect → describe (bounded) → normalize.
    ///
    /// The describe await is wrapped in [`tokio::time::timeout`]: a plugin
    /// that accepts the pipes but never answers is killed (dropping the
    /// connection reaps the child) and reported as
    /// [`PluginError::Timeout`], which [`PluginManager::scan_and_describe`]
    /// counts toward the quarantine policy.
    async fn describe_one(
        &self,
        plugin: &DiscoveredPlugin,
    ) -> Result<PluginRegistration, PluginError> {
        let process = process::spawn_plugin(&plugin.manifest.entry, &plugin.manifest.env)?;
        let connection = PluginConnection::spawn(process, self.connection_config.clone())?;
        // The on-disk manifest is the fallback for array-shape (SDK) describe
        // responses that do not echo a manifest.
        let result: DescribeResult = match tokio::time::timeout(
            self.describe_timeout,
            perform_describe(&connection, Some(plugin.manifest.clone())),
        )
        .await
        {
            Ok(result) => result?,
            Err(_elapsed) => {
                log::warn!(
                    "describe timed out after {:?} for plugin {} v{}; killing",
                    self.describe_timeout,
                    plugin.name,
                    plugin.version
                );
                // Dropping the connection kills the child (last-handle Drop
                // reaps); no clone escaped this function, so the last handle
                // drops here.
                drop(connection);
                return Err(PluginError::Timeout(self.describe_timeout));
            }
        };
        // Dropping the connection kills the child (last-handle Drop reaps).
        drop(connection);
        // The plugin's echoed manifest may carry a `./`-relative
        // entry.command or entry.args (marketplace form); anchor them to the plugin
        // directory like the scan path does.
        let mut plugin_manifest = result.plugin;
        if plugin_manifest.entry.command.starts_with("./")
            || plugin_manifest.entry.command.starts_with(".\\")
        {
            plugin_manifest.entry.command = crate::discovery::resolve_entry_command(
                &plugin.dir,
                &plugin_manifest.entry.command,
            );
        }
        for arg in &mut plugin_manifest.entry.args {
            if arg.starts_with("./") || arg.starts_with(".\\") {
                *arg = crate::discovery::resolve_entry_command(&plugin.dir, arg);
            }
        }
        Ok(PluginRegistration {
            name: plugin.name.clone(),
            version: plugin.version.clone(),
            dir: plugin.dir.clone(),
            manifest: plugin_manifest,
            nodes: normalize_nodes(result.nodes, &plugin.name),
            datasources: result.datasources,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 坏插件（入口二进制不存在）→ `scan_and_describe` 失败且 `last_error`
    /// 记录可诊断原因（UI `list_installed_plugins` 的 error 字段据此填充）。
    #[tokio::test]
    async fn describe_failure_records_last_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("broken");
        fs::create_dir_all(&plugin_dir).expect("mkdir");
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "broken",
                "version": "0.1.0",
                "description": "broken",
                "entry": { "command": "./does_not_exist" },
                "capabilities": {}
            }"#,
        )
        .expect("write manifest");

        let mut manager = PluginManager::new(dir.path().to_path_buf());
        manager.describe_timeout = Duration::from_secs(3);
        let registrations = manager.scan_and_describe().await;

        assert!(registrations.is_empty(), "broken plugin must not register");
        let error = manager
            .last_error("broken")
            .expect("last_error must record the describe failure reason");
        assert!(
            !error.is_empty(),
            "error reason must be non-empty (UI-facing)"
        );
        assert!(
            manager.quarantined().is_empty(),
            "single failure is not quarantine"
        );
    }

    /// 测试真实 Python 数据源插件 describe 与注册
    #[tokio::test]
    async fn describe_python_datasource_plugin() {
        let eolink_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../plugins/eolink");
        if !eolink_dir.exists() {
            return;
        }
        let parent_dir = eolink_dir.parent().expect("parent dir");
        let mut manager = PluginManager::new(parent_dir.to_path_buf());
        manager.describe_timeout = Duration::from_secs(5);

        let registrations = manager.scan_and_describe().await;
        let eolink_reg = registrations.iter().find(|r| r.name == "eolink");
        let eolink_reg =
            eolink_reg.unwrap_or_else(|| panic!("eolink plugin must describe successfully"));
        assert_eq!(eolink_reg.datasources.len(), 1);
        assert_eq!(eolink_reg.datasources[0].id, "eolink");

        // 确认 manager 已注册 datasource
        let datasources = manager.list_datasources();
        assert!(datasources.iter().any(|d| d.id == "eolink"));
        assert_eq!(
            manager.datasource_index.get("eolink").map(String::as_str),
            Some("eolink")
        );
    }

    /// 测试通过 `manager.datasource_call` 调用 Python 插件的 browse
    #[tokio::test]
    async fn datasource_call_python_plugin_browse() {
        let eolink_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../plugins/eolink");
        if !eolink_dir.exists() {
            return;
        }
        let parent_dir = eolink_dir.parent().expect("parent dir");
        let mut manager = PluginManager::new(parent_dir.to_path_buf());
        manager.scan_and_describe().await;

        // 调用 validate_config（空 key 校验）
        let val_res = manager
            .datasource_call(
                "eolink",
                "validate_config",
                serde_json::json!({"config": {}}),
            )
            .await
            .expect("validate_config call");
        assert_eq!(val_res["valid"], false);
        assert!(val_res["message"].is_string());
    }
}
