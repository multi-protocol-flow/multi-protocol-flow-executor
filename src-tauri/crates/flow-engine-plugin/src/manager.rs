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
use std::sync::Arc;
use std::time::Duration;

use mpe_protocol::describe::{DescribeResult, NodeDescription};
use mpe_protocol::manifest::PluginManifest;

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
    /// Upper bound for one describe handshake; exceeded, the plugin process
    /// is killed and the failure counts toward the quarantine policy
    /// (defaults to [`DESCRIBE_TIMEOUT`]).
    describe_timeout: Duration,
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
            describe_timeout: DESCRIBE_TIMEOUT,
        }
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
                    self.note_describe_success(&plugin.name);
                    self.register_plugin(registration.name.clone(), registration.manifest.clone());
                    self.add_registration(registration.clone());
                    registrations.push(registration);
                }
                Err(e) => {
                    log::warn!(
                        "describe failed for plugin {} v{}: {e}",
                        plugin.name,
                        plugin.version
                    );
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
        Ok(PluginRegistration {
            name: plugin.name.clone(),
            version: plugin.version.clone(),
            dir: plugin.dir.clone(),
            manifest: result.plugin,
            nodes: normalize_nodes(result.nodes, &plugin.name),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// Create a unique scratch directory under the OS temp dir (no tempfile
    /// dev-dependency; mirrors discovery.rs test convention).
    fn unique_temp_dir(tag: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mpe-plugin-test-{}-{}-{tag}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create temp dir {dir:?}: {e}"));
        dir
    }

    /// Write a minimal valid `plugin.json` into `dir`.
    fn write_plugin(dir: &Path, name: &str, version: &str, command: &str) -> io::Result<()> {
        std::fs::write(
            dir.join("plugin.json"),
            format!(
                r#"{{"name": "{name}", "version": "{version}", "entry": {{"command": "{command}"}}}}"#
            ),
        )
    }

    /// Like [`write_plugin`], with an explicit `entry.args` list.
    fn write_plugin_with_args(
        dir: &Path,
        name: &str,
        version: &str,
        command: &str,
        args: &[String],
    ) -> io::Result<()> {
        let args_json =
            serde_json::to_string(args).unwrap_or_else(|e| panic!("serialize args: {e}"));
        std::fs::write(
            dir.join("plugin.json"),
            format!(
                r#"{{"name": "{name}", "version": "{version}", "entry": {{"command": "{command}", "args": {args_json}}}}}"#
            ),
        )
    }

    /// A command that accepts its stdio pipes but writes nothing and does
    /// not exit for a long time (platform-appropriate): the transport sees
    /// an open, silent pipe, so the describe request is never answered.
    fn hang_command() -> (String, Vec<String>) {
        #[cfg(windows)]
        {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "ping -n 60 127.0.0.1 > nul".to_string()],
            )
        }
        #[cfg(not(windows))]
        {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "sleep 60".to_string()],
            )
        }
    }

    #[test]
    fn describe_crash_quarantines_after_n() {
        let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
        assert!(!manager.note_describe_failure("boom"));
        assert!(!manager.note_describe_failure("boom"));
        assert!(manager.quarantined().is_empty());
        // Third failure crosses the threshold.
        assert!(manager.note_describe_failure("boom"));
        assert_eq!(manager.quarantined(), vec!["boom"]);
        // Already quarantined: further failures are no-ops.
        assert!(!manager.note_describe_failure("boom"));
        assert_eq!(manager.quarantined(), vec!["boom"]);
        // Independent plugins are tracked independently.
        let mut other = PluginManager::new(PathBuf::from("/nonexistent"));
        other.note_describe_failure("a");
        other.note_describe_failure("b");
        assert!(other.quarantined().is_empty());
    }

    #[test]
    fn describe_failure_counter_resets_on_success() {
        let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
        manager.note_describe_failure("demo");
        manager.note_describe_success("demo");
        // Two more failures after the reset stay below the threshold — the
        // reset removed the earlier count.
        manager.note_describe_failure("demo");
        manager.note_describe_failure("demo");
        assert!(manager.quarantined().is_empty());
        // The third failure after the reset finally quarantines.
        assert!(manager.note_describe_failure("demo"));
        assert_eq!(manager.quarantined(), vec!["demo"]);
    }

    /// Full-path quarantine: a plugin whose process cannot even be spawned
    /// fails describe through the real spawn path; after
    /// [`MAX_DESCRIBE_FAILURES`] scans it is quarantined and later scans skip
    /// it.
    #[tokio::test]
    async fn scan_and_describe_quarantines_spawn_failure(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let root = unique_temp_dir("scan-describe-spawn-fail");
        let plugin_dir = root.join("crashy");
        std::fs::create_dir_all(&plugin_dir)?;
        write_plugin(&plugin_dir, "crashy", "1.0.0", "no-such-command-xyz")?;

        let mut manager = PluginManager::new(root.clone());
        for _ in 0..MAX_DESCRIBE_FAILURES {
            let registrations = manager.scan_and_describe().await;
            assert!(registrations.is_empty());
        }
        assert_eq!(manager.quarantined(), vec!["crashy"]);

        // The next scan skips the quarantined plugin entirely.
        let registrations = manager.scan_and_describe().await;
        assert!(registrations.is_empty());
        assert_eq!(manager.quarantined(), vec!["crashy"]);

        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    /// Full-path timeout: a plugin that accepts the pipes but never answers
    /// `describe` must not block the scan — the handshake returns within the
    /// injected bound (300ms here, far below the 5s default) and counts as a
    /// describe failure, eventually quarantining the plugin.
    #[tokio::test]
    async fn describe_timeout_returns_in_bound_and_quarantines(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let root = unique_temp_dir("describe-timeout");
        let plugin_dir = root.join("hangy");
        std::fs::create_dir_all(&plugin_dir)?;
        let (command, args) = hang_command();
        write_plugin_with_args(&plugin_dir, "hangy", "1.0.0", &command, &args)?;

        let mut manager =
            PluginManager::new(root.clone()).with_describe_timeout(Duration::from_millis(300));

        // The hung plugin must not stall the scan: the first describe
        // returns within the injected bound (not the 30s transport timeout,
        // and certainly not forever).
        let start = std::time::Instant::now();
        let registrations = manager.scan_and_describe().await;
        let first_elapsed = start.elapsed();
        assert!(registrations.is_empty());
        assert!(
            first_elapsed < Duration::from_secs(5),
            "describe of a hung plugin should return within the 300ms bound, \
             took {first_elapsed:?}"
        );

        // The timeout counts toward the quarantine counter like any other
        // describe failure.
        assert_eq!(manager.failures.get("hangy"), Some(&1));

        // Repeated timeouts accumulate; after MAX_DESCRIBE_FAILURES the
        // plugin is quarantined and later scans skip it entirely.
        for _ in 1..MAX_DESCRIBE_FAILURES {
            assert!(manager.scan_and_describe().await.is_empty());
        }
        assert_eq!(manager.quarantined(), vec!["hangy"]);
        assert!(manager.scan_and_describe().await.is_empty());
        assert_eq!(manager.quarantined(), vec!["hangy"]);

        std::fs::remove_dir_all(&root)?;
        Ok(())
    }
}
