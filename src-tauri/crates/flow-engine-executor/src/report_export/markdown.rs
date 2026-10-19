//! Markdown Summary Generators for Flow Execution and Stress Testing.

use flow_engine_core::report::{ExecutionStatus, FlowExecutionReport};

use crate::stress_test::StressTestReport;

/// Handles `&`, `<`, `>`, `"`, `'` — standard HTML/XML entities.
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Generate Markdown formatted execution summary for a Flow Execution.
pub fn generate_flow_markdown_summary(
    report: &FlowExecutionReport,
    env_name: Option<&str>,
) -> String {
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

            let escaped_name = html_escape(&node.node_name);
            let escaped_err = html_escape(err_msg);
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

/// Generate Markdown formatted execution summary for a Stress Test.
pub fn generate_stress_markdown_summary(report: &StressTestReport) -> String {
    let mut md = String::with_capacity(2048);

    let is_success = report.statistics.failed == 0 && report.statistics.total_iterations > 0;
    let badge = if is_success {
        "🟢 **PASSED**"
    } else if report.statistics.total_iterations == 0 {
        "⚪ **NO ITERATIONS**"
    } else {
        "🔴 **FAILED**"
    };

    let rate_badge = if report.statistics.success_rate >= 0.99 {
        "🟢"
    } else if report.statistics.success_rate >= 0.95 {
        "🟡"
    } else {
        "🔴"
    };

    md.push_str(&format!(
        "## {} Stress Test: {}\n\n",
        badge, report.flow_name
    ));

    md.push_str(&format!(
        "- **Flow UUID**: `{}`\n- **Report UUID**: `{}`\n- **Total Duration**: {} ms ({}s configured)\n- **Concurrent Users**: {} (Ramp-up: {}s)\n- **Total Iterations**: {}\n- **Success Rate**: {} {:.2}% ({} passed, {} failed)\n- **Throughput (RPS)**: {:.2} req/s\n",
        report.flow_uuid,
        report.uuid,
        report.duration_ms,
        report.config.duration_seconds,
        report.config.concurrent_users,
        report.config.ramp_up_seconds,
        report.statistics.total_iterations,
        rate_badge,
        report.statistics.success_rate * 100.0,
        report.statistics.successful,
        report.statistics.failed,
        report.statistics.rps
    ));

    if let Some(dataset) = &report.config.dataset_path {
        md.push_str(&format!("- **Dataset**: `{}`\n", dataset));
    }
    md.push('\n');

    // 延迟分位数表格
    md.push_str("### ⏱️ Latency Percentiles\n\n");
    md.push_str("| Metric | Latency |\n");
    md.push_str("|---|---|\n");
    md.push_str(&format!(
        "| Min | {} ms |\n| P50 (Median) | {} ms |\n| P90 | {} ms |\n| P95 | {} ms |\n| P99 | {} ms |\n| Max | {} ms |\n| Avg | {:.2} ms |\n\n",
        report.response_time_distribution.min_ms,
        report.response_time_distribution.p50_ms,
        report.response_time_distribution.p90_ms,
        report.response_time_distribution.p95_ms,
        report.response_time_distribution.p99_ms,
        report.response_time_distribution.max_ms,
        report.response_time_distribution.avg_ms
    ));

    // 节点级指标表格
    if !report.node_statistics.is_empty() {
        md.push_str("### 🧩 Node-level Performance\n\n");
        md.push_str("| Node | Type | Total Executions | Success Rate | Avg Latency | P95 Latency |\n");
        md.push_str("|---|---|---|---|---|---|\n");
        for n in &report.node_statistics {
            let escaped_name = html_escape(&n.node_name);
            md.push_str(&format!(
                "| {} | <code>{}</code> | {} | {:.2}% | {:.2} ms | {} ms |\n",
                escaped_name,
                n.node_type,
                n.total_executions,
                n.success_rate * 100.0,
                n.response_time_distribution.avg_ms,
                n.response_time_distribution.p95_ms
            ));
        }
        md.push('\n');
    }

    // 错误类型统计表格
    if !report.error_statistics.is_empty() {
        md.push_str("### ⚠️ Error Statistics\n\n");
        md.push_str("| Error Kind | Count | Share |\n");
        md.push_str("|---|---|---|\n");
        let total_failed = report.statistics.failed.max(1) as f64;
        for err in &report.error_statistics {
            let share = (err.count as f64 / total_failed) * 100.0;
            md.push_str(&format!(
                "| <code>{}</code> | {} | {:.2}% |\n",
                html_escape(&err.error_kind),
                err.count,
                share
            ));
        }
        md.push('\n');

        // 收集采样样本
        let mut sample_count = 0;
        let mut samples_md = String::new();
        for err in &report.error_statistics {
            for sample in &err.samples {
                if sample_count >= 10 {
                    break;
                }
                samples_md.push_str(&format!(
                    "- [Iter #{}] **{}**: {}\n",
                    sample.iteration,
                    html_escape(&sample.node_name),
                    html_escape(&sample.error_message)
                ));
                sample_count += 1;
            }
        }
        if !samples_md.is_empty() {
            md.push_str("<details>\n<summary>⚠️ Error Samples (First 10)</summary>\n\n");
            md.push_str(&samples_md);
            md.push_str("\n</details>\n\n");
        }
    }

    // 生命周期阶段（若有）
    if let Some(lifecycle) = &report.lifecycle {
        let status_str = |status: &ExecutionStatus| match status {
            ExecutionStatus::Success => "🟢 Success",
            ExecutionStatus::Failed => "🔴 Failed",
            ExecutionStatus::Skipped => "⚪ Skipped",
            ExecutionStatus::Cancelled => "🟡 Cancelled",
        };
        md.push_str("### 📊 Lifecycle Stages\n\n");
        md.push_str("| Stage | Status | Nodes | Duration |\n");
        md.push_str("|---|---|---|---|\n");
        if let Some(setup) = &lifecycle.setup {
            md.push_str(&format!(
                "| Setup | {} | {} | {} ms |\n",
                status_str(&setup.status),
                setup.node_reports.len(),
                setup.duration_ms
            ));
        }
        md.push_str(&format!(
            "| Main | {} | {} | {} ms |\n",
            status_str(&lifecycle.main.status),
            lifecycle.main.node_reports.len(),
            lifecycle.main.duration_ms
        ));
        if let Some(teardown) = &lifecycle.teardown {
            md.push_str(&format!(
                "| Teardown | {} | {} | {} ms |\n",
                status_str(&teardown.status),
                teardown.node_reports.len(),
                teardown.duration_ms
            ));
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stress_test::{
        ErrorSample, ErrorStatistics, NodeLevelStatistics, ResponseTimeDistribution,
        StressTestConfig, StressTestStatistics,
    };
    use flow_engine_core::error::{ErrorDetail, ErrorKind};
    use flow_engine_core::report::{ExecutionMode, NodeExecutionReport, StageExecutionReport};

    fn make_test_flow_report(status: ExecutionStatus) -> FlowExecutionReport {
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
            statistics: flow_engine_core::statistics::FlowStatistics::default(),
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
            stages: flow_engine_core::report::FlowStagesReport::default(),
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
    fn test_flow_summary_markdown_passed() {
        let report = make_test_flow_report(ExecutionStatus::Success);
        let md = generate_flow_markdown_summary(&report, Some("production"));

        assert!(md.contains("🟢 **PASSED**"));
        assert!(md.contains("Test Payment Flow"));
        assert!(md.contains("`production`"));
        assert!(md.contains("| Setup | 🟢 Success | 1 | 100 ms |"));
        assert!(md.contains("| Main | 🟢 Success | 1 | 800 ms |"));
        assert!(md.contains("| Teardown | 🟢 Success | 1 | 100 ms |"));
        assert!(!md.contains("Failed Nodes"));
    }

    #[test]
    fn test_flow_summary_markdown_failed() {
        let mut report = make_test_flow_report(ExecutionStatus::Failed);
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

        let md = generate_flow_markdown_summary(&report, None);

        assert!(md.contains("🔴 **FAILED**"));
        assert!(md.contains("Failed Nodes Details"));
        assert!(md.contains("<details>"));
        assert!(md.contains("<summary>❌ <b>Stripe Charge</b>"));
        assert!(md.contains("insufficient_funds"));
    }

    #[test]
    fn test_stress_summary_markdown() {
        let report = StressTestReport {
            uuid: "stress-rep-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Checkout Load Test".to_string(),
            start_time: 1000,
            end_time: 61000,
            duration_ms: 60000,
            config: StressTestConfig {
                concurrent_users: 50,
                ramp_up_seconds: 5,
                duration_seconds: 60,
                flow_uuid: "flow-001".to_string(),
                dataset_path: Some("data.csv".to_string()),
                ..Default::default()
            },
            statistics: StressTestStatistics {
                total_iterations: 1000,
                successful: 980,
                failed: 20,
                success_rate: 0.98,
                rps: 16.33,
            },
            response_time_distribution: ResponseTimeDistribution {
                avg_ms: 120.5,
                min_ms: 45,
                max_ms: 850,
                p50_ms: 110,
                p90_ms: 220,
                p95_ms: 310,
                p99_ms: 550,
            },
            node_statistics: vec![NodeLevelStatistics {
                node_uuid: "n-1".to_string(),
                node_name: "API Request".to_string(),
                node_type: "http:request".to_string(),
                total_executions: 1000,
                successful_executions: 980,
                failed_executions: 20,
                success_rate: 0.98,
                response_time_distribution: ResponseTimeDistribution {
                    avg_ms: 120.5,
                    p95_ms: 310,
                    ..Default::default()
                },
                error_statistics: vec![],
            }],
            error_statistics: vec![ErrorStatistics {
                error_kind: "HttpTimeout".to_string(),
                count: 20,
                samples: vec![ErrorSample::new(
                    42,
                    "n-1",
                    "API Request",
                    "Gateway timeout 504",
                )],
            }],
            ..Default::default()
        };

        let md = generate_stress_markdown_summary(&report);
        assert!(md.contains("Checkout Load Test"));
        assert!(md.contains("Concurrent Users**: 50"));
        assert!(md.contains("98.00%"));
        assert!(md.contains("16.33 req/s"));
        assert!(md.contains("Latency Percentiles"));
        assert!(md.contains("| P99 | 550 ms |"));
        assert!(md.contains("Node-level Performance"));
        assert!(md.contains("Error Statistics"));
        assert!(md.contains("Gateway timeout 504"));
    }
}
