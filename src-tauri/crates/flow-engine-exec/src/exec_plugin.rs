//! `local:exec` 节点插件

use crate::approval::{command_hash, ExecApprovalRequest};
use crate::media::capture_media;
use crate::policy::{ApprovalMode, ExecPolicy, EXEC_POLICY_PROVIDER};
use crate::process;
use crate::spec::{self, MAX_ARGS, MAX_CAPTURE_BYTES, MAX_OUTPUT_BYTES, MAX_SCRIPT_BYTES};
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{ExecMode, LocalExecConfig, NodeData};
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// 本地命令节点插件
///
/// 端口：`true`（exit code 0）/ `false`（非 0 或执行失败）。
/// 非 0 退出走 `Ok` + `false` 端口（保留 stdout/stderr 供 viewer 展示），
/// executor 对 `used_port_id == "false"` 已按失败上报并从 `output_data.error` 取错误。
pub struct LocalExecPlugin;

#[async_trait::async_trait]
impl NodePlugin for LocalExecPlugin {
    fn type_id(&self) -> &str {
        flow_engine_core::node_types::LOCAL_EXEC
    }

    fn display_name(&self) -> &str {
        "Local Command"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "terminal"
    }

    fn color(&self) -> &str {
        "#0EA5E9"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".to_string(),
                label: "Success".to_string(),
                position: "right".to_string(),
            },
            PortDef {
                id: "false".to_string(),
                label: "Failed".to_string(),
                position: "bottom".to_string(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        json!({
            "type": "local:exec",
            "mode": "argv",
            "program": "",
            "args": [],
            "env": {},
            "timeout_ms": spec::DEFAULT_TIMEOUT_MS,
            "max_output_bytes": spec::DEFAULT_MAX_OUTPUT_BYTES
        })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map))
                if map.get("success") == Some(&serde_json::Value::Bool(true)) =>
            {
                "true".to_string()
            }
            _ => "false".to_string(),
        }
    }

    fn is_port_routing(&self) -> bool {
        true
    }

    /// 报告输入只回结构字段，**绝不回环境变量值**（可能含密钥）
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        let env_names: Vec<String> = config
            .get("env")
            .and_then(|v| v.as_object())
            .map(|map| map.keys().map(|k| spec::normalize_env_name(k)).collect())
            .unwrap_or_default();
        Some(json!({
            "mode": config.get("mode").cloned().unwrap_or(json!("argv")),
            "program": config.get("program").cloned().unwrap_or(json!(null)),
            "args": config.get("args").cloned().unwrap_or(json!([])),
            "shell": config.get("shell").cloned().unwrap_or(json!(null)),
            "script": config.get("script").cloned().unwrap_or(json!(null)),
            "env_names": env_names,
            "cwd": config.get("cwd").cloned().unwrap_or(json!(null)),
            "timeout_ms": config.get("timeout_ms").cloned().unwrap_or(json!(spec::DEFAULT_TIMEOUT_MS)),
            "max_output_bytes": config
                .get("max_output_bytes")
                .cloned()
                .unwrap_or(json!(spec::DEFAULT_MAX_OUTPUT_BYTES)),
            "capture": config.get("capture").cloned().unwrap_or(json!(null)),
        }))
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let cfg: LocalExecConfig = match serde_json::from_value(config.clone()) {
            Ok(cfg) => cfg,
            Err(e) => {
                return vec![ValidationError {
                    field: "config".to_string(),
                    message: e.to_string(),
                }];
            }
        };

        let mut errors = Vec::new();
        match cfg.mode {
            ExecMode::Argv => {
                if cfg
                    .program
                    .as_deref()
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .is_none()
                {
                    errors.push(ValidationError {
                        field: "program".to_string(),
                        message: "program is required for argv mode".to_string(),
                    });
                }
                if let Some(args) = cfg.args.as_ref() {
                    if args.len() > MAX_ARGS {
                        errors.push(ValidationError {
                            field: "args".to_string(),
                            message: format!("too many args ({} > {MAX_ARGS})", args.len()),
                        });
                    }
                }
            }
            ExecMode::Shell => {
                let script = cfg.script.as_deref().unwrap_or_default();
                if script.trim().is_empty() {
                    errors.push(ValidationError {
                        field: "script".to_string(),
                        message: "script is required for shell mode".to_string(),
                    });
                } else {
                    if script.contains("{{") {
                        errors.push(ValidationError {
                            field: "script".to_string(),
                            message:
                                "script must not contain {{ }} templates; use MPE_VAR_* environment variables instead"
                                    .to_string(),
                        });
                    }
                    if script.len() > MAX_SCRIPT_BYTES {
                        errors.push(ValidationError {
                            field: "script".to_string(),
                            message: format!(
                                "script too long ({} > {MAX_SCRIPT_BYTES} bytes)",
                                script.len()
                            ),
                        });
                    }
                }
                if let Some(shell) = cfg
                    .shell
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    if !spec::is_supported_shell(shell) {
                        errors.push(ValidationError {
                            field: "shell".to_string(),
                            message: format!(
                                "unsupported shell `{shell}` (expected bash | sh | cmd | powershell | pwsh)"
                            ),
                        });
                    }
                }
            }
        }

        if let Some(timeout_ms) = cfg.timeout_ms {
            if timeout_ms == 0 || timeout_ms > crate::policy::MAX_TIMEOUT_MS {
                errors.push(ValidationError {
                    field: "timeout_ms".to_string(),
                    message: format!(
                        "timeout_ms must be between 1 and {}",
                        crate::policy::MAX_TIMEOUT_MS
                    ),
                });
            }
        }
        if let Some(max_output_bytes) = cfg.max_output_bytes {
            if max_output_bytes == 0 || max_output_bytes > MAX_OUTPUT_BYTES {
                errors.push(ValidationError {
                    field: "max_output_bytes".to_string(),
                    message: format!("max_output_bytes must be between 1 and {MAX_OUTPUT_BYTES}"),
                });
            }
        }
        if let Some(capture) = cfg.capture.as_ref() {
            if capture.path.trim().is_empty() {
                errors.push(ValidationError {
                    field: "capture.path".to_string(),
                    message: "capture.path is required".to_string(),
                });
            }
            if let Some(max_bytes) = capture.max_bytes {
                if max_bytes == 0 || max_bytes > MAX_CAPTURE_BYTES {
                    errors.push(ValidationError {
                        field: "capture.max_bytes".to_string(),
                        message: format!(
                            "capture.max_bytes must be between 1 and {MAX_CAPTURE_BYTES}"
                        ),
                    });
                }
            }
        }
        if let Some(env) = cfg.env.as_ref() {
            let mut seen: HashSet<String> = HashSet::new();
            let mut keys: Vec<&String> = env.keys().collect();
            keys.sort();
            for key in keys {
                let normalized = spec::normalize_env_name(key);
                if !seen.insert(normalized.clone()) {
                    errors.push(ValidationError {
                        field: "env".to_string(),
                        message: format!("env keys collide after normalization: `{normalized}`"),
                    });
                }
            }
        }

        errors
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let cfg = match &node.data {
            NodeData::LocalExec(cfg) => cfg.clone(),
            other => {
                return Err(config_error(format!(
                    "local:exec node has unexpected config type `{}`",
                    other.type_name()
                )));
            }
        };

        // 1. 策略闸门：provider 缺失 == 未启用
        let policy = context
            .get_provider::<ExecPolicy>(EXEC_POLICY_PROVIDER)
            .ok_or_else(|| {
                denied(
                    "",
                    "local command execution is not enabled in this environment (start with --allow-exec, or enable Settings → Local Command)",
                )
            })?;
        if !policy.enabled {
            return Err(denied(
                "",
                "local command execution is disabled (enable it in Settings → Local Command, or pass --allow-exec)",
            ));
        }

        // 2. 解析命令（变量替换、环境清洗、路径解析）
        let mut cmd = spec::build_command(&cfg, context)?;
        if policy.max_timeout_ms > 0 {
            cmd.timeout_ms = cmd.timeout_ms.min(policy.max_timeout_ms);
        }

        // 3. shell 许可
        if cmd.shell_mode && !policy.allow_shell {
            return Err(denied(
                &cmd.program.to_string_lossy(),
                "shell mode is not allowed (enable it in Settings → Local Command, or pass --allow-shell)",
            ));
        }

        // 4. 程序白名单
        if !policy.is_program_allowed(&cmd.program) {
            return Err(denied(
                &cmd.program.to_string_lossy(),
                format!(
                    "program `{}` is not in the allowed list",
                    cmd.program.display()
                ),
            ));
        }

        // 5. 审批
        if policy.approval == ApprovalMode::TrustStore {
            let provider = policy.approval_provider.clone().ok_or_else(|| {
                denied(
                    &cmd.program.to_string_lossy(),
                    "approval is required but no approval provider is available",
                )
            })?;
            let flow_scope = approval_scope(context);
            let request = ExecApprovalRequest {
                request_id: uuid::Uuid::new_v4().to_string(),
                flow_scope: flow_scope.clone(),
                flow_name: flow_name(context),
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                command_display: cmd.display.clone(),
                cwd: cmd.cwd.to_string_lossy().to_string(),
                env_names: cmd.env_names.clone(),
                inherit_env_names: cmd.inherit_env_names.clone(),
                shell_mode: cmd.shell_mode,
                command_hash: command_hash(
                    &flow_scope,
                    node,
                    &cfg,
                    &cmd.program,
                    &cmd.cwd_template,
                ),
            };
            if !provider.request_approval(request).await {
                return Err(denied(
                    &cmd.program.to_string_lossy(),
                    "command execution was not approved",
                ));
            }
        }

        // 6. 执行
        let capture = cmd.capture.clone();
        let shell_mode = cmd.shell_mode;
        let display = cmd.display.clone();
        let cwd = cmd.cwd.to_string_lossy().to_string();
        let outcome = process::run_command(cmd, context.cancel_receiver.clone()).await?;

        let success = outcome.exit_code == Some(0);
        let mut output = json!({
            "success": success,
            "exit_code": outcome.exit_code,
            "duration_ms": outcome.duration_ms,
            "stdout": outcome.stdout,
            "stderr": outcome.stderr,
            "stdout_truncated": outcome.stdout_truncated,
            "stderr_truncated": outcome.stderr_truncated,
            "command": {
                "mode": if shell_mode { "shell" } else { "argv" },
                "display": display,
            },
            "cwd": cwd,
        });

        if !success {
            let code = outcome
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string());
            let tail = tail_chars(&outcome.stderr, 2000);
            output["error"] = json!(format!("exit code {code}: {tail}"));
        }

        // 7. 产物捕获
        if success {
            if let Some((path, max_bytes)) = capture {
                match capture_media(&path, max_bytes) {
                    Ok(media) => {
                        output["media"] = media;
                    }
                    Err(err) => {
                        return Err(err.with_context(output));
                    }
                }
            }
        }

        let port = if success { "true" } else { "false" };
        let next: Vec<String> = connections
            .iter()
            .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == port)
            .map(|c| c.target_node_uuid.clone())
            .collect();

        Ok((next, Some(output)))
    }
}

