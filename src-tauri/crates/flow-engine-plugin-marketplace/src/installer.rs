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
#[derive(Debug, Clone, serde::Serialize)]
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

    // 2. Target directory + AlreadyInstalled gate (checked before download).
    let target = install_dir.join(&plugin.name);
    if target.join("plugin.json").exists() && !force {
        return Err(MarketplaceError::AlreadyInstalled(target.display().to_string()));
    }

    // Ensure install_dir exists
    fs::create_dir_all(install_dir)?;

    // 3. Download to memory.
    let bytes = client.get_bytes(&asset.url).await?;

    // 4. Optional sha256 verification.
    if let Some(expected) = asset.sha256.as_deref() {
        let digest = hex_encode(&Sha256::digest(&bytes));
        if !digest.eq_ignore_ascii_case(expected.trim()) {
            return Err(MarketplaceError::Sha256Mismatch);
        }
    }

    // 5. Extract and validate into a temporary directory in install_dir.
    let temp_dir = tempfile::Builder::new()
        .prefix(".mpe-install-")
        .tempdir_in(install_dir)?;

    install_into(&bytes, plugin, temp_dir.path())?;

    // 6. Atomic swap: remove existing target if force, then rename temp_dir into target.
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    let temp_path = temp_dir.keep();
    fs::rename(&temp_path, &target).map_err(|e| {
        let _ = fs::remove_dir_all(&temp_path);
        MarketplaceError::Io(e)
    })?;

    Ok(InstalledPluginInfo {
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        path: target.display().to_string(),
    })
}

/// Extract + manifest validation + unix exec-bit fixup into `target`.
fn install_into(
    bytes: &[u8],
    plugin: &MarketplacePlugin,
    target: &Path,
) -> Result<(), MarketplaceError> {
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

    Ok(())
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

