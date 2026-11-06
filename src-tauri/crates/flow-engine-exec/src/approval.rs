//! 审批请求 / 信任记录 / 命令哈希
//!
//! 审批由宿主实现（GUI 弹窗 + `store/exec-trust.bin`；CLI 不实现，策略为
//! [`ApprovalMode::Always`](crate::policy::ApprovalMode::Always) 时不走审批）。
//! 本模块只定义契约与哈希算法，保证"命令变更即失效"的判定口径唯一。

use flow_engine_core::node::Node;
use flow_engine_core::node_config::{ExecMode, LocalExecConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 审批请求（推送给宿主 UI，或由宿主据此判定信任）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecApprovalRequest {
    /// 请求 ID（宿主回填批准结果时使用）
    pub request_id: String,
    /// 信任范围：流程文件绝对路径，或未保存流程的 `session:<flow_uuid>`
    pub flow_scope: String,
    /// 流程名称（仅用于展示）
    pub flow_name: String,
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称（仅用于展示）
    pub node_name: String,
    /// 完整命令（可执行文件绝对路径 + 参数 / 脚本），**不含环境变量值**
    pub command_display: String,
    /// 工作目录
    pub cwd: String,
    /// 注入的环境变量名（只有名字，避免密钥上屏）
    pub env_names: Vec<String>,
    /// 额外继承的宿主环境变量名（只有名字；这些变量会原样传给子进程）
    #[serde(default)]
    pub inherit_env_names: Vec<String>,
    /// 是否 shell 模式
    pub shell_mode: bool,
    /// 命令哈希（信任判定的唯一键，与 `flow_scope` 共同构成信任记录主键）
    pub command_hash: String,
}

/// 信任记录（持久化在 `store/exec-trust.bin` 的 `approvals` 键）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecApprovalRecord {
    /// 流程文件绝对路径（未保存流程为 `None`，仅会话级有效、不写盘）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_path: Option<String>,
    /// 命令哈希（见 [`command_hash`]）
    pub command_hash: String,
    /// 批准时间（Unix 秒）
    pub created_at: u64,
    /// 命令摘要（设置页展示）
    #[serde(default)]
    pub display: String,
    /// 节点名称（设置页展示）
    #[serde(default)]
    pub node_name: String,
}

/// 审批提供者（宿主实现：GUI 弹窗 + 持久化；CLI 不实现）
#[async_trait::async_trait]
pub trait ExecApprovalProvider: Send + Sync {
    /// 请求审批；返回 `true` 表示批准。
    ///
    /// provider 自行负责信任判定与持久化：命中信任直接返回 `true`（不再弹窗），
    /// 否则展示 UI 等待用户决定，批准后写回信任。
    async fn request_approval(&self, request: ExecApprovalRequest) -> bool;
}

/// 命令哈希的字段分隔符（US，不出现在正常路径/参数中）
const HASH_SEPARATOR: char = '\u{1f}';

