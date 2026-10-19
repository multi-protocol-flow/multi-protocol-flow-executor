//! Self-Contained Single-File HTML Report Generators for Flow Execution and Stress Testing.
//!
//! Generates rich offline reports with zero external dependencies (no CDNs, no webfonts).
//! Modern responsive design with light/dark theme toggle, waterfall timeline,
//! node inspection, and embedded JSON data.

use flow_engine_core::report::{ExecutionStatus, FlowExecutionReport, NodeExecutionReport};

use crate::report_export::markdown::html_escape;
use crate::stress_test::StressTestReport;

/// Safely embeds JSON into `<script type="application/json">` preventing script injection.
fn json_safe_embed<T: serde::Serialize>(data: &T) -> String {
    match serde_json::to_string(data) {
        Ok(json) => json
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026"),
        Err(_) => "{}".to_string(),
    }
}

/// Format duration in milliseconds nicely
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{} ms", ms)
    } else if ms < 60_000 {
        format!("{:.2} s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) as f64 / 1000.0;
        format!("{}m {:.1}s", mins, secs)
    }
}

/// Format timestamp in UTC or fallback
fn format_time(ts_ms: u64) -> String {
    if ts_ms == 0 {
        return "N/A".to_string();
    }
    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts_ms as i64) {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        format!("{} ms", ts_ms)
    }
}

/// Common CSS styles for self-contained reports
const COMMON_CSS: &str = r#"
:root {
  --bg: #f8fafc;
  --surface: #ffffff;
  --surface-secondary: #f1f5f9;
  --border: #e2e8f0;
  --text-main: #0f172a;
  --text-muted: #64748b;
  --accent: #2563eb;
  --accent-light: #dbeafe;
  --success: #10b981;
  --success-bg: #d1fae5;
  --danger: #ef4444;
  --danger-bg: #fee2e2;
  --warning: #f59e0b;
  --warning-bg: #fef3c7;
  --info: #0284c7;
  --info-bg: #e0f2fe;
  --code-bg: #1e293b;
  --code-text: #f8fafc;
  --shadow: 0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1);
}

[data-theme="dark"] {
  --bg: #090d16;
  --surface: #111827;
  --surface-secondary: #1f2937;
  --border: #374151;
  --text-main: #f9fafb;
  --text-muted: #9ca3af;
  --accent: #3b82f6;
  --accent-light: #1e3a8a;
  --success: #10b981;
  --success-bg: #064e3b;
  --danger: #ef4444;
  --danger-bg: #7f1d1d;
  --warning: #f59e0b;
  --warning-bg: #78350f;
  --info: #38bdf8;
  --info-bg: #075985;
  --code-bg: #030712;
  --code-text: #f9fafb;
  --shadow: 0 4px 6px -1px rgb(0 0 0 / 0.5), 0 2px 4px -2px rgb(0 0 0 / 0.5);
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #090d16;
    --surface: #111827;
    --surface-secondary: #1f2937;
    --border: #374151;
    --text-main: #f9fafb;
    --text-muted: #9ca3af;
    --accent: #3b82f6;
    --accent-light: #1e3a8a;
    --success: #10b981;
    --success-bg: #064e3b;
    --danger: #ef4444;
    --danger-bg: #7f1d1d;
    --warning: #f59e0b;
    --warning-bg: #78350f;
    --info: #38bdf8;
    --info-bg: #075985;
    --code-bg: #030712;
    --code-text: #f9fafb;
    --shadow: 0 4px 6px -1px rgb(0 0 0 / 0.5), 0 2px 4px -2px rgb(0 0 0 / 0.5);
  }
}

* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background-color: var(--bg);
  color: var(--text-main);
  line-height: 1.5;
  padding: 24px;
  transition: background-color 0.2s, color 0.2s;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
}

header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 24px;
  padding-bottom: 20px;
  border-bottom: 1px solid var(--border);
}

.title-area h1 {
  font-size: 24px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  font-size: 13px;
  color: var(--text-muted);
}
.meta-row span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 500;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text-main);
  cursor: pointer;
  transition: all 0.15s;
}
.btn:hover {
  background: var(--surface-secondary);
  border-color: var(--text-muted);
}
.btn-primary {
  background: var(--accent);
  color: #ffffff;
  border-color: var(--accent);
}
.btn-primary:hover {
  filter: brightness(1.1);
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 9999px;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.badge-success { background: var(--success-bg); color: var(--success); }
.badge-danger { background: var(--danger-bg); color: var(--danger); }
.badge-warning { background: var(--warning-bg); color: var(--warning); }
.badge-muted { background: var(--surface-secondary); color: var(--text-muted); }

.grid-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  box-shadow: var(--shadow);
}
.card-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 4px;
}
.card-val {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-main);
}
.card-sub {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
}

