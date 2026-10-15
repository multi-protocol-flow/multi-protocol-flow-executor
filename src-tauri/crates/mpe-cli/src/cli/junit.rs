//! `JUnit` XML report generation
//!
//! Converts a [`FlowExecutionReport`] into standard `JUnit` XML (Jenkins /
//! GitHub Actions / GitLab CI compatible), used by
//! `mpe run --report-format junit [--report-file <path>]`.

use flow_engine::report::{ExecutionStatus, FlowExecutionReport, NodeExecutionReport};

/// Format an execution report as `JUnit` XML aligned with Setup / Main / Teardown stages.
///
/// Layout:
/// - root `<testsuites>` aggregating totals across all stages.
/// - one `<testsuite>` per non-empty lifecycle stage:
///   - `{flow_name} :: Setup`
///   - `{flow_name} :: Main`
///   - `{flow_name} :: Teardown`
/// - one `<testcase>` per node report; failed nodes carry `<failure>` with snapshot details,
///   skipped nodes carry `<skipped>`.
pub fn format_junit_xml(report: &FlowExecutionReport) -> String {
    struct StageData<'a> {
        name: &'a str,
        id_suffix: &'a str,
        status: ExecutionStatus,
        duration_ms: u64,
        nodes: Vec<NodeExecutionReport>,
    }

    let mut stages: Vec<StageData> = Vec::new();

    // 1. Setup stage
    if let Some(setup) = &report.stages.setup {
        if !setup.node_reports.is_empty()
            || setup.status != ExecutionStatus::Skipped
            || report
                .flow_snapshot
                .as_ref()
                .and_then(|f| f.setup.as_ref())
                .map(|s| !s.nodes.is_empty())
                .unwrap_or(false)
        {
            let nodes = if setup.node_reports.is_empty() && setup.status == ExecutionStatus::Failed {
                vec![NodeExecutionReport {
                    node_name: "Setup".to_string(),
                    node_type: "lifecycle".to_string(),
                    status: ExecutionStatus::Failed,
                    error: setup.error.clone(),
                    ..Default::default()
                }]
            } else {
                setup.node_reports.clone()
            };
            stages.push(StageData {
                name: "Setup",
                id_suffix: "setup",
                status: setup.status.clone(),
                duration_ms: setup.duration_ms,
                nodes,
            });
        }
    }

    // 2. Main stage
    let main_nodes = if !report.stages.main.node_reports.is_empty() {
        report.stages.main.node_reports.clone()
    } else if report.stages.setup.is_none()
        && report.stages.teardown.is_none()
        && !report.node_reports.is_empty()
    {
        report.node_reports.clone()
    } else if report.stages.main.status == ExecutionStatus::Skipped {
        // If Main was skipped (e.g. Setup failed), populate skipped placeholders from snapshot
        if let Some(snapshot) = &report.flow_snapshot {
            snapshot
                .nodes
                .iter()
                .map(|n| NodeExecutionReport {
                    node_uuid: n.uuid.clone(),
                    node_name: n.name.clone(),
                    node_type: n.node_type.clone(),
                    status: ExecutionStatus::Skipped,
                    skip_reason: Some("Setup stage failed".to_string()),
                    ..Default::default()
                })
                .collect()
        } else {
            vec![NodeExecutionReport {
                node_name: "Main".to_string(),
                node_type: "workflow".to_string(),
                status: ExecutionStatus::Skipped,
                skip_reason: Some("Setup stage failed".to_string()),
                ..Default::default()
            }]
        }
    } else if report.stages.main.status == ExecutionStatus::Failed {
        vec![NodeExecutionReport {
            node_name: "Main".to_string(),
            node_type: "workflow".to_string(),
            status: ExecutionStatus::Failed,
            error: report.stages.main.error.clone(),
            ..Default::default()
        }]
    } else {
        Vec::new()
    };

    let main_duration = if report.stages.main.duration_ms > 0 {
        report.stages.main.duration_ms
    } else {
        report.main_duration_ms
    };

    stages.push(StageData {
        name: "Main",
        id_suffix: "main",
        status: report.stages.main.status.clone(),
        duration_ms: main_duration,
        nodes: main_nodes,
    });

    // 3. Teardown stage
    if let Some(teardown) = &report.stages.teardown {
        if !teardown.node_reports.is_empty()
            || teardown.status != ExecutionStatus::Skipped
            || report
                .flow_snapshot
                .as_ref()
                .and_then(|f| f.teardown.as_ref())
                .map(|t| !t.nodes.is_empty())
                .unwrap_or(false)
        {
            let nodes = if teardown.node_reports.is_empty() && teardown.status == ExecutionStatus::Failed {
                vec![NodeExecutionReport {
                    node_name: "Teardown".to_string(),
                    node_type: "lifecycle".to_string(),
                    status: ExecutionStatus::Failed,
                    error: teardown.error.clone(),
                    ..Default::default()
                }]
            } else {
                teardown.node_reports.clone()
            };
            stages.push(StageData {
                name: "Teardown",
                id_suffix: "teardown",
                status: teardown.status.clone(),
                duration_ms: teardown.duration_ms,
                nodes,
            });
        }
    }

    // Totals across all suites
    let total_tests: usize = stages.iter().map(|s| s.nodes.len()).sum();
    let total_failures: usize = stages
        .iter()
        .map(|s| {
            s.nodes
                .iter()
                .filter(|n| n.status == ExecutionStatus::Failed)
                .count()
        })
        .sum();
    let total_duration_sec = millis_to_secs(report.duration_ms);

    let mut xml = String::with_capacity(512 + total_tests * 128);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuites name=\"MPE Flow Execution\" tests=\"{total_tests}\" failures=\"{total_failures}\" errors=\"0\" time=\"{total_duration_sec}\">\n"
    ));

    for stage in stages {
        let suite_name = format!("{} :: {}", report.flow_name, stage.name);
        let suite_id = format!("{}:{}", report.flow_uuid, stage.id_suffix);
        let stage_tests = stage.nodes.len();
        let stage_failures = stage
            .nodes
            .iter()
            .filter(|n| n.status == ExecutionStatus::Failed)
            .count();
        let stage_duration_sec = millis_to_secs(stage.duration_ms);

        xml.push_str(&format!(
            "  <testsuite name=\"{}\" id=\"{}\" tests=\"{stage_tests}\" failures=\"{stage_failures}\" errors=\"0\" time=\"{stage_duration_sec}\" timestamp=\"{}\">\n",
            xml_escape(&suite_name),
            xml_escape(&suite_id),
            iso8601(report.start_time),
        ));

        for node in &stage.nodes {
            let name = format!("{} ({})", node.node_name, node.node_type);
            xml.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"{}\" time=\"{}\"",
                xml_escape(&report.flow_uuid),
                xml_escape(&name),
                millis_to_secs(node.duration_ms),
            ));

            match &node.status {
                ExecutionStatus::Failed => {
                    let message = node
                        .error
                        .as_ref()
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| "Unknown error".to_string());
                    let error_type = node
                        .error
                        .as_ref()
                        .map(|e| format!("{:?}", e.error_kind))
                        .unwrap_or_else(|| "Unknown".to_string());

                    let mut details = format!("ErrorKind: {}\nMessage: {}\n", error_type, message);

                    // 提取关键输出快照（HTTP 状态码、请求 URL、断言差异）
                    if let Some(out) = &node.output_data {
                        let mut snapshots = Vec::new();
                        if let Some(status) = out.get("status").or_else(|| out.get("status_code")) {
                            snapshots.push(format!("status: {}", json_val_to_str(status)));
                        }
                        if let Some(url) = out.get("url") {
                            snapshots.push(format!("url: {}", json_val_to_str(url)));
                        }
                        if let Some(diff) = out.get("diff").or_else(|| out.get("assertion_diff")) {
                            snapshots.push(format!("diff: {}", json_val_to_str(diff)));
                        } else if let (Some(expected), Some(actual)) =
                            (out.get("expected"), out.get("actual"))
                        {
                            snapshots.push(format!(
                                "expected: {}, actual: {}",
                                json_val_to_str(expected),
                                json_val_to_str(actual)
                            ));
                        }
                        if !snapshots.is_empty() {
                            details.push_str(&format!("Snapshot: {}\n", snapshots.join(", ")));
                        }
                    }

                    if let Some(err) = &node.error {
                        if let Some(stack) = &err.stack_trace {
                            details.push_str(&format!("StackTrace:\n{}", stack));
                        }
                    }

                    xml.push_str(&format!(
                        ">\n      <failure message=\"{}\" type=\"{}\">{}</failure>\n    </testcase>\n",
                        xml_escape(&message),
                        xml_escape(&error_type),
                        xml_escape(details.trim_end()),
                    ));
                }
                ExecutionStatus::Skipped => {
                    let reason = if stage.status == ExecutionStatus::Skipped
                        && node.skip_reason.as_deref().unwrap_or("").is_empty()
                    {
                        "Setup stage failed".to_string()
                    } else {
                        node.skip_reason.clone().unwrap_or_default()
                    };

                    if reason.is_empty() {
                        xml.push_str(">\n      <skipped/>\n    </testcase>\n");
                    } else {
                        xml.push_str(&format!(
                            ">\n      <skipped message=\"{}\"/>\n    </testcase>\n",
                            xml_escape(&reason),
                        ));
                    }
                }
                ExecutionStatus::Success | ExecutionStatus::Cancelled => {
                    xml.push_str("/>\n");
                }
            }
        }

        xml.push_str("  </testsuite>\n");
    }

    xml.push_str("</testsuites>\n");
    xml
}

