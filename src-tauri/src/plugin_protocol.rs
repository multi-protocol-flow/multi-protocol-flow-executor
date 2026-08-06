//! Custom `mpe-plugin://` URI-scheme protocol (PR-1).
//!
//! Serves plugin config-panel frontend HTML from the in-memory
//! `PluginManager` registrations cache (D6) — never the filesystem, which is
//! the path-traversal defense. Each response carries the plugin's own
//! self-CSP as a header (D8) so the host does not need to whitelist plugin
//! content.
//!
//! URL contract (PR-0 + PR-1.1): the iframe src is
//! `mpe-plugin://<plugin-id>.localhost/config-panel` on macOS/Linux and
//! `http://mpe-plugin.<plugin-id>.localhost/config-panel` on Windows/Android
//! (plugin id as a subdomain of the scheme host, per wry's WebView2
//! workaround — see `wry/src/custom_protocol_workaround.rs`). The handler is
//! registered under the `mpe-plugin` scheme; on both platforms it receives a
//! `mpe-plugin://<plugin-id>.localhost/…` URL — macOS/Linux routes it
//! directly, wry reverts the Windows `http://` workaround form before
//! dispatch — so the plugin id is parsed from the host identically
//! everywhere.
//!
//! [`plugin_iframe_response`] is a pure function that takes a resolver
//! closure, so it is unit-testable without a Tauri runtime and without naming
//! `flow-engine-plugin` types from the app crate.

use tauri::http::{header, Response, StatusCode};

/// URI scheme registered for plugin frontend pages.
pub(crate) const SCHEME: &str = "mpe-plugin";
/// The only page served today; PR-0 generates exactly this page.
pub(crate) const CONFIG_PANEL_PAGE: &str = "config-panel";
/// Every plugin origin ends with this suffix: `<plugin-id>.localhost`.
const LOCALHOST_SUFFIX: &str = ".localhost";
/// D8: plugin content self-CSP sent as a response header (the host does not
/// inject its own CSP into custom-protocol responses).
const SELF_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'";
/// MIME type for served HTML pages.
const HTML_MIME: &str = "text/html; charset=utf-8";
/// MIME type for error responses.
const TEXT_MIME: &str = "text/plain; charset=utf-8";
/// Body of 404 responses.
const NOT_FOUND_BODY: &[u8] = b"not found";

/// Strictly parses the `<plugin-id>.localhost` host into the plugin id.
///
/// Returns `None` for anything that is not `{safe-id}.localhost`, where
/// `safe-id` is a non-empty run of ASCII alphanumerics plus `-`, `_`, `.`
/// with no leading/trailing dot and no `..`. Because the id is only ever
/// used as an in-memory lookup key — never as a filesystem path (D6/F7) —
/// this strict check plus the memory-only design is the whole
/// path-traversal defense.
fn parse_plugin_host(host: &str) -> Option<&str> {
    let id = host.strip_suffix(LOCALHOST_SUFFIX)?;
    if id.is_empty() || id.starts_with('.') || id.ends_with('.') || id.contains("..") {
        return None;
    }
    id.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        .then_some(id)
}

/// Builds the HTTP response for one plugin-frontend request.
///
/// `find_plugin_frontend` resolves a plugin id to its frontend
/// `(kind, content)`; the caller supplies it so this function stays free of
/// Tauri and `flow-engine-plugin` types. `host` is the raw request host
/// (e.g. `demo.localhost`), `page` the URL path without the leading `/`.
///
/// Resolution order:
/// 1. strict host parsing (bad host → 404);
/// 2. page must be [`CONFIG_PANEL_PAGE`] (unknown page → 404);
/// 3. the plugin frontend must exist (unknown plugin / no frontend → 404).
///
/// Inline content is served verbatim; content-less (iframe/url) frontends get
/// a minimal wrapper document.
pub(crate) fn plugin_iframe_response<'a, F>(
    find_plugin_frontend: F,
    host: &str,
    page: &str,
) -> Response<Vec<u8>>
where
    F: FnOnce(&str) -> Option<(&'a str, Option<&'a str>)>,
{
    let Some(plugin_id) = parse_plugin_host(host) else {
        return not_found();
    };
    if page != CONFIG_PANEL_PAGE {
        return not_found();
    }
    let Some((_kind, content)) = find_plugin_frontend(plugin_id) else {
        return not_found();
    };
    match content {
        Some(html) => ok_response(html),
        None => ok_response(&minimal_wrapper(plugin_id)),
    }
}

/// 200 HTML response carrying the D8 headers.
fn ok_response(body: &str) -> Response<Vec<u8>> {
    response(StatusCode::OK, HTML_MIME, body.as_bytes().to_vec())
}

/// 404 plain-text response.
fn not_found() -> Response<Vec<u8>> {
    response(StatusCode::NOT_FOUND, TEXT_MIME, NOT_FOUND_BODY.to_vec())
}

/// Shared response builder. Headers/status are static constants, so the
/// builder cannot actually fail; the fallback keeps production code free of
/// `unwrap`.
fn response(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_SECURITY_POLICY, SELF_CSP)
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

