//! mpe stress 子命令 — 压力测试执行与报告管理

use std::path::PathBuf;
use tokio::sync::mpsc;

use super::{CliError, StressCommands, StressReportCommands, config, output};
use super::output::CliOutput;

/// 执行 stress 子命令
pub async fn execute(
    _format: &str,
    command: StressCommands,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), CliError> {
    match command {
        StressCommands::Run {
            flow,
            concurrent_users,
            ramp_up,
            duration,
            output,
        } => execute_run(&flow, concurrent_users, ramp_up, duration, output.as_deref(), cancel_rx).await,
        StressCommands::Report { command } => execute_report(command, _format).await,
    }
}

/// 执行压力测试
async fn execute_run(
    flow_path: &str,
    concurrent_users: u32,
    ramp_up: u32,
    duration: u32,
    output_path: Option<&str>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), CliError> {
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(CliError::InvalidArgs(format!(
            "Flow file not found: {}",
            flow_path
        )));
    }

    // 加载流程文件（使用统一 FlowFile 格式）
    let flow = crate::load_flow(flow_path)
        .map_err(|e| CliError::Execution(format!("Failed to load flow file: {}", e)))?;

    // 创建压测配置
    let stress_config = crate::stress_test::StressTestConfig {
        concurrent_users,
        ramp_up_seconds: ramp_up,
        duration_seconds: duration,
        flow_uuid: flow.uuid.clone(),
        dataset_path: None,
        dataset_encoding: None,
        dataset_strategy: None,
    };

    // 创建事件通道
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    // 创建执行器
    let executor = crate::stress_test::StressTestExecutor::new(
        flow.clone(),
        stress_config.clone(),
        event_tx,
    );

    // 启动执行器
    executor
        .start()
        .await
        .map_err(|e| CliError::Execution(format!("Failed to start stress test: {}", e)))?;

    // 流式输出事件 (NDJSON)，同时监听取消信号
    let mut final_report: Option<crate::stress_test::StressTestReport> = None;
    let mut cancel_rx = cancel_rx;
    loop {
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Some(evt) => {
                        if let crate::stress_test::StressTestEvent::Ended { ref report } = evt {
                            final_report = Some((**report).clone());
                        }
                        output::output_ndjson(&evt);
                    }
                    None => break, // 通道关闭，执行完成
                }
            }
            _ = cancel_rx.changed() => {
                // Ctrl+C: 输出取消事件并退出
                eprintln!("\nStress test cancelled by user, stopping...");
                // drop executor 停止所有任务
                drop(executor);
                break;
            }
        }
    }

    // 保存报告到默认目录
    if let Some(report) = &final_report {
        let app_config = config::AppConfig::load().map_err(CliError::Config)?;
        let _ = crate::storage::save_stress_report(report, &app_config.reports_dir);
    }

    // 如指定输出路径，写入报告文件
    if let Some(path) = output_path {
        if let Some(report) = final_report {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| CliError::Execution(format!("Failed to serialize report: {}", e)))?;
            std::fs::write(path, json)
                .map_err(|e| CliError::Io(format!("Failed to write report: {}", e)))?;
        }
    }

    Ok(())
}

/// 执行压测报告管理子命令
async fn execute_report(command: StressReportCommands, format: &str) -> Result<(), CliError> {
    match command {
        StressReportCommands::List => {
            let app_config = config::AppConfig::load().map_err(CliError::Config)?;
            let reports = crate::storage::list_stress_reports(&app_config.reports_dir)
                .map_err(|e| CliError::Execution(format!("Failed to list stress reports: {}", e)))?;

            if format == "table" {
                let headers = ["UUID", "Flow Name", "Start Time"];
                let rows: Vec<Vec<String>> = reports
                    .iter()
                    .map(|r| {
                        let dt = chrono::DateTime::from_timestamp_millis(r.start_time as i64)
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| r.start_time.to_string());
                        vec![
                            r.uuid.chars().take(8).collect(),
                            r.flow_name.clone(),
                            dt,
                        ]
                    })
                    .collect();
                print!("{}", output::format_table(&headers, &rows));
            } else {
                output::output_json(&CliOutput::success(&reports));
            }
            Ok(())
        }
        StressReportCommands::Show { file_path } => {
            let report = crate::storage::load_stress_report(&PathBuf::from(&file_path))
                .map_err(|e| CliError::Execution(format!("Failed to load stress report: {}", e)))?;
            output::output_json(&CliOutput::success(&report));
            Ok(())
        }
        StressReportCommands::Delete { file_path } => {
            crate::storage::delete_stress_report(&PathBuf::from(&file_path))
                .map_err(|e| CliError::Execution(format!("Failed to delete stress report: {}", e)))?;
            output::output_json(&CliOutput::success(serde_json::json!({"deleted": file_path})));
            Ok(())
        }
    }
}
