//! Plugin manifest (`plugin.json`) parsing and validation.
//!
//! Wraps `mpe_plugin_sdk::protocol::manifest::PluginManifest` with host-side validation
//! rules: `name` / `version` / `entry` must be present (serde-enforced), the
//! `entry.command` must be non-empty, and an optional `min_host_version` must
//! parse as semver. Version gating itself lives in [`crate::semver`].
//!
//! The parse helpers live on an extension trait ([`ManifestParse`]) rather
//! than inherent methods: `PluginManifest` is defined in `mpe-plugin-sdk::protocol`, and
//! Rust forbids inherent impls on foreign types (E0116). The trait keeps the
//! intended `PluginManifest::from_str(...)` / `PluginManifest::from_path(...)`
//! call syntax with only a trait import.

use std::fs;
use std::path::Path;

use mpe_plugin_sdk::protocol::manifest::PluginManifest;

use crate::semver;

/// Error while reading or validating a plugin manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The manifest file could not be read.
    #[error("failed to read plugin.json: {0}")]
    Io(#[from] std::io::Error),
    /// The manifest is not valid JSON, or misses a required field
    /// (`name` / `version` / `entry`).
    #[error("invalid plugin.json: {0}")]
    Json(#[from] serde_json::Error),
    /// `entry.command` was empty or whitespace-only.
    #[error("plugin.json `entry.command` must be a non-empty string")]
    EmptyCommand,
    /// `min_host_version` is not parseable semver.
    #[error("plugin.json `min_host_version` `{value}` is not valid semver: {source}")]
    InvalidMinHostVersion {
        /// The offending `min_host_version` value.
        value: String,
        /// Why it failed to parse.
        source: semver::SemverError,
    },
}

/// Host-side parse helpers for [`PluginManifest`].
///
/// Implemented as an extension trait because inherent methods on
/// `PluginManifest` are impossible (foreign type, E0116) and so is a real
/// `std::str::FromStr` impl (orphan rule: both trait and type are foreign).
pub trait ManifestParse: Sized {
    /// Parse and validate a manifest from raw JSON text.
    ///
    /// Rejects JSON missing `name` / `version` / `entry` (serde-enforced),
    /// an empty `entry.command`, or an unparseable `min_host_version`.
    #[allow(clippy::should_implement_trait)] // std::str::FromStr cannot be implemented here (orphan rule)
    fn from_str(json: &str) -> Result<Self, ManifestError>;

    /// Read the manifest at `path` and parse/validate it.
    fn from_path(path: &Path) -> Result<Self, ManifestError> {
        let json = fs::read_to_string(path)?;
        Self::from_str(&json)
    }
}

impl ManifestParse for PluginManifest {
    fn from_str(json: &str) -> Result<Self, ManifestError> {
        let manifest: PluginManifest = serde_json::from_str(json)?;
        validate(&manifest)?;
        Ok(manifest)
    }
}

/// Host-side validation rules beyond the serde schema.
fn validate(manifest: &PluginManifest) -> Result<(), ManifestError> {
    if manifest.entry.command.trim().is_empty() {
        return Err(ManifestError::EmptyCommand);
    }
    if let Some(min) = &manifest.min_host_version {
        semver::parse(min).map_err(|source| ManifestError::InvalidMinHostVersion {
            value: min.clone(),
            source,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
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

    #[test]
    fn manifest_parse_command_args() -> Result<(), ManifestError> {
        let json = r#"{
            "name": "demo",
            "version": "1.2.3",
            "entry": {"command": "python", "args": ["plugin.py", "--debug"]}
        }"#;
        let manifest = PluginManifest::from_str(json)?;
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.entry.command, "python");
        assert_eq!(manifest.entry.args, vec!["plugin.py", "--debug"]);
        // Optional fields default to absent/empty.
        assert_eq!(manifest.author, None);
        assert_eq!(manifest.min_host_version, None);
        assert!(manifest.env.is_empty());
        assert!(manifest.permissions.is_empty());
        Ok(())
    }

    #[test]
    fn manifest_rejects_missing_entry() {
        // entry missing
        let json = r#"{"name": "demo", "version": "1.2.3"}"#;
        assert!(PluginManifest::from_str(json).is_err());
        // name missing
        let json = r#"{"version": "1.2.3", "entry": {"command": "demo"}}"#;
        assert!(PluginManifest::from_str(json).is_err());
        // version missing
        let json = r#"{"name": "demo", "entry": {"command": "demo"}}"#;
        assert!(PluginManifest::from_str(json).is_err());
    }

    #[test]
    fn manifest_rejects_empty_command() {
        let json = r#"{"name": "demo", "version": "1.2.3", "entry": {"command": ""}}"#;
        assert!(matches!(
            PluginManifest::from_str(json),
            Err(ManifestError::EmptyCommand)
        ));
        // whitespace-only command is also rejected
        let json = r#"{"name": "demo", "version": "1.2.3", "entry": {"command": "   "}}"#;
        assert!(matches!(
            PluginManifest::from_str(json),
            Err(ManifestError::EmptyCommand)
        ));
    }

    #[test]
    fn manifest_rejects_invalid_min_host_version() {
        let json = r#"{
            "name": "demo",
            "version": "1.2.3",
            "entry": {"command": "demo"},
            "min_host_version": "banana"
        }"#;
        assert!(matches!(
            PluginManifest::from_str(json),
            Err(ManifestError::InvalidMinHostVersion { .. })
        ));
        // parseable min_host_version is accepted (gating is semver.rs's job)
        let json = r#"{
            "name": "demo",
            "version": "1.2.3",
            "entry": {"command": "demo"},
            "min_host_version": "0.2.0"
        }"#;
        assert_eq!(
            PluginManifest::from_str(json)
                .unwrap_or_else(|e| panic!("valid manifest should parse: {e}"))
                .min_host_version
                .as_deref(),
            Some("0.2.0")
        );
    }

    #[test]
    fn manifest_round_trip_via_path() -> Result<(), Box<dyn std::error::Error>> {
        let dir = unique_temp_dir("manifest-round-trip");
        let manifest_path = dir.join("plugin.json");
        let json = r#"{
            "name": "demo",
            "version": "0.1.0",
            "author": "Acme",
            "min_host_version": "0.1.0",
            "description": "A demo plugin",
            "entry": {"command": "python", "args": ["plugin.py"]},
            "env": {"FOO": "bar"},
            "permissions": ["http.request"],
            "capabilities": {"streaming": true}
        }"#;
        std::fs::write(&manifest_path, json)?;
        let manifest = PluginManifest::from_path(&manifest_path)?;
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.author.as_deref(), Some("Acme"));
        assert_eq!(manifest.min_host_version.as_deref(), Some("0.1.0"));
        assert_eq!(manifest.entry.command, "python");
        assert_eq!(manifest.entry.args, vec!["plugin.py"]);
        assert_eq!(manifest.env.get("FOO").map(String::as_str), Some("bar"));
        assert_eq!(manifest.permissions, vec!["http.request"]);
        assert!(manifest.capabilities.streaming);
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
