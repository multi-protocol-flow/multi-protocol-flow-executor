//! 配置 → 可执行命令的解析
//!
//! 核心安全语义（见 `docs/local-exec.md` §3.4）：
//!
//! | 字段 | 是否做变量替换 |
//! |------|---------------|
//! | `program` | 是（替换结果是可执行文件路径，不经 shell） |
//! | `args[]` | 是，**逐参数**替换，替换结果作为单个参数传递，不拆词 |
//! | `env` 的值 | 是（注入为 `MPE_VAR_<NAME>`） |
//! | `cwd` / `capture.path` | 是 |
//! | `script` | **否**，含 `{{` 直接拒绝 |

use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::executor::replace_variables;
use flow_engine_core::node_config::{ExecMode, LocalExecConfig};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// 默认节点超时（毫秒）
pub const DEFAULT_TIMEOUT_MS: u64 = 600_000;
/// 默认 stdout / stderr 上限（1 MiB）
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1_048_576;
/// stdout / stderr 上限的最大值（16 MiB）
pub const MAX_OUTPUT_BYTES: usize = 16 * 1_048_576;
/// 默认产物读取上限（4 MiB，与媒体预览 V1 阈值一致）
pub const DEFAULT_CAPTURE_MAX_BYTES: usize = 4 * 1_048_576;
/// 产物读取上限的最大值（64 MiB）
pub const MAX_CAPTURE_BYTES: usize = 64 * 1_048_576;
/// `args` 数量上限
pub const MAX_ARGS: usize = 256;
/// `script` 长度上限（64 KiB）
pub const MAX_SCRIPT_BYTES: usize = 65_536;

/// 注入环境变量名前缀
pub const ENV_VAR_PREFIX: &str = "MPE_VAR_";

/// 默认继承的环境变量白名单（其余环境变量一律不传给子进程）
pub const INHERITED_ENV: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
];

/// 解析后的待执行命令
#[derive(Debug, Clone)]
pub struct ResolvedCommand {
    /// 可执行文件绝对路径
    pub program: PathBuf,
    /// 参数（argv 模式为用户参数；shell 模式为 shell flag + 脚本）
    pub args: Vec<String>,
    /// 是否 shell 模式
    pub shell_mode: bool,
    /// 展示用命令行（可执行文件绝对路径 + 参数，不含环境变量值）
    pub display: String,
    /// 工作目录
    pub cwd: PathBuf,
    /// 子进程环境变量（已清洗）
    pub env: Vec<(String, String)>,
    /// 注入的变量名（`MPE_VAR_*`，不含值）
    pub env_names: Vec<String>,
    /// 额外继承的宿主环境变量名（审批弹窗展示，取值不外泄）
    pub inherit_env_names: Vec<String>,
    /// 超时（毫秒）
    pub timeout_ms: u64,
    /// stdout / stderr 各自上限
    pub max_output_bytes: usize,
    /// 产物捕获（路径 + 上限）
    pub capture: Option<(PathBuf, usize)>,
    /// `cwd` 模板原文（参与命令哈希）
    pub cwd_template: String,
}

/// 环境变量名规范化：大写、非 `[A-Z0-9]` → `_`，并加 `MPE_VAR_` 前缀
pub fn normalize_env_name(key: &str) -> String {
    let mut name = String::with_capacity(ENV_VAR_PREFIX.len() + key.len());
    name.push_str(ENV_VAR_PREFIX);
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_uppercase());
        } else {
            name.push('_');
        }
    }
    name
}

fn config_error(message: impl Into<String>) -> ErrorDetail {
    ErrorDetail::new(ErrorKind::Configuration, message.into())
        .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
}

fn spawn_error(program: &str, message: impl Into<String>) -> ErrorDetail {
    let message = message.into();
    ErrorDetail::new(ErrorKind::Execution, message.clone())
        .with_error_code_typed(ErrorCode::E_EXEC_SPAWN_FAILED)
        .with_i18n_arg("program", program.to_string())
        .with_i18n_arg("message", message)
}

