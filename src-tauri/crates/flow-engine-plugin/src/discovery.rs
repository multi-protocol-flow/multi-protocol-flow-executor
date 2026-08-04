//! Plugin directory scanning and discovery.
//!
//! Spawn-free, synchronous scan of the plugins directory — one plugin per
//! subdirectory. A subdirectory without a `plugin.json`, with an invalid
//! manifest, or whose `min_host_version` the host does not satisfy is
//! skipped with a warning; a missing plugins directory is an empty result,
//! never an error. Non-directory entries are ignored; symlinks are followed
//! per platform default. Scanning never spawns processes and never crashes
//! the host.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mpe_protocol::manifest::PluginManifest;

use crate::manifest::{ManifestError, ManifestParse};
use crate::semver;

/// Env var overriding the host version used for `min_host_version` gating
/// (useful in tests and multi-version deployments).
pub const HOST_VERSION_ENV: &str = "MPE_HOST_VERSION";

/// A plugin found by a scan, ready for the describe handshake (M2a').
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    /// Plugin name from its manifest.
    pub name: String,
    /// Plugin version from its manifest.
    pub version: String,
    /// The plugin's install directory (contains `plugin.json`).
    pub dir: PathBuf,
    /// The parsed and validated manifest.
    pub manifest: PluginManifest,
}

/// Why a plugin directory was skipped by a scan.
#[derive(Debug)]
pub enum ScanSkipReason {
    /// The entry is not a directory.
    NotADirectory,
    /// The directory has no `plugin.json`.
    MissingManifest,
    /// The `plugin.json` could not be read or failed validation.
    InvalidManifest(ManifestError),
    /// The plugin's `min_host_version` exceeds the host version.
    IncompatibleHostVersion {
        /// Host version used for gating.
        host: String,
        /// The plugin's required minimum host version.
        required: String,
    },
}

/// Outcome of scanning one plugin directory.
#[derive(Debug)]
pub enum ScanEntry {
    /// A plugin directory that passed validation and gating.
    Discovered(Box<DiscoveredPlugin>),
    /// A plugin directory that was skipped, with the reason.
    Skipped {
        /// The skipped directory.
        dir: PathBuf,
        /// Why it was skipped.
        reason: ScanSkipReason,
    },
}

/// Spawn-free, synchronous plugins-directory scanner.
pub struct PluginScan;

impl PluginScan {
    /// Scan `plugins_dir`, returning every plugin that passes validation and
    /// the `min_host_version` gate.
    ///
    /// The host version is taken from [`HOST_VERSION_ENV`] when set, falling
    /// back to this crate's own package version. A missing or unreadable
    /// directory yields an empty result, not an error.
    pub fn scan(plugins_dir: &Path) -> Vec<DiscoveredPlugin> {
        Self::scan_with_errors(plugins_dir, &host_version())
            .into_iter()
            .filter_map(|entry| match entry {
                ScanEntry::Discovered(plugin) => Some(*plugin),
                ScanEntry::Skipped { .. } => None,
            })
            .collect()
    }

    /// Scan `plugins_dir`, reporting the outcome of every directory entry
    /// (including skip reasons) while logging a warning per skip.
    ///
    /// A missing or unreadable directory yields an empty vector, not an
    /// error.
    pub fn scan_with_errors(plugins_dir: &Path, host_version: &str) -> Vec<ScanEntry> {
        let entries = match fs::read_dir(plugins_dir) {
            Ok(entries) => entries,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    log::debug!("plugins dir {} missing; skip scan", plugins_dir.display());
                } else {
                    log::warn!("plugins dir {} unreadable: {e}", plugins_dir.display());
                }
                return Vec::new();
            }
        };
        let mut outcomes = Vec::new();
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(e) => {
                    log::warn!("bad entry in {}: {e}", plugins_dir.display());
                    continue;
                }
            };
            if !path.is_dir() {
                // `is_dir` follows symlinks per platform default; broken
                // symlinks and files fall through here and are ignored.
                log::debug!("skipping non-directory entry {}", path.display());
                outcomes.push(ScanEntry::Skipped {
                    dir: path,
                    reason: ScanSkipReason::NotADirectory,
                });
                continue;
            }
            outcomes.push(scan_plugin_dir(&path, host_version));
        }
        outcomes
    }
}

/// The host version used for gating: [`HOST_VERSION_ENV`] override, else the
/// crate's own package version.
fn host_version() -> String {
    std::env::var(HOST_VERSION_ENV).unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
}

