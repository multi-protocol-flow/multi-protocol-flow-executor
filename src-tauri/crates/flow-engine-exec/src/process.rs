//! 子进程启动、输出捕获、超时与取消
//!
//! workspace 的 `tokio` 未启用 `process` feature，因此用 `std::process::Command`
//! 在 `spawn_blocking` 的阻塞线程内执行，并在该线程内自行轮询 `try_wait`：
//!
//! - 超时 / 取消必须由阻塞线程观察并杀进程组——`spawn_blocking` 的 `JoinHandle`
//!   被 drop 时线程仍在运行，executor 取消时只会 drop future，节点拿不到回调；
//! - 输出超过上限后**继续读并丢弃**，否则管道写满会让子进程阻塞（死锁）。

use crate::spec::ResolvedCommand;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use serde_json::json;
use tokio::sync::watch;

/// 轮询间隔
const POLL_INTERVAL_MS: u64 = 25;
/// 读缓冲区大小
const READ_CHUNK: usize = 8192;
/// SIGTERM → SIGKILL 的宽限期（进程在此窗口内优雅退出则不再 SIGKILL）
const KILL_GRACE: Duration = Duration::from_millis(2000);

/// 执行结果
#[derive(Debug, Clone, Default)]
pub struct ExecOutcome {
    /// 退出码（被信号终止时为 `None`）
    pub exit_code: Option<i32>,
    /// 标准输出（可能被截断）
    pub stdout: String,
    /// 标准错误（可能被截断）
    pub stderr: String,
    /// stdout 是否发生截断
    pub stdout_truncated: bool,
    /// stderr 是否发生截断
    pub stderr_truncated: bool,
    /// 是否因超时终止
    pub timed_out: bool,
    /// 是否因取消终止
    pub cancelled: bool,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

/// 执行命令
///
/// `cancel` 为 `Some` 时每个轮询周期检查一次，收到 `true` 即刻终止进程组并返回
/// `E_CANCELLED`；超过 `timeout_ms` 返回 `E_TIMEOUT`。
pub async fn run_command(
    cmd: ResolvedCommand,
    cancel: Option<watch::Receiver<bool>>,
) -> Result<ExecOutcome, ErrorDetail> {
    let handle = tokio::task::spawn_blocking(move || run_blocking(cmd, cancel));
    handle.await.map_err(|e| {
        let message = format!("command worker thread failed: {e}");
        ErrorDetail::new(ErrorKind::Execution, message.clone())
            .with_error_code_typed(ErrorCode::E_EXECUTION_ERROR)
            .with_i18n_arg("message", message)
    })?
}

fn run_blocking(
    cmd: ResolvedCommand,
    cancel: Option<watch::Receiver<bool>>,
) -> Result<ExecOutcome, ErrorDetail> {
    let start = Instant::now();

    let mut command = Command::new(&cmd.program);
    command.args(&cmd.args);
    command.current_dir(&cmd.cwd);
    command.env_clear();
    for (key, value) in &cmd.env {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    configure_process_group(&mut command);

    let mut child = command.spawn().map_err(|e| {
        let message = format!("failed to start `{}`: {e}", cmd.program.display());
        ErrorDetail::new(ErrorKind::Execution, message.clone())
            .with_error_code_typed(ErrorCode::E_EXEC_SPAWN_FAILED)
            .with_i18n_arg("program", cmd.program.display().to_string())
            .with_i18n_arg("message", message)
    })?;

    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| spawn_reader(pipe, cmd.max_output_bytes));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| spawn_reader(pipe, cmd.max_output_bytes));