.section {
  margin-bottom: 28px;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-main);
}

.table-responsive {
  width: 100%;
  overflow-x: auto;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
}
table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: 13px;
}
th {
  background: var(--surface-secondary);
  color: var(--text-muted);
  font-weight: 600;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}
td {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}
tr:last-child td { border-bottom: none; }
tr:hover td { background: var(--surface-secondary); }

/* Waterfall Timeline */
.waterfall-box {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  box-shadow: var(--shadow);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.waterfall-row {
  display: grid;
  grid-template-columns: 200px 1fr 90px;
  align-items: center;
  gap: 12px;
  font-size: 12px;
}
.wf-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}
.wf-track {
  position: relative;
  height: 20px;
  background: var(--surface-secondary);
  border-radius: 4px;
  overflow: hidden;
}
.wf-bar {
  position: absolute;
  top: 0;
  height: 100%;
  border-radius: 4px;
  min-width: 4px;
  display: flex;
  align-items: center;
  padding: 0 4px;
  font-size: 10px;
  color: #ffffff;
  font-weight: 600;
  white-space: nowrap;
}
.wf-bar-success { background: var(--success); }
.wf-bar-failed { background: var(--danger); }
.wf-bar-skipped { background: var(--text-muted); }
.wf-duration {
  text-align: right;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  color: var(--text-muted);
}

/* Node Accordions & Filter Toolbar */
.filter-toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 14px;
  flex-wrap: wrap;
}
.search-input {
  flex: 1;
  min-width: 220px;
  padding: 7px 12px;
  font-size: 13px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text-main);
  outline: none;
}
.search-input:focus { border-color: var(--accent); }

.node-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-bottom: 10px;
  box-shadow: var(--shadow);
  overflow: hidden;
}
.node-card-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
}
.node-card-header:hover { background: var(--surface-secondary); }
.node-title-group {
  display: flex;
  align-items: center;
  gap: 10px;
}
.node-body {
  padding: 16px;
  border-top: 1px solid var(--border);
  background: var(--surface);
  display: none;
}
.node-card.open .node-body { display: block; }
.node-card.open .arrow-icon { transform: rotate(90deg); }

.arrow-icon {
  width: 16px;
  height: 16px;
  transition: transform 0.15s;
  fill: currentColor;
}

pre {
  background: var(--code-bg);
  color: var(--code-text);
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  font-size: 12px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  margin-top: 6px;
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.9em;
}

.alert-error {
  background: var(--danger-bg);
  color: var(--danger);
  border: 1px solid var(--danger);
  padding: 12px;
  border-radius: 6px;
  margin-bottom: 12px;
  font-size: 13px;
}

/* Toast */
#toast {
  position: fixed;
  bottom: 24px;
  right: 24px;
  padding: 10px 18px;
  background: #111827;
  color: #f9fafb;
  border-radius: 6px;
  font-size: 13px;
  box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.3);
  opacity: 0;
  transform: translateY(10px);
  transition: all 0.2s;
  pointer-events: none;
  z-index: 9999;
}
#toast.show { opacity: 1; transform: translateY(0); }
"#;

/// Common JavaScript for theme toggling and interactive UI
const COMMON_JS: &str = r#"
function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme');
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', next);
  try { localStorage.setItem('mpe_report_theme', next); } catch(e) {}
}

// Load saved theme if any
(function() {
  try {
    const saved = localStorage.getItem('mpe_report_theme');
    if (saved) document.documentElement.setAttribute('data-theme', saved);
  } catch(e) {}
})();

function showToast(msg) {
  const t = document.getElementById('toast');
  if (!t) return;
  t.innerText = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2200);
}

function copyRawJson() {
  const el = document.getElementById('__REPORT_DATA__');
  if (!el) return;
  navigator.clipboard.writeText(el.textContent).then(() => {
    showToast('Raw JSON copied to clipboard');
  }).catch(() => {
    showToast('Failed to copy JSON');
  });
}

function toggleNodeCard(headerEl) {
  const card = headerEl.closest('.node-card');
  if (card) card.classList.toggle('open');
}

function expandAllNodes(expand) {
  document.querySelectorAll('.node-card').forEach(c => {
    if (expand) c.classList.add('open');
    else c.classList.remove('open');
  });
}

function filterNodes() {
  const query = (document.getElementById('node-search')?.value || '').toLowerCase();
  const filter = window.currentStatusFilter || 'all';

  document.querySelectorAll('.node-card').forEach(card => {
    const name = card.getAttribute('data-name') || '';
    const type = card.getAttribute('data-type') || '';
    const status = card.getAttribute('data-status') || '';

    const matchesQuery = !query || name.includes(query) || type.includes(query);
    const matchesStatus = filter === 'all' || status === filter;

    if (matchesQuery && matchesStatus) {
      card.style.display = 'block';
    } else {
      card.style.display = 'none';
    }
  });
}