/// 计算命令哈希
///
/// 输入（按顺序以 `\u{1f}` 连接，UTF-8）：
/// `flow_scope` | `node.uuid` | `node.node_type` | `mode` | 解析后的可执行文件绝对路径 |
/// `args` 模板（逐项）或 `script` 原文 | `cwd` 模板 |
/// `inherit_env`（排序） | `env` 键值模板（排序） | `capture.path` 模板
///
/// 要点：
/// - 用**模板原文**（`args` / `script` / `cwd` 未替换），变量值变化不触发重复确认；
/// - `program` 用**解析后的绝对路径**——否则把 `{{ffmpeg_bin}}` 指向另一个二进制
///   就能复用已有批准；
/// - `inherit_env` / `env` / `capture` 也参与哈希：这三项能改变子进程可见的密钥与
///   会被内联进报告的产物，若不入哈希，改配置即可在不重新确认的情况下扩大权限；
/// - 包含 `flow_scope`（流程路径）：复制/移动流程文件后自动失效。
pub fn command_hash(
    flow_scope: &str,
    node: &Node,
    cfg: &LocalExecConfig,
    resolved_program: &Path,
    cwd_template: &str,
) -> String {
    let mut hasher = Sha256::new();
    let mode = match cfg.mode {
        ExecMode::Argv => "argv",
        ExecMode::Shell => "shell",
    };
    let body_field = match cfg.mode {
        ExecMode::Argv => cfg
            .args
            .clone()
            .unwrap_or_default()
            .join(HASH_SEPARATOR_STR),
        ExecMode::Shell => cfg.script.clone().unwrap_or_default(),
    };

    let mut inherit_env: Vec<String> = cfg
        .inherit_env
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.trim().to_ascii_uppercase())
        .filter(|name| !name.is_empty())
        .collect();
    inherit_env.sort();
    inherit_env.dedup();

    let mut env_entries: Vec<String> = cfg
        .env
        .as_ref()
        .map(|map| {
            map.iter()
                .map(|(key, value)| format!("{}={}", key.trim(), value))
                .collect()
        })
        .unwrap_or_default();
    env_entries.sort();

    let capture_path = cfg
        .capture
        .as_ref()
        .map(|capture| capture.path.trim().to_string())
        .unwrap_or_default();

    let fields: [&str; 10] = [
        flow_scope,
        node.uuid.as_str(),
        node.node_type.as_str(),
        mode,
        &resolved_program.to_string_lossy(),
        body_field.as_str(),
        cwd_template,
        &inherit_env.join(HASH_SEPARATOR_STR),
        &env_entries.join(HASH_SEPARATOR_STR),
        &capture_path,
    ];
    for (idx, field) in fields.iter().enumerate() {
        if idx > 0 {
            hasher.update([HASH_SEPARATOR as u8]);
        }
        hasher.update(field.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

/// `args` 内部的分隔符（避免 `["a","b"]` 与 `["a\u{1f}b"]` 碰撞）
const HASH_SEPARATOR_STR: &str = "\u{1f}";

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::NodeData;
    use serde_json::json;

    fn node(args: Vec<&str>) -> Node {
        Node {
            uuid: "n1".to_string(),
            node_type: "local:exec".to_string(),
            name: "exec".to_string(),
            data: NodeData::LocalExec(LocalExecConfig {
                program: Some("/bin/echo".to_string()),
                args: Some(args.into_iter().map(str::to_string).collect()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn cfg(args: Vec<&str>) -> LocalExecConfig {
        LocalExecConfig {
            program: Some("/bin/echo".to_string()),
            args: Some(args.into_iter().map(str::to_string).collect()),
            ..Default::default()
        }
    }

    #[test]
    fn hash_is_stable_for_same_template() {
        let node = node(vec!["{{x}}"]);
        let cfg = cfg(vec!["{{x}}"]);
        let a = command_hash(
            "/flows/a.mpf",
            &node,
            &cfg,
            Path::new("/bin/echo"),
            "{{dir}}",
        );
        let b = command_hash(
            "/flows/a.mpf",
            &node,
            &cfg,
            Path::new("/bin/echo"),
            "{{dir}}",
        );
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn hash_changes_with_args() {
        let node = node(vec!["{{x}}"]);
        let a = command_hash(
            "/flows/a.mpf",
            &node,
            &cfg(vec!["{{x}}"]),
            Path::new("/bin/echo"),
            "",
        );
        let b = command_hash(
            "/flows/a.mpf",
            &node,
            &cfg(vec!["{{y}}"]),
            Path::new("/bin/echo"),
            "",
        );
        assert_ne!(a, b);
    }

    #[test]
    fn hash_changes_with_flow_scope() {
        let node = node(vec!["-n"]);
        let cfg = cfg(vec!["-n"]);
        let a = command_hash("/flows/a.mpf", &node, &cfg, Path::new("/bin/echo"), "");
        let b = command_hash("/flows/b.mpf", &node, &cfg, Path::new("/bin/echo"), "");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_changes_with_program_path() {
        let node = node(vec!["hi"]);
        let cfg = cfg(vec!["hi"]);
        let a = command_hash("/flows/a.mpf", &node, &cfg, Path::new("/bin/echo"), "");
        let b = command_hash("/flows/a.mpf", &node, &cfg, Path::new("/usr/bin/echo"), "");
        assert_ne!(a, b);
    }

    #[test]
    fn approval_request_serializes_camel_case() {
        let req = ExecApprovalRequest {
            request_id: "r1".to_string(),
            flow_scope: "/f.mpf".to_string(),
            flow_name: "f".to_string(),
            node_uuid: "n1".to_string(),
            node_name: "n".to_string(),
            command_display: "/bin/echo hi".to_string(),
            cwd: "/tmp".to_string(),
            env_names: vec!["MPE_VAR_X".to_string()],
            inherit_env_names: vec!["AWS_SECRET_ACCESS_KEY".to_string()],
            shell_mode: false,
            command_hash: "sha256:dead".to_string(),
        };
        let value = serde_json::to_value(&req).unwrap_or(json!(null));
        assert_eq!(value["requestId"], json!("r1"));
        assert_eq!(value["commandHash"], json!("sha256:dead"));
        assert_eq!(value["envNames"], json!(["MPE_VAR_X"]));
        assert_eq!(
            value["inheritEnvNames"],
            json!(["AWS_SECRET_ACCESS_KEY"])
        );
    }
    #[test]
    fn hash_changes_with_inherit_env() {
        let node = node(vec!["hi"]);
        let base = cfg(vec!["hi"]);
        let mut with_inherit = cfg(vec!["hi"]);
        with_inherit.inherit_env = Some(vec!["AWS_SECRET_ACCESS_KEY".to_string()]);
        let a = command_hash("/f.mpf", &node, &base, Path::new("/bin/echo"), "");
        let b = command_hash("/f.mpf", &node, &with_inherit, Path::new("/bin/echo"), "");
        assert_ne!(a, b, "继承新的宿主变量必须要求重新审批");
    }

    #[test]
    fn hash_is_order_insensitive_for_inherit_env() {
        let node = node(vec!["hi"]);
        let mut a_cfg = cfg(vec!["hi"]);
        a_cfg.inherit_env = Some(vec!["B".to_string(), "A".to_string()]);
        let mut b_cfg = cfg(vec!["hi"]);
        b_cfg.inherit_env = Some(vec!["A".to_string(), "B".to_string()]);
        let a = command_hash("/f.mpf", &node, &a_cfg, Path::new("/bin/echo"), "");
        let b = command_hash("/f.mpf", &node, &b_cfg, Path::new("/bin/echo"), "");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_changes_with_env_templates() {
        let node = node(vec!["hi"]);
        let base = cfg(vec!["hi"]);
        let mut with_env = cfg(vec!["hi"]);
        let mut env = std::collections::HashMap::new();
        env.insert("token".to_string(), "{{token}}".to_string());
        with_env.env = Some(env);
        let a = command_hash("/f.mpf", &node, &base, Path::new("/bin/echo"), "");
        let b = command_hash("/f.mpf", &node, &with_env, Path::new("/bin/echo"), "");
        assert_ne!(a, b, "新增注入变量必须要求重新审批");
    }

    #[test]
    fn hash_changes_with_capture_path() {
        let node = node(vec!["hi"]);
        let base = cfg(vec!["hi"]);
        let mut with_capture = cfg(vec!["hi"]);
        with_capture.capture = Some(flow_engine_core::node_config::ExecCaptureConfig {
            path: "out.mp4".to_string(),
            max_bytes: None,
        });
        let a = command_hash("/f.mpf", &node, &base, Path::new("/bin/echo"), "");
        let b = command_hash("/f.mpf", &node, &with_capture, Path::new("/bin/echo"), "");
        assert_ne!(a, b, "产物路径会内联进报告，必须要求重新审批");
    }
}
