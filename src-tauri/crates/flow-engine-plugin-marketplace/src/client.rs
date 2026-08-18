//! Marketplace HTTP client.

use crate::error::MarketplaceError;
use crate::model::MarketplacePlugin;

/// Thin client for the plugin marketplace HTTP API (`docs/plugin-marketplace.md`).
pub struct MarketplaceClient {
    base_url: String,
    /// Shared HTTP client (one connection pool for listing and downloads).
    client: reqwest::Client,
}

impl MarketplaceClient {
    /// Create a client for `base_url`. A trailing `/` is stripped so
    /// `new("http://host/")` and `new("http://host")` behave identically.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Fetch the plugin listing (`GET {base}/plugins`).
    ///
    /// The server responds `{"plugins": [MarketplacePlugin, ...]}`. Any
    /// non-2xx status is an [`MarketplaceError::Http`]; a body that is not
    /// valid JSON or lacks the `plugins` field is a
    /// [`MarketplaceError::BadResponse`].
    pub async fn list_plugins(&self) -> Result<Vec<MarketplacePlugin>, MarketplaceError> {
        let url = format!("{}/plugins", self.base_url);
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(MarketplaceError::Http {
                status: status.as_u16(),
                url,
            });
        }
        let body = response.bytes().await?;
        let payload: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| MarketplaceError::BadResponse(e.to_string()))?;
        let plugins = payload.get("plugins").ok_or_else(|| {
            MarketplaceError::BadResponse("response is missing the `plugins` field".into())
        })?;
        let plugins: Vec<MarketplacePlugin> = serde_json::from_value(plugins.clone())
            .map_err(|e| MarketplaceError::BadResponse(e.to_string()))?;
        Ok(plugins)
    }

    /// Download `url` into memory, failing on any non-2xx status.
    pub(crate) async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, MarketplaceError> {
        let bytes = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }
}

