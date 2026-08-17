//! Plugin installation: download, verify, extract, validate.
//!
//! Pipeline (each step failing aborts and removes any partially-created
//! target directory so a failed install never leaves a half-installed
//! plugin behind):
//! 1. Platform gate — the current platform key must have an asset.
//! 2. Download the archive to memory.
//! 3. Verify the sha256 digest when the asset advertises one.
//! 4. Refuse to overwrite an existing install unless `force`.
//! 5. Extract with path-traversal protection (no absolute paths, `..`
//!    components, or Windows drive prefixes; symlink entries skipped).
//! 6. Validate the resulting `plugin.json` against the `mpe-plugin-sdk`
//!    manifest schema and the marketplace entry's name.

use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

use mpe_plugin_sdk::protocol::manifest::PluginManifest;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::client::MarketplaceClient;
use crate::error::MarketplaceError;
use crate::model::MarketplacePlugin;
use crate::platform::current_platform;

/// Metadata about a successfully installed plugin.
#[derive(Debug, Clone)]
pub struct InstalledPluginInfo {
    /// Plugin name (from the marketplace entry and manifest).
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// The plugin's install directory (contains `plugin.json`).
    pub path: String,
}

/// Download, verify and extract `plugin` into `install_dir/<name>/`.
///
/// `force` re-installs over an existing `<name>/plugin.json`; otherwise an
/// existing install is an [`MarketplaceError::AlreadyInstalled`] error.
///
/// # Errors
/// - [`MarketplaceError::PlatformUnsupported`] — no asset for this platform.
/// - [`MarketplaceError::Sha256Mismatch`] — digest mismatch.
/// - [`MarketplaceError::AlreadyInstalled`] — exists and `force` is false.
/// - [`MarketplaceError::Extract`] — unsafe archive path or bad archive.
/// - [`MarketplaceError::InvalidManifest`] — `plugin.json` fails validation.
pub async fn install_plugin(
    client: &MarketplaceClient,
    plugin: &MarketplacePlugin,
    install_dir: &Path,
    force: bool,
) -> Result<InstalledPluginInfo, MarketplaceError> {
    // 1. Platform gate.
    let platform = current_platform().ok_or_else(|| platform_unsupported(plugin))?;
    let asset = plugin
        .platforms
        .get(&platform)
        .ok_or_else(|| platform_unsupported(plugin))?;

    // 2. Download to memory.
    let bytes = client.get_bytes(&asset.url).await?;

    // 3. Optional sha256 verification.
    if let Some(expected) = asset.sha256.as_deref() {
        let digest = hex_encode(&Sha256::digest(&bytes));
        if !digest.eq_ignore_ascii_case(expected.trim()) {
            return Err(MarketplaceError::Sha256Mismatch);
        }
    }

    // 4. Target directory + AlreadyInstalled gate.
    let target = install_dir.join(&plugin.name);
    if target.join("plugin.json").exists() && !force {
        return Err(MarketplaceError::AlreadyInstalled(target.display().to_string()));
    }
    // Start from a clean slate (also removes a partial install from a
    // previous crash when force is set).
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::create_dir_all(&target)?;

    // 5-8. Extract, validate, chmod. Any failure cleans up the target dir so
    // no half-install is ever left behind.
    let result = install_into(&bytes, plugin, &target);
    if result.is_err() {
        let _ = fs::remove_dir_all(&target);
    }
    result
}

/// Extract + manifest validation + unix exec-bit fixup into `target`.
fn install_into(
    bytes: &[u8],
    plugin: &MarketplacePlugin,
    target: &Path,
) -> Result<InstalledPluginInfo, MarketplaceError> {
    extract_archive(bytes, target, &plugin.name)?;

    let manifest_path = target.join("plugin.json");
    let raw = fs::read(&manifest_path)?;
    let manifest: PluginManifest = serde_json::from_slice(&raw)
        .map_err(|e| MarketplaceError::InvalidManifest(e.to_string()))?;
    if manifest.name != plugin.name || manifest.entry.command.trim().is_empty() {
        return Err(MarketplaceError::InvalidManifest(format!(
            "name mismatch or empty entry.command (expected name `{}`)",
            plugin.name
        )));
    }

    // On unix, make the entry command executable when it is a file inside
    // the plugin directory (interpreter commands like `python` are not files
    // here and are left untouched).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let command_path = target.join(&manifest.entry.command);
        if command_path.exists() {
            fs::set_permissions(&command_path, fs::Permissions::from_mode(0o755))?;
        }
    }

    Ok(InstalledPluginInfo {
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        path: target.display().to_string(),
    })
}