function setStatusFilter(btn, status) {
  window.currentStatusFilter = status;
  document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('btn-primary'));
  btn.classList.add('btn-primary');
  filterNodes();
}
"#;

/// Generate self-contained HTML report for Flow Execution.
pub fn generate_flow_report_html(
    report: &FlowExecutionReport,
    env_name: Option<&str>,
) -> String {
    let mut html = String::with_capacity(32 * 1024);

    let is_success = report.status == ExecutionStatus::Success;
    let badge_class = if is_success { "badge-success" } else { "badge-danger" };
    let badge_text = if is_success { "PASSED" } else { "FAILED" };

    // Collect all nodes from setup, main, teardown
    let mut all_nodes: Vec<(&'static str, &NodeExecutionReport)> = Vec::new();
    if let Some(setup) = &report.stages.setup {
        for n in &setup.node_reports {
            all_nodes.push(("Setup", n));
        }
    }
    for n in &report.stages.main.node_reports {
        all_nodes.push(("Main", n));
    }
    if let Some(teardown) = &report.stages.teardown {
        for n in &teardown.node_reports {
            all_nodes.push(("Teardown", n));
        }
    }
    // Fallback if stages were empty
    if all_nodes.is_empty() {
        for n in &report.node_reports {
            all_nodes.push(("Main", n));
        }
    }

    let total_nodes = all_nodes.len();
    let success_nodes = all_nodes.iter().filter(|(_, n)| n.status == ExecutionStatus::Success).count();
    let failed_nodes = all_nodes.iter().filter(|(_, n)| n.status == ExecutionStatus::Failed).count();
    let skipped_nodes = all_nodes.iter().filter(|(_, n)| n.status == ExecutionStatus::Skipped).count();

    let json_data = json_safe_embed(report);

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!(
        "<title>MPE Execution Report: {}</title>\n",
        html_escape(&report.flow_name)
    ));
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str("\n</style>\n</head>\n<body>\n<div class=\"container\">\n");

    // Header
    html.push_str("<header>\n<div class=\"title-area\">\n");
    html.push_str(&format!(
        "<h1><span>{}</span> <span class=\"badge {}\">{}</span></h1>\n",
        html_escape(&report.flow_name),
        badge_class,
        badge_text
    ));
    html.push_str("<div class=\"meta-row\">\n");
    html.push_str(&format!(
        "<span><b>UUID:</b> <code>{}</code></span>\n",
        html_escape(&report.flow_uuid)
    ));
    html.push_str(&format!(
        "<span><b>Env:</b> <code>{}</code></span>\n",
        html_escape(env_name.unwrap_or("default"))
    ));
    html.push_str(&format!(
        "<span><b>Started:</b> {}</span>\n",
        format_time(report.start_time)
    ));
    html.push_str(&format!(
        "<span><b>Duration:</b> {}</span>\n",
        format_duration(report.duration_ms)
    ));
    html.push_str("</div>\n</div>\n");

    html.push_str("<div class=\"header-actions\">\n");
    html.push_str("<button class=\"btn\" onclick=\"toggleTheme()\" title=\"Toggle Theme\">🌓 Theme</button>\n");
    html.push_str("<button class=\"btn\" onclick=\"copyRawJson()\" title=\"Copy Raw JSON\">📋 Copy JSON</button>\n");
    html.push_str("</div>\n</header>\n");

    // Metric Summary Cards
    html.push_str("<div class=\"grid-cards\">\n");
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Total Nodes</div><div class=\"card-val\">{}</div><div class=\"card-sub\">{} executed</div></div>\n",
        total_nodes, total_nodes - skipped_nodes
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Passed</div><div class=\"card-val\" style=\"color: var(--success);\">{}</div><div class=\"card-sub\">{:.1}% success rate</div></div>\n",
        success_nodes,
        if total_nodes > 0 { (success_nodes as f64 / total_nodes as f64) * 100.0 } else { 0.0 }
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Failed</div><div class=\"card-val\" style=\"color: var(--danger);\">{}</div><div class=\"card-sub\">{} skipped</div></div>\n",
        failed_nodes, skipped_nodes
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Total Time</div><div class=\"card-val\">{}</div><div class=\"card-sub\">Main: {}</div></div>\n",
        format_duration(report.duration_ms),
        format_duration(report.main_duration_ms)
    ));
    html.push_str("</div>\n");

    // Lifecycle stages table if present
    html.push_str("<div class=\"section\">\n");
    html.push_str("<div class=\"section-header\"><div class=\"section-title\">Lifecycle Stages</div></div>\n");
    html.push_str("<div class=\"table-responsive\"><table>\n<thead><tr><th>Stage</th><th>Status</th><th>Nodes</th><th>Duration</th></tr></thead>\n<tbody>\n");

    let stage_row = |name: &str, status: &ExecutionStatus, count: usize, dur: u64| {
        let (s_class, s_txt) = match status {
            ExecutionStatus::Success => ("badge-success", "Success"),
            ExecutionStatus::Failed => ("badge-danger", "Failed"),
            ExecutionStatus::Skipped => ("badge-muted", "Skipped"),
            ExecutionStatus::Cancelled => ("badge-warning", "Cancelled"),
        };
        format!(
            "<tr><td><b>{}</b></td><td><span class=\"badge {}\">{}</span></td><td>{}</td><td>{}</td></tr>\n",
            name, s_class, s_txt, count, format_duration(dur)
        )
    };

    if let Some(setup) = &report.stages.setup {
        html.push_str(&stage_row("Setup", &setup.status, setup.node_reports.len(), setup.duration_ms));
    }
    let main_dur = if report.stages.main.duration_ms > 0 { report.stages.main.duration_ms } else { report.main_duration_ms };
    let main_count = if !report.stages.main.node_reports.is_empty() { report.stages.main.node_reports.len() } else { report.node_reports.len() };
    html.push_str(&stage_row("Main", &report.stages.main.status, main_count, main_dur));

    if let Some(teardown) = &report.stages.teardown {
        html.push_str(&stage_row("Teardown", &teardown.status, teardown.node_reports.len(), teardown.duration_ms));
    }
    html.push_str("</tbody></table></div></div>\n");

    // Waterfall Timeline Section
    if !all_nodes.is_empty() {
        let total_dur = report.duration_ms.max(1);
        let base_start = report.start_time;

        html.push_str("<div class=\"section\">\n");
        html.push_str("<div class=\"section-header\"><div class=\"section-title\">Execution Waterfall</div></div>\n");
        html.push_str("<div class=\"waterfall-box\">\n");

        for (stage, n) in &all_nodes {
            let offset_ms = if base_start > 0 && n.start_time >= base_start {
                n.start_time - base_start
            } else {
                0
            };
            let left_pct = ((offset_ms as f64 / total_dur as f64) * 100.0).clamp(0.0, 95.0);
            let width_pct = ((n.duration_ms as f64 / total_dur as f64) * 100.0).clamp(1.5, 100.0 - left_pct);

            let bar_class = match n.status {
                ExecutionStatus::Success => "wf-bar-success",
                ExecutionStatus::Failed => "wf-bar-failed",
                _ => "wf-bar-skipped",
            };

            html.push_str("<div class=\"waterfall-row\">\n");
            html.push_str(&format!(
                "<div class=\"wf-label\" title=\"[{}] {}\">[{}] {}</div>\n",
                stage, html_escape(&n.node_name), stage, html_escape(&n.node_name)
            ));
            html.push_str("<div class=\"wf-track\">\n");
            html.push_str(&format!(
                "<div class=\"wf-bar {}\" style=\"left: {:.2}%; width: {:.2}%;\" title=\"{} ({})\">{}</div>\n",
                bar_class,
                left_pct,
                width_pct,
                format_duration(n.duration_ms),
                html_escape(&n.node_name),
                format_duration(n.duration_ms)
            ));
            html.push_str("</div>\n");
            html.push_str(&format!("<div class=\"wf-duration\">{}</div>\n", format_duration(n.duration_ms)));
            html.push_str("</div>\n");
        }
        html.push_str("</div></div>\n");
    }

    // Node Details Section with Search & Filter
    html.push_str("<div class=\"section\">\n");
    html.push_str("<div class=\"section-header\">\n");
    html.push_str("<div class=\"section-title\">Node Execution Details</div>\n");
    html.push_str("<div style=\"display: flex; gap: 8px;\">\n");
    html.push_str("<button class=\"btn\" onclick=\"expandAllNodes(true)\">Expand All</button>\n");
    html.push_str("<button class=\"btn\" onclick=\"expandAllNodes(false)\">Collapse All</button>\n");
    html.push_str("</div>\n</div>\n");

    // Filter toolbar
    html.push_str("<div class=\"filter-toolbar\">\n");
    html.push_str("<input type=\"text\" id=\"node-search\" class=\"search-input\" placeholder=\"Filter by node name or type...\" oninput=\"filterNodes()\">\n");
    html.push_str("<button class=\"btn filter-btn btn-primary\" onclick=\"setStatusFilter(this, 'all')\">All</button>\n");
    html.push_str("<button class=\"btn filter-btn\" onclick=\"setStatusFilter(this, 'failed')\">Failed Only</button>\n");
    html.push_str("<button class=\"btn filter-btn\" onclick=\"setStatusFilter(this, 'success')\">Success Only</button>\n");
    html.push_str("</div>\n");

    // Node Cards
    html.push_str("<div id=\"node-list\">\n");
    for (stage, n) in &all_nodes {
        let status_code = match n.status {
            ExecutionStatus::Success => "success",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Skipped => "skipped",
            ExecutionStatus::Cancelled => "cancelled",
        };
        let status_badge_class = match n.status {
            ExecutionStatus::Success => "badge-success",
            ExecutionStatus::Failed => "badge-danger",
            _ => "badge-muted",
        };
        let open_class = if n.status == ExecutionStatus::Failed { "node-card open" } else { "node-card" };

        html.push_str(&format!(
            "<div class=\"{}\" data-name=\"{}\" data-type=\"{}\" data-status=\"{}\">\n",
            open_class,
            html_escape(&n.node_name.to_lowercase()),
            html_escape(&n.node_type.to_lowercase()),
            status_code
        ));

        // Header
        html.push_str("<div class=\"node-card-header\" onclick=\"toggleNodeCard(this)\">\n");
        html.push_str("<div class=\"node-title-group\">\n");
        html.push_str("<svg class=\"arrow-icon\" viewBox=\"0 0 20 20\"><path d=\"M6 6L14 10L6 14Z\"/></svg>\n");
        html.push_str(&format!(
            "<b>{}</b> <span style=\"font-size: 11px; color: var(--text-muted);\">[<code>{}</code>] (<code>{}</code>)</span>\n",
            html_escape(&n.node_name),
            stage,
            html_escape(&n.node_type)
        ));
        html.push_str("</div>\n");

        html.push_str("<div style=\"display: flex; align-items: center; gap: 10px;\">\n");
        html.push_str(&format!(
            "<span class=\"badge {}\">{}</span>\n",
            status_badge_class, status_code
        ));
        html.push_str(&format!(
            "<span style=\"font-size: 12px; color: var(--text-muted);\">{}</span>\n",
            format_duration(n.duration_ms)
        ));
        html.push_str("</div>\n</div>\n");

        // Body
        html.push_str("<div class=\"node-body\">\n");

        // Error Banner
        if let Some(err) = &n.error {
            html.push_str("<div class=\"alert-error\">\n");
            html.push_str(&format!(
                "<b>Error [{:?}]:</b> {}\n",
                err.error_kind,
                html_escape(&err.message)
            ));
            if let Some(stack) = &err.stack_trace {
                html.push_str(&format!("<pre><code>{}</code></pre>\n", html_escape(stack)));
            }
            html.push_str("</div>\n");
        }

        // Output Data
        if let Some(out) = &n.output_data {
            html.push_str("<div style=\"margin-bottom: 12px;\">\n");
            html.push_str("<div style=\"font-size: 12px; font-weight: 600; color: var(--text-muted);\">Output Data:</div>\n");
            let out_str = serde_json::to_string_pretty(out).unwrap_or_default();
            html.push_str(&format!("<pre><code>{}</code></pre>\n", html_escape(&out_str)));
            html.push_str("</div>\n");
        }

        // Resolved Config
        if let Some(cfg) = &n.resolved_config {
            html.push_str("<div style=\"margin-bottom: 12px;\">\n");
            html.push_str("<div style=\"font-size: 12px; font-weight: 600; color: var(--text-muted);\">Resolved Configuration:</div>\n");
            let cfg_str = serde_json::to_string_pretty(cfg).unwrap_or_default();
            html.push_str(&format!("<pre><code>{}</code></pre>\n", html_escape(&cfg_str)));
            html.push_str("</div>\n");
        }

        // Variable Changes
        if let Some(vars) = &n.variable_changes {
            if !vars.is_empty() {
                html.push_str("<div style=\"margin-bottom: 12px;\">\n");
                html.push_str("<div style=\"font-size: 12px; font-weight: 600; color: var(--text-muted);\">Variable Changes:</div>\n");
                let vars_str = serde_json::to_string_pretty(vars).unwrap_or_default();
                html.push_str(&format!("<pre><code>{}</code></pre>\n", html_escape(&vars_str)));
                html.push_str("</div>\n");
            }
        }

        html.push_str("</div>\n</div>\n");
    }
    html.push_str("</div></div>\n");

    // Toast
    html.push_str("<div id=\"toast\"></div>\n");

    // Embedded Raw JSON
    html.push_str(&format!(
        "<script type=\"application/json\" id=\"__REPORT_DATA__\">{}</script>\n",
        json_data
    ));

    // Embedded Script
    html.push_str("<script>\n");
    html.push_str(COMMON_JS);
    html.push_str("\n</script>\n");

    html.push_str("</div>\n</body>\n</html>");
    html
}