/// 解析配置为可执行命令
///
/// 只做解析与清洗，**不做策略/审批校验**（那是 [`crate::exec_plugin`] 的职责）。
pub fn build_command(
    cfg: &LocalExecConfig,
    context: &ExecutionContext,
) -> Result<ResolvedCommand, ErrorDetail> {
    let (env, env_names, inherit_env_names) = build_env(cfg, context)?;
    let path_env = env
        .iter()
        .find(|(key, _)| key == "PATH")
        .map(|(_, v)| v.as_str());

    // cwd 必须在解析 program 之前确定：相对路径的 program 以 cwd 为基准
    // （与 args / capture 的相对路径语义保持一致）
    let outer_cwd_template = cfg.cwd.clone().unwrap_or_default();
    let cwd = resolve_cwd(cfg, context)?;

    let (program, args, display, shell_mode) = match cfg.mode {
        ExecMode::Argv => build_argv(cfg, context, path_env, &cwd)?,
        ExecMode::Shell => build_shell(cfg, path_env)?,
    };

    let timeout_ms = cfg
        .timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(1, crate::policy::MAX_TIMEOUT_MS);
    let max_output_bytes = cfg
        .max_output_bytes
        .unwrap_or(DEFAULT_MAX_OUTPUT_BYTES)
        .clamp(1, MAX_OUTPUT_BYTES);

    let capture = match cfg.capture.as_ref() {
        Some(capture_cfg) => {
            let replaced = replace_variables(&capture_cfg.path, context)
                .map_err(|e| config_error(format!("capture.path: {e}")))?;
            let path = context.resolve_file_path(replaced.trim());
            let max_bytes = capture_cfg
                .max_bytes
                .unwrap_or(DEFAULT_CAPTURE_MAX_BYTES)
                .clamp(1, MAX_CAPTURE_BYTES);
            Some((path, max_bytes))
        }
        None => None,
    };

    Ok(ResolvedCommand {
        program,
        args,
        shell_mode,
        display,
        cwd,
        env,
        env_names,
        inherit_env_names,
        timeout_ms,
        max_output_bytes,
        capture,
        cwd_template: outer_cwd_template,
    })
}

fn build_argv(
    cfg: &LocalExecConfig,
    context: &ExecutionContext,
    path_env: Option<&str>,
    cwd: &Path,
) -> Result<(PathBuf, Vec<String>, String, bool), ErrorDetail> {
    let program_template = cfg
        .program
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if program_template.is_empty() {
        return Err(config_error("program is required for argv mode"));
    }
    let resolved_program = replace_variables(&program_template, context)
        .map_err(|e| config_error(format!("program: {e}")))?;
    let program = resolve_executable_in(&resolved_program, path_env.map(OsStr::new), Some(cwd))
        .ok_or_else(|| spawn_error(&resolved_program, "executable not found"))?;

    let mut args = Vec::new();
    for template in cfg.args.as_deref().unwrap_or_default() {
        let value = replace_variables(template, context)
            .map_err(|e| config_error(format!("args: {e}")))?;
        args.push(value);
    }
    if args.len() > MAX_ARGS {
        return Err(config_error(format!(
            "too many args ({} > {MAX_ARGS})",
            args.len()
        )));
    }

    let mut display = program.to_string_lossy().to_string();
    for arg in &args {
        display.push(' ');
        display.push_str(arg);
    }
    Ok((program, args, display, false))
}

