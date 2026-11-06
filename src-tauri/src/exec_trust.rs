//! `local:exec` 审批与信任库（Tauri 侧）
//!
//! - 策略来源：`MPE_ALLOW_EXEC` 等环境变量（CI 用）优先，其次设置文件
//!   `store/settings.bin` 的 `execEnabled` / `execAllowShell` / `execAllowedPrograms`；
//! - 信任库：`store/exec-trust.bin` 的 `approvals` 键，主键为
//!   （流程绝对路径, 命令哈希）；未保存流程（无 `flow_path`）只在本次会话内记忆；
//! - 审批 UI：emit `exec-approval-request` 事件，前端弹窗后调用
//!   [`respond_exec_approval`] 回填结果。

use flow_engine::exec::{
    ApprovalMode, ExecApprovalProvider, ExecApprovalRecord, ExecApprovalRequest, ExecPolicy,
    MAX_TIMEOUT_MS,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

/// 信任库文件
const TRUST_STORE: &str = "store/exec-trust.bin";
/// 设置文件（与前端 `settings.bin` 同源）
const SETTINGS_STORE: &str = "store/settings.bin";
/// 信任记录在信任库中的键
const APPROVALS_KEY: &str = "approvals";
/// 审批等待上限（无人值守时不永久挂起）
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);
/// 推送给前端的审批事件名
pub const APPROVAL_EVENT: &str = "exec-approval-request";

/// 审批运行时状态（挂载为 Tauri managed state）
#[derive(Default)]
pub struct ExecApprovalState {
    /// 等待前端回填的审批请求
    pub pending: std::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>,
    /// 会话级批准（未保存流程：`<flow_scope>|<command_hash>`，不写盘）
    pub session: std::sync::Mutex<HashSet<String>>,
    /// 全局串行化：压测/并发下同一命令只弹一次窗
    pub gate: tokio::sync::Mutex<()>,
    /// 信任库读-改-写串行化（`remember` 与撤销/清空命令共用，避免丢更新）
    pub store: tokio::sync::Mutex<()>,
}

impl ExecApprovalState {
    /// 会话级批准的键
    fn session_key(flow_scope: &str, command_hash: &str) -> String {
        format!("{flow_scope}|{command_hash}")
    }
}

/// 构造当前执行策略（无论开关是否打开都注入，使错误为 `E_EXEC_DENIED` 而非"未注入"）
pub fn build_exec_policy(app: &tauri::AppHandle, state: &Arc<ExecApprovalState>) -> ExecPolicy {
    let enabled = env_flag("MPE_ALLOW_EXEC")
        .unwrap_or_else(|| settings_bool(app, "execEnabled").unwrap_or(false));
    let allow_shell = env_flag("MPE_ALLOW_SHELL")
        .unwrap_or_else(|| settings_bool(app, "execAllowShell").unwrap_or(false));
    let allowed_programs = env_list("MPE_ALLOW_EXEC_CMDS")
        .unwrap_or_else(|| settings_string_list(app, "execAllowedPrograms"));

    ExecPolicy {
        enabled,
        allowed_programs,
        allow_shell,
        max_timeout_ms: MAX_TIMEOUT_MS,
        approval: ApprovalMode::TrustStore,
        approval_provider: Some(Arc::new(GuiApprovalProvider {
            app: app.clone(),
            state: Arc::clone(state),
        })),
    }
}

