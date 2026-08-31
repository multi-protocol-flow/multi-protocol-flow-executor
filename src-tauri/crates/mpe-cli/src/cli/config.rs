//! CLI config module — multi-level configuration loading
//!
//! Configuration priority (highest to lowest):
//! 1. Environment variables (`MPE_FLOWS_DIR`, `MPE_REPORTS_DIR`)
//! 2. Config file (`<data_dir>/config.toml` or `$MPE_CONFIG`)
//! 3. Engine defaults (`flow_engine::storage::get_flows_dir()` etc.)
//!
//! Config file format:
//! ```toml
//! [storage]
//! flows_dir = "/path/to/flows"
//! reports_dir = "/path/to/reports"
//! ```

use std::path::PathBuf;

use serde::Deserialize;

// ── Config file structure ──────────────────────────────────────────────

/// Top-level config file structure
#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    /// Storage-related configuration
    #[serde(default)]
    storage: StorageConfig,
    /// Plugin-related configuration
    #[serde(default)]
    plugins: PluginConfig,
}

/// Plugin configuration from config file
#[derive(Debug, Default, Deserialize)]
struct PluginConfig {
    /// Max concurrent requests per plugin process
    #[serde(default)]
    max_concurrency: Option<usize>,
    /// Queue capacity
    #[serde(default)]
    queue_capacity: Option<usize>,
}

/// Storage configuration from config file
///
/// All fields are optional, fall back to env vars or engine defaults when missing.
#[derive(Debug, Default, Deserialize)]
struct StorageConfig {
    /// Flow file storage directory
    #[serde(default)]
    flows_dir: Option<PathBuf>,
    /// Report storage directory
    #[serde(default)]
    reports_dir: Option<PathBuf>,
    /// Plugin directory
    #[serde(default)]
    plugins_dir: Option<PathBuf>,
}

// ── Resolved config ────────────────────────────────────────────────────

/// CLI application config (all paths resolved, no Option)
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Flow file storage directory
    pub flows_dir: PathBuf,
    /// Report storage directory
    pub reports_dir: PathBuf,
    /// Plugin directory
    pub plugins_dir: PathBuf,
    /// Plugin connection configuration
    pub plugin_connection_config: flow_engine::ConnectionConfig,
}

impl AppConfig {
    /// Load config from multi-level sources
    ///
    /// Resolution priority: env vars > config file > engine defaults.
    /// No error if config file doesn't exist, silently use defaults.
    ///
    /// # Errors
    /// Returns error only when engine default paths cannot be determined (e.g., HOME env var missing).
    pub fn load() -> Result<AppConfig, String> {
        // 1. Try loading config file
        let config_file = Self::load_config_file()?;

        // 2. Resolve field by field: env vars > config file > engine defaults
        let flows_dir = Self::resolve_path(
            "MPE_FLOWS_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.flows_dir.clone()),
            &flow_engine::storage::get_flows_dir,
        )?;

        let reports_dir = Self::resolve_path(
            "MPE_REPORTS_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.reports_dir.clone()),
            &flow_engine::storage::get_reports_dir,
        )?;

        // Plugins: env > config file > data dir/plugins > ./plugins fallback
        let plugins_dir = Self::resolve_path(
            "MPE_PLUGIN_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.plugins_dir.clone()),
            &|| {
                flow_engine::storage::get_data_dir()
                    .map(|d| d.join("plugins"))
                    .or_else(|_| Ok(PathBuf::from("plugins")))
            },
        )?;

        let mut plugin_connection_config = flow_engine::ConnectionConfig::default();
        if let Some(concurrency) = std::env::var("MPE_PLUGIN_MAX_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&c| c > 0)
            .or_else(|| config_file.as_ref().and_then(|c| c.plugins.max_concurrency).filter(|&c| c > 0))
        {
            plugin_connection_config.max_concurrent_requests = concurrency;
        }
        if let Some(queue_cap) = std::env::var("MPE_PLUGIN_QUEUE_CAPACITY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&q| q > 0)
            .or_else(|| config_file.as_ref().and_then(|c| c.plugins.queue_capacity).filter(|&q| q > 0))
        {
            plugin_connection_config.queue_capacity = queue_cap;
        }

        Ok(AppConfig {
            flows_dir,
            reports_dir,
            plugins_dir,
            plugin_connection_config,
        })
    }

    /// Try to load and parse config file
    ///
    /// Config file path: `$MPE_CONFIG` env var, or `<data_dir>/config.toml`
    /// Returns `Ok(None)` when file doesn't exist, no error.
    fn load_config_file() -> Result<Option<ConfigFile>, String> {
        let config_path = match std::env::var("MPE_CONFIG") {
            Ok(p) => PathBuf::from(p),
            Err(_) => flow_engine::storage::get_data_dir()?.join("config.toml"),
        };

        // Silently return None when config file doesn't exist
        if !config_path.try_exists().map_err(|e| {
            format!(
                "failed to check config file ({}): {}",
                config_path.display(),
                e
            )
        })? {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            format!(
                "failed to read config file ({}): {}",
                config_path.display(),
                e
            )
        })?;

        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            format!(
                "failed to parse config file ({}): {}",
                config_path.display(),
                e
            )
        })?;