    let deadline = start + Duration::from_millis(cmd.timeout_ms);
    let mut exit_code: Option<i32> = None;
    let mut timed_out = false;
    let mut cancelled = false;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_code = status.code();
                break;
            }
            Ok(None) => {}
            Err(e) => {
                terminate_child(&mut child);
                join_reader(stdout_reader);
                join_reader(stderr_reader);
                return Err(ErrorDetail::new(
                    ErrorKind::Execution,
                    format!("failed to poll `{}`: {e}", cmd.program.display()),
                )
                .with_error_code_typed(ErrorCode::E_EXEC_SPAWN_FAILED)
                .with_i18n_arg("program", cmd.program.display().to_string())
                .with_i18n_arg("message", e.to_string()));
            }
        }

        if cancel.as_ref().is_some_and(|rx| *rx.borrow()) {
            cancelled = true;
            terminate_child(&mut child);
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            terminate_child(&mut child);
            break;
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    let (stdout, stdout_truncated) = join_reader(stdout_reader);
    let (stderr, stderr_truncated) = join_reader(stderr_reader);

    let duration_ms = start.elapsed().as_millis() as u64;

    // 部分输出对排查超时/取消原因很关键，随 error.context 一起带回报告
    if timed_out {
        let message = format!("command timed out after {}ms", cmd.timeout_ms);
        return Err(ErrorDetail::new(ErrorKind::Timeout, message.clone())
            .with_error_code_typed(ErrorCode::E_TIMEOUT)
            .with_i18n_arg("duration", cmd.timeout_ms.to_string())
            .with_i18n_arg("message", message)
            .with_context(json!({
                "stdout": stdout,
                "stderr": stderr,
                "stdout_truncated": stdout_truncated,
                "stderr_truncated": stderr_truncated,
                "duration_ms": duration_ms,
            })));
    }
    if cancelled {
        let message = "command cancelled".to_string();
        return Err(ErrorDetail::new(ErrorKind::Cancelled, message.clone())
            .with_error_code_typed(ErrorCode::E_CANCELLED)
            .with_i18n_arg("message", message)
            .with_context(json!({
                "stdout": stdout,
                "stderr": stderr,
                "stdout_truncated": stdout_truncated,
                "stderr_truncated": stderr_truncated,
                "duration_ms": duration_ms,
            })));
    }

    Ok(ExecOutcome {
        exit_code,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        timed_out,
        cancelled,
        duration_ms,
    })
}

/// 终止进程组（blocking）
///
/// Unix：`SIGTERM` 到整个进程组 → 宽限期 → `SIGKILL`
/// （子 shell 起的后台进程也在同一进程组内）
/// Windows：`taskkill /PID <pid> /T /F`
///
/// 返回 `true` 表示至少发出了一次信号；外部工具缺失时返回 `false`，
/// 调用方应回退到 [`Child::kill`]，否则 `wait()` 可能永久阻塞。
pub fn kill_process_group(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let term_sent = signal_group(pid, "-TERM");
        std::thread::sleep(KILL_GRACE);
        let kill_sent = signal_group(pid, "-KILL");
        term_sent || kill_sent
    }
    #[cfg(windows)]
    {
        taskkill_tree(pid)
    }
}

