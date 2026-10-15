//! GitHub Actions Step Summary generation
//!
//! Generates rich Markdown summaries for CI/CD runs and writes to `$GITHUB_STEP_SUMMARY`
//! or explicit `--summary <path>`.

use flow_engine::report::{ExecutionStatus, FlowExecutionReport};
use std::path::Path;

use super::CliError;

/// Generate Markdown formatted execution summary
pub fn generate_markdown_summary(report: &FlowExecutionReport, env_name: Option<&str>) -> String {
    let mut md = String::with_capacity(1024);

    let is_success = report.status == ExecutionStatus::Success;
    let badge = if is_success {
        "🟢 **PASSED**"
    } else {
        "🔴 **FAILED**"
    };

    md.push_str(&format!(
        "## {} Flow Execution: {}\n\n",
        badge, report.flow_name
    ));

    let env_display = env_name.unwrap_or("default");
    md.push_str(&format!(
        "- **Status**: {}\n- **Flow UUID**: `{}`\n- **Environment**: `{}`\n- **Total Duration**: {} ms\n- **Main Duration**: {} ms\n\n",
        if is_success { "Success" } else { "Failed" },
        report.flow_uuid,
        env_display,
        report.duration_ms,
        report.main_duration_ms
    ));

    // 阶段统计表格
    md.push_str("### 📊 Lifecycle Stages\n\n");
    md.push_str("| Stage | Status | Nodes (Executed/Total) | Duration |\n");
    md.push_str("|---|---|---|---|\n");

    let status_str = |status: &ExecutionStatus| match status {
        ExecutionStatus::Success => "🟢 Success",
        ExecutionStatus::Failed => "🔴 Failed",
        ExecutionStatus::Skipped => "⚪ Skipped",
        ExecutionStatus::Cancelled => "🟡 Cancelled",
    };

    if let Some(setup) = &report.stages.setup {
        let executed = setup.node_reports.len();
        md.push_str(&format!(
            "| Setup | {} | {} | {} ms |\n",
            status_str(&setup.status),
            executed,
            setup.duration_ms
        ));
    }

    let main_executed = if !report.stages.main.node_reports.is_empty() {
        report.stages.main.node_reports.len()
    } else {
        report.node_reports.len()
    };
    let main_duration = if report.stages.main.duration_ms > 0 {
        report.stages.main.duration_ms
    } else {
        report.main_duration_ms
    };
    md.push_str(&format!(
        "| Main | {} | {} | {} ms |\n",
        status_str(&report.stages.main.status),
        main_executed,
        main_duration
    ));

    if let Some(teardown) = &report.stages.teardown {
        let executed = teardown.node_reports.len();
        md.push_str(&format!(
            "| Teardown | {} | {} | {} ms |\n",
            status_str(&teardown.status),
            executed,
            teardown.duration_ms
        ));
    }
    md.push('\n');

    // 收集所有阶段的失败节点
    let mut failed_nodes = Vec::new();
    if let Some(setup) = &report.stages.setup {
        for n in &setup.node_reports {
            if n.status == ExecutionStatus::Failed {
                failed_nodes.push(("Setup", n));
            }
        }
    }
    for n in &report.stages.main.node_reports {
        if n.status == ExecutionStatus::Failed {
            failed_nodes.push(("Main", n));
        }
    }
    if let Some(teardown) = &report.stages.teardown {
        for n in &teardown.node_reports {
            if n.status == ExecutionStatus::Failed {
                failed_nodes.push(("Teardown", n));
            }
        }
    }

    // 若 stages 中未单独收集，回退到顶级 node_reports
    if failed_nodes.is_empty() {
        for n in &report.node_reports {
            if n.status == ExecutionStatus::Failed {
                failed_nodes.push(("Main", n));
            }
        }
    }

    if !failed_nodes.is_empty() {
        md.push_str("### ❌ Failed Nodes Details\n\n");
        for (stage, node) in failed_nodes {
            let err_msg = node
                .error
                .as_ref()
                .map(|e| e.message.as_str())
                .unwrap_or("Unknown error");
            let err_kind = node
                .error
                .as_ref()
                .map(|e| format!("{:?}", e.error_kind))
                .unwrap_or_else(|| "Unknown".to_string());

            let escaped_name = super::junit::xml_escape(&node.node_name);
            let escaped_err = super::junit::xml_escape(err_msg);
            md.push_str(&format!(
                "<details>\n<summary>❌ <b>{}</b> [<code>{}</code>] (<code>{}</code>) — {}</summary>\n\n",
                escaped_name, stage, node.node_type, escaped_err
            ));

            md.push_str(&format!("- **Error Kind**: `{}`\n", err_kind));
            md.push_str(&format!("- **Duration**: {} ms\n", node.duration_ms));

            if let Some(out) = &node.output_data {
                let formatted_json = serde_json::to_string_pretty(out).unwrap_or_default();
                md.push_str("\n**Output / Error Snapshot:**\n```json\n");
                md.push_str(&formatted_json);
                md.push_str("\n```\n");
            }

            if let Some(err) = &node.error {
                if let Some(stack) = &err.stack_trace {
                    md.push_str("\n**Stack Trace:**\n```text\n");
                    md.push_str(stack);
                    md.push_str("\n```\n");
                }
            }

            md.push_str("</details>\n\n");
        }
    }

    md
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
    use flow_engine::report::{ExecutionMode, NodeExecutionReport, StageExecutionReport};

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
