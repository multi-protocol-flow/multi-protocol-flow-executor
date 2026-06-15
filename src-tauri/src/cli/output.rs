//! CLI output module - unified JSON output format
//!
//! Provides three output modes:
//! - `output_json` — formatted JSON output (default mode)
//! - `output_error` — error JSON output
//! - `output_ndjson` — single-line JSON output with flush (NDJSON streaming mode)

use std::io::Write;

// ── Exit code constants ─────────────────────────────────────────────

/// CLI exit code: success
pub const EXIT_SUCCESS: i32 = 0;
/// CLI exit code: execution error
pub const EXIT_FAILURE: i32 = 1;
/// CLI exit code: invalid arguments
pub const EXIT_INVALID_ARGS: i32 = 2;

// ── Unified output struct ───────────────────────────────────────────

/// CLI unified output format
///
/// ```json
/// { "success": true,  "data": { ... } }
/// { "success": false, "error": "..." }
/// ```
#[derive(serde::Serialize)]
pub struct CliOutput<T: serde::Serialize> {
    /// Whether execution was successful
    pub success: bool,
    /// Output data on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error message on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: serde::Serialize> CliOutput<T> {
    /// Create success output
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl CliOutput<()> {
    /// Create error output (no data, only error message)
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ── Output functions ────────────────────────────────────────────────

/// Output formatted JSON to stdout
pub fn output_json<T: serde::Serialize>(output: &CliOutput<T>) {
    match serde_json::to_string_pretty(output) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Output serialization failed: {}", e),
    }
}

/// Output error JSON to stdout
pub fn output_error(message: impl Into<String>) {
    let output = CliOutput::error(message);
    output_json(&output);
}

/// Output single-line JSON to stdout with flush (NDJSON mode)
pub fn output_ndjson<T: serde::Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(json) => {
            println!("{}", json);
            // flush failure does not affect main flow, log only
            if let Err(e) = flush_stdout() {
                eprintln!("{}", e);
            }
        }
        Err(e) => eprintln!("Event serialization failed: {}", e),
    }
}

/// Flush stdout buffer
pub fn flush_stdout() -> Result<(), String> {
    std::io::stdout()
        .flush()
        .map_err(|e| format!("flush failed: {}", e))
}

// ── Table formatting ─────────────────────────────────────────────────

/// Format data as a text table
///
/// Columns auto-resize, separated by two spaces, with a separator line below headers.
pub fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut result = String::new();

    // Headers
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            result.push_str("  ");
        }
        result.push_str(&format!("{:width$}", h, width = widths[i]));
    }
    result.push('\n');

    // Separator
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            result.push_str("  ");
        }
        result.push_str(&"-".repeat(*w));
    }
    result.push('\n');

    // Data rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                result.push_str("  ");
            }
            let w = widths.get(i).copied().unwrap_or(0);
            result.push_str(&format!("{:width$}", cell, width = w));
        }
        result.push('\n');
    }

    result
}