        Ok(Some(config))
    }

    /// Resolve a single path field by priority
    ///
    /// Priority: env var > config file value > engine default
    fn resolve_path(
        env_var: &str,
        file_value: Option<PathBuf>,
        default_fn: &dyn Fn() -> Result<PathBuf, String>,
    ) -> Result<PathBuf, String> {
        // Highest priority: env var
        if let Ok(val) = std::env::var(env_var) {
            let path = PathBuf::from(val);
            if path.is_absolute() {
                return Ok(path);
            }
            return Err(format!(
                "value of env var {} is not an absolute path: {}",
                env_var,
                path.display()
            ));
        }

        // Second priority: config file
        if let Some(path) = file_value {
            if path.is_absolute() {
                return Ok(path);
            }
            return Err(format!(
                "path in config file is not absolute: {}",
                path.display()
            ));
        }

        // Lowest priority: engine default
        default_fn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    /// Global mutex to serialize all env-var-related tests, avoid parallel race
    static ENV_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn test_load_config_returns_defaults_when_no_file() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| panic!("{e}"));
        // Clear any existing env vars for test isolation
        std::env::remove_var("MPE_CONFIG");
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_PLUGIN_DIR");

        // Should successfully load defaults without config file
        let config = AppConfig::load();
        assert!(
            config.is_ok(),
            "should use defaults when no config file, no error expected"
        );

        let config = config.unwrap_or_else(|e| panic!("confirmed is_ok, should not fail: {e}"));
        assert!(config.flows_dir.to_string_lossy().contains("flows"));
        assert!(config.reports_dir.to_string_lossy().contains("reports"));
        assert!(config.plugins_dir.to_string_lossy().contains("plugins"));
    }

    #[test]
    fn test_env_var_overrides_default() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| panic!("{e}"));

        // Clear all related env vars for test isolation
        std::env::remove_var("MPE_CONFIG");
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_PLUGIN_DIR");

        let temp = std::env::temp_dir();

        std::env::set_var("MPE_FLOWS_DIR", temp.to_string_lossy().as_ref());
        std::env::set_var("MPE_REPORTS_DIR", temp.to_string_lossy().as_ref());

        let config = AppConfig::load().unwrap_or_else(|e| panic!("should load successfully: {e}"));

        assert_eq!(config.flows_dir, temp);
        assert_eq!(config.reports_dir, temp);

        // Cleanup
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_PLUGIN_DIR");
    }

    #[test]
    fn test_relative_path_in_env_var_fails() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| panic!("{e}"));

        // Clear all related env vars to avoid parallel test race
        std::env::remove_var("MPE_CONFIG");
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_PLUGIN_DIR");

        std::env::set_var("MPE_FLOWS_DIR", "relative/path");

        let result = AppConfig::load();
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("absolute path"));

        std::env::remove_var("MPE_FLOWS_DIR");

        // MPE_PLUGIN_DIR relative path fails the same way
        std::env::set_var("MPE_PLUGIN_DIR", "relative/path");
        let result = AppConfig::load();
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("absolute path"));
        std::env::remove_var("MPE_PLUGIN_DIR");
    }

    #[test]
    fn test_config_file_with_storage_section() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| panic!("{e}"));

        let temp_dir =
            tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create temp dir: {e}"));
        let config_path = temp_dir.path().join("config.toml");

        let flows = temp_dir.path().join("my_flows");
        let reports = temp_dir.path().join("my_reports");
        let plugins = temp_dir.path().join("my_plugins");

        // Create directories to use absolute paths
        std::fs::create_dir_all(&flows).unwrap_or_else(|e| panic!("failed to create dir: {e}"));
        std::fs::create_dir_all(&reports).unwrap_or_else(|e| panic!("failed to create dir: {e}"));
        std::fs::create_dir_all(&plugins).unwrap_or_else(|e| panic!("failed to create dir: {e}"));

        let toml_content = format!(
            "[storage]\nflows_dir = \"{}\"\nreports_dir = \"{}\"\nplugins_dir = \"{}\"\n",
            flows.to_string_lossy().replace('\\', "\\\\"),
            reports.to_string_lossy().replace('\\', "\\\\"),
            plugins.to_string_lossy().replace('\\', "\\\\")
        );
        std::fs::write(&config_path, toml_content)
            .unwrap_or_else(|e| panic!("failed to write config file: {e}"));

        // Clear env vars, use config file
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_PLUGIN_DIR");
        std::env::set_var("MPE_CONFIG", config_path.to_string_lossy().as_ref());

        let config = AppConfig::load()
            .unwrap_or_else(|e| panic!("should load config file successfully: {e}"));
        assert_eq!(config.flows_dir, flows);
        assert_eq!(config.reports_dir, reports);
        assert_eq!(config.plugins_dir, plugins);

        // Cleanup
        std::env::remove_var("MPE_CONFIG");
    }
}
