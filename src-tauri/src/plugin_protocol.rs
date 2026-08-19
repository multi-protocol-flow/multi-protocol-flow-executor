//! Custom `mpe-plugin://` URI-scheme protocol (PR-1).
//!
//! Serves plugin config-panel frontend HTML from the in-memory
//! `PluginManager` registrations cache (D6) — never the filesystem, which is
//! the path-traversal defense. Each response carries the plugin's own
//! self-CSP as a header (D8) so the host does not need to whitelist plugin
//! content.
//!
//! CSP revision (D8 修订): **inline** frontends (plugin binary embeds the
//! full panel HTML) get `INLINE_SELF_CSP`, which relaxes `script-src` with
//! `'unsafe-inline'`. Rationale: inline content lives in the same trust
//! domain as the plugin binary (both ship from the same on-disk artifact),
//! and the panel runs in a sandboxed opaque-origin iframe (`allow-scripts`
//! only, D2) whose content cannot reach the host DOM or `window.__TAURI__`.
//! The content-less wrapper page keeps the strict `SELF_CSP` — it carries no
//! scripts at all, so nothing is relaxed.
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
/// The config-panel page; PR-0 generates exactly this page.
pub(crate) const CONFIG_PANEL_PAGE: &str = "config-panel";
/// The report-viewer page (plugin `NodeDescription.viewer`, e.g. the grpc
/// plugin's report iframe).
pub(crate) const VIEWER_PAGE: &str = "viewer";
/// Every plugin origin ends with this suffix: `<plugin-id>.localhost`.
const LOCALHOST_SUFFIX: &str = ".localhost";
/// D8: strict plugin content self-CSP (content-less wrapper page).
const SELF_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'";
/// D8 修订: inline panel CSP — `'unsafe-inline'` for `script-src` because the
/// panel HTML (including its inline JS) is served verbatim from the plugin
/// binary (same trust domain) into a sandboxed opaque-origin iframe.
const INLINE_SELF_CSP: &str =
    "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'";
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
/// `find_plugin_frontend` resolves a plugin id + page to its frontend
/// `(kind, content)`; the caller supplies it so this function stays free of
/// Tauri and `flow-engine-plugin` types. `host` is the raw request host
/// (e.g. `demo.localhost`), `page` the URL path without the leading `/`.
///
/// Resolution order:
/// 1. strict host parsing (bad host → 404);
/// 2. page must be [`CONFIG_PANEL_PAGE`] or [`VIEWER_PAGE`] (unknown → 404);
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
    F: FnOnce(&str, &str) -> Option<(&'a str, Option<&'a str>)>,
{
    let Some(plugin_id) = parse_plugin_host(host) else {
        return not_found();
    };
    if page != CONFIG_PANEL_PAGE && page != VIEWER_PAGE {
        return not_found();
    }
    let Some((_kind, content)) = find_plugin_frontend(plugin_id, page) else {
        return not_found();
    };
    match content {
        // Inline panel: serve verbatim with the relaxed inline CSP.
        Some(html) => ok_response(html, INLINE_SELF_CSP),
        // Content-less frontend: minimal wrapper keeps the strict CSP.
        None => ok_response(&minimal_wrapper(plugin_id), SELF_CSP),
    }
}

/// 200 HTML response carrying the D8 headers.
fn ok_response(body: &str, csp: &str) -> Response<Vec<u8>> {
    response(StatusCode::OK, HTML_MIME, body.as_bytes().to_vec(), csp)
}

/// 404 plain-text response.
fn not_found() -> Response<Vec<u8>> {
    response(
        StatusCode::NOT_FOUND,
        TEXT_MIME,
        NOT_FOUND_BODY.to_vec(),
        SELF_CSP,
    )
}

/// Shared response builder. Headers/status are static constants, so the
/// builder cannot actually fail; the fallback keeps production code free of
/// `unwrap`.
fn response(
    status: StatusCode,
    content_type: &str,
    body: Vec<u8>,
    csp: &str,
) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_SECURITY_POLICY, csp)
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

    /// config-panel / viewer 两页路由：内联内容原样返回 + INLINE_SELF_CSP；
    /// 未知 page 404。
    #[test]
    fn serves_config_panel_and_viewer_pages() {
        let resolver = |_plugin: &str, page: &str| -> Option<(&'static str, Option<&'static str>)> {
            match page {
                "config-panel" => Some(("inline", Some("<html>panel</html>"))),
                "viewer" => Some(("inline", Some("<html>viewer</html>"))),
                _ => None,
            }
        };
        let panel = plugin_iframe_response(resolver, "grpc.localhost", "config-panel");
        assert_eq!(panel.status(), StatusCode::OK);
        assert_eq!(panel.body().as_slice(), b"<html>panel</html>");
        let csp = panel
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(csp.contains("unsafe-inline"), "inline page gets inline CSP");

        let viewer = plugin_iframe_response(resolver, "grpc.localhost", "viewer");
        assert_eq!(viewer.status(), StatusCode::OK);
        assert_eq!(viewer.body().as_slice(), b"<html>viewer</html>");

        let unknown = plugin_iframe_response(resolver, "grpc.localhost", "nope");
        assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    }

    /// viewer 页回退：无 viewer 内容时走 minimal wrapper（严格 CSP）。
    #[test]
    fn viewer_without_content_gets_wrapper() {
        let resolver = |_plugin: &str, page: &str| -> Option<(&'static str, Option<&'static str>)> {
            match page {
                "config-panel" => Some(("inline", Some("<html>panel</html>"))),
                _ => None,
            }
        };
        let viewer = plugin_iframe_response(resolver, "grpc.localhost", "viewer");
        assert_eq!(viewer.status(), StatusCode::NOT_FOUND, "no viewer → 404");
    }
}
