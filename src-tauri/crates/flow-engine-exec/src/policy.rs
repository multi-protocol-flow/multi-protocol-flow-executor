//! 本地命令执行策略
//!
//! 策略由宿主（Tauri app / CLI）注入到执行上下文的 provider 表，节点在执行前
//! 逐层校验：provider 存在 → `enabled` → 白名单 → shell 许可 → 审批。
//! **缺省即拒绝**：取不到 provider 一律 `E_EXEC_DENIED`。

use crate::approval::ExecApprovalProvider;
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// provider 名称（`ExecutionContext::get_provider` 的 key）
pub const EXEC_POLICY_PROVIDER: &str = "exec_policy";

/// 单节点超时上限（毫秒），与执行器的节点超时钳位一致
pub const MAX_TIMEOUT_MS: u64 = 3_600_000;

/// 审批模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalMode {
    /// 走信任库 + 宿主弹窗（GUI）
    #[default]
    TrustStore,
    /// 不需要审批（CLI 显式 `--allow-exec` 已经是一次人工确认）
    Always,
}

/// 本地命令执行策略
///
/// 由宿主或 CLI 注入（[`EXEC_POLICY_PROVIDER`]），缺失即拒绝执行。
#[derive(Clone)]
pub struct ExecPolicy {
    /// 总开关
    pub enabled: bool,
    /// 允许的程序白名单（空 = 不限制，仅 `enabled` 生效）
    pub allowed_programs: Vec<String>,
    /// 是否允许 shell 模式
    pub allow_shell: bool,
    /// 单节点超时上限
    pub max_timeout_ms: u64,
    /// 审批模式
    pub approval: ApprovalMode,
    /// 审批提供者（`ApprovalMode::TrustStore` 时必需）
    pub approval_provider: Option<Arc<dyn ExecApprovalProvider>>,
}

impl Default for ExecPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_programs: Vec::new(),
            allow_shell: false,
            max_timeout_ms: MAX_TIMEOUT_MS,
            approval: ApprovalMode::TrustStore,
            approval_provider: None,
        }
    }
}

impl std::fmt::Debug for ExecPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecPolicy")
            .field("enabled", &self.enabled)
            .field("allowed_programs", &self.allowed_programs)
            .field("allow_shell", &self.allow_shell)
            .field("max_timeout_ms", &self.max_timeout_ms)
            .field("approval", &self.approval)
            .field("approval_provider", &self.approval_provider.is_some())
            .finish()
    }
}

impl ExecPolicy {
    /// 构造 CLI 策略（`--allow-exec` 即一次显式人工确认，无需二次审批）
    pub fn cli(allow_exec: bool, allowed: Vec<String>, allow_shell: bool) -> Self {
        Self {
            enabled: allow_exec,
            allowed_programs: allowed
                .into_iter()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect(),
            allow_shell,
            max_timeout_ms: MAX_TIMEOUT_MS,
            approval: ApprovalMode::Always,
            approval_provider: None,
        }
    }

    /// 挂载审批提供者
    pub fn with_approval_provider(mut self, provider: Arc<dyn ExecApprovalProvider>) -> Self {
        self.approval_provider = Some(provider);
        self
    }

    /// 判定解析后的可执行文件是否被白名单允许
    ///
    /// 白名单条目按形态分流：
    /// - 不含路径分隔符 → 与文件名比较（Windows 忽略大小写并忽略 `.exe` 后缀）
    /// - 含路径分隔符 → 双方 `canonicalize` 后比较
    pub fn is_program_allowed(&self, resolved: &Path) -> bool {
        if self.allowed_programs.is_empty() {
            return true;
        }
        let file_name = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let resolved_abs =
            std::fs::canonicalize(resolved).unwrap_or_else(|_| resolved.to_path_buf());

        self.allowed_programs.iter().any(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return false;
            }
            if entry.contains('/') || entry.contains('\\') {
                match std::fs::canonicalize(entry) {
                    Ok(candidate) => candidate == resolved_abs,
                    Err(_) => false,
                }
            } else {
                program_name_matches(entry, file_name)
            }
        })
    }
}

#[cfg(windows)]
fn program_name_matches(entry: &str, file_name: &str) -> bool {
    let normalize = |value: &str| -> String {
        let lowered = value.trim().to_ascii_lowercase();
        lowered
            .strip_suffix(".exe")
            .map(str::to_string)
            .unwrap_or(lowered)
    };
    normalize(entry) == normalize(file_name)
}

#[cfg(not(windows))]
fn program_name_matches(entry: &str, file_name: &str) -> bool {
    entry == file_name
}

/// 构造 provider 映射（app 与 CLI 共用的唯一入口）
pub fn policy_providers(policy: ExecPolicy) -> HashMap<String, Arc<dyn Any + Send + Sync>> {
    let mut providers: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
    providers.insert(EXEC_POLICY_PROVIDER.to_string(), Arc::new(policy));
    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_disabled() {
        let policy = ExecPolicy::default();
        assert!(!policy.enabled);
        assert!(!policy.allow_shell);
        assert!(policy.allowed_programs.is_empty());
        assert_eq!(policy.max_timeout_ms, MAX_TIMEOUT_MS);
        assert_eq!(policy.approval, ApprovalMode::TrustStore);
        assert!(policy.approval_provider.is_none());
    }

    #[test]
    fn empty_whitelist_allows_everything() {
        let policy = ExecPolicy::default();
        assert!(policy.is_program_allowed(Path::new("/bin/sh")));
    }

    #[test]
    fn whitelist_matches_file_name() {
        let policy = ExecPolicy {
            allowed_programs: vec!["sh".to_string()],
            ..Default::default()
        };
        assert!(policy.is_program_allowed(Path::new("/bin/sh")));
        assert!(!policy.is_program_allowed(Path::new("/bin/bash")));
    }

    #[test]
    fn whitelist_matches_absolute_path() {
        let policy = ExecPolicy {
            allowed_programs: vec!["/bin/sh".to_string()],
            ..Default::default()
        };
        assert!(policy.is_program_allowed(Path::new("/bin/sh")));
        assert!(!policy.is_program_allowed(Path::new("/bin/bash")));
    }

    #[test]
    fn cli_policy_uses_always_approval() {
        let policy = ExecPolicy::cli(true, vec![" ffmpeg ".to_string()], true);
        assert!(policy.enabled);
        assert!(policy.allow_shell);
        assert_eq!(policy.allowed_programs, vec!["ffmpeg".to_string()]);
        assert_eq!(policy.approval, ApprovalMode::Always);
    }

    #[test]
    fn policy_providers_registers_under_known_key() {
        let providers = policy_providers(ExecPolicy::cli(true, Vec::new(), false));
        assert!(providers.contains_key(EXEC_POLICY_PROVIDER));
        assert!(providers[EXEC_POLICY_PROVIDER]
            .clone()
            .downcast::<ExecPolicy>()
            .is_ok());
    }
}
