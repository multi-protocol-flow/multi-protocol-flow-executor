//! GitHub Actions Step Summary generation
//!
//! Generates rich Markdown summaries for CI/CD runs and writes to `$GITHUB_STEP_SUMMARY`
//! or explicit `--summary <path>`.

use flow_engine::report::FlowExecutionReport;
use std::path::Path;

use super::CliError;

/// Generate Markdown formatted execution summary
pub fn generate_markdown_summary(report: &FlowExecutionReport, env_name: Option<&str>) -> String {
    flow_engine::report_export::generate_flow_markdown_summary(report, env_name)
}

/// Write Stress Test Step Summary to file if requested via CLI option or `GITHUB_STEP_SUMMARY` environment variable
pub fn write_stress_step_summary_if_requested(
    report: &flow_engine::stress_test::StressTestReport,
    summary_path: Option<&Path>,
) -> Result<(), CliError> {
    let target_path = summary_path
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var_os("GITHUB_STEP_SUMMARY")
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
        });
    if let Some(path) = target_path {
        let markdown = flow_engine::report_export::generate_stress_markdown_summary(report);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                CliError::Io(format!(
                    "Failed to open step summary file {}: {}",
                    path.display(),
                    e
                ))
            })?;

        file.write_all(markdown.as_bytes()).map_err(|e| {
            CliError::Io(format!(
                "Failed to write step summary to {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    Ok(())
}

/// Write Step Summary to file if requested via CLI option or `GITHUB_STEP_SUMMARY` environment variable
pub fn write_step_summary_if_requested(
    report: &FlowExecutionReport,
    env_name: Option<&str>,
    summary_path: Option<&Path>,
) -> Result<(), CliError> {
    let target_path = summary_path
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var_os("GITHUB_STEP_SUMMARY")
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
        });
    if let Some(path) = target_path {
        let markdown = generate_markdown_summary(report, env_name);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                CliError::Io(format!(
                    "Failed to open step summary file {}: {}",
                    path.display(),
                    e
                ))
            })?;

        file.write_all(markdown.as_bytes()).map_err(|e| {
            CliError::Io(format!(
                "Failed to write step summary to {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine::error::{ErrorDetail, ErrorKind};
    use flow_engine::report::{ExecutionMode, ExecutionStatus, NodeExecutionReport, StageExecutionReport};

    fn make_test_report(status: ExecutionStatus) -> FlowExecutionReport {
        let mut report = FlowExecutionReport {
            uuid: "report-123".to_string(),
            flow_uuid: "flow-abc".to_string(),
            flow_name: "Test Payment Flow".to_string(),
            start_time: 1750000000000,
            end_time: 1750000001000,
            duration_ms: 1000,
            total_nodes: 3,
            executed_count: 3,
            status,
            node_reports: vec![],
            statistics: flow_engine::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
            stages: flow_engine::report::FlowStagesReport::default(),
            main_duration_ms: 800,
        };

        report.stages.setup = Some(StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 100,
            node_reports: vec![NodeExecutionReport {
                node_name: "Init Token".to_string(),
                node_type: "script".to_string(),
                status: ExecutionStatus::Success,
                duration_ms: 100,
                ..Default::default()
            }],
            error: None,
        });

        report.stages.main = StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 800,
            node_reports: vec![NodeExecutionReport {
                node_name: "Stripe Charge".to_string(),
                node_type: "http:request".to_string(),
                status: ExecutionStatus::Success,
                duration_ms: 800,
                ..Default::default()
            }],
            error: None,
        };

        report.stages.teardown = Some(StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 100,
            node_reports: vec![NodeExecutionReport {
                node_name: "Cleanup".to_string(),
                node_type: "script".to_string(),
                status: ExecutionStatus::Success,
                duration_ms: 100,
                ..Default::default()
            }],
            error: None,
        });

        report
    }

    #[test]
    fn test_summary_markdown_passed() {
        let report = make_test_report(ExecutionStatus::Success);
        let md = generate_markdown_summary(&report, Some("production"));

        assert!(md.contains("🟢 **PASSED**"));
        assert!(md.contains("Test Payment Flow"));
        assert!(md.contains("`production`"));
        assert!(md.contains("| Setup | 🟢 Success | 1 | 100 ms |"));
        assert!(md.contains("| Main | 🟢 Success | 1 | 800 ms |"));
        assert!(md.contains("| Teardown | 🟢 Success | 1 | 100 ms |"));
        assert!(!md.contains("Failed Nodes"));
    }

    #[test]
    fn test_summary_markdown_failed_node() {
        let mut report = make_test_report(ExecutionStatus::Failed);
        let failed_node = NodeExecutionReport {
            node_name: "Stripe Charge".to_string(),
            node_type: "http:request".to_string(),
            status: ExecutionStatus::Failed,
            duration_ms: 450,
            output_data: Some(serde_json::json!({
                "status": 402,
                "error": "insufficient_funds"
            })),
            error: Some(ErrorDetail {
                error_kind: ErrorKind::Execution,
                error_code: None,
                i18n_args: None,
                message: "HTTP 402 Payment Required".to_string(),
                stack_trace: None,
                context: None,
            }),
            ..Default::default()
        };
        report.stages.main.status = ExecutionStatus::Failed;
        report.stages.main.node_reports = vec![failed_node];

        let md = generate_markdown_summary(&report, None);

        assert!(md.contains("🔴 **FAILED**"));
        assert!(md.contains("Failed Nodes Details"));
        assert!(md.contains("<details>"));
        assert!(md.contains("<summary>❌ <b>Stripe Charge</b>"));
        assert!(md.contains("insufficient_funds"));
    }
}
