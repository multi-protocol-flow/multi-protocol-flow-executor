//! mpe env 子命令 — 全局环境文件管理（list / show / set / delete）
//!
//! 环境是流程之外的外挂 YAML 文件：`<environments_dir>/<name>.yaml` 为共享定义，
//! `<name>.local.yaml` 为机器私有覆盖层（可进 gitignore 保存密钥）。
//! 所有子命令的目录解析与 `run` / `debug` / `stress run` 完全一致。

use std::path::{Path, PathBuf};

use flow_engine::flow::EnvironmentSet;
use flow_engine::storage::{
    delete_environment_file, delete_environment_local_override, get_environments_dir,
    list_environment_files, load_environment_file, load_environment_file_raw,
    save_environment_file,
};

use super::output::CliOutput;
use super::variables::parse_vars;
use super::{output, CliError, EnvCommands};

/// 执行 env 子命令
pub async fn execute(format: &str, command: EnvCommands) -> Result<(), CliError> {
    let dir = environments_dir()?;

    match command {
        EnvCommands::List => execute_list(format, &dir),
        EnvCommands::Show { name, source } => execute_show(format, &dir, &name, &source),
        EnvCommands::Set { name, vars, local } => execute_set(&dir, &name, &vars, local),
        EnvCommands::Delete { name, local } => execute_delete(&dir, &name, local),
    }
}

/// 解析生效的环境目录（与 run/debug/stress 共用 `MPE_ENVIRONMENTS_DIR` / 配置 / 默认派生）
fn environments_dir() -> Result<PathBuf, CliError> {
    let app_config = super::config::AppConfig::load().map_err(CliError::Config)?;
    flow_engine::storage::set_flows_dir_override(Some(app_config.flows_dir.clone()));
    flow_engine::storage::set_environments_dir_override(Some(app_config.environments_dir.clone()));
    get_environments_dir().map_err(CliError::execution)
}

/// 列出所有环境（名称、变量数、是否存在本地覆盖）
fn execute_list(format: &str, dir: &Path) -> Result<(), CliError> {
    let envs = list_environment_files(dir).map_err(CliError::execution)?;

    if format == "table" {
        let headers = ["NAME", "VARIABLES", "LOCAL_OVERRIDE", "PATH"];
        let rows: Vec<Vec<String>> = envs
            .iter()
            .map(|e| {
                vec![
                    e.name.clone(),
                    e.variable_count.to_string(),
                    if e.has_local_override { "yes" } else { "no" }.to_string(),
                    e.file_path.clone(),
                ]
            })
            .collect();
        print!("{}", output::format_table(&headers, &rows));
        return Ok(());
    }

    output::output_json(&CliOutput::success(serde_json::json!({
        "environments_dir": dir.to_string_lossy(),
        "environments": envs,
    })));
    Ok(())
}

/// 打印环境变量（默认叠加本地覆盖后的最终取值，也可单看某一层）
fn execute_show(format: &str, dir: &Path, name: &str, source: &str) -> Result<(), CliError> {
    let env = load_layer(dir, name, source)?;

    if format == "table" {
        let headers = ["KEY", "VALUE"];
        let rows: Vec<Vec<String>> = env
            .variables
            .iter()
            .map(|(k, v)| vec![k.clone(), v.to_string()])
            .collect();
        print!("{}", output::format_table(&headers, &rows));
        return Ok(());
    }

    output::output_json(&CliOutput::success(serde_json::json!({
        "name": env.name,
        "source": source,
        "variables": env.variables,
    })));
    Ok(())
}

/// 写入变量到共享文件或本地覆盖文件
fn execute_set(dir: &Path, name: &str, vars: &[String], is_local: bool) -> Result<(), CliError> {
    let updates = parse_vars(vars)?;
    if updates.is_empty() {
        return Err(CliError::InvalidArgs(
            "no variables provided (expected KEY=VALUE)".to_string(),
        ));
    }

    // 只读目标层：把合并结果写回 base 会把 `.local.yaml` 的机器私有值固化进共享文件
    let mut env = match load_environment_file_raw(name, is_local, dir) {
        Ok(existing) => existing,
        Err(_) => EnvironmentSet {
            name: name.to_string(),
            description: None,
            variables: Default::default(),
        },
    };
    for (key, value) in &updates {
        env.variables.insert(key.clone(), value.clone());
    }

    save_environment_file(&env, is_local, dir).map_err(CliError::execution)?;

    output::output_json(&CliOutput::success(serde_json::json!({
        "name": env.name,
        "target": if is_local { "local" } else { "base" },
        "updated": updates.keys().collect::<Vec<_>>(),
        "variable_count": env.variables.len(),
    })));
    Ok(())
}

/// 删除环境（默认同时删除 `.local.yaml`，`--local` 只删本地覆盖）
fn execute_delete(dir: &Path, name: &str, local_only: bool) -> Result<(), CliError> {
    if local_only {
        delete_environment_local_override(name, dir).map_err(CliError::execution)?;
    } else {
        delete_environment_file(name, dir).map_err(CliError::execution)?;
    }

    output::output_json(&CliOutput::success(serde_json::json!({
        "deleted": name,
        "target": if local_only { "local" } else { "all" },
    })));
    Ok(())
}