/// Extract a zip archive into `target`, rejecting any entry whose path
/// escapes `target` (absolute paths, `..` components, Windows drive
/// prefixes). Symlink entries are skipped with a warning rather than
/// followed.
///
/// A leading `<plugin_name>/` component (the common "zip root contains the
/// plugin directory" layout) is stripped so the archive's `plugin.json`
/// lands directly in `target`.
fn extract_archive(
    bytes: &[u8],
    target: &Path,
    plugin_name: &str,
) -> Result<(), MarketplaceError> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| MarketplaceError::Extract(e.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| MarketplaceError::Extract(e.to_string()))?;
        let raw_name = entry.name().to_string();
        let name = strip_plugin_root(&raw_name, plugin_name);
        if name.is_empty() {
            log::debug!("skipping zip entry that is only the plugin root: {raw_name}");
            continue;
        }

        // Symlink entries: skip and warn (never follow).
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                log::warn!("skipping symlink entry in plugin archive: {name}");
                continue;
            }
        }

        let dest = safe_join(target, &name)?;
        if entry.is_dir() {
            fs::create_dir_all(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Remove a leading `<plugin_name>/` component from a zip entry name (the
/// archive root contains the plugin directory itself).
fn strip_plugin_root(entry_name: &str, plugin_name: &str) -> String {
    let path = Path::new(entry_name);
    let mut components = path.components();
    if let Some(Component::Normal(first)) = components.next() {
        if first == plugin_name {
            return components.as_path().to_string_lossy().into_owned();
        }
    }
    entry_name.to_string()
}

/// Join a zip entry name onto `target`, rejecting any path that would escape
/// it.
fn safe_join(target: &Path, entry_name: &str) -> Result<PathBuf, MarketplaceError> {
    let path = Path::new(entry_name);
    if path.is_absolute() {
        return Err(MarketplaceError::Extract(format!(
            "absolute path in archive: {entry_name}"
        )));
    }
    let mut out = target.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(MarketplaceError::Extract(format!(
                    "parent traversal in archive: {entry_name}"
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(MarketplaceError::Extract(format!(
                    "absolute/prefix path in archive: {entry_name}"
                )));
            }
        }
    }
    Ok(out)
}

/// Hex-encode a byte slice (no external hex dependency).
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Build a [`MarketplaceError::PlatformUnsupported`] for `plugin`, with the
/// available platform keys sorted for display.
fn platform_unsupported(plugin: &MarketplacePlugin) -> MarketplaceError {
    let mut available: Vec<String> = plugin.platforms.keys().cloned().collect();
    available.sort();
    MarketplaceError::PlatformUnsupported { available }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use tempfile::TempDir;

    /// Build a marketplace plugin entry pointing at the given asset URL.
    fn plugin_with_asset(url: String, sha256: Option<String>) -> MarketplacePlugin {
        let mut platforms = HashMap::new();
        platforms.insert(
            crate::platform::current_platform()
                .unwrap_or_else(|| panic!("current platform must be known")),
            crate::model::PlatformAsset { url, sha256, size: None },
        );
        MarketplacePlugin {
            name: "mongo".into(),
            version: "1.0.0".into(),
            description: None,
            author: None,
            min_host_version: None,
            icon: None,
            categories: Vec::new(),
            updated_at: None,
            platforms,
        }
    }

    /// Zip bytes for a plugin archive whose root contains `mongo/plugin.json`
    /// and `mongo/mongo`. `extra_entries` adds raw entries (name + bytes)
    /// for traversal tests.
    fn build_zip(manifest_json: &str, extra_entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer
            .start_file("mongo/plugin.json", options)
            .unwrap_or_else(|e| panic!("zip write failed: {e}"));
        writer
            .write_all(manifest_json.as_bytes())
            .unwrap_or_else(|e| panic!("zip write failed: {e}"));
        writer
            .start_file("mongo/mongo", options)
            .unwrap_or_else(|e| panic!("zip write failed: {e}"));
        writer
            .write_all(b"#!/bin/sh\necho mongo\n")
            .unwrap_or_else(|e| panic!("zip write failed: {e}"));
        for (name, bytes) in extra_entries {
            writer
                .start_file(*name, options)
                .unwrap_or_else(|e| panic!("zip write failed: {e}"));
            writer
                .write_all(bytes)
                .unwrap_or_else(|e| panic!("zip write failed: {e}"));
        }
        let data = writer
            .finish()
            .unwrap_or_else(|e| panic!("zip finish failed: {e}"));
        data.into_inner()
    }

    /// One-shot HTTP server serving (`path`, `status`, `body_bytes`) responses in
    /// order; once the fixture list is exhausted the listener is dropped and
    /// the thread exits, so `join()` never blocks.
    fn serve_bytes(responses: Vec<(String, u16, Vec<u8>)>) -> (String, thread::JoinHandle<()>) {
        let listener =
            TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind failed: {e}"));
        let addr = listener.local_addr().unwrap_or_else(|e| panic!("addr failed: {e}"));
        let handle = thread::spawn(move || {
            for (_path, status, body) in responses {
                let (mut stream, _) = listener
                    .accept()
                    .unwrap_or_else(|e| panic!("accept failed: {e}"));
                let mut buf = [0u8; 8192];
                let _ = stream
                    .read(&mut buf)
                    .unwrap_or_else(|e| panic!("read failed: {e}"));
                let reason = if status == 200 { "OK" } else { "ERROR" };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(header.as_bytes())
                    .unwrap_or_else(|e| panic!("write failed: {e}"));
                stream
                    .write_all(&body)
                    .unwrap_or_else(|e| panic!("write failed: {e}"));
            }
        });
        (format!("http://{addr}"), handle)
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex_encode(&Sha256::digest(bytes))
    }

    fn join_fixture(handle: thread::JoinHandle<()>) {
        handle.join().unwrap_or_else(|_| panic!("fixture server panicked"));
    }

    fn tempdir() -> TempDir {
        TempDir::new().unwrap_or_else(|e| panic!("tempdir failed: {e}"))
    }

    async fn install_ok(
        client: &MarketplaceClient,
        plugin: &MarketplacePlugin,
        dir: &Path,
        force: bool,
    ) -> InstalledPluginInfo {
        install_plugin(client, plugin, dir, force)
            .await
            .unwrap_or_else(|e| panic!("install should succeed: {e}"))
    }

    async fn install_err(
        client: &MarketplaceClient,
        plugin: &MarketplacePlugin,
        dir: &Path,
        force: bool,
    ) -> MarketplaceError {
        match install_plugin(client, plugin, dir, force).await {
            Err(e) => e,
            Ok(_) => panic!("install should fail"),
        }
    }

    #[tokio::test]
    async fn install_plugin_extracts_and_validates() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(
            r#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#,
            &[],
        );
        let digest = sha256_hex(&zip);
        let (base, handle) = serve_bytes(vec![("/mongo.zip".into(), 200, zip.clone())]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let info = install_ok(
            &client,
            &plugin_with_asset(format!("{base}/mongo.zip"), Some(digest)),
            temp.path(),
            false,
        )
        .await;
        assert_eq!(info.name, "mongo");
        assert!(temp.path().join("mongo/plugin.json").exists());
        assert!(temp.path().join("mongo/mongo").exists());
        // Unix: entry command must be executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(temp.path().join("mongo/mongo"))
                .unwrap_or_else(|e| panic!("stat failed: {e}"))
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "entry command must be executable");
        }
        join_fixture(handle);
    }

    #[tokio::test]
    async fn sha256_mismatch_is_rejected() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(
            r#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#,
            &[],
        );
        let (base, handle) = serve_bytes(vec![("/mongo.zip".into(), 200, zip)]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let err = install_err(
            &client,
            &plugin_with_asset(format!("{base}/mongo.zip"), Some("deadbeef".into())),
            temp.path(),
            false,
        )
        .await;
        assert!(matches!(err, MarketplaceError::Sha256Mismatch), "got {err:?}");
        assert!(!temp.path().join("mongo").exists(), "no half-install on digest failure");
        join_fixture(handle);
    }

    #[tokio::test]
    async fn already_installed_gate_and_force_override() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(
            r#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#,
            &[],
        );
        let digest = sha256_hex(&zip);
        // Three install attempts: success, AlreadyInstalled gate (downloads
        // before the gate per the pipeline order), force re-install.
        let (base, handle) = serve_bytes(vec![
            ("/mongo.zip".into(), 200, zip.clone()),
            ("/mongo.zip".into(), 200, zip.clone()),
            ("/mongo.zip".into(), 200, zip),
        ]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let plugin = plugin_with_asset(format!("{base}/mongo.zip"), Some(digest));

        install_ok(&client, &plugin, temp.path(), false).await;

        // Second install without force → AlreadyInstalled.
        let err = install_err(&client, &plugin, temp.path(), false).await;
        assert!(
            matches!(err, MarketplaceError::AlreadyInstalled(_)),
            "got {err:?}"
        );

        // Force re-install succeeds.
        install_ok(&client, &plugin, temp.path(), true).await;
        assert!(temp.path().join("mongo/plugin.json").exists());
        join_fixture(handle);
    }

    #[tokio::test]
    async fn parent_traversal_entry_is_rejected() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(
            r#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#,
            &[("../evil", b"pwned")],
        );
        let (base, handle) = serve_bytes(vec![("/mongo.zip".into(), 200, zip)]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let err = install_err(
            &client,
            &plugin_with_asset(format!("{base}/mongo.zip"), None),
            temp.path(),
            false,
        )
        .await;
        assert!(matches!(err, MarketplaceError::Extract(_)), "got {err:?}");
        // No file escaped the target, and no half-install remains.
        assert!(!temp.path().join("evil").exists());
        assert!(!temp.path().join("mongo").exists());
        join_fixture(handle);
    }

    #[tokio::test]
    async fn absolute_path_entry_is_rejected() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(
            r#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#,
            &[("/etc/passwd", b"nope")],
        );
        let (base, handle) = serve_bytes(vec![("/mongo.zip".into(), 200, zip)]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let err = install_err(
            &client,
            &plugin_with_asset(format!("{base}/mongo.zip"), None),
            temp.path(),
            false,
        )
        .await;
        assert!(matches!(err, MarketplaceError::Extract(_)), "got {err:?}");
        assert!(!temp.path().join("mongo").exists());
        join_fixture(handle);
    }

    #[tokio::test]
    async fn invalid_manifest_cleans_up_target() {
        crate::test_support::install_crypto_provider();
        let zip = build_zip(r#"{"name":"other","version":"1.0.0","entry":{"command":"x"}}"#, &[]);
        let (base, handle) = serve_bytes(vec![("/mongo.zip".into(), 200, zip)]);
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let err = install_err(
            &client,
            &plugin_with_asset(format!("{base}/mongo.zip"), None),
            temp.path(),
            false,
        )
        .await;
        assert!(
            matches!(err, MarketplaceError::InvalidManifest(_)),
            "got {err:?}"
        );
        assert!(!temp.path().join("mongo").exists(), "no half-install after bad manifest");
        join_fixture(handle);
    }

    #[tokio::test]
    async fn platform_unsupported_when_no_current_platform_asset() {
        crate::test_support::install_crypto_provider();
        let (base, handle) = serve_bytes(vec![]);
        let mut plugin = plugin_with_asset(format!("{base}/mongo.zip"), None);
        // Replace the asset key with an impossible one.
        let bogus = "solaris-sparc".to_string();
        let url = plugin
            .platforms
            .remove(
                &crate::platform::current_platform()
                    .unwrap_or_else(|| panic!("current platform must be known")),
            )
            .unwrap_or_else(|| panic!("current platform asset must exist"))
            .url;
        plugin.platforms.insert(
            bogus.clone(),
            crate::model::PlatformAsset { url, sha256: None, size: None },
        );
        let client = MarketplaceClient::new(base.clone());
        let temp = tempdir();
        let err = install_err(&client, &plugin, temp.path(), false).await;
        match err {
            MarketplaceError::PlatformUnsupported { available } => {
                assert_eq!(available, vec![bogus]);
            }
            other => panic!("expected PlatformUnsupported, got {other:?}"),
        }
        assert!(!temp.path().join("mongo").exists());
        join_fixture(handle);
    }
}
