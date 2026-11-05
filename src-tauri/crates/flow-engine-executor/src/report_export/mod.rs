//! Report Export Module (Markdown & Self-contained HTML)
//!
//! Provides unified generators for:
//! - Markdown summaries suitable for CI/CD `$GITHUB_STEP_SUMMARY`, PR comments, and clipboard sharing.
//! - Self-contained single-file HTML reports with zero external dependencies (no CDNs),
//!   modern styling, waterfall charts, and interactive filtering.

pub mod html;
pub mod markdown;

use serde_json::Value;

pub use html::{generate_flow_report_html, generate_stress_report_html};
pub use markdown::{generate_flow_markdown_summary, generate_stress_markdown_summary};

/// 把单个 `output_data` 里的内联媒体载荷替换为占位符
fn strip_output_media(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Some(media) = obj.get_mut("media").and_then(|m| m.as_object_mut()) else {
        return;
    };
    if media.contains_key("data_url") {
        media.insert(
            "data_url".to_string(),
            Value::String("[media omitted]".to_string()),
        );
    }
}

/// 递归剥离变量树中 `output_data` 下的内联媒体（`$report` 副本等）
fn scrub_output_data(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            for (key, child) in obj.iter_mut() {
                if key == "output_data" {
                    strip_output_media(child);
                }
                scrub_output_data(child);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                scrub_output_data(item);
            }
        }
        _ => {}
    }
}

/// 导出前把内联媒体载荷替换为占位符，避免导出文件被 base64 撑爆。
///
/// 只改写 `output_data.media.data_url`（存在时），保留 `kind` / `mime` / `bytes`
/// 等元信息，便于导出文件里仍能看出该节点返回过媒体。
pub(crate) fn strip_inline_media(report: &mut flow_engine_core::report::FlowExecutionReport) {
    fn strip_nodes(nodes: &mut [flow_engine_core::report::NodeExecutionReport]) {
        for node in nodes.iter_mut() {
            if let Some(output_data) = node.output_data.as_mut() {
                strip_output_media(output_data);
            }
        }
    }

    strip_nodes(&mut report.node_reports);
    strip_nodes(&mut report.stages.main.node_reports);
    if let Some(setup) = report.stages.setup.as_mut() {
        strip_nodes(&mut setup.node_reports);
    }
    if let Some(teardown) = report.stages.teardown.as_mut() {
        strip_nodes(&mut teardown.node_reports);
    }

    // 执行器把整份报告镜像进 `final_variables.$report`（供 teardown 引用 `{{$report.*}}`），
    // 该副本同样携带内联媒体，需要一并剥离。
    if let Some(variables) = report.final_variables.as_mut() {
        scrub_output_data(variables);
    }
}
