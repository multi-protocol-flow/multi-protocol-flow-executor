//! 本机信息探测（只读）
//!
//! 优先纯读取（`/proc`、`sysctl`、环境变量），只有工具链探测才起进程，且命令
//! **硬编码**、不走 `local:exec` 的审批链。单项探测失败返回 `null`，**绝不**让
//! 节点整体失败。

use flow_engine_core::context::ExecutionContext;
use flow_engine_core::node_config::LocalSysinfoConfig;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 每条硬编码探针的超时
pub const PROBE_TIMEOUT_MS: u64 = 5_000;
/// 探针输出上限
const PROBE_OUTPUT_LIMIT: u64 = 4 * 1024 * 1024;

/// 支持的采集项
pub const SUPPORTED_CHECKS: &[&str] = &["os", "cpu", "mem", "disk", "ffmpeg"];

/// 采集本机信息
pub async fn collect(
    cfg: &LocalSysinfoConfig,
    context: &ExecutionContext,
) -> Result<serde_json::Value, flow_engine_core::error::ErrorDetail> {
    let checks: Vec<String> = cfg
        .checks
        .iter()
        .map(|c| c.trim().to_ascii_lowercase())
        .filter(|c| !c.is_empty())
        .collect();

    let disk_path = match cfg
        .disk_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(template) => match flow_engine_core::executor::replace_variables(template, context) {
            Ok(replaced) => Some(context.resolve_file_path(replaced.trim())),
            Err(e) => {
                return Err(flow_engine_core::error::ErrorDetail::new(
                    flow_engine_core::error::ErrorKind::Configuration,
                    format!("disk_path: {e}"),
                )
                .with_error_code_typed(flow_engine_core::error::ErrorCode::E_CONFIG_ERROR)
                .with_i18n_arg("message", e));
            }
        },
        None => context
            .flow_file_path
            .as_deref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok()),
    };

    let handle =
        tokio::task::spawn_blocking(move || collect_blocking(&checks, disk_path.as_deref()));
    handle.await.map_err(|e| {
        let message = format!("sysinfo probe thread failed: {e}");
        flow_engine_core::error::ErrorDetail::new(
            flow_engine_core::error::ErrorKind::Execution,
            message.clone(),
        )
        .with_error_code_typed(flow_engine_core::error::ErrorCode::E_EXECUTION_ERROR)
        .with_i18n_arg("message", message)
    })
}

/// 同步探测（在 `spawn_blocking` 内执行）
pub fn collect_blocking(checks: &[String], disk_path: Option<&Path>) -> serde_json::Value {
    let has = |name: &str| checks.iter().any(|c| c == name);
    let mut output = serde_json::Map::new();
    output.insert("success".to_string(), serde_json::Value::Bool(true));

    if has("os") {
        output.insert("os".to_string(), serde_json::json!(std::env::consts::OS));
        output.insert(
            "arch".to_string(),
            serde_json::json!(std::env::consts::ARCH),
        );
        output.insert("kernel_version".to_string(), opt_string(kernel_version()));
        output.insert("hostname".to_string(), opt_string(hostname()));
    }
    if has("cpu") {
        output.insert(
            "cores".to_string(),
            serde_json::json!(std::thread::available_parallelism().ok().map(|n| n.get())),
        );
        output.insert("cpu_brand".to_string(), opt_string(cpu_brand()));
    }
    if has("mem") {
        let (total, available) = memory();
        output.insert("mem_total_bytes".to_string(), serde_json::json!(total));
        output.insert(
            "mem_available_bytes".to_string(),
            serde_json::json!(available),
        );
    }
    if has("disk") {
        let (total, available) = disk_usage(disk_path);
        output.insert("disk_total_bytes".to_string(), serde_json::json!(total));
        output.insert(
            "disk_available_bytes".to_string(),
            serde_json::json!(available),
        );
    }
    if has("ffmpeg") {
        let version = run_probe("ffmpeg", &["-version"])
            .and_then(|out| out.lines().next().map(|line| line.trim().to_string()))
            .filter(|line| !line.is_empty());
        let filters = run_probe("ffmpeg", &["-hide_banner", "-filters"]);
        output.insert("ffmpeg_version".to_string(), opt_string(version));
        output.insert(
            "ffmpeg_path".to_string(),
            opt_string(
                super::spec::resolve_executable("ffmpeg", std::env::var_os("PATH").as_deref())
                    .map(|p| p.to_string_lossy().to_string()),
            ),
        );
        output.insert(
            "has_drawtext".to_string(),
            serde_json::json!(filters.map(|list| list.contains(" drawtext "))),
        );
    }

    output.insert(
        "path_entries".to_string(),
        serde_json::json!(std::env::var("PATH")
            .map(|value| std::env::split_paths(&value).count())
            .unwrap_or(0)),
    );
    output.insert(
        "cwd".to_string(),
        serde_json::json!(std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()),
    );

    serde_json::Value::Object(output)
}