/// 终止并回收子进程（避免僵尸进程）
///
/// 顺序：进程组 `SIGTERM` → 宽限期内轮询退出 → `SIGKILL` 进程组 →
/// `child.kill()`（外部 `kill` 不可用时仍能杀掉组长）→ `wait()` 回收。
/// 任何一步都不能省掉 `child.kill()`：`kill`/`taskkill` 缺失时若直接
/// `wait()` 会永久阻塞，而 `spawn_blocking` 的线程无法被取消。
fn terminate_child(child: &mut Child) {
    let pid = child.id();
    let group_signalled = signal_group_terminate(pid);

    if group_signalled {
        let deadline = Instant::now() + KILL_GRACE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(_) => break,
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
        signal_group_kill(pid);
    }

    // 兜底：直接杀组长，保证后续 wait() 一定能返回
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn signal_group_terminate(pid: u32) -> bool {
    pid != 0 && signal_group(pid, "-TERM")
}

#[cfg(unix)]
fn signal_group_kill(pid: u32) -> bool {
    pid != 0 && signal_group(pid, "-KILL")
}

#[cfg(windows)]
fn signal_group_terminate(pid: u32) -> bool {
    // `CREATE_NEW_PROCESS_GROUP` 下的进程没有 POSIX 语义，直接结束进程树
    pid != 0 && taskkill_tree(pid)
}

#[cfg(windows)]
fn signal_group_kill(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
fn taskkill_tree(pid: u32) -> bool {
    Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: &str) -> bool {
    Command::new("kill")
        .arg(signal)
        .arg(format!("-{pid}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// 让子进程进入独立进程组，从而可以整组终止
#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

/// 让子进程进入独立进程组（`CREATE_NEW_PROCESS_GROUP`，安全 API 无 unsafe）
#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// Windows `CREATE_NEW_PROCESS_GROUP`
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// 起一个读线程，累计到上限后继续读并丢弃
fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> std::thread::JoinHandle<(String, bool)> {
    std::thread::spawn(move || {
        let mut buf = [0u8; READ_CHUNK];
        let mut kept: Vec<u8> = Vec::new();
        let mut dropped: usize = 0;
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let room = limit.saturating_sub(kept.len());
                    let take = n.min(room);
                    kept.extend_from_slice(&buf[..take]);
                    dropped += n - take;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let mut text = String::from_utf8_lossy(&kept).to_string();
        if dropped > 0 {
            text.push_str(&format!("\n…[truncated {dropped} bytes]"));
        }
        (text, dropped > 0)
    })
}

fn join_reader(handle: Option<std::thread::JoinHandle<(String, bool)>>) -> (String, bool) {
    match handle {
        Some(handle) => handle.join().unwrap_or_else(|_| (String::new(), false)),
        None => (String::new(), false),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::spec::ResolvedCommand;
    use std::path::PathBuf;

    fn sh(script: &str) -> ResolvedCommand {
        ResolvedCommand {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_string(), script.to_string()],
            shell_mode: false,
            display: format!("/bin/sh -c {script}"),
            cwd: std::env::temp_dir(),
            env: vec![("PATH".to_string(), "/usr/bin:/bin".to_string())],
            env_names: Vec::new(),
            inherit_env_names: Vec::new(),
            timeout_ms: 30_000,
            max_output_bytes: 1_048_576,
            capture: None,
            cwd_template: String::new(),
        }
    }

    #[tokio::test]
    async fn captures_stdout() {
        let outcome = run_command(sh("printf hi"), None)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, "hi");
        assert!(!outcome.stdout_truncated);
    }

    #[tokio::test]
    async fn reports_non_zero_exit() {
        let outcome = run_command(sh("printf boom >&2; exit 3"), None)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(outcome.exit_code, Some(3));
        assert_eq!(outcome.stderr, "boom");
    }

    #[tokio::test]
    async fn arguments_are_not_word_split() {
        let mut cmd = sh("");
        cmd.program = PathBuf::from("/bin/echo");
        cmd.args = vec!["a b; rm -rf /".to_string()];
        let outcome = run_command(cmd, None)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(outcome.stdout, "a b; rm -rf /\n");
    }

    #[tokio::test]
    async fn injects_only_whitelisted_env() {
        std::env::set_var("MPE_PROCESS_TEST_LEAK", "1");
        let mut cmd = sh("env");
        cmd.env.push(("MPE_VAR_X".to_string(), "value".to_string()));
        let outcome = run_command(cmd, None)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        std::env::remove_var("MPE_PROCESS_TEST_LEAK");
        assert!(outcome.stdout.contains("MPE_VAR_X=value"));
        assert!(!outcome.stdout.contains("MPE_PROCESS_TEST_LEAK"));
    }

    #[tokio::test]
    async fn times_out_and_terminates() {
        let mut cmd = sh("sleep 10");
        cmd.timeout_ms = 200;
        let start = Instant::now();
        let err = run_command(cmd, None).await.expect_err("must time out");
        assert_eq!(err.error_code, Some(ErrorCode::E_TIMEOUT));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn cancels_running_command() {
        let (tx, rx) = watch::channel(false);
        let mut cmd = sh("sleep 10");
        cmd.timeout_ms = 30_000;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            let _ = tx.send(true);
        });
        let start = Instant::now();
        let err = run_command(cmd, Some(rx)).await.expect_err("must cancel");
        assert_eq!(err.error_code, Some(ErrorCode::E_CANCELLED));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn truncates_large_output_without_deadlock() {
        let mut cmd = sh("yes x | head -c 1000000; exit 0");
        cmd.max_output_bytes = 1024;
        let outcome = run_command(cmd, None)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout_truncated);
        assert!(outcome.stdout.contains("[truncated"));
        assert!(outcome.stdout.len() < 4096);
    }

    #[tokio::test]
    async fn timeout_error_carries_partial_output() {
        let mut cmd = sh("printf before-timeout; sleep 10");
        cmd.timeout_ms = 200;
        let err = run_command(cmd, None).await.expect_err("must time out");
        assert_eq!(err.error_code, Some(ErrorCode::E_TIMEOUT));
        let context = err.context.unwrap_or(serde_json::Value::Null);
        assert_eq!(context["stdout"], json!("before-timeout"));
    }

    #[tokio::test]
    async fn kills_background_children_of_the_group() {
        // 后台 sleep 属于同一进程组，TERM 后应一并消失
        let mut cmd = sh("sleep 30 & echo started; sleep 30");
        cmd.timeout_ms = 300;
        let err = run_command(cmd, None).await.expect_err("must time out");
        assert_eq!(err.error_code, Some(ErrorCode::E_TIMEOUT));
    }
}
