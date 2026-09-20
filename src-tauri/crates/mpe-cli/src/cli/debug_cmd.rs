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

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::output;
use super::variables::{parse_vars, resolve_execution_variables};

/// 判断是否为 `SessionEnded` 事件
fn is_session_ended(event: &flow_engine::debug::DebugEvent) -> bool {
    matches!(event, flow_engine::debug::DebugEvent::SessionEnded { .. })
}

// ── 主入口 ────────────────────────────────────────────────────────

/// 启动调试会话
///
/// 从 `flow_path` 加载流程，创建双向通道，执行调试循环。
/// stdin 读取 DebugCommand，stdout 输出 DebugEvent（NDJSON 格式）。
pub async fn execute(
    _format: &str,
    flow_path: &str,
    env: Option<&str>,
    breakpoints: &[String],
    vars: &[String],
    registry: std::sync::Arc<flow_engine::registry::NodeRegistry>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), super::CliError> {
    // 1. 验证文件存在
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(super::CliError::InvalidArgs(format!(
            "Flow file not found: {}",
            flow_path
        )));
    }

    let app_config = super::config::AppConfig::load().map_err(super::CliError::Config)?;
    flow_engine::storage::set_flows_dir_override(Some(app_config.flows_dir.clone()));
    flow_engine::storage::set_environments_dir_override(Some(app_config.environments_dir.clone()));

    // 2. 加载流程
    let flow = flow_engine::storage::load_flow(flow_path)
        .map_err(|e| super::CliError::execution(format!("Failed to load flow: {}", e)))?;

    // 3. 变量分层：流程兜底 → 环境文件 → 本地覆盖 → --var 运行时覆盖
    let cli_vars = if vars.is_empty() {
        None
    } else {
        Some(parse_vars(vars)?)
    };

    let variables = resolve_execution_variables(&flow, env, cli_vars.as_ref())?;

    // 4. 创建通道
    let (debug_event_tx, mut debug_event_rx) =
        mpsc::unbounded_channel::<flow_engine::debug::DebugEvent>();
    let (debug_cmd_tx, debug_cmd_rx) =
        mpsc::unbounded_channel::<flow_engine::debug::DebugCommand>();

    // 5. 启动调试会话（后台任务）
    let bp_list = breakpoints.to_vec();
    let flow_clone = flow.clone();
    let debug_handle = tokio::spawn(async move {
        let executor = flow_engine::debug::DebuggableFlowExecutor::new()
            .with_flow_file_path(path)
            .with_registry(registry);
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
                let _ = debug_cmd_tx.send(flow_engine::debug::DebugCommand::EndDebug);
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
                        match serde_json::from_str::<flow_engine::debug::DebugCommand>(trimmed) {
                            Ok(cmd) => {
                                if debug_cmd_tx.send(cmd).is_err() {
                                    // 调试会话已结束
                                    break;
                                }
                            }
                            Err(e) => {
                                // 输出解析Error到 stderr，保持 stdout 仅输出事件
                                eprintln!("Invalid command JSON: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // stdin EOF — 发送 EndDebug 并退出
                        let _ = debug_cmd_tx.send(flow_engine::debug::DebugCommand::EndDebug);
                        // 继续读取事件直到 SessionEnded
                    }
                    Err(e) => {
                        // stdin 读取Error
                        eprintln!("stdin read error: {}", e);
                        let _ = debug_cmd_tx.send(flow_engine::debug::DebugCommand::EndDebug);
                    }
                }
            }
        }
    }

    // 7. 等待调试会话结束
    let _ = debug_handle.await;

    Ok(())
}
