//! `JUnit` XML report generation
//!
//! Converts a [`FlowExecutionReport`] into standard `JUnit` XML (Jenkins /
//! GitHub Actions / GitLab CI compatible), used by
//! `mpe run --report-format junit [--report-file <path>]`.

use flow_engine::report::{ExecutionStatus, FlowExecutionReport};

/// Format an execution report as `JUnit` XML.
///
/// Layout:
/// - root `<testsuites>` aggregating totals,
/// - one `<testsuite>` per flow run,
/// - one `<testcase>` per node report; failed nodes carry `<failure>`,
///   skipped nodes `<skipped>`. Durations are seconds (`JUnit` convention),
///   timestamps ISO 8601.
pub fn format_junit_xml(report: &FlowExecutionReport) -> String {
    let tests = report.node_reports.len();
    let failures = report
        .node_reports
        .iter()
        .filter(|r| r.status == ExecutionStatus::Failed)
        .count();
    let duration_sec = millis_to_secs(report.duration_ms);

    let mut xml = String::with_capacity(256 + report.node_reports.len() * 128);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuites name=\"MPE Flow Execution\" tests=\"{tests}\" failures=\"{failures}\" errors=\"0\" time=\"{duration_sec}\">\n"
    ));
    xml.push_str(&format!(
        "  <testsuite name=\"{}\" id=\"{}\" tests=\"{tests}\" failures=\"{failures}\" errors=\"0\" time=\"{duration_sec}\" timestamp=\"{}\">\n",
        xml_escape(&report.flow_name),
        xml_escape(&report.flow_uuid),
        iso8601(report.start_time),
    ));

    for node in &report.node_reports {
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
                let details = node
                    .error
                    .as_ref()
                    .and_then(|e| e.stack_trace.clone())
                    .unwrap_or_else(|| message.clone());
                xml.push_str(&format!(
                    ">\n      <failure message=\"{}\" type=\"{}\">{}</failure>\n    </testcase>\n",
                    xml_escape(&message),
                    xml_escape(&error_type),
                    xml_escape(&details),
                ));
            }
            ExecutionStatus::Skipped => {
                let reason = node.skip_reason.clone().unwrap_or_default();
                xml.push_str(&format!(
                    ">\n      <skipped message=\"{}\"/>\n    </testcase>\n",
                    xml_escape(&reason),
                ));
            }
            ExecutionStatus::Success | ExecutionStatus::Cancelled => {
                xml.push_str("/>\n");
            }
        }
    }

    xml.push_str("  </testsuite>\n");
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
        .map(|t| t.to_rfc3339())
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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine::error::{ErrorDetail, ErrorKind};
    use flow_engine::report::{ExecutionMode, NodeExecutionReport};

    fn node_report(
        name: &str,
        node_type: &str,
        status: ExecutionStatus,
        duration_ms: u64,
    ) -> NodeExecutionReport {
        NodeExecutionReport {
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
        }
    }

    /// 正常流程：总数统计、秒级耗时、嵌套结构、无 `<failure>`
    #[test]
    fn test_junit_xml_normal_flow() {
        let report = flow_report(vec![
            node_report("Entry", "entry", ExecutionStatus::Success, 10),
            node_report("HTTP Request", "http", ExecutionStatus::Success, 120),
        ]);
        let xml = format_junit_xml(&report);

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(xml.contains(
            "<testsuites name=\"MPE Flow Execution\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"1.500\">"
        ));
        assert!(xml.contains(
            "<testsuite name=\"My Flow\" id=\"flow-1\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"1.500\" timestamp=\""
        ));
        assert!(
            xml.contains("<testcase classname=\"flow-1\" name=\"Entry (entry)\" time=\"0.010\"/>")
        );
        assert!(xml.contains(
            "<testcase classname=\"flow-1\" name=\"HTTP Request (http)\" time=\"0.120\"/>"
        ));
        assert!(!xml.contains("<failure"));
        assert!(xml.trim_end().ends_with("</testsuites>"));
    }

    /// 失败节点：`<failure message="..." type="...">` 与异常信息提取
    #[test]
    fn test_junit_xml_failure_node() {
        let mut node = node_report("Assertion", "assertion", ExecutionStatus::Failed, 5);
        node.error = Some(ErrorDetail {
            error_kind: ErrorKind::Assertion,
            error_code: None,
            i18n_args: None,
            message: "expected 1 to equal 2".to_string(),
            stack_trace: Some("at assert_line_42".to_string()),
            context: None,
        });
        let report = flow_report(vec![node]);
        let xml = format_junit_xml(&report);

        assert!(xml.contains(
            "<testsuites name=\"MPE Flow Execution\" tests=\"1\" failures=\"1\" errors=\"0\""
        ));
        assert!(xml.contains(
            "<testcase classname=\"flow-1\" name=\"Assertion (assertion)\" time=\"0.005\">"
        ));
        assert!(xml.contains(
            "<failure message=\"expected 1 to equal 2\" type=\"Assertion\">at assert_line_42</failure>"
        ));
    }

    /// 跳过节点：`<skipped message="..."/>`
    #[test]
    fn test_junit_xml_skipped_node() {
        let mut node = node_report("Optional", "conditional", ExecutionStatus::Skipped, 0);
        node.skip_reason = Some("condition false".to_string());
        let report = flow_report(vec![node]);
        let xml = format_junit_xml(&report);

        assert!(xml.contains(
            "<testcase classname=\"flow-1\" name=\"Optional (conditional)\" time=\"0.000\">"
        ));
        assert!(xml.contains("<skipped message=\"condition false\"/>"));
    }

    /// 特殊符号转义：`<`、`>`、`&`、引号在节点名与报错信息中安全转义
    #[test]
    fn test_junit_xml_escaping() {
        assert_eq!(
            xml_escape("a & b < c > d \"e\" 'f'"),
            "a &amp; b &lt; c &gt; d &quot;e&quot; &apos;f&apos;"
        );

        let mut node = node_report(
            "Parse <XML> & \"quote\"",
            "script",
            ExecutionStatus::Failed,
            3,
        );
        node.error = Some(ErrorDetail::new(
            ErrorKind::ScriptRuntime,
            "error <tag> & \"quoted\" 'apos'".to_string(),
        ));
        let report = flow_report(vec![node]);
        let xml = format_junit_xml(&report);

        assert!(xml.contains("name=\"Parse &lt;XML&gt; &amp; &quot;quote&quot; (script)\""));
        assert!(
            xml.contains("message=\"error &lt;tag&gt; &amp; &quot;quoted&quot; &apos;apos&apos;\"")
        );
        assert!(!xml.contains("<tag>"));
        assert!(!xml.contains("&amp;amp;"));
    }
}
