//! Report Export Module (Markdown & Self-contained HTML)
//!
//! Provides unified generators for:
//! - Markdown summaries suitable for CI/CD `$GITHUB_STEP_SUMMARY`, PR comments, and clipboard sharing.
//! - Self-contained single-file HTML reports with zero external dependencies (no CDNs),
//!   modern styling, waterfall charts, and interactive filtering.

pub mod html;
pub mod markdown;

pub use html::{generate_flow_report_html, generate_stress_report_html};
pub use markdown::{generate_flow_markdown_summary, generate_stress_markdown_summary};
