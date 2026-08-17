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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Spawn a one-shot HTTP server on a random port serving the given
    /// `(path, status, body)` responses in request order; returns the base
    /// URL and a join handle.
    fn serve(responses: Vec<(String, u16, String)>) -> (String, thread::JoinHandle<()>) {
        let listener =
            TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind failed: {e}"));
        let addr = listener.local_addr().unwrap_or_else(|e| panic!("addr failed: {e}"));
        let handle = thread::spawn(move || {
            for (_path, status, body) in responses {
                let (mut stream, _) = listener
                    .accept()
                    .unwrap_or_else(|e| panic!("accept failed: {e}"));
                let mut buf = [0u8; 4096];
                let _ = stream
                    .read(&mut buf)
                    .unwrap_or_else(|e| panic!("read failed: {e}"));
                let reason = if status == 200 { "OK" } else { "ERROR" };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(header.as_bytes())
                    .unwrap_or_else(|e| panic!("write failed: {e}"));
                stream
                    .write_all(body.as_bytes())
                    .unwrap_or_else(|e| panic!("write failed: {e}"));
            }
        });
        (format!("http://{addr}"), handle)
    }

    fn join_fixture(handle: thread::JoinHandle<()>) {
        handle.join().unwrap_or_else(|_| panic!("fixture server panicked"));
    }

    #[tokio::test]
    async fn list_plugins_parses_listing() {
        crate::test_support::install_crypto_provider();
        let body = r#"{"plugins":[{"name":"mongo","version":"1.0.0","platforms":{"linux-x64":{"url":"http://example.com/mongo.zip","sha256":"abc"}}}]}"#;
        let (base, handle) = serve(vec![("/plugins".into(), 200, body.into())]);
        let client = MarketplaceClient::new(base);
        let plugins = client
            .list_plugins()
            .await
            .unwrap_or_else(|e| panic!("list should succeed: {e}"));
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "mongo");
        assert_eq!(plugins[0].version, "1.0.0");
        assert!(plugins[0].description.is_none());
        let asset = plugins[0]
            .platforms
            .get("linux-x64")
            .unwrap_or_else(|| panic!("linux-x64 asset present"));
        assert_eq!(asset.sha256.as_deref(), Some("abc"));
        join_fixture(handle);
    }

    #[tokio::test]
    async fn list_plugins_reports_http_error() {
        crate::test_support::install_crypto_provider();
        let (base, handle) = serve(vec![("/plugins".into(), 500, "boom".into())]);
        let client = MarketplaceClient::new(base);
        let err = match client.list_plugins().await {
            Err(e) => e,
            Ok(_) => panic!("list should fail"),
        };
        match err {
            MarketplaceError::Http { status, url } => {
                assert_eq!(status, 500);
                assert!(url.contains("/plugins"), "url should be the request URL: {url}");
            }
            other => panic!("expected Http error, got {other:?}"),
        }
        join_fixture(handle);
    }

    #[tokio::test]
    async fn list_plugins_reports_bad_response() {
        crate::test_support::install_crypto_provider();
        let (base, handle) = serve(vec![("/plugins".into(), 200, "not json".into())]);
        let client = MarketplaceClient::new(base);
        let err = match client.list_plugins().await {
            Err(e) => e,
            Ok(_) => panic!("list should fail"),
        };
        assert!(matches!(err, MarketplaceError::BadResponse(_)), "got {err:?}");
        join_fixture(handle);
    }

    #[tokio::test]
    async fn base_url_trailing_slash_is_stripped() {
        crate::test_support::install_crypto_provider();
        let body = r#"{"plugins":[]}"#;
        let (base, handle) = serve(vec![("/plugins".into(), 200, body.into())]);
        let client = MarketplaceClient::new(format!("{base}/"));
        let plugins = client
            .list_plugins()
            .await
            .unwrap_or_else(|e| panic!("list should succeed: {e}"));
        assert!(plugins.is_empty());
        join_fixture(handle);
    }
}
