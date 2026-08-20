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

use mpe_plugin_sdk::protocol::manifest::PluginManifest;

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

/// Resolve a marketplace-contract `entry.command` (e.g. `./mongo`) against
/// the plugin directory.
///
/// The process layer spawns with `Command::new(&entry.command)` and does not
/// chdir, so a `./`-prefixed command would resolve against the host's CWD.
/// Bare names (`python3`, `demo`) keep PATH lookup semantics; absolute paths
/// pass through untouched — only the explicit `./` / `.\` marketplace form
/// is anchored to the plugin directory.
pub(crate) fn resolve_entry_command(dir: &Path, command: &str) -> String {
    let stripped = command
        .strip_prefix("./")
        .or_else(|| command.strip_prefix(".\\"))
        .unwrap_or(command);
    dir.join(stripped).to_string_lossy().into_owned()
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
    let mut manifest = match PluginManifest::from_str(&text) {
        Ok(manifest) => manifest,
        Err(e) => {
            log::warn!("invalid plugin.json in {}: {e}; skipping", dir.display());
            return ScanEntry::Skipped {
                dir: dir.to_path_buf(),
                reason: ScanSkipReason::InvalidManifest(e),
            };
        }
    };
    // Marketplace-contract manifests carry a `./`-relative entry.command or
    // `./`-relative entry.args (e.g. `{"command": "python3", "args": ["./eolink.py"]}`);
    // anchor them to the plugin directory so the spawn layer (no chdir) can
    // launch them regardless of the host's CWD.
    if manifest.entry.command.starts_with("./") || manifest.entry.command.starts_with(".\\") {
        manifest.entry.command = resolve_entry_command(dir, &manifest.entry.command);
    }
    for arg in &mut manifest.entry.args {
        if arg.starts_with("./") || arg.starts_with(".\\") {
            *arg = resolve_entry_command(dir, arg);
        }
    }
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
    use super::*;

    #[test]
    fn scan_anchors_relative_command_and_args() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("python_plugin");
        fs::create_dir_all(&plugin_dir).expect("mkdir");
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "python_plugin",
                "version": "0.1.0",
                "entry": {
                    "command": "python3",
                    "args": ["./script.py", "--flag"]
                }
            }"#,
        )
        .expect("write");

        let entry = scan_plugin_dir(&plugin_dir, "0.1.0");
        match entry {
            ScanEntry::Discovered(plugin) => {
                assert_eq!(plugin.manifest.entry.command, "python3");
                let expected_script = plugin_dir.join("script.py").to_string_lossy().into_owned();
                assert_eq!(plugin.manifest.entry.args[0], expected_script);
                assert_eq!(plugin.manifest.entry.args[1], "--flag");
            }
            other => panic!("expected discovered, got {other:?}"),
        }
    }
}