fn build_shell(
    cfg: &LocalExecConfig,
    path_env: Option<&str>,
) -> Result<(PathBuf, Vec<String>, String, bool), ErrorDetail> {
    let script = cfg.script.as_deref().unwrap_or_default();
    if script.trim().is_empty() {
        return Err(config_error("script is required for shell mode"));
    }
    // 防御性二次校验：validate 已拒绝，这里再挡一层（配置可能来自未经校验的路径）
    if script.contains("{{") {
        return Err(config_error(
            "script must not contain {{ }} templates; use MPE_VAR_* environment variables instead",
        ));
    }
    if script.len() > MAX_SCRIPT_BYTES {
        return Err(config_error(format!(
            "script too long ({} > {MAX_SCRIPT_BYTES} bytes)",
            script.len()
        )));
    }

    let shell_name = cfg
        .shell
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(default_shell_name);

    let (executable, flags) = shell_invocation(&shell_name)?;
    let program = resolve_executable(executable, path_env.map(OsStr::new))
        .ok_or_else(|| spawn_error(executable, "shell executable not found"))?;

    let mut args: Vec<String> = flags.iter().map(|f| (*f).to_string()).collect();
    args.push(script.to_string());

    let mut display = program.to_string_lossy().to_string();
    for part in &args {
        display.push(' ');
        display.push_str(part);
    }
    Ok((program, args, display, true))
}

fn default_shell_name() -> String {
    if cfg!(windows) {
        "cmd".to_string()
    } else {
        "sh".to_string()
    }
}

/// shell 名称 → (可执行文件, 前置 flag)
fn shell_invocation(shell: &str) -> Result<(&'static str, Vec<&'static str>), ErrorDetail> {
    let (executable, flags): (&'static str, Vec<&'static str>) =
        match shell.to_ascii_lowercase().as_str() {
            "bash" => ("bash", vec!["-c"]),
            "sh" => ("sh", vec!["-c"]),
            "cmd" => ("cmd.exe", vec!["/C"]),
            "powershell" => (
                "powershell.exe",
                vec!["-NoProfile", "-NonInteractive", "-Command"],
            ),
            "pwsh" => ("pwsh", vec!["-NoProfile", "-NonInteractive", "-Command"]),
            other => {
                return Err(config_error(format!(
                    "unsupported shell `{other}` (expected bash | sh | cmd | powershell | pwsh)"
                )));
            }
        };
    Ok((executable, flags))
}

/// 判定 shell 名是否受支持（供 `validate` 复用）
pub fn is_supported_shell(shell: &str) -> bool {
    shell_invocation(shell).is_ok()
}

fn resolve_cwd(cfg: &LocalExecConfig, context: &ExecutionContext) -> Result<PathBuf, ErrorDetail> {
    let template = cfg.cwd.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match template {
        Some(template) => {
            let replaced = replace_variables(template, context)
                .map_err(|e| config_error(format!("cwd: {e}")))?;
            let resolved = context.resolve_file_path(&replaced);
            let canonical = std::fs::canonicalize(&resolved).map_err(|e| {
                config_error(format!(
                    "cwd `{}` is not accessible: {e}",
                    resolved.display()
                ))
            })?;
            if !canonical.is_dir() {
                return Err(config_error(format!(
                    "cwd `{}` is not a directory",
                    canonical.display()
                )));
            }
            Ok(canonical)
        }
        None => {
            if let Some(flow_path) = context.flow_file_path.as_deref() {
                if let Some(parent) = flow_path.parent() {
                    let dir = if parent.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        parent
                    };
                    return std::fs::canonicalize(dir).map_err(|e| {
                        config_error(format!(
                            "flow directory `{}` is not accessible: {e}",
                            dir.display()
                        ))
                    });
                }
            }
            std::env::current_dir()
                .map_err(|e| config_error(format!("cannot determine working directory: {e}")))
        }
    }
}

