//! Marketplace wire types (`GET {base}/plugins` response).
//!
//! Field names are camelCase to align with the frontend TypeScript types
//! (`src/types/plugins.ts`). The server contract is documented in
//! `docs/plugin-marketplace.md`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A plugin listing entry from the marketplace.
///
/// The server returns one entry per (name, version) pair, ordered by version
/// descending for equal names; the client takes the first matching entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplacePlugin {
    /// Unique plugin name (also the install-directory name).
    pub name: String,
    /// Plugin version (semver).
    pub version: String,
    /// One-line description.
    #[serde(default)]
    pub description: Option<String>,
    /// Author name.
    #[serde(default)]
    pub author: Option<String>,
    /// Minimum host version required (semver; `None` = no gating).
    #[serde(default)]
    pub min_host_version: Option<String>,
    /// Icon identifier or URL for the marketplace UI.
    #[serde(default)]
    pub icon: Option<String>,
    /// Category tags.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Last update timestamp (free-form; display only).
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Per-platform download assets, keyed by platform key
    /// (`windows-x64|windows-arm64|linux-x64|linux-arm64|macos-x64|macos-arm64`).
    pub platforms: HashMap<String, PlatformAsset>,
}

/// A downloadable asset for one platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAsset {
    /// Absolute URL of the plugin zip archive.
    pub url: String,
    /// Optional sha256 digest of the archive bytes (hex). When present the
    /// installer verifies the download against it.
    #[serde(default)]
    pub sha256: Option<String>,
    /// Optional archive size in bytes (display only).
    #[serde(default)]
    pub size: Option<u64>,
}