fn env_flag(name: &str) -> Option<bool> {
    let value = std::env::var(name).ok()?;
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    Some(matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn env_list(name: &str) -> Option<Vec<String>> {
    let value = std::env::var(name).ok()?;
    let items: Vec<String> = value
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();
    (!items.is_empty()).then_some(items)
}

fn settings_value(app: &tauri::AppHandle, key: &str) -> Option<serde_json::Value> {
    app.store(SETTINGS_STORE).ok()?.get(key)
}

fn settings_bool(app: &tauri::AppHandle, key: &str) -> Option<bool> {
    settings_value(app, key)?.as_bool()
}

fn settings_string_list(app: &tauri::AppHandle, key: &str) -> Vec<String> {
    settings_value(app, key)
        .and_then(|value| value.as_array().cloned())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// 读取信任记录（解析失败按空处理，不阻塞执行）
pub fn read_approvals(app: &tauri::AppHandle) -> Vec<ExecApprovalRecord> {
    let Some(value) = app
        .store(TRUST_STORE)
        .ok()
        .and_then(|store| store.get(APPROVALS_KEY))
    else {
        return Vec::new();
    };
    serde_json::from_value(value).unwrap_or_else(|e| {
        log::warn!("[ExecTrust] Failed to parse approvals: {e}");
        Vec::new()
    })
}

/// 写入信任记录
pub fn write_approvals(
    app: &tauri::AppHandle,
    approvals: &[ExecApprovalRecord],
) -> Result<(), String> {
    let store = app
        .store(TRUST_STORE)
        .map_err(|e| format!("failed to open trust store: {e}"))?;
    let value = serde_json::to_value(approvals)
        .map_err(|e| format!("failed to serialize approvals: {e}"))?;
    store.set(APPROVALS_KEY, value);
    store
        .save()
        .map_err(|e| format!("failed to persist trust store: {e}"))
}

fn unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// GUI 审批提供者：命中信任直接放行，否则推送事件等待用户决定
struct GuiApprovalProvider {
    app: tauri::AppHandle,
    state: Arc<ExecApprovalState>,
}

impl GuiApprovalProvider {
    fn is_trusted(&self, request: &ExecApprovalRequest) -> bool {
        if request.flow_scope.starts_with("session:") {
            let key = ExecApprovalState::session_key(&request.flow_scope, &request.command_hash);
            return self
                .state
                .session
                .lock()
                .map(|set| set.contains(&key))
                .unwrap_or(false);
        }
        read_approvals(&self.app).iter().any(|record| {
            record.flow_path.as_deref() == Some(request.flow_scope.as_str())
                && record.command_hash == request.command_hash
        })
    }

    async fn remember(&self, request: &ExecApprovalRequest) {
        if request.flow_scope.starts_with("session:") {
            let key = ExecApprovalState::session_key(&request.flow_scope, &request.command_hash);
            if let Ok(mut set) = self.state.session.lock() {
                set.insert(key);
            }
            return;
        }

        // 与撤销/清空命令串行化，避免读-改-写丢更新
        let _store_guard = self.state.store.lock().await;
        let mut approvals = read_approvals(&self.app);
        let already = approvals.iter().any(|record| {
            record.flow_path.as_deref() == Some(request.flow_scope.as_str())
                && record.command_hash == request.command_hash
        });
        if already {
            return;
        }
        approvals.push(ExecApprovalRecord {
            flow_path: Some(request.flow_scope.clone()),
            command_hash: request.command_hash.clone(),
            created_at: unix_timestamp_secs(),
            display: request.command_display.clone(),
            node_name: request.node_name.clone(),
        });
        if let Err(e) = write_approvals(&self.app, &approvals) {
            log::warn!("[ExecTrust] Failed to persist approval: {e}");
        }
    }
}

/// 请求被 drop（含 executor 取消节点 future）时清理 pending 表，避免泄漏
struct PendingGuard {
    state: Arc<ExecApprovalState>,
    request_id: String,
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.state.pending.lock() {
            pending.remove(&self.request_id);
        }
    }
}

#[async_trait::async_trait]
impl ExecApprovalProvider for GuiApprovalProvider {
    async fn request_approval(&self, request: ExecApprovalRequest) -> bool {
        // 快路径：已信任的命令不必排队等别人的弹窗（否则一个未处理的弹窗会把
        // 全系统所有已批准命令阻塞到 APPROVAL_TIMEOUT）
        if self.is_trusted(&request) {
            return true;
        }

        // 串行化：并发（压测/数据集）时只弹一次窗
        let _guard = self.state.gate.lock().await;

        // 拿到锁后复查：等待期间别的执行可能已经批准并写入了信任库
        if self.is_trusted(&request) {
            return true;
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let Ok(mut pending) = self.state.pending.lock() else {
                return false;
            };
            pending.insert(request.request_id.clone(), tx);
        }
        let pending_guard = PendingGuard {
            state: Arc::clone(&self.state),
            request_id: request.request_id.clone(),
        };

        if let Err(e) = self.app.emit(APPROVAL_EVENT, &request) {
            log::warn!("[ExecTrust] Failed to emit approval request: {e}");
            return false;
        }

        let approved = match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(approved)) => approved,
            Ok(Err(_)) => false,
            Err(_) => {
                log::warn!(
                    "[ExecTrust] Approval timed out after {}s for `{}`",
                    APPROVAL_TIMEOUT.as_secs(),
                    request.command_display
                );
                false
            }
        };
        drop(pending_guard);

        if approved {
            self.remember(&request).await;
        }
        approved
    }
}

