//! 执行期变量分层解析
//!
//! 所有"执行流程"的 CLI 子命令（`run` / `debug` / `stress run`）都必须走同一套
//! 合并规则，否则同一个流程在 CLI 内部会因入口不同而解析到不同的变量：
//!
//! 优先级由低到高：
//! 1. `flow.initial_variables`（流程兜底）
//! 2. 环境文件 `<name>.yaml`
//! 3. 本地私有覆盖 `<name>.local.yaml`（`load_environment_file` 自动叠加）
//! 4. 运行时覆盖（`--var`，调试单步注入）

use std::collections::HashMap;

use super::CliError;

/// 解析 `--var key=value` 参数为变量表
///
/// 值优先按 JSON 解析（`n=1`、`flag=true`、`obj={"a":1}`），
/// JSON 解析失败时退化为字符串。
pub(crate) fn parse_vars(vars: &[String]) -> Result<HashMap<String, serde_json::Value>, CliError> {
    let mut map = HashMap::new();
    for v in vars {
        let (key, val) = v.split_once('=').ok_or_else(|| {
            CliError::InvalidArgs(format!("--var format error (expected key=value): {}", v))
        })?;
        let json_val =
            serde_json::from_str(val).unwrap_or(serde_json::Value::String(val.to_string()));
        map.insert(key.to_string(), json_val);
    }
    Ok(map)
}

/// 按分层规则解析本次执行生效的变量
///
/// 显式指定了环境却加载失败时必须直接报错：静默退回流程兜底变量会让流程在错误的
/// 端点上执行（例如 prod 流程跑在兜底指向的 dev/staging 地址）。
///
/// # 返回
/// - `Ok(Some(value))`: 合并后的变量对象（非空）
/// - `Ok(None)`: 无任何变量层
/// - `Err(_)`: 环境已指定但加载失败
pub(crate) fn resolve_execution_variables(
    flow: &flow_engine::flow::Flow,
    env_name: Option<&str>,
    runtime_vars: Option<&HashMap<String, serde_json::Value>>,
) -> Result<Option<serde_json::Value>, CliError> {
    let env_set = match env_name.map(str::trim) {
        Some(name) if !name.is_empty() => {
            let env_dir = flow_engine::storage::get_environments_dir().map_err(|e| {
                CliError::execution(format!("Failed to get environments directory: {}", e))
            })?;
            // `load_environment_file` 自动叠加同名 `.local.yaml` 私有覆盖
            Some(
                flow_engine::storage::load_environment_file(name, &env_dir).map_err(|e| {
                    CliError::execution(format!("Failed to load environment '{}': {}", name, e))
                })?,
            )
        }
        _ => None,
    };

    let merged = flow_engine::flow::merge_variable_layers(
        flow.initial_variables.as_ref(),
        env_set.as_ref(),
        None,
        runtime_vars,
    );

    if merged.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(
            merged.into_iter().collect::<serde_json::Map<_, _>>(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine::flow::{EnvironmentSet, Flow};

    fn lock_env_dir() -> std::sync::MutexGuard<'static, ()> {
        super::super::ENV_DIR_OVERRIDE_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn flow_with_fallback() -> Flow {
        let mut vars = HashMap::new();
        vars.insert("SHARED".to_string(), serde_json::json!("from-flow"));
        vars.insert("FLOW_ONLY".to_string(), serde_json::json!("flow-val"));
        Flow {
            uuid: "cli-vars-test".to_string(),
            name: "CLI Vars Test".to_string(),
            description: None,
            initial_variables: Some(vars),
            nodes: vec![],
            connections: vec![],
            hooks: None,
        }
    }

    fn write_env(dir: &std::path::Path, name: &str, is_local: bool, vars: &[(&str, &str)]) {
        let env = EnvironmentSet {
            name: name.to_string(),
            description: None,
            variables: vars
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
                .collect(),
        };
        flow_engine::storage::save_environment_file(&env, is_local, dir)
            .unwrap_or_else(|e| panic!("failed to write env file: {e}"));
    }

    fn as_obj(value: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
        value
            .as_object()
            .unwrap_or_else(|| panic!("expected object, got {value}"))
    }

    #[test]
    fn resolve_layers_flow_env_local_and_runtime() {
        let _guard = lock_env_dir();
        let temp = tempfile::TempDir::new().unwrap_or_else(|e| panic!("{e}"));
        flow_engine::storage::set_environments_dir_override(Some(temp.path().to_path_buf()));

        write_env(
            temp.path(),
            "dev",
            false,
            &[("SHARED", "from-dev"), ("DEV_ONLY", "d")],
        );
        write_env(
            temp.path(),
            "dev",
            true,
            &[("SHARED", "from-local"), ("SECRET", "s")],
        );

        let mut runtime = HashMap::new();
        runtime.insert("SHARED".to_string(), serde_json::json!("from-runtime"));
        let result =
            resolve_execution_variables(&flow_with_fallback(), Some("dev"), Some(&runtime))
                .unwrap_or_else(|e| panic!("{e}"));

        flow_engine::storage::set_environments_dir_override(None);

        let obj = as_obj(result.as_ref().unwrap_or_else(|| panic!("expected some")));
        assert_eq!(obj["SHARED"], serde_json::json!("from-runtime"));
        assert_eq!(obj["FLOW_ONLY"], serde_json::json!("flow-val"));
        assert_eq!(obj["DEV_ONLY"], serde_json::json!("d"));
        assert_eq!(obj["SECRET"], serde_json::json!("s"));
    }

    #[test]
    fn resolve_without_env_keeps_flow_fallback_only() {
        let _guard = lock_env_dir();
        let result = resolve_execution_variables(&flow_with_fallback(), None, None)
            .unwrap_or_else(|e| panic!("{e}"));
        let obj = as_obj(result.as_ref().unwrap_or_else(|| panic!("expected some")));
        assert_eq!(obj["SHARED"], serde_json::json!("from-flow"));
        assert_eq!(obj.len(), 2);
    }

    #[test]
    fn resolve_missing_env_is_an_error() {
        let _guard = lock_env_dir();
        let temp = tempfile::TempDir::new().unwrap_or_else(|e| panic!("{e}"));
        flow_engine::storage::set_environments_dir_override(Some(temp.path().to_path_buf()));

        let err = resolve_execution_variables(&flow_with_fallback(), Some("nope"), None)
            .err()
            .unwrap_or_else(|| panic!("expected error for missing environment"));

        flow_engine::storage::set_environments_dir_override(None);

        assert!(
            matches!(err, CliError::Execution { .. }),
            "expected execution error, got {err:?}"
        );
    }

    #[test]
    fn parse_vars_keeps_json_types() {
        let parsed =
            parse_vars(&["n=1".to_string(), "s=abc".to_string()]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(parsed["n"], serde_json::json!(1));
        assert_eq!(parsed["s"], serde_json::json!("abc"));

        assert!(parse_vars(&["broken".to_string()]).is_err());
    }
}
