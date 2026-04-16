//! CLI 配置模块 - 多层级配置加载
//!
//! 配置优先级（从高到低）:
//! 1. 环境变量 (`MPE_FLOWS_DIR`, `MPE_REPORTS_DIR`, `MPE_LOGS_DIR`)
//! 2. 配置文件 (`<data_dir>/config.toml` 或 `$MPE_CONFIG`)
//! 3. 引擎默认值 (`app_lib::get_flows_dir()` 等)
//!
//! 配置文件格式:
//! ```toml
//! [storage]
//! flows_dir = "/path/to/flows"
//! reports_dir = "/path/to/reports"
//! logs_dir = "/path/to/logs"
//! ```

use std::path::PathBuf;

use serde::Deserialize;

// ── 配置文件结构 ──────────────────────────────────────────────────

/// 配置文件顶层结构
#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    /// 存储相关配置
    #[serde(default)]
    storage: StorageConfig,
}

/// 配置文件中的存储配置
///
/// 所有字段均为可选，缺失时回退到环境变量或引擎默认值。
#[derive(Debug, Default, Deserialize)]
struct StorageConfig {
    /// 流程文件存储目录
    #[serde(default)]
    flows_dir: Option<PathBuf>,
    /// 报告存储目录
    #[serde(default)]
    reports_dir: Option<PathBuf>,
    /// 日志存储目录
    #[serde(default)]
    logs_dir: Option<PathBuf>,
}

// ── 解析后的配置 ──────────────────────────────────────────────────

/// CLI 应用配置（所有路径已解析，不包含 Option）
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 流程文件存储目录
    pub flows_dir: PathBuf,
    /// 报告存储目录
    pub reports_dir: PathBuf,
    /// 日志存储目录
    pub logs_dir: PathBuf,
}

impl AppConfig {
    /// 从多层级来源加载配置
    ///
    /// 解析优先级: 环境变量 > 配置文件 > 引擎默认值。
    /// 配置文件不存在时不报错，静默使用默认值。
    ///
    /// # 错误
    /// 仅在引擎默认路径无法确定时返回错误（如 HOME 环境变量缺失）。
    pub fn load() -> Result<AppConfig, String> {
        // 1. 尝试加载配置文件
        let config_file = Self::load_config_file()?;

        // 2. 逐字段解析: 环境变量 > 配置文件 > 引擎默认值
        let flows_dir = Self::resolve_path(
            "MPE_FLOWS_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.flows_dir.clone()),
            &app_lib::get_flows_dir,
        )?;

        let reports_dir = Self::resolve_path(
            "MPE_REPORTS_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.reports_dir.clone()),
            &app_lib::get_reports_dir,
        )?;

        let logs_dir = Self::resolve_path(
            "MPE_LOGS_DIR",
            config_file
                .as_ref()
                .and_then(|c| c.storage.logs_dir.clone()),
            &|| app_lib::get_data_dir().map(|d| d.join("logs")),
        )?;

        Ok(AppConfig {
            flows_dir,
            reports_dir,
            logs_dir,
        })
    }

