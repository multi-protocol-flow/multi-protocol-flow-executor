//! Error types for the plugin marketplace client and installer.

/// Errors surfaced by marketplace listing and plugin installation.
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    /// The marketplace returned a non-2xx HTTP status.
    #[error("marketplace HTTP error: status {status} for {url}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// The URL that returned the non-2xx status.
        url: String,
    },
    /// The HTTP request itself failed.
    #[error("marketplace request failed: {0}")]
    Request(#[from] reqwest::Error),
    /// The response body did not match the expected JSON shape.
    #[error("bad marketplace response: {0}")]
    BadResponse(String),
    /// No download asset exists for the current platform.
    #[error("no asset for the current platform; available: {}", .available.join(", "))]
    PlatformUnsupported {
        /// Sorted list of platform keys the plugin provides.
        available: Vec<String>,
    },
    /// A requested plugin does not exist in the marketplace listing.
    #[error("plugin not found: {0}")]
    NotFound(String),
    /// The plugin is already installed at the target directory.
    #[error("plugin already installed at {0}")]
    AlreadyInstalled(String),
    /// The downloaded archive's sha256 did not match the advertised digest.
    #[error("sha256 mismatch: archive does not match the advertised digest")]
    Sha256Mismatch,
    /// The archive could not be safely extracted.
    #[error("failed to extract plugin archive: {0}")]
    Extract(String),
    /// The installed `plugin.json` failed manifest validation.
    #[error("invalid plugin manifest: {0}")]
    InvalidManifest(String),
    /// I/O failure while installing.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