/// 构造清洗后的子进程环境
fn build_env(
    cfg: &LocalExecConfig,
    context: &ExecutionContext,
) -> Result<(Vec<(String, String)>, Vec<String>, Vec<String>), ErrorDetail> {
    let mut env: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut inherited_names: Vec<String> = Vec::new();

    // 白名单内的默认继承项不计入 `inherit_env_names`（用户没有额外授权）
    let default_inherited: HashSet<String> = INHERITED_ENV
        .iter()
        .map(|name| name.to_ascii_uppercase())
        .collect();

    let extra_inherited = cfg.inherit_env.clone().unwrap_or_default();
    let inherited = INHERITED_ENV
        .iter()
        .copied()
        .chain(extra_inherited.iter().map(|s| s.as_str()));
    for name in inherited {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        // 宿主内部变量一律不继承（避免配置/密钥泄漏）
        if name.to_ascii_uppercase().starts_with("MPE_") {
            continue;
        }
        let key = name.to_ascii_uppercase();
        if !seen.insert(key.clone()) {
            continue;
        }
        if let Ok(value) = std::env::var(name) {
            env.push((name.to_string(), value));
            if !default_inherited.contains(&key) {
                inherited_names.push(key);
            }
        }
    }

    let mut injected: Vec<String> = Vec::new();
    if let Some(map) = cfg.env.as_ref() {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        let mut normalized_seen: HashSet<String> = HashSet::new();
        for key in keys {
            let name = normalize_env_name(key);
            if !normalized_seen.insert(name.clone()) {
                return Err(config_error(format!(
                    "env keys collide after normalization: `{name}`"
                )));
            }
            let template = map.get(key).map(String::as_str).unwrap_or_default();
            let value = replace_variables(template, context)
                .map_err(|e| config_error(format!("env.{key}: {e}")))?;
            env.push((name.clone(), value));
            injected.push(name);
        }
    }

    Ok((env, injected, inherited_names))
}

/// 解析可执行文件
///
/// - 含路径分隔符 → 直接判定该路径
/// - 否则在 `path_env` 的每个目录下按候选名查找（Windows 追加 `PATHEXT`）
pub fn resolve_executable(program: &str, path_env: Option<&OsStr>) -> Option<PathBuf> {
    resolve_executable_in(program, path_env, None)
}

/// 解析可执行文件（`base` 为相对路径的基准目录）
///
/// 带分隔符的相对路径（如 `bin/ffmpeg`）以 `base` 为基准，与 `cwd` /
/// `capture.path` 的相对路径语义保持一致；`base` 为 `None` 时退回进程 CWD。
pub fn resolve_executable_in(
    program: &str,
    path_env: Option<&OsStr>,
    base: Option<&Path>,
) -> Option<PathBuf> {
    let program = program.trim();
    if program.is_empty() {
        return None;
    }
    if program.contains('/') || program.contains('\\') {
        let raw = Path::new(program);
        let path = match base {
            Some(base) if raw.is_relative() => base.join(raw),
            _ => raw.to_path_buf(),
        };
        return is_executable_file(&path)
            .then(|| std::fs::canonicalize(&path).unwrap_or(path));
    }

    let path_env = path_env?;
    for dir in std::env::split_paths(path_env) {
        let dir = if dir.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            dir
        };
        for candidate in executable_candidates(program) {
            let candidate_path = dir.join(&candidate);
            if is_executable_file(&candidate_path) {
                return Some(std::fs::canonicalize(&candidate_path).unwrap_or(candidate_path));
            }
        }
    }
    None
}

#[cfg(windows)]
fn executable_candidates(program: &str) -> Vec<String> {
    let mut candidates = vec![program.to_string()];
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let has_known_ext = pathext
        .split(';')
        .filter(|ext| !ext.trim().is_empty())
        .any(|ext| {
            program
                .to_ascii_lowercase()
                .ends_with(&ext.trim().to_ascii_lowercase())
        });
    if !has_known_ext {
        for ext in pathext.split(';').filter(|ext| !ext.trim().is_empty()) {
            candidates.push(format!("{program}{}", ext.trim().to_ascii_lowercase()));
        }
    }
    candidates
}