/// Generate self-contained HTML report for Stress Testing.
pub fn generate_stress_report_html(report: &StressTestReport) -> String {
    let mut html = String::with_capacity(32 * 1024);

    let is_success = report.statistics.failed == 0 && report.statistics.total_iterations > 0;
    let badge_class = if is_success { "badge-success" } else { "badge-danger" };
    let badge_text = if is_success { "PASSED" } else { "FAILED" };

    let json_data = json_safe_embed(report);

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!(
        "<title>MPE Stress Test Report: {}</title>\n",
        html_escape(&report.flow_name)
    ));
    html.push_str("<style>\n");
    html.push_str(COMMON_CSS);
    html.push_str(r#"
.chart-container {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
  box-shadow: var(--shadow);
  margin-bottom: 24px;
}
.chart-svg {
  width: 100%;
  height: 220px;
}
"#);
    html.push_str("\n</style>\n</head>\n<body>\n<div class=\"container\">\n");

    // Header
    html.push_str("<header>\n<div class=\"title-area\">\n");
    html.push_str(&format!(
        "<h1><span>{}</span> <span class=\"badge {}\">{}</span></h1>\n",
        html_escape(&report.flow_name),
        badge_class,
        badge_text
    ));
    html.push_str("<div class=\"meta-row\">\n");
    html.push_str(&format!(
        "<span><b>Report UUID:</b> <code>{}</code></span>\n",
        html_escape(&report.uuid)
    ));
    html.push_str(&format!(
        "<span><b>Flow UUID:</b> <code>{}</code></span>\n",
        html_escape(&report.flow_uuid)
    ));
    html.push_str(&format!(
        "<span><b>Started:</b> {}</span>\n",
        format_time(report.start_time)
    ));
    html.push_str(&format!(
        "<span><b>Duration:</b> {}</span>\n",
        format_duration(report.duration_ms)
    ));
    html.push_str("</div>\n</div>\n");

    html.push_str("<div class=\"header-actions\">\n");
    html.push_str("<button class=\"btn\" onclick=\"toggleTheme()\" title=\"Toggle Theme\">🌓 Theme</button>\n");
    html.push_str("<button class=\"btn\" onclick=\"copyRawJson()\" title=\"Copy Raw JSON\">📋 Copy JSON</button>\n");
    html.push_str("</div>\n</header>\n");

    // Metric Summary Cards
    html.push_str("<div class=\"grid-cards\">\n");
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Total Iterations</div><div class=\"card-val\">{}</div><div class=\"card-sub\">{} concurrent users</div></div>\n",
        report.statistics.total_iterations, report.config.concurrent_users
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Success Rate</div><div class=\"card-val\" style=\"color: {};\">{:.2}%</div><div class=\"card-sub\">{} passed, {} failed</div></div>\n",
        if report.statistics.success_rate >= 0.99 { "var(--success)" } else if report.statistics.success_rate >= 0.95 { "var(--warning)" } else { "var(--danger)" },
        report.statistics.success_rate * 100.0,
        report.statistics.successful,
        report.statistics.failed
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">Throughput (RPS)</div><div class=\"card-val\">{:.1}</div><div class=\"card-sub\">req / second</div></div>\n",
        report.statistics.rps
    ));
    html.push_str(&format!(
        "<div class=\"card\"><div class=\"card-label\">P95 Latency</div><div class=\"card-val\">{} ms</div><div class=\"card-sub\">Avg: {:.1} ms</div></div>\n",
        report.response_time_distribution.p95_ms,
        report.response_time_distribution.avg_ms
    ));
    html.push_str("</div>\n");

    // Latency Distribution Chart (Self-contained SVG Bar Chart)
    html.push_str("<div class=\"section\">\n");
    html.push_str("<div class=\"section-header\"><div class=\"section-title\">Latency Percentiles Distribution</div></div>\n");
    html.push_str("<div class=\"chart-container\">\n");

    let dist = &report.response_time_distribution;
    let max_lat = dist.max_ms.max(1) as f64;
    let items = [
        ("Min", dist.min_ms as f64),
        ("P50", dist.p50_ms as f64),
        ("P90", dist.p90_ms as f64),
        ("P95", dist.p95_ms as f64),
        ("P99", dist.p99_ms as f64),
        ("Max", dist.max_ms as f64),
    ];

    html.push_str("<svg class=\"chart-svg\" viewBox=\"0 0 600 200\" preserveAspectRatio=\"none\">\n");
    // Draw baseline
    html.push_str("<line x1=\"20\" y1=\"160\" x2=\"580\" y2=\"160\" stroke=\"var(--border)\" stroke-width=\"1\"/>\n");

    let bar_width = 50.0;
    let spacing = 90.0;
    let start_x = 45.0;

    for (i, (lbl, val)) in items.iter().enumerate() {
        let x = start_x + (i as f64 * spacing);
        let height = ((val / max_lat) * 120.0).max(4.0);
        let y = 160.0 - height;
        let color = if *lbl == "P99" || *lbl == "Max" {
            "var(--danger)"
        } else if *lbl == "P95" || *lbl == "P90" {
            "var(--warning)"
        } else {
            "var(--accent)"
        };

        // Bar rect
        html.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"{}\"/>\n",
            x, y, bar_width, height, color
        ));
        // Value text
        html.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" font-weight=\"600\" fill=\"var(--text-main)\" text-anchor=\"middle\">{:.0}ms</text>\n",
            x + (bar_width / 2.0), y - 6.0, val
        ));
        // Label text
        html.push_str(&format!(
            "<text x=\"{:.1}\" y=\"180\" font-size=\"12\" font-weight=\"500\" fill=\"var(--text-muted)\" text-anchor=\"middle\">{}</text>\n",
            x + (bar_width / 2.0), lbl
        ));
    }
    html.push_str("</svg>\n");

    // Table below chart
    html.push_str("<div class=\"table-responsive\" style=\"margin-top: 14px;\"><table>\n");
    html.push_str("<thead><tr><th>Metric</th><th>Min</th><th>P50 (Median)</th><th>P90</th><th>P95</th><th>P99</th><th>Max</th><th>Average</th></tr></thead>\n");
    html.push_str(&format!(
        "<tbody><tr><td><b>Latency</b></td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{:.2} ms</td></tr></tbody>\n",
        dist.min_ms, dist.p50_ms, dist.p90_ms, dist.p95_ms, dist.p99_ms, dist.max_ms, dist.avg_ms
    ));
    html.push_str("</table></div>\n");
    html.push_str("</div></div>\n");

    // Node-level Statistics Table
    if !report.node_statistics.is_empty() {
        html.push_str("<div class=\"section\">\n");
        html.push_str("<div class=\"section-header\"><div class=\"section-title\">Node-level Metrics</div></div>\n");
        html.push_str("<div class=\"table-responsive\"><table>\n");
        html.push_str("<thead><tr><th>Node Name</th><th>Type</th><th>Total</th><th>Success</th><th>Failed</th><th>Success Rate</th><th>Avg Latency</th><th>P95 Latency</th></tr></thead>\n<tbody>\n");
        for n in &report.node_statistics {
            html.push_str(&format!(
                "<tr><td><b>{}</b></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.2} ms</td><td>{} ms</td></tr>\n",
                html_escape(&n.node_name),
                html_escape(&n.node_type),
                n.total_executions,
                n.successful_executions,
                n.failed_executions,
                n.success_rate * 100.0,
                n.response_time_distribution.avg_ms,
                n.response_time_distribution.p95_ms
            ));
        }
        html.push_str("</tbody></table></div></div>\n");
    }

    // Error Statistics Table & Samples
    if !report.error_statistics.is_empty() {
        html.push_str("<div class=\"section\">\n");
        html.push_str("<div class=\"section-header\"><div class=\"section-title\">Error Statistics</div></div>\n");
        html.push_str("<div class=\"table-responsive\"><table>\n");
        html.push_str("<thead><tr><th>Error Kind</th><th>Count</th><th>Share</th></tr></thead>\n<tbody>\n");
        let total_failed = report.statistics.failed.max(1) as f64;
        for err in &report.error_statistics {
            let share = (err.count as f64 / total_failed) * 100.0;
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{:.2}%</td></tr>\n",
                html_escape(&err.error_kind),
                err.count,
                share
            ));
        }
        html.push_str("</tbody></table></div>\n");

        // Error Samples accordion
        html.push_str("<div style=\"margin-top: 14px;\">\n");
        for err in &report.error_statistics {
            for sample in &err.samples {
                html.push_str("<div class=\"node-card\">\n");
                html.push_str("<div class=\"node-card-header\" onclick=\"toggleNodeCard(this)\">\n");
                html.push_str(&format!(
                    "<div class=\"node-title-group\"><span class=\"badge badge-danger\">Iter #{}</span> <b>{}</b> <span style=\"font-size: 12px; color: var(--text-muted);\">{}</span></div>\n",
                    sample.iteration,
                    html_escape(&sample.node_name),
                    html_escape(&sample.error_message)
                ));
                html.push_str("<svg class=\"arrow-icon\" viewBox=\"0 0 20 20\"><path d=\"M6 6L14 10L6 14Z\"/></svg>\n");
                html.push_str("</div>\n");

                html.push_str("<div class=\"node-body\">\n");
                html.push_str(&format!(
                    "<div class=\"alert-error\"><b>Error Message:</b> {}</div>\n",
                    html_escape(&sample.error_message)
                ));
                if let Some(detail) = &sample.error_detail {
                    let detail_json = serde_json::to_string_pretty(detail).unwrap_or_default();
                    html.push_str(&format!("<pre><code>{}</code></pre>\n", html_escape(&detail_json)));
                }
                html.push_str("</div>\n</div>\n");
            }
        }
        html.push_str("</div></div>\n");
    }

    // Toast
    html.push_str("<div id=\"toast\"></div>\n");

    // Embedded Raw JSON
    html.push_str(&format!(
        "<script type=\"application/json\" id=\"__REPORT_DATA__\">{}</script>\n",
        json_data
    ));

    // Embedded Script
    html.push_str("<script>\n");
    html.push_str(COMMON_JS);
    html.push_str("\n</script>\n");

    html.push_str("</div>\n</body>\n</html>");
    html
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

    fn make_test_flow_report() -> FlowExecutionReport {
        let mut report = FlowExecutionReport {
            uuid: "rep-flow-1".to_string(),
            flow_uuid: "flow-1".to_string(),
            flow_name: "Order Processing <script>alert(1)</script>".to_string(),
            start_time: 1700000000000,
            end_time: 1700000005000,
            duration_ms: 5000,
            total_nodes: 2,
            executed_count: 2,
            status: ExecutionStatus::Failed,
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
            main_duration_ms: 4500,
        };

        report.stages.main = StageExecutionReport {
            status: ExecutionStatus::Failed,
            duration_ms: 4500,
            node_reports: vec![
                NodeExecutionReport {
                    node_name: "Authorize Card".to_string(),
                    node_type: "http:request".to_string(),
                    status: ExecutionStatus::Success,
                    duration_ms: 1500,
                    start_time: 1700000000000,
                    output_data: Some(serde_json::json!({"auth": "ok"})),
                    ..Default::default()
                },
                NodeExecutionReport {
                    node_name: "Capture Card".to_string(),
                    node_type: "http:request".to_string(),
                    status: ExecutionStatus::Failed,
                    duration_ms: 3000,
                    start_time: 1700000001500,
                    error: Some(ErrorDetail {
                        error_kind: ErrorKind::Execution,
                        error_code: None,
                        i18n_args: None,
                        message: "Network Timeout".to_string(),
                        stack_trace: None,
                        context: None,
                    }),
                    ..Default::default()
                },
            ],
            error: None,
        };

        report
    }

    #[test]
    fn test_flow_report_html_generation_and_xss_protection() {
        let report = make_test_flow_report();
        let html = generate_flow_report_html(&report, Some("staging"));

        // Must be self contained
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("cdn."));

        // Must escape dangerous script tags
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));

        // Must contain key UI elements
        assert!(html.contains("Authorize Card"));
        assert!(html.contains("Capture Card"));
        assert!(html.contains("Network Timeout"));
        assert!(html.contains("Execution Waterfall"));
        assert!(html.contains("id=\"__REPORT_DATA__\""));
    }

    #[test]
    fn test_stress_report_html_generation() {
        let report = StressTestReport {
            uuid: "stress-rep-002".to_string(),
            flow_uuid: "flow-002".to_string(),
            flow_name: "High Load Checkout".to_string(),
            start_time: 1000,
            end_time: 31000,
            duration_ms: 30000,
            config: StressTestConfig {
                concurrent_users: 100,
                ramp_up_seconds: 5,
                duration_seconds: 30,
                flow_uuid: "flow-002".to_string(),
                ..Default::default()
            },
            statistics: StressTestStatistics {
                total_iterations: 5000,
                successful: 4950,
                failed: 50,
                success_rate: 0.99,
                rps: 165.0,
            },
            response_time_distribution: ResponseTimeDistribution {
                avg_ms: 85.0,
                min_ms: 20,
                max_ms: 600,
                p50_ms: 70,
                p90_ms: 120,
                p95_ms: 180,
                p99_ms: 350,
            },
            node_statistics: vec![NodeLevelStatistics {
                node_uuid: "node-1".to_string(),
                node_name: "HTTP Checkout".to_string(),
                node_type: "http:request".to_string(),
                total_executions: 5000,
                successful_executions: 4950,
                failed_executions: 50,
                success_rate: 0.99,
                response_time_distribution: ResponseTimeDistribution {
                    avg_ms: 85.0,
                    p95_ms: 180,
                    ..Default::default()
                },
                error_statistics: vec![],
            }],
            error_statistics: vec![ErrorStatistics {
                error_kind: "ConnectError".to_string(),
                count: 50,
                samples: vec![ErrorSample::new(
                    123,
                    "node-1",
                    "HTTP Checkout",
                    "Connection reset by peer",
                )],
            }],
            ..Default::default()
        };

        let html = generate_stress_report_html(&report);

        // Self contained check
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));

        // Check content
        assert!(html.contains("High Load Checkout"));
        assert!(html.contains("165.0"));
        assert!(html.contains("99.00%"));
        assert!(html.contains("chart-svg"));
        assert!(html.contains("HTTP Checkout"));
        assert!(html.contains("Connection reset by peer"));
    }
}