fn opt_string(value: Option<String>) -> serde_json::Value {
    match value {
        Some(value) => serde_json::Value::String(value),
        None => serde_json::Value::Null,
    }
}

fn read_trimmed(path: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// 起一条硬编码探针并收集 stdout（上限内），超时直接终止
fn run_probe(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // 独立读线程：避免探针输出超过管道缓冲区导致子进程阻塞
    let reader = child.stdout.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.take(PROBE_OUTPUT_LIMIT).read_to_end(&mut buf);
            buf
        })
    });

    let start = Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
        if start.elapsed() > Duration::from_millis(PROBE_TIMEOUT_MS) {
            let _ = child.kill();
            let _ = child.wait();
            break false;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let bytes = reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    if !success && bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).to_string())
}

#[cfg(target_os = "linux")]
fn kernel_version() -> Option<String> {
    read_trimmed("/proc/sys/kernel/osrelease")
}

#[cfg(target_os = "macos")]
fn kernel_version() -> Option<String> {
    run_probe("sysctl", &["-n", "kern.osrelease"]).map(|s| s.trim().to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn kernel_version() -> Option<String> {
    None
}

fn hostname() -> Option<String> {
    for key in ["HOSTNAME", "COMPUTERNAME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    read_trimmed("/proc/sys/kernel/hostname").or_else(|| {
        run_probe("hostname", &[])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

#[cfg(target_os = "linux")]
fn cpu_brand() -> Option<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    content.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().to_string())
    })
}

#[cfg(target_os = "macos")]
fn cpu_brand() -> Option<String> {
    run_probe("sysctl", &["-n", "machdep.cpu.brand_string"]).map(|s| s.trim().to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn cpu_brand() -> Option<String> {
    std::env::var("PROCESSOR_IDENTIFIER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "linux")]
fn memory() -> (Option<u64>, Option<u64>) {
    (parse_meminfo("MemTotal"), parse_meminfo("MemAvailable"))
}

#[cfg(target_os = "linux")]
fn parse_meminfo(key: &str) -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start_matches(':').trim();
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn memory() -> (Option<u64>, Option<u64>) {
    let total =
        run_probe("sysctl", &["-n", "hw.memsize"]).and_then(|s| s.trim().parse::<u64>().ok());
    (total, None)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn memory() -> (Option<u64>, Option<u64>) {
    (None, None)
}

#[cfg(unix)]
fn disk_usage(path: Option<&Path>) -> (Option<u64>, Option<u64>) {
    let target = path.unwrap_or_else(|| Path::new("."));
    let Some(out) = run_probe("df", &["-kP", &target.to_string_lossy()]) else {
        return (None, None);
    };
    let line = out.lines().nth(1).unwrap_or_default();
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return (None, None);
    }
    let total = cols[1].parse::<u64>().ok().map(|kb| kb * 1024);
    let available = cols[3].parse::<u64>().ok().map(|kb| kb * 1024);
    (total, available)
}

#[cfg(windows)]
fn disk_usage(path: Option<&Path>) -> (Option<u64>, Option<u64>) {
    let target = path.unwrap_or_else(|| Path::new("."));
    let Some(out) = run_probe("fsutil", &["volume", "diskfree", &target.to_string_lossy()]) else {
        return (None, None);
    };
    let mut total = None;
    let mut available = None;
    for line in out.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let parsed = value
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok());
        match key.trim() {
            "Total bytes" => total = parsed,
            "Total free bytes" => available = parsed,
            _ => {}
        }
    }
    (total, available)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checks(names: &[&str]) -> Vec<String> {
        names.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn only_requested_keys_are_present() {
        let output = collect_blocking(&checks(&["os", "mem"]), None);
        assert_eq!(output["success"], serde_json::json!(true));
        assert!(output.get("os").is_some());
        assert!(output.get("arch").is_some());
        assert!(output.get("mem_total_bytes").is_some());
        assert!(output.get("cpu_brand").is_none());
        assert!(output.get("ffmpeg_version").is_none());
        assert!(output.get("disk_total_bytes").is_none());
        assert!(output.get("cwd").is_some());
    }

    #[test]
    fn cpu_and_disk_keys_exist_when_requested() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let output = collect_blocking(&checks(&["cpu", "disk"]), Some(dir.path()));
        assert!(output.get("cores").is_some());
        #[cfg(unix)]
        {
            assert!(output.get("disk_total_bytes").is_some());
        }
    }

    #[test]
    fn ffmpeg_check_never_fails_the_node() {
        let output = collect_blocking(&checks(&["ffmpeg"]), None);
        assert_eq!(output["success"], serde_json::json!(true));
        assert!(output.get("ffmpeg_version").is_some());
        assert!(output.get("has_drawtext").is_some());
    }

    #[test]
    fn empty_checks_still_succeeds() {
        let output = collect_blocking(&[], None);
        assert_eq!(output["success"], serde_json::json!(true));
        assert!(output.get("os").is_none());
        assert!(output.get("path_entries").is_some());
    }
}
