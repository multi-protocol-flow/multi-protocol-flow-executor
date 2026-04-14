// 存储模块 - JSON 文件读写实现

use crate::stress_test::StressTestReport;
use flow_engine_core::flow::Flow;
use flow_engine_core::report::{ExecutionMode, FlowExecutionReport, ReportMetadata};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// 获取用户数据目录路径
///
/// 在不同平台返回相应的用户数据目录：
/// - Windows: %APPDATA%/multi-protocol-flow-executor
/// - macOS: ~/Library/Application Support/multi-protocol-flow-executor
/// - Linux: ~/.local/share/multi-protocol-flow-executor
pub fn get_data_dir() -> Result<PathBuf, String> {
    let app_name = "multi-protocol-flow-executor";

    let base_dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map_err(|e| format!("获取 APPDATA 环境变量失败: {}", e))?
            .into()
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map_err(|e| format!("获取 HOME 环境变量失败: {}", e))
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))?
    } else {
        // Linux 和其他类 Unix 系统
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map_err(|e| format!("获取 HOME 环境变量失败: {}", e))
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            })?
    };

    Ok(base_dir.join(app_name))
}

/// 获取 flows 子目录路径
pub fn get_flows_dir() -> Result<PathBuf, String> {
    Ok(get_data_dir()?.join("flows"))
}

/// 确保 flows 子目录存在
///
/// 如果目录不存在则创建，包括所有必要的父目录
pub fn ensure_flows_dir() -> Result<PathBuf, String> {
    let flows_dir = get_flows_dir()?;

    // 检查目录是否已存在（使用 try_exists 避免竞态条件）
    if !flows_dir
        .try_exists()
        .map_err(|e| format!("检查目录失败: {}", e))?
    {
        // 创建目录（包括所有必要的父目录）
        fs::create_dir_all(&flows_dir).map_err(|e| format!("创建 flows 目录失败: {}", e))?;
    }

    Ok(flows_dir)
}

/// 保存流程到 JSON 文件
///
/// # 参数
/// - `flow`: 要保存的流程引用
/// - `path`: 文件路径（可以是绝对路径或相对于 flows 目录的路径）
///
/// # 返回
/// - `Ok(())`: 保存成功
/// - `Err(String)`: 保存失败，包含错误信息
pub fn save_flow(flow: &Flow, path: &str) -> Result<(), String> {
    let file_path = if std::path::Path::new(path).is_absolute() {
        // 规范化绝对路径以避免重复分隔符
        PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path))
    } else {
        // 相对路径，使用 flows 目录
        ensure_flows_dir()?.join(path)
    };

    // 确保父目录存在（处理 Windows 上的竞态条件）
    if let Some(parent) = file_path.parent() {
        if !parent
            .try_exists()
            .map_err(|e| format!("检查父目录失败: {}", e))?
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建父目录 {:?} 失败: {}", parent, e))?;
        }
    }

    // 序列化为 JSON
    let json_string =
        serde_json::to_string_pretty(flow).map_err(|e| format!("序列化 Flow 失败: {}", e))?;

    // 创建并写入文件
    let mut file =
        File::create(&file_path).map_err(|e| format!("创建文件 {:?} 失败: {}", file_path, e))?;

    file.write_all(json_string.as_bytes())
        .map_err(|e| format!("写入文件 {:?} 失败: {}", file_path, e))?;

    Ok(())
}

/// 从 JSON 文件加载流程
///
/// # 参数
/// - `path`: 文件路径（可以是绝对路径或相对于 flows 目录的路径）
///
/// # 返回
/// - `Ok(Flow)`: 加载的流程
/// - `Err(String)`: 加载失败，包含错误信息
pub fn load_flow(path: &str) -> Result<Flow, String> {
    let file_path = if std::path::Path::new(path).is_absolute() {
        // 规范化绝对路径以避免重复分隔符
        PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path))
    } else {
        // 相对路径，使用 flows 目录
        get_flows_dir()?.join(path)
    };

    // 读取文件内容
    let mut file =
        File::open(&file_path).map_err(|e| format!("打开文件 {:?} 失败: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取文件 {:?} 失败: {}", file_path, e))?;

    // 反序列化为 Flow
    let flow: Flow =
        serde_json::from_str(&content).map_err(|e| format!("反序列化 Flow 失败: {}", e))?;

    Ok(flow)
}

