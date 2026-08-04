//! `PluginManager`: scan + describe handshake + describe-crash quarantine.
//!
//! Owns the plugins directory, the per-plugin describe connections, and the
//! quarantine policy: a plugin whose describe handshake fails
//! [`MAX_DESCRIBE_FAILURES`] times (spawn failure, RPC timeout, connection
//! crash, or error response) is disabled for the rest of this manager's
//! lifetime and skipped with a warning on every later scan. A successful
//! describe resets the failure counter.
//!
//! Process lifecycle (on-demand start, idle recycle, restart policy,
//! shutdown) is M2b's scope and deliberately absent here so that milestone
//! can extend this struct without rewriting it.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use mpe_protocol::describe::{DescribeResult, NodeDescription};
use mpe_protocol::manifest::PluginManifest;

use crate::describe::{normalize_nodes, perform_describe};
use crate::discovery::{DiscoveredPlugin, PluginScan};
use crate::error::PluginError;
use crate::process;
use crate::transport::{ConnectionConfig, PluginConnection};

/// Describe attempts before a plugin is quarantined for this manager's
/// lifetime.
pub const MAX_DESCRIBE_FAILURES: u32 = 3;

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

/// `PluginManager`: plugins-directory scan, describe handshake, quarantine.
pub struct PluginManager {
    plugins_dir: PathBuf,
    connection_config: ConnectionConfig,
    /// Describe-failure counts per plugin name (reset on success).
    failures: HashMap<String, u32>,
    /// Plugin names disabled for this manager's lifetime.
    quarantined: HashSet<String>,
}

impl PluginManager {
    /// Creates a manager that scans `plugins_dir` with default connection
    /// settings. Lifecycle configuration lands in M2b.
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            connection_config: ConnectionConfig::default(),
            failures: HashMap::new(),
            quarantined: HashSet::new(),
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

    /// One full handshake: spawn → connect → describe → normalize.
    async fn describe_one(
        &self,
        plugin: &DiscoveredPlugin,
    ) -> Result<PluginRegistration, PluginError> {
        let process = process::spawn_plugin(&plugin.manifest.entry, &plugin.manifest.env)?;
        let connection = PluginConnection::spawn(process, self.connection_config.clone())?;
        let result: DescribeResult = perform_describe(&connection).await?;
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
}
