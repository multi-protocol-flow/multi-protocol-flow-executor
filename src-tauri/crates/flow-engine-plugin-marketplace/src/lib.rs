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

#[cfg(test)]
pub(crate) mod test_support {
    /// Install the rustls ring crypto provider exactly once (reqwest is
    /// unified to `rustls-no-provider` by workspace-hack, so a provider must
    /// be installed before any `Client` is built in tests).
    pub(crate) fn install_crypto_provider() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }
}
