//! Marketplace HTTP client.

use crate::error::MarketplaceError;
use crate::model::MarketplacePlugin;

/// Default timeout for HTTP requests.
const DEFAULT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// Default timeout for establishing TCP/TLS connections.
const DEFAULT_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// Standard User-Agent header to prevent CDN/WAF blocking.
const DEFAULT_USER_AGENT: &str = concat!("mpe-marketplace/", env!("CARGO_PKG_VERSION"));

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
        let client = reqwest::Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .timeout(DEFAULT_REQUEST_TIMEOUT)
            .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Resolve the primary plugins endpoint URL from `base_url`.
    /// If `base_url` already points directly to `/plugins` or `/plugins.json`,
    /// it is used as-is instead of duplicating `/plugins`.
    fn primary_plugins_url(&self) -> String {
        if self.base_url.ends_with("/plugins") || self.base_url.ends_with("/plugins.json") {
            self.base_url.clone()
        } else {
            format!("{}/plugins", self.base_url)
        }
    }

    /// Resolve the fallback plugins endpoint URL if applicable.
    fn fallback_plugins_url(&self) -> Option<String> {
        let primary = self.primary_plugins_url();
        if primary.ends_with("/plugins") {
            Some(format!("{}.json", primary))
        } else {
            None
        }
    }

    /// Fetch the plugin listing (`GET {base}/plugins`).
    ///
    /// The server responds `{"plugins": [MarketplacePlugin, ...]}`. Any
    /// non-2xx status is an [`MarketplaceError::Http`]; a body that is not
    /// valid JSON or lacks the `plugins` field is a
    /// [`MarketplaceError::BadResponse`].
    pub async fn list_plugins(&self) -> Result<Vec<MarketplacePlugin>, MarketplaceError> {
        let primary_url = self.primary_plugins_url();
        match self.fetch_plugins_from_url(&primary_url).await {
            Ok(plugins) => Ok(plugins),
            Err(MarketplaceError::Http { status: 404, .. }) => {
                if let Some(fallback_url) = self.fallback_plugins_url() {
                    log::debug!("Primary plugin url 404, trying fallback {fallback_url}");
                    self.fetch_plugins_from_url(&fallback_url).await
                } else {
                    Err(MarketplaceError::Http {
                        status: 404,
                        url: primary_url,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn fetch_plugins_from_url(
        &self,
        url: &str,
    ) -> Result<Vec<MarketplacePlugin>, MarketplaceError> {
        // Try with 1 retry on transient network errors or 5xx server errors
        let mut last_err = None;
        for attempt in 0..2 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            let response = match self
                .client
                .get(url)
                .header("Accept", "application/json, text/plain, */*")
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    last_err = Some(MarketplaceError::Request(err));
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let status_code = status.as_u16();
                if attempt == 0 {
                    log::debug!(
                        "marketplace GET {url} -> {status_code}; headers: {:?}",
                        response.headers()
                    );
                }
                let err = MarketplaceError::Http {
                    status: status_code,
                    url: url.to_string(),
                };
                if status.is_server_error() && attempt == 0 {
                    last_err = Some(err);
                    continue;
                }
                return Err(err);
            }

            let body = response.bytes().await?;
            let payload: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|e| MarketplaceError::BadResponse(e.to_string()))?;
            let plugins = payload.get("plugins").ok_or_else(|| {
                MarketplaceError::BadResponse("response is missing the `plugins` field".into())
            })?;
            let plugins: Vec<MarketplacePlugin> = serde_json::from_value(plugins.clone())
                .map_err(|e| MarketplaceError::BadResponse(e.to_string()))?;
            return Ok(plugins);
        }

        Err(last_err
            .unwrap_or_else(|| MarketplaceError::BadResponse("unknown request failure".into())))
    }

    /// Download `url` into memory, failing on any non-2xx status.
    pub(crate) async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, MarketplaceError> {
        let bytes = self
            .client
            .get(url)
            .header("Accept", "*/*")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static CRYPTO_INIT: Once = Once::new();
    fn ensure_crypto_provider() {
        // rustls-no-provider: 构建 reqwest Client 前必须安装 CryptoProvider
        CRYPTO_INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[test]
    fn test_url_normalization() {
        ensure_crypto_provider();
        let c1 = MarketplaceClient::new("https://example.com/registry".into());
        assert_eq!(
            c1.primary_plugins_url(),
            "https://example.com/registry/plugins"
        );
        assert_eq!(
            c1.fallback_plugins_url(),
            Some("https://example.com/registry/plugins.json".into())
        );

        let c2 = MarketplaceClient::new("https://example.com/registry/".into());
        assert_eq!(
            c2.primary_plugins_url(),
            "https://example.com/registry/plugins"
        );

        let c3 = MarketplaceClient::new("https://example.com/registry/plugins".into());
        assert_eq!(
            c3.primary_plugins_url(),
            "https://example.com/registry/plugins"
        );

        let c4 = MarketplaceClient::new("https://example.com/registry/plugins.json".into());
        assert_eq!(
            c4.primary_plugins_url(),
            "https://example.com/registry/plugins.json"
        );
        assert_eq!(c4.fallback_plugins_url(), None);
    }
}