/// 信任范围：流程文件绝对路径；未保存流程退化为 `session:<flow_uuid>`
fn approval_scope(context: &ExecutionContext) -> String {
    match context.flow_file_path.as_deref() {
        Some(path) => std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string(),
        None => format!("session:{}", context.flow_uuid),
    }
}

fn flow_name(context: &ExecutionContext) -> String {
    context
        .flow_file_path
        .as_deref()
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| context.flow_uuid.clone())
}

fn tail_chars(text: &str, limit: usize) -> String {
    let total = text.chars().count();
    if total <= limit {
        return text.to_string();
    }
    text.chars().skip(total - limit).collect()
}

fn config_error(message: String) -> ErrorDetail {
    ErrorDetail::new(ErrorKind::Configuration, message.clone())
        .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
        .with_i18n_arg("message", message)
}

fn denied(program: &str, message: impl Into<String>) -> ErrorDetail {
    let message = message.into();
    ErrorDetail::new(ErrorKind::Execution, message.clone())
        .with_error_code_typed(ErrorCode::E_EXEC_DENIED)
        .with_i18n_arg("program", program.to_string())
        .with_i18n_arg("message", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::ExecApprovalProvider;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct AlwaysApprove;
    #[async_trait::async_trait]
    impl ExecApprovalProvider for AlwaysApprove {
        async fn request_approval(&self, _request: ExecApprovalRequest) -> bool {
            true
        }
    }

    struct AlwaysDeny;
    #[async_trait::async_trait]
    impl ExecApprovalProvider for AlwaysDeny {
        async fn request_approval(&self, _request: ExecApprovalRequest) -> bool {
            false
        }
    }

    fn node(cfg: LocalExecConfig) -> Node {
        Node {
            uuid: "exec-1".to_string(),
            node_type: "local:exec".to_string(),
            name: "ffmpeg".to_string(),
            data: NodeData::LocalExec(cfg),
            ..Default::default()
        }
    }

    fn conns(from_port: &str) -> Vec<Connection> {
        vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "exec-1".to_string(),
            source_port_id: from_port.to_string(),
            target_node_uuid: "next".to_string(),
            target_port_id: "in".to_string(),
        }]
    }

    fn argv_cfg(script: &str) -> LocalExecConfig {
        LocalExecConfig {
            mode: ExecMode::Argv,
            program: Some("/bin/sh".to_string()),
            args: Some(vec!["-c".to_string(), script.to_string()]),
            ..Default::default()
        }
    }

    async fn run(
        cfg: LocalExecConfig,
        policy: Option<ExecPolicy>,
        context: &mut ExecutionContext,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        if let Some(policy) = policy {
            context.set_provider(EXEC_POLICY_PROVIDER, Arc::new(policy));
        }
        LocalExecPlugin
            .execute(&node(cfg), context, &conns("true"), &HashMap::new())
            .await
    }

    fn enabled_policy() -> ExecPolicy {
        ExecPolicy {
            enabled: true,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn denies_without_policy_provider() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let err = run(argv_cfg("true"), None, &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
        assert!(err.message.contains("--allow-exec"));
    }

    #[tokio::test]
    async fn denies_when_disabled() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = ExecPolicy {
            enabled: false,
            ..enabled_policy()
        };
        let err = run(argv_cfg("true"), Some(policy), &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
    }

    #[tokio::test]
    async fn denies_when_program_not_whitelisted() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = ExecPolicy {
            allowed_programs: vec!["ffmpeg".to_string()],
            ..enabled_policy()
        };
        let err = run(argv_cfg("true"), Some(policy), &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
        assert!(err.message.contains("not in the allowed list"));
    }

    #[tokio::test]
    async fn denies_shell_mode_without_permission() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let cfg = LocalExecConfig {
            mode: ExecMode::Shell,
            program: None,
            shell: Some("bash".to_string()),
            script: Some("true".to_string()),
            ..Default::default()
        };
        let policy = ExecPolicy {
            allow_shell: false,
            ..enabled_policy()
        };
        let err = run(cfg, Some(policy), &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
        assert!(err.message.contains("shell mode"));
    }

    #[tokio::test]
    async fn approval_denied_blocks_execution() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = enabled_policy().with_approval_provider(Arc::new(AlwaysDeny));
        let err = run(argv_cfg("true"), Some(policy), &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
        assert!(err.message.contains("not approved"));
    }

    #[tokio::test]
    async fn approval_required_without_provider_is_denied() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let err = run(argv_cfg("true"), Some(enabled_policy()), &mut context)
            .await
            .expect_err("must deny");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_DENIED));
        assert!(err.message.contains("approval"));
    }

    #[tokio::test]
    async fn success_routes_true_port() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = enabled_policy().with_approval_provider(Arc::new(AlwaysApprove));
        let (next, output) = run(argv_cfg("printf hi"), Some(policy), &mut context)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        let output = output.expect("output");
        assert_eq!(output["success"], json!(true));
        assert_eq!(output["exit_code"], json!(0));
        assert_eq!(output["stdout"], json!("hi"));
        assert_eq!(output["command"]["mode"], json!("argv"));
        assert_eq!(next, vec!["next".to_string()]);
        assert_eq!(LocalExecPlugin.resolve_port(&Some(output)), "true");
    }

    #[tokio::test]
    async fn non_zero_exit_routes_false_port() {
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = enabled_policy().with_approval_provider(Arc::new(AlwaysApprove));
        let (next, output) = run(
            argv_cfg("printf boom >&2; exit 3"),
            Some(policy),
            &mut context,
        )
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
        let output = output.expect("output");
        assert_eq!(output["success"], json!(false));
        assert_eq!(output["exit_code"], json!(3));
        assert!(output["error"]
            .as_str()
            .is_some_and(|e| e.contains("exit code 3") && e.contains("boom")));
        // 传入的连接从 true 端口出发，非 0 时不应命中
        assert!(next.is_empty());
        assert_eq!(LocalExecPlugin.resolve_port(&Some(output)), "false");
    }

    #[tokio::test]
    async fn report_input_hides_env_values() {
        let config = json!({
            "type": "local:exec",
            "mode": "shell",
            "shell": "bash",
            "script": "echo $MPE_VAR_SECRET",
            "env": {"secret": "super-secret-value"}
        });
        let input = LocalExecPlugin
            .build_report_input(&config)
            .expect("report input");
        let rendered = input.to_string();
        assert!(!rendered.contains("super-secret-value"));
        assert_eq!(input["env_names"], json!(["MPE_VAR_SECRET"]));
    }

    #[tokio::test]
    async fn captured_artifact_is_returned_as_media() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let out = dir.path().join("out.png");
        let mut bytes = vec![0x89, 0x50, 0x4E, 0x47];
        bytes.extend_from_slice(&[0u8; 20]);
        std::fs::write(&out, &bytes).unwrap_or_else(|e| panic!("{e}"));

        let mut cfg = argv_cfg("true");
        cfg.capture = Some(flow_engine_core::node_config::ExecCaptureConfig {
            path: out.to_string_lossy().to_string(),
            max_bytes: Some(1024 * 1024),
        });

        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = enabled_policy().with_approval_provider(Arc::new(AlwaysApprove));
        let (_, output) = run(cfg, Some(policy), &mut context)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        let output = output.expect("output");
        assert_eq!(output["media"]["mime"], json!("image/png"));
        assert!(output["media"]["data_url"].is_string());
    }

    #[tokio::test]
    async fn missing_capture_file_fails_with_context() {
        let mut cfg = argv_cfg("printf ok");
        cfg.capture = Some(flow_engine_core::node_config::ExecCaptureConfig {
            path: "/definitely/not/here/out.mp4".to_string(),
            max_bytes: None,
        });
        let mut context = ExecutionContext::new("f".to_string(), 0);
        let policy = enabled_policy().with_approval_provider(Arc::new(AlwaysApprove));
        let err = run(cfg, Some(policy), &mut context)
            .await
            .expect_err("capture must fail");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_CAPTURE_FAILED));
        let ctx = err.context.expect("stdout context preserved");
        assert_eq!(ctx["stdout"], json!("ok"));
    }

    #[test]
    fn validate_rejects_script_template() {
        let errors = LocalExecPlugin.validate(&json!({
            "type": "local:exec",
            "mode": "shell",
            "shell": "bash",
            "script": "echo {{x}}"
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "script");
        assert!(errors[0].message.contains("MPE_VAR_"));
    }

    #[test]
    fn validate_requires_program_for_argv() {
        let errors = LocalExecPlugin.validate(&json!({
            "type": "local:exec",
            "mode": "argv",
            "program": "  "
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "program");
    }

    #[test]
    fn validate_default_config_only_requires_program() {
        // 新拖入的节点尚未配置 `program`，此时只应有这一条错误
        let errors = LocalExecPlugin.validate(&LocalExecPlugin.default_config());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "program");
    }

    #[test]
    fn validate_accepts_filled_argv_config() {
        let errors = LocalExecPlugin.validate(&json!({
            "type": "local:exec",
            "mode": "argv",
            "program": "/bin/echo",
            "args": ["hi"],
            "timeout_ms": 600000,
            "max_output_bytes": 1048576,
            "capture": {"path": "/tmp/out.mp4", "max_bytes": 4194304}
        }));
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn validate_rejects_unknown_shell_and_limits() {
        let errors = LocalExecPlugin.validate(&json!({
            "type": "local:exec",
            "mode": "shell",
            "shell": "zsh",
            "script": "true",
            "timeout_ms": 0,
            "max_output_bytes": 0,
            "capture": {"path": "", "max_bytes": 0}
        }));
        let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"shell"));
        assert!(fields.contains(&"timeout_ms"));
        assert!(fields.contains(&"max_output_bytes"));
        assert!(fields.contains(&"capture.path"));
        assert!(fields.contains(&"capture.max_bytes"));
    }

    #[test]
    fn validate_rejects_env_normalization_collision() {
        let errors = LocalExecPlugin.validate(&json!({
            "type": "local:exec",
            "mode": "argv",
            "program": "/bin/echo",
            "env": {"a-b": "1", "a_b": "2"}
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "env");
    }

    #[test]
    fn approval_is_requested_on_every_execution_when_hash_changes() {
        // 哈希稳定：同一模板同一流程 → 相同 hash（宿主据此跳过重复审批）
        let cfg = argv_cfg("printf hi");
        let n = node(cfg.clone());
        let context = ExecutionContext::new("f".to_string(), 0);
        let scope = approval_scope(&context);
        let a = command_hash(&scope, &n, &cfg, std::path::Path::new("/bin/sh"), "");
        let b = command_hash(&scope, &n, &cfg, std::path::Path::new("/bin/sh"), "");
        assert_eq!(a, b);

        let mut changed = cfg.clone();
        changed.args = Some(vec!["-c".to_string(), "printf bye".to_string()]);
        let c = command_hash(&scope, &n, &changed, std::path::Path::new("/bin/sh"), "");
        assert_ne!(a, c);
    }

    #[test]
    fn approval_provider_is_invoked_once_per_execution() {
        struct Counting(Arc<AtomicBool>);
        #[async_trait::async_trait]
        impl ExecApprovalProvider for Counting {
            async fn request_approval(&self, _request: ExecApprovalRequest) -> bool {
                self.0.store(true, Ordering::SeqCst);
                true
            }
        }
        let flag = Arc::new(AtomicBool::new(false));
        let policy = enabled_policy().with_approval_provider(Arc::new(Counting(flag.clone())));
        let mut context = ExecutionContext::new("f".to_string(), 0);
        context.set_provider(EXEC_POLICY_PROVIDER, Arc::new(policy));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("{e}"));
        let result = runtime.block_on(LocalExecPlugin.execute(
            &node(argv_cfg("true")),
            &mut context,
            &conns("true"),
            &HashMap::new(),
        ));
        assert!(result.is_ok());
        assert!(flag.load(Ordering::SeqCst));
    }
}