/// Minimal wrapper for content-less (iframe/url) frontends. The plugin id is
/// guaranteed by [`parse_plugin_host`] to be `[A-Za-z0-9._-]+`, so embedding
/// it verbatim is injection-safe.
fn minimal_wrapper(plugin_id: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head><meta charset=\"utf-8\">\n\
         <title>{plugin_id} config panel</title></head>\n\
         <body>\n<div id=\"mpe-plugin-root\" data-plugin=\"{plugin_id}\"></div>\n\
         </body>\n</html>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::http::Uri;

    /// Resolver mirroring the host-side glue: plugins `demo` (inline) and
    /// `widget` (iframe) have frontends; `nofront` is a registered plugin
    /// with none; anything else is an unknown plugin.
    fn fake_frontend<'a>(plugin_id: &str) -> Option<(&'a str, Option<&'a str>)> {
        match plugin_id {
            "demo" => Some(("inline", Some("<p>hello from demo</p>"))),
            "widget" => Some(("iframe", None)),
            _ => None,
        }
    }

    #[test]
    fn serves_inline_frontend_with_headers() {
        let response = plugin_iframe_response(fake_frontend, "demo.localhost", "config-panel");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            response
                .headers()
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .unwrap()
                .to_str()
                .unwrap(),
            "nosniff"
        );
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .unwrap()
                .to_str()
                .unwrap(),
            "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'"
        );
        assert_eq!(response.body(), b"<p>hello from demo</p>");
    }

    #[test]
    fn unknown_plugin_returns_404() {
        let response = plugin_iframe_response(fake_frontend, "ghost.localhost", "config-panel");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn plugin_without_frontend_returns_404() {
        // `nofront` is a registered plugin whose nodes carry no frontend —
        // the resolver returns `None`, the same 404 as an unknown plugin.
        let response = plugin_iframe_response(fake_frontend, "nofront.localhost", "config-panel");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unknown_page_returns_404() {
        let response = plugin_iframe_response(fake_frontend, "demo.localhost", "viewer");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn iframe_kind_serves_minimal_wrapper() {
        let response = plugin_iframe_response(fake_frontend, "widget.localhost", "config-panel");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(response.body().clone()).unwrap();
        assert!(body.contains("mpe-plugin-root"));
        assert!(body.contains("widget"));
    }

    #[test]
    fn parse_plugin_host_is_strict() {
        assert_eq!(parse_plugin_host("demo.localhost"), Some("demo"));
        assert_eq!(
            parse_plugin_host("com.acme.demo.localhost"),
            Some("com.acme.demo")
        );
        assert_eq!(parse_plugin_host("demo"), None); // no `.localhost`
        assert_eq!(parse_plugin_host("evil"), None); // no `.localhost`
        assert_eq!(parse_plugin_host(".localhost"), None); // empty id
        assert_eq!(parse_plugin_host("a..b.localhost"), None); // `..` traversal
        assert_eq!(parse_plugin_host("../etc.localhost"), None); // slash
        assert_eq!(parse_plugin_host("evil:80.localhost"), None); // colon
        assert_eq!(parse_plugin_host(".demo.localhost"), None); // leading dot
        assert_eq!(parse_plugin_host("demo..localhost"), None); // trailing dot
    }

    #[test]
    fn malicious_requests_are_404() {
        assert_eq!(
            plugin_iframe_response(fake_frontend, "evil", "config-panel").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            plugin_iframe_response(fake_frontend, "../etc.localhost", "config-panel").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            plugin_iframe_response(fake_frontend, "demo.localhost", "../../etc").status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn bare_localhost_host_is_rejected() {
        // `mpe-plugin://localhost/config-panel` carries no plugin id; the
        // host "localhost" does NOT end with ".localhost" (it is exactly
        // "localhost" without a leading dot), so strict parsing rejects it.
        assert_eq!(parse_plugin_host("localhost"), None);
        assert_eq!(
            plugin_iframe_response(fake_frontend, "localhost", "config-panel").status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn windows_url_form_reverts_to_macos_host() {
        // PR-1.1 Windows iframe src: the plugin id is a subdomain of the
        // scheme host (`http://mpe-plugin.<id>.localhost/…`). wry's
        // revert_uri_work_around strips the `http://mpe-plugin.` prefix and
        // feeds the handler `mpe-plugin://<id>.localhost/…` — the identical
        // host macOS/Linux URLs carry, so parse_plugin_host resolves both.
        let win_uri: Uri = "http://mpe-plugin.acme.localhost/config-panel"
            .parse()
            .unwrap();
        let host = win_uri.host().unwrap();
        assert_eq!(host, "mpe-plugin.acme.localhost");
        // Emulate the wry revert (custom_protocol_workaround.rs):
        let reverted_host = host.strip_prefix("mpe-plugin.").unwrap();
        assert_eq!(reverted_host, "acme.localhost");
        assert_eq!(parse_plugin_host(reverted_host), Some("acme"));
        // And the macOS URL for the same plugin yields the same host string.
        let mac_uri: Uri = "mpe-plugin://acme.localhost/config-panel".parse().unwrap();
        assert_eq!(mac_uri.host().unwrap(), reverted_host);
    }
}
