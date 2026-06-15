//! mpe report 子命令 — 报告管理 (list/show/delete)

use super::{CliError, ReportCommands, config, output};
use super::output::CliOutput;

/// 执行 report 管理子命令
pub async fn execute(
    _format: &str,
    command: ReportCommands,
) -> Result<(), CliError> {
    match command {
        ReportCommands::List => execute_list(_format).await,
        ReportCommands::Show { uuid } => execute_show(&uuid).await,
        ReportCommands::Delete { uuid } => execute_delete(&uuid).await,
    }
}

async fn execute_list(format: &str) -> Result<(), CliError> {
    let app_config = config::AppConfig::load().map_err(CliError::Config)?;
    let reports = crate::list_reports(&app_config.reports_dir)
        .map_err(|e| CliError::Execution(format!("Failed to list reports: {}", e)))?;

    if format == "table" {
        let headers = ["UUID", "Flow Name", "Status", "Execution Time", "Duration(ms)"];
        let rows: Vec<Vec<String>> = reports
            .iter()
            .map(|r| {
                let dt = chrono::DateTime::from_timestamp_millis(r.timestamp as i64)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| r.timestamp.to_string());
                vec![
                    r.uuid.chars().take(8).collect(),
                    r.flow_name.clone(),
                    r.status.clone(),
                    dt,
                    r.duration_ms.to_string(),
                ]
            })
            .collect();
        print!("{}", output::format_table(&headers, &rows));
    } else {
        let cli_output = CliOutput::success(&reports);
        output::output_json(&cli_output);
    }
    Ok(())
}

async fn execute_show(uuid: &str) -> Result<(), CliError> {
    let app_config = config::AppConfig::load().map_err(CliError::Config)?;
    let report = crate::load_report(uuid, &app_config.reports_dir)
        .map_err(|e| CliError::Execution(format!("Failed to load report: {}", e)))?;
    let cli_output = CliOutput::success(&report);
    output::output_json(&cli_output);
    Ok(())
}

async fn execute_delete(uuid: &str) -> Result<(), CliError> {
    let app_config = config::AppConfig::load().map_err(CliError::Config)?;
    crate::delete_report(uuid, &app_config.reports_dir)
        .map_err(|e| CliError::Execution(format!("Failed to delete report: {}", e)))?;
    let cli_output = CliOutput::success(serde_json::json!({"deleted": uuid}));
    output::output_json(&cli_output);
    Ok(())
}