/// 前端回填审批结果
#[tauri::command]
pub fn respond_exec_approval(
    request_id: String,
    approved: bool,
    state: tauri::State<'_, Arc<ExecApprovalState>>,
) -> Result<(), String> {
    let sender = state
        .pending
        .lock()
        .map_err(|_| "approval state poisoned".to_string())?
        .remove(&request_id)
        .ok_or_else(|| "unknown approval request".to_string())?;
    sender
        .send(approved)
        .map_err(|_| "approval request already closed".to_string())
}

/// 列出已批准的信任记录
#[tauri::command]
pub fn list_exec_approvals(app: tauri::AppHandle) -> Vec<ExecApprovalRecord> {
    read_approvals(&app)
}

/// 撤销单条信任记录
#[tauri::command]
pub async fn revoke_exec_approval(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<ExecApprovalState>>,
    flow_path: Option<String>,
    command_hash: String,
) -> Result<(), String> {
    // 与 `remember` 共用同一把锁，避免撤销被并发批准覆盖
    let _store_guard = state.store.lock().await;
    let mut approvals = read_approvals(&app);
    approvals
        .retain(|record| !(record.command_hash == command_hash && record.flow_path == flow_path));
    write_approvals(&app, &approvals)
}

/// 清空全部信任记录（含会话级批准）
#[tauri::command]
pub async fn clear_exec_approvals(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<ExecApprovalState>>,
) -> Result<(), String> {
    let _store_guard = state.store.lock().await;
    write_approvals(&app, &[])?;
    if let Ok(mut session) = state.session.lock() {
        session.clear();
    }
    Ok(())
}

/// 读取应用数据目录下的信任库路径（调试/展示用）
pub fn trust_store_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    Some(dir.join(TRUST_STORE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_key_is_stable_and_distinct() {
        let a = ExecApprovalState::session_key("session:f1", "sha256:aa");
        let b = ExecApprovalState::session_key("session:f1", "sha256:aa");
        let c = ExecApprovalState::session_key("session:f1", "sha256:bb");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn env_flag_parses_truthy_values() {
        std::env::set_var("MPE_TEST_EXEC_FLAG", "1");
        assert_eq!(env_flag("MPE_TEST_EXEC_FLAG"), Some(true));
        std::env::set_var("MPE_TEST_EXEC_FLAG", "no");
        assert_eq!(env_flag("MPE_TEST_EXEC_FLAG"), Some(false));
        std::env::remove_var("MPE_TEST_EXEC_FLAG");
        assert_eq!(env_flag("MPE_TEST_EXEC_FLAG"), None);
    }

    #[test]
    fn env_list_splits_and_trims() {
        std::env::set_var("MPE_TEST_EXEC_CMDS", " ffmpeg , /bin/sh ,, ");
        assert_eq!(
            env_list("MPE_TEST_EXEC_CMDS"),
            Some(vec!["ffmpeg".to_string(), "/bin/sh".to_string()])
        );
        std::env::set_var("MPE_TEST_EXEC_CMDS", "   ");
        assert_eq!(env_list("MPE_TEST_EXEC_CMDS"), None);
        std::env::remove_var("MPE_TEST_EXEC_CMDS");
    }
}