/// Scan one plugin directory: parse `plugin.json`, gate on `min_host_version`.
fn scan_plugin_dir(dir: &Path, host_version: &str) -> ScanEntry {
    let manifest_path = dir.join("plugin.json");
    let text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            log::warn!("plugin dir {} has no plugin.json; skipping", dir.display());
            return ScanEntry::Skipped {
                dir: dir.to_path_buf(),
                reason: ScanSkipReason::MissingManifest,
            };
        }
        Err(e) => {
            log::warn!("cannot read {}: {e}; skipping", manifest_path.display());
            return ScanEntry::Skipped {
                dir: dir.to_path_buf(),
                reason: ScanSkipReason::InvalidManifest(ManifestError::Io(e)),
            };
        }
    };
    let manifest = match PluginManifest::from_str(&text) {
        Ok(manifest) => manifest,
        Err(e) => {
            log::warn!("invalid plugin.json in {}: {e}; skipping", dir.display());
            return ScanEntry::Skipped {
                dir: dir.to_path_buf(),
                reason: ScanSkipReason::InvalidManifest(e),
            };
        }
    };
    if let Some(required) = &manifest.min_host_version {
        if !semver::host_satisfies(host_version, required) {
            log::warn!(
                "plugin {} needs host >= {required}, have {host_version}",
                manifest.name
            );
            return ScanEntry::Skipped {
                dir: dir.to_path_buf(),
                reason: ScanSkipReason::IncompatibleHostVersion {
                    host: host_version.to_string(),
                    required: required.clone(),
                },
            };
        }
    }
    ScanEntry::Discovered(Box::new(DiscoveredPlugin {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        dir: dir.to_path_buf(),
        manifest,
    }))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// Create a unique scratch directory under the OS temp dir (no tempfile
    /// dev-dependency: the parallel M1 worker owns Cargo.toml edits).
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
    fn write_plugin(
        dir: &Path,
        name: &str,
        version: &str,
        min_host: Option<&str>,
    ) -> io::Result<()> {
        let json = format!(
            r#"{{"name": "{name}", "version": "{version}", {min}"entry": {{"command": "demo"}}}}"#,
            min = min_host
                .map(|v| format!("\"min_host_version\": \"{v}\","))
                .unwrap_or_default(),
        );
        std::fs::write(dir.join("plugin.json"), json)
    }

    #[test]
    fn scan_finds_plugin_dir() -> Result<(), Box<dyn std::error::Error>> {
        let root = unique_temp_dir("scan-finds");
        let plugin_dir = root.join("demo");
        std::fs::create_dir_all(&plugin_dir)?;
        write_plugin(&plugin_dir, "demo", "1.0.0", None)?;

        let found = PluginScan::scan(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "demo");
        assert_eq!(found[0].version, "1.0.0");
        Ok(())
    }

    /// Summarize scan outcomes as sorted `(dir name, outcome kind)` pairs.
    fn summarize(entries: &[ScanEntry]) -> Vec<(&str, &str)> {
        let mut out = entries
            .iter()
            .map(|e| match e {
                ScanEntry::Discovered(p) => (
                    p.dir.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    "discovered",
                ),
                ScanEntry::Skipped { dir, reason } => (
                    dir.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    match reason {
                        ScanSkipReason::NotADirectory => "not-a-dir",
                        ScanSkipReason::MissingManifest => "missing",
                        ScanSkipReason::InvalidManifest(_) => "invalid",
                        ScanSkipReason::IncompatibleHostVersion { .. } => "incompatible",
                    },
                ),
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    #[test]
    fn scan_ignores_invalid_dir() -> Result<(), Box<dyn std::error::Error>> {
        let root = unique_temp_dir("scan-invalid");
        let good = root.join("good");
        let bad_json = root.join("bad-json");
        let no_manifest = root.join("no-manifest");
        for dir in [&good, &bad_json, &no_manifest] {
            std::fs::create_dir_all(dir)?;
        }
        write_plugin(&good, "good", "1.0.0", None)?;
        std::fs::write(bad_json.join("plugin.json"), "{ not json")?;
        // stray file at the plugins-dir root is not a plugin dir
        std::fs::write(root.join("stray.txt"), "hi")?;
        let entries = PluginScan::scan_with_errors(&root, "0.1.0");
        let reasons = summarize(&entries);
        assert_eq!(
            reasons,
            vec![
                ("bad-json", "invalid"),
                ("good", "discovered"),
                ("no-manifest", "missing"),
                ("stray.txt", "not-a-dir")
            ]
        );
        Ok(())
    }

    #[test]
    fn scan_skips_semver_incompatible() -> Result<(), Box<dyn std::error::Error>> {
        let root = unique_temp_dir("scan-semver");
        let too_new = root.join("too-new");
        let old_ok = root.join("old-ok");
        let no_gate = root.join("no-gate");
        for dir in [&too_new, &old_ok, &no_gate] {
            std::fs::create_dir_all(dir)?;
        }
        write_plugin(&too_new, "too-new", "1.0.0", Some("0.2.0"))?;
        write_plugin(&old_ok, "old-ok", "1.0.0", Some("0.1.0"))?;
        write_plugin(&no_gate, "no-gate", "1.0.0", None)?;

        let entries = PluginScan::scan_with_errors(&root, "0.1.0");
        let reasons = summarize(&entries);
        assert_eq!(
            reasons,
            vec![
                ("no-gate", "discovered"),
                ("old-ok", "discovered"),
                ("too-new", "incompatible")
            ]
        );
        Ok(())
    }

    #[test]
    fn scan_missing_dir_empty() {
        let root = unique_temp_dir("scan-missing").join("does-not-exist");
        assert!(PluginScan::scan(&root).is_empty());
        assert!(PluginScan::scan_with_errors(&root, "0.1.0").is_empty());
    }
}
