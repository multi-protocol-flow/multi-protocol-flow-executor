//! mpe debug 子命令 — NDJSON 双向调试协议
//!
//! 通过 stdin/stdout 实现 NDJSON 双向通信:
//! - stdout: 输出 DebugEvent（每行一个 JSON）
//! - stdin:  接收 DebugCommand（每行一个 JSON）
//!
//! 协议示例:
//! ```text
//! → {"SessionStarted":{"flow_uuid":"..."}}
//! → {"PausedAtNode":{"node_uuid":"...","reason":{...},...}}
//! ← "Continue"
//! → {"NodeExecuted":{"entry":{...}}}
//! → {"SessionEnded":{"reason":{...},...}}
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::output;

// ── 变量解析 ──────────────────────────────────────────────────────

/// 解析 `--var key=value` 参数为 HashMap
///
/// 值优先尝试 JSON 解析，失败则作为纯字符串处理。
fn parse_vars(vars: &[String]) -> Result<HashMap<String, serde_json::Value>, super::CliError> {
    let mut map = HashMap::new();
    for v in vars {
        let (key, val) = v.split_once('=').ok_or_else(|| {
            super::CliError::InvalidArgs(format!("--var 格式错误 (需要 key=value): {}", v))
        })?;
        let json_val =
            serde_json::from_str(val).unwrap_or(serde_json::Value::String(val.to_string()));
        map.insert(key.to_string(), json_val);
    }
    Ok(map)
}

/// 判断是否为 SessionEnded 事件
fn is_session_ended(event: &app_lib::debug::DebugEvent) -> bool {
    matches!(event, app_lib::debug::DebugEvent::SessionEnded { .. })
}

// ── 主入口 ────────────────────────────────────────────────────────

/// 启动调试会话
///
/// 从 `flow_path` 加载流程，创建双向通道，执行调试循环。
/// stdin 读取 DebugCommand，stdout 输出 DebugEvent（NDJSON 格式）。
pub async fn execute(
    _format: &str,
    flow_path: &str,
    breakpoints: &[String],
    vars: &[String],
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), super::CliError> {
    // 1. 验证文件存在
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(super::CliError::InvalidArgs(format!(
            "流程文件不存在: {}",
            flow_path
        )));
    }

    // 2. 加载流程
    let flow = app_lib::load_flow(flow_path)
        .map_err(|e| super::CliError::Execution(format!("加载流程失败: {}", e)))?;

    // 3. 解析变量
    let variables: Option<serde_json::Value> = if vars.is_empty() {
        None
    } else {
        let map = parse_vars(vars)?;
        Some(serde_json::Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, v))
                .collect::<serde_json::Map<_, _>>(),
        ))
    };

    // 4. 创建通道
    let (debug_event_tx, mut debug_event_rx) = mpsc::unbounded_channel::<app_lib::debug::DebugEvent>();
    let (debug_cmd_tx, debug_cmd_rx) = mpsc::unbounded_channel::<app_lib::debug::DebugCommand>();

    // 5. 启动调试会话（后台任务）
    let bp_list = breakpoints.to_vec();
    let flow_clone = flow.clone();
    let debug_handle = tokio::spawn(async move {
        let executor = app_lib::debug::DebuggableFlowExecutor::new()
            .with_flow_file_path(path);
        executor
            .start_debug(
                &flow_clone,
                variables,
                bp_list,
                None, // CLI 模式不需要日志发送器
                debug_event_tx,
                debug_cmd_rx,
            )
            .await
    });

    // 6. 双向 I/O 循环
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut cancel_rx = cancel_rx;

    loop {
        tokio::select! {
            // Ctrl+C 取消信号
            _ = cancel_rx.changed() => {
                // 收到取消信号，发送 EndDebug 命令
                let _ = debug_cmd_tx.send(app_lib::debug::DebugCommand::EndDebug);
                // 继续读取事件直到 SessionEnded
            }
            // 事件: 调试器 → stdout (NDJSON)
            event = debug_event_rx.recv() => {
                match event {
                    Some(evt) => {
                        let is_end = is_session_ended(&evt);
                        output::output_ndjson(&evt);
                        if is_end {
                            break;
                        }
                    }
                    None => break, // 通道关闭
                }
            }
            // 命令: stdin → 调试器
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        // 尝试解析为 DebugCommand
                        match serde_json::from_str::<app_lib::debug::DebugCommand>(trimmed) {
                            Ok(cmd) => {
                                if debug_cmd_tx.send(cmd).is_err() {
                                    // 调试会话已结束
                                    break;
                                }
                            }
                            Err(e) => {
                                // 输出解析错误到 stderr，保持 stdout 仅输出事件
                                eprintln!("无效的命令 JSON: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // stdin EOF — 发送 EndDebug 并退出
                        let _ = debug_cmd_tx.send(app_lib::debug::DebugCommand::EndDebug);
                        // 继续读取事件直到 SessionEnded
                    }
                    Err(e) => {
                        // stdin 读取错误
                        eprintln!("stdin 读取错误: {}", e);
                        let _ = debug_cmd_tx.send(app_lib::debug::DebugCommand::EndDebug);
                    }
                }
            }
        }
    }

    // 7. 等待调试会话结束
    let _ = debug_handle.await;

    Ok(())
}