// ============================================================================
// 报告存储函数
// ============================================================================

/// 获取 reports 子目录路径（仅 Tauri 模式使用）
pub fn get_reports_dir() -> Result<PathBuf, String> {
    Ok(get_data_dir()?.join("reports"))
}

/// 保存报告到指定目录
///
/// # 参数
/// - `report`: 报告对象
/// - `reports_dir`: 存储目录路径（由调用方决定）
pub fn save_report(report: &FlowExecutionReport, reports_dir: &Path) -> Result<(), String> {
    // 确保目录存在
    fs::create_dir_all(reports_dir).map_err(|e| format!("创建报告目录失败: {}", e))?;

    // 根据执行模式决定文件名前缀
    let filename = match report.execution_mode {
        ExecutionMode::Debug => format!("debug_{}.json", report.uuid),
        ExecutionMode::Normal => format!("{}.json", report.uuid),
    };

    // 构建文件路径
    let file_path = reports_dir.join(filename);

    // 序列化为 JSON
    let json_string =
        serde_json::to_string_pretty(report).map_err(|e| format!("序列化报告失败: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file =
        NamedTempFile::new_in(reports_dir).map_err(|e| format!("创建临时文件失败: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("保存报告文件失败: {}", e))?;

    Ok(())
}

/// 加载报告
///
/// # 参数
/// - `uuid`: 报告 UUID
/// - `reports_dir`: 存储目录路径
pub fn load_report(uuid: &str, reports_dir: &Path) -> Result<FlowExecutionReport, String> {
    // 尝试两种文件名格式：普通模式和调试模式
    let normal_path = reports_dir.join(format!("{}.json", uuid));
    let debug_path = reports_dir.join(format!("debug_{}.json", uuid));

    // 确定使用哪个文件路径
    let file_path = if normal_path
        .try_exists()
        .map_err(|e| format!("检查报告文件失败: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("检查报告文件失败: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("报告文件不存在: {}", uuid));
    };

    // 读取文件内容
    let mut file =
        File::open(&file_path).map_err(|e| format!("打开报告文件 {:?} 失败: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取报告文件 {:?} 失败: {}", file_path, e))?;

    // 反序列化
    let report: FlowExecutionReport =
        serde_json::from_str(&content).map_err(|e| format!("解析报告文件失败: {}", e))?;

    Ok(report)
}

/// 列出报告元数据
///
/// # 参数
/// - `reports_dir`: 存储目录路径
pub fn list_reports(reports_dir: &Path) -> Result<Vec<ReportMetadata>, String> {
    // 确保目录存在
    if !reports_dir
        .try_exists()
        .map_err(|e| format!("检查报告目录失败: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 JSON 文件
    for entry in fs::read_dir(reports_dir).map_err(|e| format!("读取报告目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        // 只处理 .json 文件
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            // 读取并解析报告
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(report) = serde_json::from_str::<FlowExecutionReport>(&content) {
                        reports.push(ReportMetadata {
                            uuid: report.uuid,
                            flow_uuid: report.flow_uuid,
                            flow_name: report.flow_name,
                            status: report.status.to_string().to_lowercase(),
                            timestamp: report.start_time,
                            duration_ms: report.duration_ms,
                            execution_mode: report.execution_mode,
                        });
                    }
                }
            }
        }
    }

    // 按时间戳倒序排序
    reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(reports)
}

/// 删除报告
///
/// # 参数
/// - `uuid`: 报告 UUID
/// - `reports_dir`: 存储目录路径
pub fn delete_report(uuid: &str, reports_dir: &Path) -> Result<(), String> {
    // 尝试两种文件名格式：普通模式和调试模式
    let normal_path = reports_dir.join(format!("{}.json", uuid));
    let debug_path = reports_dir.join(format!("debug_{}.json", uuid));

    // 确定使用哪个文件路径
    let file_path = if normal_path
        .try_exists()
        .map_err(|e| format!("检查报告文件失败: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("检查报告文件失败: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("报告文件不存在: {}", uuid));
    };

    // 删除文件
    fs::remove_file(&file_path).map_err(|e| format!("删除报告文件 {:?} 失败: {}", file_path, e))?;

    Ok(())
}

// ============================================================================
// 压测报告存储函数
// ============================================================================

/// 压测报告摘要（用于列表展示）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressTestReportSummary {
    /// 报告唯一 ID
    pub uuid: String,
    /// 关联的流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 测试开始时间（毫秒时间戳）
    pub start_time: u64,
}

/// 保存压测报告到指定目录
///
/// # 参数
/// - `report`: 压测报告对象
/// - `reports_dir`: 存储目录路径（由调用方决定）
///
/// # 返回
/// - `Ok(PathBuf)`: 保存的文件路径
/// - `Err(String)`: 保存失败，包含错误信息
///
/// # 文件命名格式
/// `performance_{flow_name}_{YYYYMMDD_HHMMSS}.json`
pub fn save_stress_report(
    report: &StressTestReport,
    reports_dir: &Path,
) -> Result<PathBuf, String> {
    // 确保目录存在
    fs::create_dir_all(reports_dir).map_err(|e| format!("创建压测报告目录失败: {}", e))?;

    // 生成时间戳：YYYYMMDD_HHMMSS
    let timestamp = format_timestamp(report.start_time);

    // 清理流程名称中的非法文件名字符
    let safe_flow_name = sanitize_filename(&report.flow_name);

    // 构建文件名
    let filename = format!("performance_{}_{}.json", safe_flow_name, timestamp);

    // 构建文件路径
    let file_path = reports_dir.join(filename);

    // 序列化为 JSON
    let json_string =
        serde_json::to_string_pretty(report).map_err(|e| format!("序列化压测报告失败: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file =
        NamedTempFile::new_in(reports_dir).map_err(|e| format!("创建临时文件失败: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("保存压测报告文件失败: {}", e))?;

    Ok(file_path)
}

/// 加载压测报告
///
/// # 参数
/// - `file_path`: 报告文件路径
///
/// # 返回
/// - `Ok(StressTestReport)`: 加载的压测报告
/// - `Err(String)`: 加载失败，包含错误信息
pub fn load_stress_report(file_path: &Path) -> Result<StressTestReport, String> {
    // 检查文件是否存在
    if !file_path
        .try_exists()
        .map_err(|e| format!("检查压测报告文件失败: {}", e))?
    {
        return Err(format!("压测报告文件不存在: {:?}", file_path));
    }

    // 读取文件内容
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开压测报告文件 {:?} 失败: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取压测报告文件 {:?} 失败: {}", file_path, e))?;

    // 反序列化
    let report: StressTestReport =
        serde_json::from_str(&content).map_err(|e| format!("解析压测报告文件失败: {}", e))?;

    Ok(report)
}

/// 列出压测报告摘要
///
/// # 参数
/// - `reports_dir`: 存储目录路径
///
/// # 返回
/// - `Ok(Vec<StressTestReportSummary>)`: 报告摘要列表，按开始时间倒序排序
/// - `Err(String)`: 读取失败，包含错误信息
pub fn list_stress_reports(reports_dir: &Path) -> Result<Vec<StressTestReportSummary>, String> {
    // 确保目录存在
    if !reports_dir
        .try_exists()
        .map_err(|e| format!("检查压测报告目录失败: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 performance_*.json 文件
    for entry in fs::read_dir(reports_dir).map_err(|e| format!("读取压测报告目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        // 获取文件名
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // 只处理 performance_*.json 文件
        if !filename.starts_with("performance_") || !filename.ends_with(".json") {
            continue;
        }

        // 读取并解析报告
        if let Ok(mut file) = File::open(&path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(report) = serde_json::from_str::<StressTestReport>(&content) {
                    reports.push(StressTestReportSummary {
                        uuid: report.uuid,
                        flow_uuid: report.flow_uuid,
                        flow_name: report.flow_name,
                        start_time: report.start_time,
                    });
                }
            }
        }
    }

    // 按开始时间倒序排序
    reports.sort_by(|a, b| b.start_time.cmp(&a.start_time));

    Ok(reports)
}

/// 将毫秒时间戳格式化为 `YYYYMMDD_HHMMSS` 格式
fn format_timestamp(timestamp_ms: u64) -> String {
    use chrono::{TimeZone, Utc};

    // 将毫秒时间戳转换为 chrono DateTime
    let secs = (timestamp_ms / 1000) as i64;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;

    if let Some(datetime) = Utc.timestamp_opt(secs, nanos).single() {
        datetime.format("%Y%m%d_%H%M%S").to_string()
    } else {
        // 回退：使用当前时间
        let now = chrono::Utc::now();
        now.format("%Y%m%d_%H%M%S").to_string()
    }
}

/// 清理文件名中的非法字符
fn sanitize_filename(name: &str) -> String {
    // 替换 Windows 和 Unix 都不允许的字符
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', ' '];
    let mut result = name.to_string();
    for c in invalid_chars {
        result = result.replace(c, "_");
    }
    // 限制长度，避免文件名过长
    if result.len() > 50 {
        result = result.chars().take(50).collect();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 get_data_dir() 返回正确的路径格式
    ///
    /// 验证点：
    /// - 返回的路径以应用名称 "multi-protocol-flow-executor" 结尾
    /// - 路径是绝对路径（在环境变量可用时）
    #[test]
    fn test_get_data_dir_contains_app_name() {
        let result = get_data_dir();

        // get_data_dir() 在环境变量可用时应返回 Ok
        if let Ok(path) = result {
            // 验证路径以应用名称结尾
            assert!(
                path.to_string_lossy()
                    .ends_with("multi-protocol-flow-executor"),
                "数据目录路径应以 'multi-protocol-flow-executor' 结尾，实际为: {:?}",
                path
            );
        }
        // 如果环境变量不可用（如 CI 环境中某些情况），测试应跳过
    }

    /// 测试 get_data_dir() 返回的路径是有效的目录路径
    #[test]
    fn test_get_data_dir_is_valid_path() {
        if let Ok(path) = get_data_dir() {
            // 验证路径不为空
            assert!(!path.as_os_str().is_empty(), "数据目录路径不应为空");

            // 验证路径组件数量合理（至少包含用户目录和应用名）
            let components: Vec<_> = path.components().collect();
            assert!(
                components.len() >= 2,
                "路径组件数量应至少为2，实际为: {}",
                components.len()
            );
        }
    }

    /// 测试 get_flows_dir() 返回数据目录下的 flows 子目录
    #[test]
    fn test_get_flows_dir() {
        if let Ok(data_dir) = get_data_dir() {
            let flows_dir = get_flows_dir();

            assert!(flows_dir.is_ok(), "get_flows_dir() 应返回 Ok");

            let flows_path = flows_dir.unwrap();
            let expected = data_dir.join("flows");

            assert_eq!(
                flows_path, expected,
                "flows 目录应为数据目录下的 'flows' 子目录"
            );
        }
    }

    /// 测试 get_reports_dir() 返回数据目录下的 reports 子目录
    #[test]
    fn test_get_reports_dir() {
        if let Ok(data_dir) = get_data_dir() {
            let reports_dir = get_reports_dir();

            assert!(reports_dir.is_ok(), "get_reports_dir() 应返回 Ok");

            let reports_path = reports_dir.unwrap();
            let expected = data_dir.join("reports");

            assert_eq!(
                reports_path, expected,
                "reports 目录应为数据目录下的 'reports' 子目录"
            );
        }
    }

    /// 测试日志目录组合逻辑（模拟 CLI 中的用法）
    ///
    /// CLI 中使用: get_data_dir().map(|d| d.join("logs")).unwrap_or_default()
    #[test]
    fn test_log_dir_composition() {
        let log_dir = get_data_dir().map(|d| d.join("logs"));

        if let Ok(path) = log_dir {
            // 验证日志目录路径以 "logs" 结尾
            assert!(
                path.to_string_lossy().ends_with("logs"),
                "日志目录路径应以 'logs' 结尾，实际为: {:?}",
                path
            );

            // 验证日志目录的父目录是数据目录
            if let Some(parent) = path.parent() {
                let data_dir = get_data_dir().unwrap();
                assert_eq!(parent, &data_dir, "日志目录的父目录应为数据目录");
            }
        }
    }

    /// 测试默认值回退逻辑（模拟 CLI 中 unwrap_or_default 的行为）
    #[test]
    fn test_log_dir_fallback_to_default() {
        // 模拟 CLI 中的逻辑：如果 get_data_dir() 失败，使用默认空路径
        let log_dir: PathBuf = get_data_dir().map(|d| d.join("logs")).unwrap_or_default();

        // 如果 get_data_dir() 成功，log_dir 应该是非空路径
        // 如果失败（某些测试环境），log_dir 应该是空路径
        if get_data_dir().is_ok() {
            assert!(
                !log_dir.as_os_str().is_empty(),
                "当 get_data_dir() 成功时，日志目录路径不应为空"
            );
        } else {
            assert!(
                log_dir.as_os_str().is_empty(),
                "当 get_data_dir() 失败时，应回退到空路径"
            );
        }
    }

    /// 测试 ensure_flows_dir() 创建目录
    #[test]
    fn test_ensure_flows_dir_creates_directory() {
        // 此测试需要文件系统访问，跳过如果环境不允许
        if let Ok(flows_dir) = ensure_flows_dir() {
            // 验证目录存在
            assert!(
                flows_dir.try_exists().unwrap_or(false),
                "flows 目录应该存在"
            );

            // 验证目录名正确
            assert!(
                flows_dir.to_string_lossy().ends_with("flows"),
                "目录名应为 'flows'"
            );

            // 清理：尝试删除创建的目录（如果为空）
            let _ = std::fs::remove_dir(&flows_dir);
        }
    }

    // ============================================================================
    // 压测报告存储测试
    // ============================================================================

    /// 测试 format_timestamp 函数
    #[test]
    fn test_format_timestamp() {
        // 使用 chrono 验证时间戳转换
        use chrono::{TimeZone, Utc};

        // 创建一个已知的时间
        let datetime = Utc
            .with_ymd_and_hms(2024, 1, 15, 10, 30, 45)
            .single()
            .unwrap();
        let timestamp_ms = datetime.timestamp_millis() as u64;

        let formatted = format_timestamp(timestamp_ms);

        // 验证格式正确（YYYYMMDD_HHMMSS）
        assert!(formatted.starts_with("20240115_"));
        assert_eq!(formatted.len(), 15); // "YYYYMMDD_HHMMSS" = 15 字符

        // 验证格式化后的时间可以解析
        let year: i32 = formatted[0..4].parse().unwrap();
        let month: u32 = formatted[4..6].parse().unwrap();
        let day: u32 = formatted[6..8].parse().unwrap();
        let hour: u32 = formatted[9..11].parse().unwrap();
        let minute: u32 = formatted[11..13].parse().unwrap();
        let second: u32 = formatted[13..15].parse().unwrap();

        assert_eq!(year, 2024);
        assert_eq!(month, 1);
        assert_eq!(day, 15);
        assert_eq!(hour, 10);
        assert_eq!(minute, 30);
        assert_eq!(second, 45);
    }

    /// 测试 sanitize_filename 函数
    #[test]
    fn test_sanitize_filename() {
        // 测试普通名称
        assert_eq!(sanitize_filename("Test Flow"), "Test_Flow");

        // 测试包含非法字符的名称
        assert_eq!(sanitize_filename("Test<Flow>"), "Test_Flow_");
        assert_eq!(sanitize_filename("Test:Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test/Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test\\Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test|Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test?Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test*Flow"), "Test_Flow");
        assert_eq!(sanitize_filename("Test\"Flow"), "Test_Flow");

        // 测试长度限制
        let long_name = "a".repeat(100);
        let sanitized = sanitize_filename(&long_name);
        assert_eq!(sanitized.len(), 50);
    }

    /// 测试 StressTestReportSummary 序列化
    #[test]
    fn test_stress_test_report_summary_serialization() {
        let summary = StressTestReportSummary {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1705315845000,
        };

        let json = serde_json::to_string(&summary).expect("序列化失败");
        assert!(json.contains("report-001"));
        assert!(json.contains("flow-001"));
        assert!(json.contains("Test Flow"));
        assert!(json.contains("uuid"));
        assert!(json.contains("flowUuid"));
        assert!(json.contains("flowName"));
        assert!(json.contains("startTime"));

        let deserialized: StressTestReportSummary =
            serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized, summary);
    }

    /// 测试 save_stress_report 和 load_stress_report
    #[test]
    fn test_save_and_load_stress_report() {
        use crate::stress_test::{
            ResponseTimeDistribution, StressTestConfig, StressTestReport, StressTestStatistics,
        };
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path();

        // 创建测试报告
        let report = StressTestReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1705315845000,
            end_time: 1705315905000,
            duration_ms: 60000,
            config: StressTestConfig {
                concurrent_users: 10,
                ramp_up_seconds: 10,
                duration_seconds: 60,
                flow_uuid: "flow-001".to_string(),
            },
            statistics: StressTestStatistics {
                total_iterations: 100,
                successful: 95,
                failed: 5,
                success_rate: 0.95,
                rps: 10.0,
            },
            response_time_distribution: ResponseTimeDistribution {
                avg_ms: 150.5,
                min_ms: 50,
                max_ms: 500,
                p50_ms: 140,
                p90_ms: 300,
                p95_ms: 400,
                p99_ms: 450,
            },
            error_statistics: vec![],
            node_statistics: vec![],
            node_type_statistics: vec![],
        };

        // 保存报告
        let saved_path = save_stress_report(&report, reports_dir).expect("保存报告失败");

        // 验证文件名格式
        let filename = saved_path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("performance_Test_Flow_"));
        assert!(filename.ends_with(".json"));

        // 加载报告
        let loaded_report = load_stress_report(&saved_path).expect("加载报告失败");

        // 验证报告内容
        assert_eq!(loaded_report.uuid, report.uuid);
        assert_eq!(loaded_report.flow_uuid, report.flow_uuid);
        assert_eq!(loaded_report.flow_name, report.flow_name);
        assert_eq!(loaded_report.start_time, report.start_time);
        assert_eq!(loaded_report.end_time, report.end_time);
        assert_eq!(loaded_report.duration_ms, report.duration_ms);
        assert_eq!(
            loaded_report.statistics.total_iterations,
            report.statistics.total_iterations
        );
    }

    /// 测试 list_stress_reports
    #[test]
    fn test_list_stress_reports() {
        use crate::stress_test::{
            ResponseTimeDistribution, StressTestConfig, StressTestReport, StressTestStatistics,
        };
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path();

        // 创建多个测试报告
        let reports = vec![
            StressTestReport {
                uuid: "report-001".to_string(),
                flow_uuid: "flow-001".to_string(),
                flow_name: "Flow A".to_string(),
                start_time: 1705315845000, // 较早
                end_time: 1705315905000,
                duration_ms: 60000,
                config: StressTestConfig::default(),
                statistics: StressTestStatistics::default(),
                response_time_distribution: ResponseTimeDistribution::default(),
                error_statistics: vec![],
                node_statistics: vec![],
                node_type_statistics: vec![],
            },
            StressTestReport {
                uuid: "report-002".to_string(),
                flow_uuid: "flow-002".to_string(),
                flow_name: "Flow B".to_string(),
                start_time: 1705315945000, // 较晚
                end_time: 1705316005000,
                duration_ms: 60000,
                config: StressTestConfig::default(),
                statistics: StressTestStatistics::default(),
                response_time_distribution: ResponseTimeDistribution::default(),
                error_statistics: vec![],
                node_statistics: vec![],
                node_type_statistics: vec![],
            },
            StressTestReport {
                uuid: "report-003".to_string(),
                flow_uuid: "flow-003".to_string(),
                flow_name: "Flow C".to_string(),
                start_time: 1705315895000, // 中间
                end_time: 1705315955000,
                duration_ms: 60000,
                config: StressTestConfig::default(),
                statistics: StressTestStatistics::default(),
                response_time_distribution: ResponseTimeDistribution::default(),
                error_statistics: vec![],
                node_statistics: vec![],
                node_type_statistics: vec![],
            },
        ];

        // 保存所有报告
        for report in &reports {
            save_stress_report(report, reports_dir).expect("保存报告失败");
        }

        // 列出报告
        let summaries = list_stress_reports(reports_dir).expect("列出报告失败");

        // 验证数量
        assert_eq!(summaries.len(), 3);

        // 验证按时间倒序排序（最新的在前）
        assert_eq!(summaries[0].uuid, "report-002"); // start_time: 1705315945000
        assert_eq!(summaries[1].uuid, "report-003"); // start_time: 1705315895000
        assert_eq!(summaries[2].uuid, "report-001"); // start_time: 1705315845000
    }

    /// 测试 list_stress_reports 空目录
    #[test]
    fn test_list_stress_reports_empty_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path();

        let summaries = list_stress_reports(reports_dir).expect("列出报告失败");
        assert!(summaries.is_empty());
    }

    /// 测试 load_stress_report 文件不存在
    #[test]
    fn test_load_stress_report_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let non_existent_path = temp_dir.path().join("non_existent.json");

        let result = load_stress_report(&non_existent_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }
}