/// 按层加载环境：`merged`（base + local）/ `base` / `local`
fn load_layer(dir: &Path, name: &str, source: &str) -> Result<EnvironmentSet, CliError> {
    match source {
        "merged" => load_environment_file(name, dir).map_err(CliError::execution),
        "base" => load_environment_file_raw(name, false, dir).map_err(CliError::execution),
        "local" => load_environment_file_raw(name, true, dir).map_err(CliError::execution),
        other => Err(CliError::InvalidArgs(format!(
            "unsupported --source '{}', expected 'merged', 'base' or 'local'",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _guard: std::sync::MutexGuard<'static, ()>,
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let guard = super::super::ENV_DIR_OVERRIDE_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("{e}"));
            flow_engine::storage::set_environments_dir_override(Some(dir.path().to_path_buf()));
            Self { _guard: guard, dir }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            flow_engine::storage::set_environments_dir_override(None);
        }
    }

    fn load(env: &EnvironmentSet, is_local: bool, dir: &Path) {
        save_environment_file(env, is_local, dir).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn set_writes_only_the_target_layer() {
        let fx = Fixture::new();
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("API".to_string(), serde_json::json!("https://dev.api"))]
                    .into_iter()
                    .collect(),
            },
            false,
            fx.path(),
        );
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [(
                    "LOCAL_SECRET".to_string(),
                    serde_json::json!("machine-private"),
                )]
                .into_iter()
                .collect(),
            },
            true,
            fx.path(),
        );

        execute_set(fx.path(), "dev", &["NEW_KEY=1".to_string()], false)
            .unwrap_or_else(|e| panic!("{e:?}"));

        let base =
            load_environment_file_raw("dev", false, fx.path()).unwrap_or_else(|e| panic!("{e}"));
        let local =
            load_environment_file_raw("dev", true, fx.path()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(base.variables.get("NEW_KEY"), Some(&serde_json::json!(1)));
        assert!(!base.variables.contains_key("LOCAL_SECRET"));
        assert_eq!(local.variables.len(), 1);
    }

    #[test]
    fn set_local_layer_keeps_base_untouched() {
        let fx = Fixture::new();
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("API".to_string(), serde_json::json!("https://dev.api"))]
                    .into_iter()
                    .collect(),
            },
            false,
            fx.path(),
        );

        execute_set(fx.path(), "dev", &["TOKEN=abc".to_string()], true)
            .unwrap_or_else(|e| panic!("{e:?}"));

        let base =
            load_environment_file_raw("dev", false, fx.path()).unwrap_or_else(|e| panic!("{e}"));
        let local =
            load_environment_file_raw("dev", true, fx.path()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(base.variables.len(), 1);
        assert_eq!(
            local.variables.get("TOKEN"),
            Some(&serde_json::json!("abc"))
        );
    }

    #[test]
    fn show_layers_resolve_as_expected() {
        let fx = Fixture::new();
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("API".to_string(), serde_json::json!("base"))]
                    .into_iter()
                    .collect(),
            },
            false,
            fx.path(),
        );
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("API".to_string(), serde_json::json!("local"))]
                    .into_iter()
                    .collect(),
            },
            true,
            fx.path(),
        );

        let merged = load_layer(fx.path(), "dev", "merged").unwrap_or_else(|e| panic!("{e:?}"));
        let base = load_layer(fx.path(), "dev", "base").unwrap_or_else(|e| panic!("{e:?}"));
        let local = load_layer(fx.path(), "dev", "local").unwrap_or_else(|e| panic!("{e:?}"));

        assert_eq!(
            merged.variables.get("API"),
            Some(&serde_json::json!("local"))
        );
        assert_eq!(base.variables.get("API"), Some(&serde_json::json!("base")));
        assert_eq!(
            local.variables.get("API"),
            Some(&serde_json::json!("local"))
        );
        assert!(load_layer(fx.path(), "dev", "bogus").is_err());
    }

    #[test]
    fn delete_removes_local_override_first() {
        let fx = Fixture::new();
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: Default::default(),
            },
            false,
            fx.path(),
        );
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: Default::default(),
            },
            true,
            fx.path(),
        );

        execute_delete(fx.path(), "dev", true).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(load_environment_file_raw("dev", false, fx.path()).is_ok());
        assert!(load_environment_file_raw("dev", true, fx.path()).is_err());

        execute_delete(fx.path(), "dev", false).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(load_environment_file_raw("dev", false, fx.path()).is_err());
    }

    #[test]
    fn list_reports_local_override_flag() {
        let fx = Fixture::new();
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("API".to_string(), serde_json::json!("x"))]
                    .into_iter()
                    .collect(),
            },
            false,
            fx.path(),
        );
        load(
            &EnvironmentSet {
                name: "dev".to_string(),
                description: None,
                variables: [("SECRET".to_string(), serde_json::json!("s"))]
                    .into_iter()
                    .collect(),
            },
            true,
            fx.path(),
        );

        execute_list("json", fx.path()).unwrap_or_else(|e| panic!("{e:?}"));

        let envs = list_environment_files(fx.path()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].variable_count, 1);
        assert!(envs[0].has_local_override);
    }
}