#[cfg(not(windows))]
fn executable_candidates(program: &str) -> Vec<String> {
    vec![program.to_string()]
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && (meta.permissions().mode() & 0o111) != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::{ExecCaptureConfig, ExecMode};
    use std::collections::HashMap;

    fn ctx() -> ExecutionContext {
        ExecutionContext::new("flow-1".to_string(), 0)
    }

    fn base_cfg() -> LocalExecConfig {
        LocalExecConfig {
            mode: ExecMode::Argv,
            program: Some("/bin/echo".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn normalize_env_name_uppercases_and_prefixes() {
        assert_eq!(normalize_env_name("video_url"), "MPE_VAR_VIDEO_URL");
        assert_eq!(normalize_env_name("a-b.c"), "MPE_VAR_A_B_C");
        assert_eq!(normalize_env_name("X"), "MPE_VAR_X");
    }

    #[test]
    fn argv_mode_resolves_program_and_args() {
        let mut context = ctx();
        context.variables.set_variable(
            "x".to_string(),
            serde_json::json!("a b; rm -rf /"),
            String::new(),
        );
        let cfg = LocalExecConfig {
            args: Some(vec!["{{x}}".to_string(), "lit".to_string()]),
            ..base_cfg()
        };
        let resolved = build_command(&cfg, &context).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            resolved.args,
            vec!["a b; rm -rf /".to_string(), "lit".to_string()]
        );
        assert!(!resolved.shell_mode);
        assert!(resolved.program.is_absolute());
        assert!(resolved.display.contains("/bin/echo") || resolved.display.contains("echo"));
    }

    #[test]
    fn shell_mode_rejects_templates() {
        let cfg = LocalExecConfig {
            mode: ExecMode::Shell,
            program: None,
            shell: Some("bash".to_string()),
            script: Some("echo {{x}}".to_string()),
            ..Default::default()
        };
        let err = build_command(&cfg, &ctx()).expect_err("template must be rejected");
        assert_eq!(err.error_code, Some(ErrorCode::E_CONFIG_ERROR));
        assert!(err.message.contains("MPE_VAR_"));
    }

    #[test]
    fn shell_mode_builds_flags() {
        let cfg = LocalExecConfig {
            mode: ExecMode::Shell,
            program: None,
            shell: Some("bash".to_string()),
            script: Some("echo $MPE_VAR_X".to_string()),
            env: Some(HashMap::from([("x".to_string(), "hello".to_string())])),
            ..Default::default()
        };
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(resolved.shell_mode);
        assert_eq!(
            resolved.args.last().map(String::as_str),
            Some("echo $MPE_VAR_X")
        );
        assert!(resolved.args.contains(&"-c".to_string()));
        assert_eq!(resolved.env_names, vec!["MPE_VAR_X".to_string()]);
        assert!(resolved
            .env
            .iter()
            .any(|(k, v)| k == "MPE_VAR_X" && v == "hello"));
    }

    #[test]
    fn unknown_shell_is_rejected() {
        let cfg = LocalExecConfig {
            mode: ExecMode::Shell,
            program: None,
            shell: Some("zshh".to_string()),
            script: Some("true".to_string()),
            ..Default::default()
        };
        let err = build_command(&cfg, &ctx()).expect_err("unknown shell must be rejected");
        assert!(err.message.contains("unsupported shell"));
    }

    #[test]
    fn env_is_cleaned_of_host_mpe_variables() {
        // SAFETY: 单测内设置进程级环境变量，键名唯一避免与并行测试冲突
        std::env::set_var("MPE_SHOULD_NOT_LEAK", "1");
        let resolved = build_command(&base_cfg(), &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(!resolved.env.iter().any(|(k, _)| k == "MPE_SHOULD_NOT_LEAK"));
        std::env::remove_var("MPE_SHOULD_NOT_LEAK");
    }

    #[test]
    fn inherit_env_skips_mpe_prefix() {
        let cfg = LocalExecConfig {
            inherit_env: Some(vec!["MPE_SECRET".to_string(), "HOME".to_string()]),
            ..base_cfg()
        };
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(!resolved.env.iter().any(|(k, _)| k == "MPE_SECRET"));
        assert_eq!(
            resolved.env.iter().filter(|(k, _)| k == "HOME").count(),
            usize::from(std::env::var("HOME").is_ok())
        );
    }

    #[test]
    fn env_key_collision_is_rejected() {
        let cfg = LocalExecConfig {
            env: Some(HashMap::from([
                ("a-b".to_string(), "1".to_string()),
                ("a_b".to_string(), "2".to_string()),
            ])),
            ..base_cfg()
        };
        let err = build_command(&cfg, &ctx()).expect_err("collision must be rejected");
        assert!(err.message.contains("collide"));
    }

    #[test]
    fn capture_path_is_resolved() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let cfg = LocalExecConfig {
            capture: Some(ExecCaptureConfig {
                path: dir.path().join("out.mp4").to_string_lossy().to_string(),
                max_bytes: Some(1024),
            }),
            ..base_cfg()
        };
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        let (path, max_bytes) = resolved.capture.expect("capture must be present");
        assert_eq!(max_bytes, 1024);
        assert!(path.to_string_lossy().ends_with("out.mp4"));
    }

    #[test]
    fn timeout_and_output_are_clamped() {
        let cfg = LocalExecConfig {
            timeout_ms: Some(u64::MAX),
            max_output_bytes: Some(usize::MAX),
            ..base_cfg()
        };
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(resolved.timeout_ms, crate::policy::MAX_TIMEOUT_MS);
        assert_eq!(resolved.max_output_bytes, MAX_OUTPUT_BYTES);
    }

    #[test]
    fn missing_program_is_config_error() {
        let cfg = LocalExecConfig {
            program: None,
            ..base_cfg()
        };
        let err = build_command(&cfg, &ctx()).expect_err("program required");
        assert_eq!(err.error_code, Some(ErrorCode::E_CONFIG_ERROR));
    }

    #[test]
    fn unresolvable_program_is_spawn_error() {
        let cfg = LocalExecConfig {
            program: Some("definitely-not-a-real-binary-xyz".to_string()),
            ..base_cfg()
        };
        let err = build_command(&cfg, &ctx()).expect_err("must not resolve");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_SPAWN_FAILED));
    }

    #[test]
    fn relative_program_resolves_against_cwd() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let bin = dir.path().join("hello.sh");
        std::fs::write(&bin, "#!/bin/sh\necho hi\n").unwrap_or_else(|e| panic!("{e}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let mut cfg = base_cfg();
        cfg.program = Some("./hello.sh".to_string());
        cfg.cwd = Some(dir.path().to_string_lossy().to_string());
        cfg.args = Some(Vec::new());
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(resolved.program, std::fs::canonicalize(&bin).unwrap_or(bin));
    }

    #[test]
    fn inherit_env_names_lists_only_extra_variables() {
        std::env::set_var("SPEC_TEST_EXTRA", "secret-value");
        let mut cfg = base_cfg();
        cfg.program = Some("/bin/echo".to_string());
        cfg.args = Some(vec!["hi".to_string()]);
        cfg.inherit_env = Some(vec![
            "SPEC_TEST_EXTRA".to_string(),
            "PATH".to_string(),
            "MPE_INTERNAL_SHOULD_SKIP".to_string(),
        ]);
        let resolved = build_command(&cfg, &ctx()).unwrap_or_else(|e| panic!("{e:?}"));
        std::env::remove_var("SPEC_TEST_EXTRA");
        // 只有用户显式追加的变量进入审批展示；默认白名单与 MPE_* 不计入
        assert_eq!(resolved.inherit_env_names, vec!["SPEC_TEST_EXTRA".to_string()]);
        let keys: Vec<&str> = resolved.env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(!keys.contains(&"MPE_INTERNAL_SHOULD_SKIP"));
    }

    #[test]
    fn resolve_executable_finds_sh() {
        let path = std::env::var("PATH").unwrap_or_default();
        let found = resolve_executable("sh", Some(OsStr::new(&path)));
        assert!(found.is_some_and(|p| p.is_absolute()));
    }
}
