//! CLI 输出模块 - 统一 JSON 输出格式
//!
//! 提供三种输出模式:
//! - `output_json` — 格式化 JSON 输出 (默认模式)
//! - `output_error` — 错误 JSON 输出
//! - `output_ndjson` — 单行 JSON 输出 + flush (NDJSON 流式模式)

use std::io::Write;

// ── 退出码常量 ──────────────────────────────────────────────────────

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_INVALID_ARGS: i32 = 2;

// ── 统一输出结构体 ──────────────────────────────────────────────────

/// CLI 统一输出格式
///
/// ```json
/// { "success": true,  "data": { ... } }
/// { "success": false, "error": "..." }
/// ```
#[derive(serde::Serialize)]
pub struct CliOutput<T: serde::Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: serde::Serialize> CliOutput<T> {
    /// 创建成功输出
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl CliOutput<()> {
    /// 创建错误输出 (无数据，只有错误信息)
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ── 输出函数 ────────────────────────────────────────────────────────

/// 输出格式化 JSON 到 stdout
pub fn output_json<T: serde::Serialize>(output: &CliOutput<T>) {
    match serde_json::to_string_pretty(output) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("序列化输出失败: {}", e),
    }
}

/// 输出错误 JSON 到 stdout
pub fn output_error(message: impl Into<String>) {
    let output = CliOutput::error(message);
    output_json(&output);
}

/// 输出单行 JSON 到 stdout 并 flush (NDJSON 模式)
pub fn output_ndjson<T: serde::Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(json) => {
            println!("{}", json);
            // flush 失败不影响主流程，仅记录日志
            if let Err(e) = flush_stdout() {
                eprintln!("{}", e);
            }
        }
        Err(e) => eprintln!("序列化事件失败: {}", e),
    }
}

/// 刷新 stdout 缓冲区
pub fn flush_stdout() -> Result<(), String> {
    std::io::stdout()
        .flush()
        .map_err(|e| format!("flush 失败: {}", e))
}

// ── 表格格式化 ──────────────────────────────────────────────────────

/// 将数据格式化为文本表格
///
/// 列宽自适应，列间用两个空格分隔，表头下方添加分隔线。
pub fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    // 计算列宽
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut result = String::new();

    // 表头
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            result.push_str("  ");
        }
        result.push_str(&format!("{:width$}", h, width = widths[i]));
    }
    result.push('\n');

    // 分隔线
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            result.push_str("  ");
        }
        result.push_str(&"-".repeat(*w));
    }
    result.push('\n');

    // 数据行
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
