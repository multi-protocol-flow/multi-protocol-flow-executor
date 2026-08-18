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

