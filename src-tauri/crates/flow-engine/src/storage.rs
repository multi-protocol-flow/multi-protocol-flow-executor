// 存储模块 - JSON 文件读写实现

use crate::flow::Flow;
use crate::report::{FlowExecutionReport, ReportMetadata};
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
    fs::create_dir_all(reports_dir)
        .map_err(|e| format!("创建报告目录失败: {}", e))?;
    
    // 构建文件路径
    let file_path = reports_dir.join(format!("{}.json", report.uuid));
    
    // 序列化为 JSON
    let json_string = serde_json::to_string_pretty(report)
        .map_err(|e| format!("序列化报告失败: {}", e))?;
    
    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file = NamedTempFile::new_in(reports_dir)
        .map_err(|e| format!("创建临时文件失败: {}", e))?;
    
    // 写入内容到临时文件
    temp_file.write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    
    // 原子重命名到最终路径
    temp_file.persist(&file_path)
        .map_err(|e| format!("保存报告文件失败: {}", e))?;
    
    Ok(())
}

/// 加载报告
/// 
/// # 参数
/// - `uuid`: 报告 UUID
/// - `reports_dir`: 存储目录路径
pub fn load_report(uuid: &str, reports_dir: &Path) -> Result<FlowExecutionReport, String> {
    let file_path = reports_dir.join(format!("{}.json", uuid));
    
    // 检查文件是否存在
    if !file_path.try_exists().map_err(|e| format!("检查报告文件失败: {}", e))? {
        return Err(format!("报告文件不存在: {}", uuid));
    }
    
    // 读取文件内容
    let mut file = File::open(&file_path)
        .map_err(|e| format!("打开报告文件 {:?} 失败: {}", file_path, e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取报告文件 {:?} 失败: {}", file_path, e))?;
    
    // 反序列化
    let report: FlowExecutionReport = serde_json::from_str(&content)
        .map_err(|e| format!("解析报告文件失败: {}", e))?;
    
    Ok(report)
}

/// 列出报告元数据
/// 
/// # 参数
/// - `reports_dir`: 存储目录路径
pub fn list_reports(reports_dir: &Path) -> Result<Vec<ReportMetadata>, String> {
    // 确保目录存在
    if !reports_dir.try_exists().map_err(|e| format!("检查报告目录失败: {}", e))? {
        return Ok(Vec::new());
    }
    
    let mut reports = Vec::new();
    
    // 读取目录中的所有 JSON 文件
    for entry in fs::read_dir(reports_dir)
        .map_err(|e| format!("读取报告目录失败: {}", e))?
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
    let file_path = reports_dir.join(format!("{}.json", uuid));
    
    // 检查文件是否存在
    if !file_path.try_exists().map_err(|e| format!("检查报告文件失败: {}", e))? {
        return Err(format!("报告文件不存在: {}", uuid));
    }
    
    // 删除文件
    fs::remove_file(&file_path)
        .map_err(|e| format!("删除报告文件 {:?} 失败: {}", file_path, e))?;
    
    Ok(())
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
                path.to_string_lossy().ends_with("multi-protocol-flow-executor"),
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
            assert!(components.len() >= 2, "路径组件数量应至少为2，实际为: {}", components.len());
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
        let log_dir = get_data_dir()
            .map(|d| d.join("logs"));
        
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
                assert_eq!(
                    parent, &data_dir,
                    "日志目录的父目录应为数据目录"
                );
            }
        }
    }

    /// 测试默认值回退逻辑（模拟 CLI 中 unwrap_or_default 的行为）
    #[test]
    fn test_log_dir_fallback_to_default() {
        // 模拟 CLI 中的逻辑：如果 get_data_dir() 失败，使用默认空路径
        let log_dir: PathBuf = get_data_dir()
            .map(|d| d.join("logs"))
            .unwrap_or_default();
        
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
}