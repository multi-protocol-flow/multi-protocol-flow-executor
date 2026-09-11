//! Plugin marketplace client + installer.
//!
//! Shared by the GUI (plugin settings dialog market tab) and the CLI
//! (`mpe plugin install`): lists plugins from a marketplace HTTP API,
//! downloads platform assets, verifies their sha256 digest, and extracts
//! them into a plugins directory with path-traversal protection.
//!
//! The marketplace HTTP contract is defined in `docs/plugin-marketplace.md`;
//! this crate's [`model`] types are the alignment baseline for a real
//! marketplace backend.

pub mod client;
pub mod error;
pub mod installer;
pub mod model;
pub mod platform;

pub use client::MarketplaceClient;
pub use error::MarketplaceError;
pub use installer::{install_plugin, InstalledPluginInfo};
pub use model::{MarketplacePlugin, PlatformAsset};
pub use platform::current_platform;
