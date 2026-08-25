//! mpe plugin subcommand — plugin directory / installed list / install from marketplace

use std::path::PathBuf;

use serde_json::json;

use super::output::CliOutput;
use super::{CliError, PluginCommands};

/// Execute plugin management subcommand
pub async fn execute(command: PluginCommands) -> Result<(), CliError> {
    match command {
        PluginCommands::Dir => execute_dir().await,
        PluginCommands::List => execute_list().await,
        PluginCommands::Install {
            name,
            version,
            dir,
            registry,
            force,
        } => execute_install(&name, version.as_deref(), dir, registry.as_deref(), force).await,
    }
}

/// Show the effective plugins directory (env > config.toml > data dir > ./plugins)
async fn execute_dir() -> Result<(), CliError> {
    let dir = super::plugins_dir();
    let source = if std::env::var_os("MPE_PLUGIN_DIR").is_some() {
        "env"
    } else if config_file_has_plugins_dir() {
        "config"
    } else {
        "default"
    };
    let output = CliOutput::success(json!({ "dir": dir, "source": source }));
    super::output::output_json(&output);
    Ok(())
}

/// Whether the config file (`$MPE_CONFIG` or `<data_dir>/config.toml`) sets
/// `[storage].plugins_dir` (only consulted when `MPE_PLUGIN_DIR` is unset,
/// since env takes precedence).
fn config_file_has_plugins_dir() -> bool {
    let config_path = match std::env::var("MPE_CONFIG") {
        Ok(p) => PathBuf::from(p),
        Err(_) => match flow_engine::storage::get_data_dir() {
            Ok(d) => d.join("config.toml"),
            Err(_) => return false,
        },
    };
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(config) = content.parse::<toml::Value>() else {
        return false;
    };
    config
        .get("storage")
        .and_then(|storage| storage.get("plugins_dir"))
        .and_then(|v| v.as_str())
        .is_some()
}

/// List installed plugins (scan of the effective plugins directory)
async fn execute_list() -> Result<(), CliError> {
    let dir = super::plugins_dir();
    let host_version = std::env::var("MPE_HOST_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let entries = flow_engine::PluginScan::scan_with_errors(&dir, &host_version);
    let list: Vec<serde_json::Value> = entries
        .into_iter()
        .filter_map(|entry| match entry {
            flow_engine::ScanEntry::Discovered(plugin) => Some(json!({
                "name": plugin.name,
                "version": plugin.version,
                "description": plugin.manifest.description,
                "path": plugin.dir,
                "error": null,
            })),
            flow_engine::ScanEntry::Skipped { dir, reason } => {
                if matches!(
                    reason,
                    flow_engine::ScanSkipReason::MissingManifest
                        | flow_engine::ScanSkipReason::NotADirectory
                ) {
                    None
                } else {
                    Some(json!({
                        "name": dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                        "version": null,
                        "description": null,
                        "path": dir,
                        "error": format!("{reason:?}"),
                    }))
                }
            }
        })
        .collect();
    let output = CliOutput::success(list);
    super::output::output_json(&output);
    Ok(())
}

/// Install a plugin from a marketplace
///
/// 1. Resolve the registry URL: `--registry` > `MPE_PLUGIN_REGISTRY`.
/// 2. Resolve the install dir: `--dir` > effective plugins directory.
/// 3. List the marketplace, pick the first entry matching `name` (or the
///    exact `version` when given — the server contract orders by version
///    descending).
/// 4. Download + sha256 verify + extract + manifest validate.
async fn execute_install(
    name: &str,
    version: Option<&str>,
    dir: Option<PathBuf>,
    registry: Option<&str>,
    force: bool,
) -> Result<(), CliError> {
    let registry_url = match registry {
        Some(url) => url.to_string(),
        None => std::env::var("MPE_PLUGIN_REGISTRY").map_err(|_| {
            CliError::InvalidArgs(
                "plugin install requires --registry or MPE_PLUGIN_REGISTRY".to_string(),
            )
        })?,
    };

    let install_dir = dir.unwrap_or_else(super::plugins_dir);

    let client = flow_engine::flow_engine_plugin_marketplace::MarketplaceClient::new(registry_url);
    let plugins = client
        .list_plugins()
        .await
        .map_err(|e| CliError::execution(e.to_string()))?;

    let plugin = plugins
        .iter()
        .find(|plugin| {
            plugin.name == name && version.is_none_or(|v| plugin.version == v)
        })
        .ok_or_else(|| {
            CliError::execution(format!(
                "plugin {name}{} not found in marketplace",
                version.map(|v| format!("@{v}")).unwrap_or_default()
            ))
        })?;

    let info = flow_engine::flow_engine_plugin_marketplace::install_plugin(
        &client,
        plugin,
        &install_dir,
        force,
    )
    .await
    .map_err(|e| CliError::execution(e.to_string()))?;

    let output = CliOutput::success(json!({
        "name": info.name,
        "version": info.version,
        "path": info.path,
    }));
    super::output::output_json(&output);
    Ok(())
}