/// Convert milliseconds to a seconds string with millisecond precision.
fn millis_to_secs(ms: u64) -> String {
    format!("{:.3}", ms as f64 / 1000.0)
}

/// Convert a millisecond timestamp to ISO 8601 (UTC). Empty on invalid input.
fn iso8601(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Escape a string for safe inclusion in XML text or attribute content.
///
/// Handles `&`, `<`, `>`, `"`, `'` — the five predefined XML entities.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn json_val_to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine::error::{ErrorDetail, ErrorKind};
    use flow_engine::report::{ExecutionMode, NodeExecutionReport, StageExecutionReport};

    fn node_report(
        name: &str,
        node_type: &str,
        status: ExecutionStatus,
        duration_ms: u64,
    ) -> NodeExecutionReport {
        NodeExecutionReport {
            sub_flow_report: None,
            node_uuid: format!("uuid-{}", name),
            node_type: node_type.to_string(),
            node_name: name.to_string(),
            start_time: 1_750_000_000_000,
            end_time: 1_750_000_000_000 + duration_ms,
            duration_ms,
            status,
            input_data: None,
            output_data: None,
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: None,
            is_detail_stripped: None,
        }
    }

    fn flow_report(node_reports: Vec<NodeExecutionReport>) -> FlowExecutionReport {
        FlowExecutionReport {
            uuid: "report-1".to_string(),
            flow_uuid: "flow-1".to_string(),
            flow_name: "My Flow".to_string(),
            start_time: 1_750_000_000_000,
            end_time: 1_750_000_001_500,
            duration_ms: 1500,
            total_nodes: node_reports.len() as u32,
            executed_count: node_reports.len() as u32,
            status: ExecutionStatus::Success,
            node_reports,
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
            main_duration_ms: 1500,
        }
    }

    /// 正常流程：主阶段对齐 `My Flow :: Main`
    #[test]
    fn test_junit_xml_normal_flow() {
        let report = flow_report(vec![
            node_report("Entry", "entry", ExecutionStatus::Success, 10),
            node_report(
                "HTTP Request",
                "http:request",
                ExecutionStatus::Success,
                120,
            ),
        ]);
        let xml = format_junit_xml(&report);

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(xml.contains(
            "<testsuites name=\"MPE Flow Execution\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"1.500\">"
        ));
        assert!(xml.contains(
            "<testsuite name=\"My Flow :: Main\" id=\"flow-1:main\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"1.500\""
        ));
        assert!(
            xml.contains("<testcase classname=\"flow-1\" name=\"Entry (entry)\" time=\"0.010\"/>")
        );
        assert!(xml.contains(
            "<testcase classname=\"flow-1\" name=\"HTTP Request (http:request)\" time=\"0.120\"/>"
        ));
        assert!(!xml.contains("<failure"));
        assert!(xml.trim_end().ends_with("</testsuites>"));
    }

    /// 三阶段对齐：Setup, Main, Teardown 各生成独立 `<testsuite>`
    #[test]
    fn test_junit_xml_three_stages() {
        let mut report = flow_report(vec![]);
        report.stages.setup = Some(StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 200,
            node_reports: vec![node_report("Setup Script", "script", ExecutionStatus::Success, 200)],
            error: None,
        });
        report.stages.main = StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 800,
            node_reports: vec![node_report("Main HTTP", "http:request", ExecutionStatus::Success, 800)],
            error: None,
        };
        report.stages.teardown = Some(StageExecutionReport {
            status: ExecutionStatus::Success,
            duration_ms: 100,
            node_reports: vec![node_report("Teardown Script", "script", ExecutionStatus::Success, 100)],
            error: None,
        });

        let xml = format_junit_xml(&report);
        assert!(xml.contains("<testsuite name=\"My Flow :: Setup\" id=\"flow-1:setup\" tests=\"1\""));
        assert!(xml.contains("<testsuite name=\"My Flow :: Main\" id=\"flow-1:main\" tests=\"1\""));
        assert!(xml.contains("<testsuite name=\"My Flow :: Teardown\" id=\"flow-1:teardown\" tests=\"1\""));
        assert!(xml.contains("tests=\"3\" failures=\"0\""));
    }

    /// 阶段跳过：Setup 失败导致 Main 跳过，Main 中输出 skipped 提示
    #[test]
    fn test_junit_xml_main_skipped() {
        let mut report = flow_report(vec![]);
        report.stages.setup = Some(StageExecutionReport {
            status: ExecutionStatus::Failed,
            duration_ms: 50,
            node_reports: vec![node_report("Setup Script", "script", ExecutionStatus::Failed, 50)],
            error: None,
        });
        report.stages.main = StageExecutionReport {
            status: ExecutionStatus::Skipped,
            duration_ms: 0,
            node_reports: vec![],
            error: None,
        };

        let xml = format_junit_xml(&report);
        assert!(xml.contains("<testsuite name=\"My Flow :: Setup\""));
        assert!(xml.contains("<testsuite name=\"My Flow :: Main\""));
        assert!(xml.contains("<skipped message=\"Setup stage failed\"/>"));
    }

    /// 失败节点：包含 ErrorKind 与 Snapshot（HTTP 状态码、请求 URL、断言差异）
    #[test]
    fn test_junit_xml_failure_node_with_snapshot() {
        let mut node = node_report("HTTP Check", "http:request", ExecutionStatus::Failed, 50);
        node.error = Some(ErrorDetail {
            error_kind: ErrorKind::Assertion,
            error_code: None,
            i18n_args: None,
            message: "expected status 200 got 500".to_string(),
            stack_trace: Some("at line 42".to_string()),
            context: None,
        });
        node.output_data = Some(serde_json::json!({
            "status": 500,
            "url": "https://api.example.com/checkout",
            "diff": "- 200\n+ 500"
        }));

        let report = flow_report(vec![node]);
        let xml = format_junit_xml(&report);

        assert!(xml.contains("<failure message=\"expected status 200 got 500\" type=\"Assertion\">"));
        assert!(xml.contains("ErrorKind: Assertion"));
        assert!(xml.contains("status: 500"));
        assert!(xml.contains("url: https://api.example.com/checkout"));
        assert!(xml.contains("diff: - 200\n+ 500"));
        assert!(xml.contains("StackTrace:\nat line 42"));
    }

    /// 阶段失败但无节点上报时：必须生成失败 testcase 确保 CI 不漏报
    #[test]
    fn test_junit_xml_empty_failed_stage_synthesizes_failure_testcase() {
        let mut report = flow_report(vec![]);
        report.stages.setup = Some(StageExecutionReport {
            status: ExecutionStatus::Failed,
            duration_ms: 10,
            node_reports: vec![],
            error: Some(ErrorDetail {
                error_kind: ErrorKind::Execution,
                error_code: None,
                i18n_args: None,
                message: "Setup initialization failed".to_string(),
                stack_trace: None,
                context: None,
            }),
        });

        let xml = format_junit_xml(&report);
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("<testsuite name=\"My Flow :: Setup\" id=\"flow-1:setup\" tests=\"1\" failures=\"1\""));
        assert!(xml.contains("<failure message=\"Setup initialization failed\" type=\"Execution\">"));
    }

    /// 主流程中条件分支跳过：不应误标为 Setup stage failed
    #[test]
    fn test_junit_xml_conditional_skip_in_main_stage() {
        let mut skipped_node = node_report("Branch Node", "http:request", ExecutionStatus::Skipped, 0);
        skipped_node.skip_reason = None; // 条件分支未设置显式原因

        let report = flow_report(vec![skipped_node]);
        let xml = format_junit_xml(&report);

        assert!(xml.contains("<testsuite name=\"My Flow :: Main\""));
        assert!(xml.contains("<skipped/>"));
        assert!(!xml.contains("Setup stage failed"));
    }
}