    /// 尝试加载并解析配置文件
    ///
    /// 配置文件路径: `$MPE_CONFIG` 环境变量，或 `<data_dir>/config.toml`
    /// 文件不存在时返回 `Ok(None)`，不报错。
    fn load_config_file() -> Result<Option<ConfigFile>, String> {
        let config_path = match std::env::var("MPE_CONFIG") {
            Ok(p) => PathBuf::from(p),
            Err(_) => app_lib::get_data_dir()?.join("config.toml"),
        };

        // 配置文件不存在时静默返回 None
        if !config_path
            .try_exists()
            .map_err(|e| format!("检查配置文件失败 ({}): {}", config_path.display(), e))?
        {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败 ({}): {}", config_path.display(), e))?;

        let config: ConfigFile = toml::from_str(&content)
            .map_err(|e| format!("解析配置文件失败 ({}): {}", config_path.display(), e))?;

        Ok(Some(config))
    }

    /// 按优先级解析单个路径字段
    ///
    /// 优先级: 环境变量 > 配置文件值 > 引擎默认值
    fn resolve_path(
        env_var: &str,
        file_value: Option<PathBuf>,
        default_fn: &dyn Fn() -> Result<PathBuf, String>,
    ) -> Result<PathBuf, String> {
        // 最高优先级: 环境变量
        if let Ok(val) = std::env::var(env_var) {
            let path = PathBuf::from(val);
            if path.is_absolute() {
                return Ok(path);
            }
            return Err(format!(
                "环境变量 {} 的值不是绝对路径: {}",
                env_var,
                path.display()
            ));
        }

        // 次高优先级: 配置文件
        if let Some(path) = file_value {
            if path.is_absolute() {
                return Ok(path);
            }
            return Err(format!("配置文件中的路径不是绝对路径: {}", path.display()));
        }

        // 最低优先级: 引擎默认值
        default_fn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_returns_defaults_when_no_file() {
        // 清除可能存在的环境变量，确保测试隔离
        std::env::remove_var("MPE_CONFIG");
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_LOGS_DIR");

        // 在无配置文件的情况下应成功加载默认值
        let config = AppConfig::load();
        assert!(config.is_ok(), "无配置文件时应使用默认值，不应报错");

        let config = config.expect("已确认 is_ok，不应失败");
        assert!(config.flows_dir.to_string_lossy().contains("flows"));
        assert!(config.reports_dir.to_string_lossy().contains("reports"));
        assert!(config.logs_dir.to_string_lossy().contains("logs"));
    }

    #[test]
    fn test_env_var_overrides_default() {
        let temp = std::env::temp_dir();

        std::env::set_var("MPE_FLOWS_DIR", temp.to_string_lossy().as_ref());
        std::env::set_var("MPE_REPORTS_DIR", temp.to_string_lossy().as_ref());
        std::env::set_var("MPE_LOGS_DIR", temp.to_string_lossy().as_ref());

        let config = AppConfig::load().expect("应成功加载");

        assert_eq!(config.flows_dir, temp);
        assert_eq!(config.reports_dir, temp);
        assert_eq!(config.logs_dir, temp);

        // 清理
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_LOGS_DIR");
    }

    #[test]
    fn test_relative_path_in_env_var_fails() {
        std::env::set_var("MPE_FLOWS_DIR", "relative/path");

        let result = AppConfig::load();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不是绝对路径"));

        std::env::remove_var("MPE_FLOWS_DIR");
    }

    #[test]
    fn test_config_file_with_storage_section() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("config.toml");

        let flows = temp_dir.path().join("my_flows");
        let reports = temp_dir.path().join("my_reports");
        let logs = temp_dir.path().join("my_logs");

        // 创建目录以使用绝对路径
        std::fs::create_dir_all(&flows).expect("创建目录失败");
        std::fs::create_dir_all(&reports).expect("创建目录失败");
        std::fs::create_dir_all(&logs).expect("创建目录失败");

        let toml_content = format!(
            "[storage]\nflows_dir = \"{}\"\nreports_dir = \"{}\"\nlogs_dir = \"{}\"\n",
            flows.to_string_lossy().replace('\\', "\\\\"),
            reports.to_string_lossy().replace('\\', "\\\\"),
            logs.to_string_lossy().replace('\\', "\\\\")
        );
        std::fs::write(&config_path, toml_content).expect("写入配置文件失败");

        // 清除环境变量，使用配置文件
        std::env::remove_var("MPE_FLOWS_DIR");
        std::env::remove_var("MPE_REPORTS_DIR");
        std::env::remove_var("MPE_LOGS_DIR");
        std::env::set_var("MPE_CONFIG", config_path.to_string_lossy().as_ref());

        let config = AppConfig::load().expect("应成功加载配置文件");
        assert_eq!(config.flows_dir, flows);
        assert_eq!(config.reports_dir, reports);
        assert_eq!(config.logs_dir, logs);

        // 清理
        std::env::remove_var("MPE_CONFIG");
    }
}
