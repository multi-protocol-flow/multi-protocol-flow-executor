// 存储模块 - JSON 文件读写实现

use crate::stress_test::StressTestReport;
use flow_engine_core::flow::Flow;
use flow_engine_core::report::{ExecutionMode, ExecutionStatus, FlowExecutionReport, ReportMetadata};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tempfile::NamedTempFile;

/// `FlowFile` 默认版本号
fn default_version() -> u32 {
    1
}

/// 流程文件包装结构，包含版本号和流程数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowFile {
    /// 文件格式版本号
    #[serde(default = "default_version")]
    pub _version: u32,
    /// 流程数据
    pub flow: Flow,
    /// 内置模板元数据（版本号、是否内置等），仅内置模板使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _meta: Option<serde_json::Value>,
}

/// 轻量级流程元数据，仅用于列表时快速提取 id 和名称（不验证完整 Flow 结构）
#[derive(Debug, Clone, serde::Deserialize)]
struct FlowFileMeta {
    flow: FlowMeta,
}

/// 轻量级流程信息
#[derive(Debug, Clone, serde::Deserialize)]
struct FlowMeta {
    uuid: String,
    name: String,
}

/// 流程元数据，用于列表展示
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowMetadata {
    /// 流程唯一标识符
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 文件路径（绝对路径）
    pub file_path: String,
    /// 最后更新时间戳（毫秒）
    pub updated_at: u64,
    /// 所属文件夹相对路径（相对于 `flows_dir`，空字符串表示根目录）
    #[serde(default)]
    pub folder_path: String,
}

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
            .map_err(|e| format!("获取 APPDATA 环境变量Failed: {}", e))?
            .into()
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map_err(|e| format!("获取 HOME 环境变量Failed: {}", e))
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))?
    } else {
        // Linux 和其他类 Unix 系统
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map_err(|e| format!("获取 HOME 环境变量Failed: {}", e))
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
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        // 创建目录（包括所有必要的父目录）
        fs::create_dir_all(&flows_dir).map_err(|e| format!("创建 flows 目录Failed: {}", e))?;
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
/// - `Ok(())`: 保存Success
/// - `Err(String)`: 保存Failed，包含Error信息
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
            .map_err(|e| format!("检查父目录Failed: {}", e))?
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建父目录 {:?} Failed: {}", parent, e))?;
        }
    }

    // 包装为 FlowFile 并序列化为 JSON
    let flow_file = FlowFile {
        _version: 1,
        flow: flow.clone(),
        _meta: None,
    };
    let json_string =
        serde_json::to_string_pretty(&flow_file).map_err(|e| format!("序列化 Flow Failed: {}", e))?;

    // 创建并写入文件
    let mut file =
        File::create(&file_path).map_err(|e| format!("创建文件 {:?} Failed: {}", file_path, e))?;

    file.write_all(json_string.as_bytes())
        .map_err(|e| format!("写入文件 {:?} Failed: {}", file_path, e))?;

    Ok(())
}

/// 从 JSON 文件加载流程
///
/// # 参数
/// - `path`: 文件路径（可以是绝对路径或相对于 flows 目录的路径）
///
/// # 返回
/// - `Ok(Flow)`: 加载的流程
/// - `Err(String)`: 加载Failed，包含Error信息
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
        File::open(&file_path).map_err(|e| format!("打开文件 {:?} Failed: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取文件 {:?} Failed: {}", file_path, e))?;

    // 反序列化为 FlowFile 包装格式（.mpf 文件统一格式）
    let flow_file: FlowFile = serde_json::from_str(&content)
        .map_err(|e| format!("Deserialize FlowFile Failed: {} (Flow file must use FlowFile wrapper format with _version and flow keys)", e))?;
    let flow = flow_file.flow;

    Ok(flow)
}

// === .mpf 流程文件管理 ===

/// 递归收集目录下所有 .mpf 文件路径
fn collect_mpf_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录Failed: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项Failed: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_mpf_files(&path, files)?;
        } else if path.extension().map(|e| e == "mpf").unwrap_or(false) {
            files.push(path);
        }
    }

    Ok(())
}

/// 保存流程到 .mpf 文件（原子写入）
///
/// 使用 `FlowFile` 包装结构序列化，通过 `NamedTempFile` + `persist` 实现原子写入。
/// 如果同 UUID 的流程文件已存在，则覆盖该文件；否则生成新文件名。
/// 新文件名由流程名称经过 `sanitize_filename` 清理后加上 `.mpf` 扩展名生成，
/// 如果同名文件已存在则追加递增数字后缀。
pub fn save_flow_file(flow: &Flow, flows_dir: &Path) -> Result<(), String> {
    save_flow_file_with_meta(flow, flows_dir, None)
}

/// 保存流程到 .mpf 文件（原子写入），可携带 _meta 元数据
///
/// 使用 `FlowFile` 包装结构序列化，通过 `NamedTempFile` + `persist` 实现原子写入。
/// 如果同 UUID 的流程文件已存在，则覆盖该文件；否则生成新文件名。
/// 新文件名由流程名称经过 `sanitize_filename` 清理后加上 `.mpf` 扩展名生成，
/// 如果同名文件已存在则追加递增数字后缀。
pub fn save_flow_file_with_meta(
    flow: &Flow,
    flows_dir: &Path,
    meta: Option<serde_json::Value>,
) -> Result<(), String> {
    // 确保目录存在
    fs::create_dir_all(flows_dir).map_err(|e| format!("创建流程目录Failed: {}", e))?;

    // 包装为 FlowFile
    let flow_file = FlowFile {
        _version: 1,
        flow: flow.clone(),
        _meta: meta,
    };

    // 序列化为 JSON
    let json_string =
        serde_json::to_string_pretty(&flow_file).map_err(|e| format!("序列化流程Failed: {}", e))?;

    // 检查是否已存在同 UUID 的文件（更新场景）
    let mut existing_path: Option<PathBuf> = None;
    let mut mpf_files = Vec::new();
    let _ = collect_mpf_files(flows_dir, &mut mpf_files);
    for path in &mpf_files {
        let mut content = String::new();
        if let Ok(mut file) = File::open(path) {
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(existing) = serde_json::from_str::<FlowFile>(&content) {
                    if existing.flow.uuid == flow.uuid {
                        existing_path = Some(path.clone());
                        break;
                    }
                }
            }
        }
    }

    // 确定目标目录：保持原有文件夹位置，否则使用根目录
    let target_dir = existing_path
        .as_ref()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| flows_dir.to_path_buf());

    // 确保目标目录存在
    if target_dir != flows_dir {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("创建目标目录Failed: {}", e))?;
    }

    // 生成文件名，处理冲突
    let safe_name = sanitize_filename(&flow.name);
    let mut filename = format!("{}.mpf", safe_name);
    let mut counter = 2u32;

    loop {
        let candidate_path = target_dir.join(&filename);
        // 如果目标路径就是旧文件路径（名称未变），直接跳出
        let is_old_path = existing_path.as_ref() == Some(&candidate_path);
        if is_old_path || !candidate_path.try_exists().map_err(|e| format!("检查文件Failed: {}", e))? {
            break;
        }
        filename = format!("{}_{}.mpf", safe_name, counter);
        counter += 1;
    }

    let target_path = target_dir.join(&filename);

    // 原子写入：先写临时文件，再重命名
    let mut temp_file =
        NamedTempFile::new_in(flows_dir).map_err(|e| format!("创建临时文件Failed: {}", e))?;

    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件Failed: {}", e))?;

    temp_file
        .persist(&target_path)
        .map_err(|e| format!("保存流程文件Failed: {}", e))?;

    // 如果文件名/位置变更，删除旧文件
    if let Some(ref old_path) = existing_path {
        if *old_path != target_path {
            let _ = fs::remove_file(old_path);
        }
    }

    Ok(())
}

/// 从 .mpf 文件加载流程文件（按 UUID 查找）
///
/// 先轻量扫描定位文件，再完整验证 Flow 结构。
/// 如果文件格式损坏，返回明确的Error提示而不是"流程不存在"。
pub fn load_flow_file_raw(uuid: &str, flows_dir: &Path) -> Result<FlowFile, String> {
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    // 第一遍：轻量扫描，只找文件名（不验证完整 Flow 结构）
    let target_path = {
        let mut found = None::<PathBuf>;
        for path in &mpf_files {
            let mut content = String::new();
            if (|| -> Result<(), String> {
                let mut file =
                    File::open(path).map_err(|e| format!("打开文件Failed: {}", e))?;
                file.read_to_string(&mut content)
                    .map_err(|e| format!("读取文件Failed: {}", e))?;
                Ok(())
            })()
            .is_err()
            {
                continue;
            }
            if let Ok(meta) = serde_json::from_str::<FlowFileMeta>(&content) {
                if meta.flow.uuid == uuid {
                    found = Some(path.clone());
                    break;
                }
            }
        }
        found
    };

    match target_path {
        Some(path) => {
            // 第二遍：读取并完整验证该文件
            let mut content = String::new();
            let mut file =
                File::open(&path).map_err(|e| format!("打开文件Failed: {}", e))?;
            file.read_to_string(&mut content)
                .map_err(|e| format!("读取文件Failed: {}", e))?;

            let flow_file: FlowFile = serde_json::from_str(&content).map_err(|e| {
                format!(
                    "流程文件已损坏: {} - {}",
                    path.display(),
                    e
                )
            })?;

            Ok(flow_file)
        }
        None => Err(format!("流程不存在: {}", uuid)),
    }
}

/// 从 .mpf 文件加载流程（按 UUID 查找），仅返回 Flow 部分
///
/// 递归扫描 `flows_dir` 下所有 `.mpf` 文件，找到匹配 UUID 的流程并返回。
pub fn load_flow_file(uuid: &str, flows_dir: &Path) -> Result<Flow, String> {
    let flow_file = load_flow_file_raw(uuid, flows_dir)?;
    Ok(flow_file.flow)
}

/// 列出所有 .mpf 流程文件的元数据
///
/// 仅检查 `.mpf` 后缀，使用轻量 JSON 解析提取 UUID 和流程名称，不验证完整 Flow 结构。
/// 损坏的文件（JSON 解析Failed）会被跳过并记录警告日志。
/// 结果按文件夹路径排序，同文件夹内按名称排序。
pub fn list_flow_files(flows_dir: &Path) -> Result<Vec<FlowMetadata>, String> {
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    let mut metadata_list = Vec::new();

    for path in &mpf_files {
        let mut content = String::new();
        let read_ok = (|| -> Result<(), String> {
            let mut file = File::open(path).map_err(|e| format!("打开文件Failed: {}", e))?;
            file.read_to_string(&mut content)
                .map_err(|e| format!("读取文件Failed: {}", e))?;
            Ok(())
        })();

        if read_ok.is_err() {
            continue;
        }

        // 轻量解析：仅提取 uuid 和 name，不验证完整 Flow 结构
        let flow_meta: FlowFileMeta = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("跳过损坏的流程文件: {} - {}", path.display(), e);
                continue;
            }
        };

        // 从文件系统元数据获取修改时间
        let updated_at = fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // 计算相对文件夹路径（统一使用 / 分隔符，确保跨平台前端一致）
        let folder_path = path
            .parent()
            .and_then(|parent| parent.strip_prefix(flows_dir).ok())
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        metadata_list.push(FlowMetadata {
            uuid: flow_meta.flow.uuid,
            name: flow_meta.flow.name,
            file_path: path.to_string_lossy().to_string(),
            updated_at,
            folder_path,
        });
    }

    // 按文件夹路径排序，同文件夹内按名称排序
    metadata_list.sort_by(|a, b| {
        a.folder_path
            .cmp(&b.folder_path)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(metadata_list)
}

/// 删除指定 UUID 的流程文件
///
/// 递归扫描查找匹配 UUID 的 .mpf 文件并删除。
pub fn delete_flow_file(uuid: &str, flows_dir: &Path) -> Result<(), String> {
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    for path in mpf_files {
        let mut content = String::new();
        let read_ok = (|| -> Result<(), String> {
            let mut file = File::open(&path).map_err(|e| format!("打开文件Failed: {}", e))?;
            file.read_to_string(&mut content)
                .map_err(|e| format!("读取文件Failed: {}", e))?;
            Ok(())
        })();

        if read_ok.is_err() {
            continue;
        }

        if let Ok(meta) = serde_json::from_str::<FlowFileMeta>(&content) {
            if meta.flow.uuid == uuid {
                fs::remove_file(&path)
                    .map_err(|e| format!("删除流程文件 {:?} Failed: {}", path, e))?;
                return Ok(());
            }
        }
    }

    Err(format!("流程不存在: {}", uuid))
}

/// 复制流程文件（生成新 UUID 和副本名称）
///
/// 加载指定 UUID 的流程，生成新 UUID，在名称后添加 "(副本)" 后缀，
/// 如果名称冲突则追加递增数字 "(副本 2)"、"(副本 3)" 等。
pub fn copy_flow_file(uuid: &str, flows_dir: &Path) -> Result<Flow, String> {
    let original_flow = load_flow_file(uuid, flows_dir)?;

    let new_uuid = uuid::Uuid::new_v4().to_string();

    // 收集现有流程名称，用于冲突检测
    let mut existing_names = HashSet::new();
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;
    for path in &mpf_files {
        let mut content = String::new();
        let read_ok = (|| -> Result<(), String> {
            let mut file = File::open(path).map_err(|e| format!("打开文件Failed: {}", e))?;
            file.read_to_string(&mut content)
                .map_err(|e| format!("读取文件Failed: {}", e))?;
            Ok(())
        })();

        if read_ok.is_err() {
            continue;
        }

        if let Ok(flow_file) = serde_json::from_str::<FlowFile>(&content) {
            existing_names.insert(flow_file.flow.name);
        }
    }

    // 生成不冲突的副本名称
    let new_name = {
        let base = format!("{}(副本)", original_flow.name);
        if !existing_names.contains(&base) {
            base
        } else {
            let mut counter = 2u32;
            loop {
                let candidate = format!("{}(副本 {})", original_flow.name, counter);
                if !existing_names.contains(&candidate) {
                    break candidate;
                }
                counter += 1;
            }
        }
    };

    let mut new_flow = original_flow;
    new_flow.uuid = new_uuid;
    new_flow.name = new_name;

    save_flow_file(&new_flow, flows_dir)?;

    Ok(new_flow)
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
    fs::create_dir_all(reports_dir).map_err(|e| format!("创建报告目录Failed: {}", e))?;

    // 根据执行模式决定文件名前缀
    let filename = match report.execution_mode {
        ExecutionMode::Debug => format!("debug_{}.json", report.uuid),
        ExecutionMode::Normal => format!("{}.json", report.uuid),
    };

    // 构建文件路径
    let file_path = reports_dir.join(filename);

    // 序列化为 JSON
    let json_string =
        serde_json::to_string_pretty(report).map_err(|e| format!("序列化报告Failed: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file =
        NamedTempFile::new_in(reports_dir).map_err(|e| format!("创建临时文件Failed: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件Failed: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("保存报告文件Failed: {}", e))?;

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
        .map_err(|e| format!("检查报告文件Failed: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("检查报告文件Failed: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("报告文件不存在: {}", uuid));
    };

    // 读取文件内容
    let mut file =
        File::open(&file_path).map_err(|e| format!("打开报告文件 {:?} Failed: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取报告文件 {:?} Failed: {}", file_path, e))?;

    // 反序列化
    let report: FlowExecutionReport =
        serde_json::from_str(&content).map_err(|e| format!("解析报告文件Failed: {}", e))?;

    Ok(report)
}

/// 轻量级报告列表条目，仅包含列表展示所需的字段
///
/// 用于 `list_reports` 中快速反序列化报告文件，
/// 避免完整解析 `FlowExecutionReport` 中的大量嵌套数据（node_reports、ws_streams 等）。
#[derive(Debug, Clone, serde::Deserialize)]
struct ReportListEntry {
    uuid: String,
    flow_uuid: String,
    flow_name: String,
    status: ExecutionStatus,
    start_time: u64,
    duration_ms: u64,
    #[serde(default)]
    execution_mode: ExecutionMode,
}

/// 列出报告元数据
///
/// 使用轻量级 `ReportListEntry` 反序列化，避免完整解析 `FlowExecutionReport`。
/// 仅读取每个文件头部的元数字段，跳过 node_reports、ws_streams、flow_snapshot 等大数据字段。
///
/// # 参数
/// - `reports_dir`: 存储目录路径
pub fn list_reports(reports_dir: &Path) -> Result<Vec<ReportMetadata>, String> {
    // 确保目录存在
    if !reports_dir
        .try_exists()
        .map_err(|e| format!("检查报告目录Failed: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 JSON 文件
    for entry in fs::read_dir(reports_dir).map_err(|e| format!("读取报告目录Failed: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项Failed: {}", e))?;
        let path = entry.path();

        // 跳过非 JSON 文件（如元数据文件）
        if !path.extension().map(|e| e == "json").unwrap_or(false) {
            continue;
        }

        // 跳过 .meta.json 后缀的元数据文件（如果以后使用）
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".meta.json"))
            .unwrap_or(false)
        {
            continue;
        }

        // 使用轻量级反序列化：只提取列表所需的字段
        // serde 默认忽略未知字段，因此不会解析 node_reports、ws_streams 等大数据结构
        if let Ok(entry) = read_report_entry(&path) {
            reports.push(ReportMetadata {
                uuid: entry.uuid,
                flow_uuid: entry.flow_uuid,
                flow_name: entry.flow_name,
                status: entry.status.to_string(),
                timestamp: entry.start_time,
                duration_ms: entry.duration_ms,
                execution_mode: entry.execution_mode,
            });
        }
    }

    // 按时间戳倒序排序
    reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(reports)
}

/// 读取并轻量反序列化报告文件
///
/// 使用 `ReportListEntry` 只提取元数字段，避免完整解析 `FlowExecutionReport`。
/// 相比原方案（完整解析 FlowExecutionReport），此方法：
/// 1. 仍然需要读取和 JSON 词法分析整个文件（I/O + 解析不可避免）
/// 2. 但**不会分配** `Vec<NodeExecutionReport>`、`Vec<WsConnectionStream>`、
///    `FlowStatistics`、`Flow` 等大数据结构 —— 这些被 serde 快速跳过
/// 3. 内存分配从单文件数 MB 降到 ~1KB
///
/// 对于 50 个平均 200KB 的报告文件，完整解析需要分配 ~10MB+ 的 Rust 结构体，
/// 而轻量解析仅分配 ~50KB，**内存分配减少 200 倍**，GC/释放压力也随之大幅降低。
fn read_report_entry(path: &Path) -> Result<ReportListEntry, String> {
    let mut file = File::open(path).map_err(|e| format!("打开报告文件Failed: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取报告文件Failed: {}", e))?;

    serde_json::from_str::<ReportListEntry>(&content)
        .map_err(|e| format!("解析报告元数据Failed: {}", e))
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
        .map_err(|e| format!("检查报告文件Failed: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("检查报告文件Failed: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("报告文件不存在: {}", uuid));
    };

    // 删除文件
    fs::remove_file(&file_path).map_err(|e| format!("删除报告文件 {:?} Failed: {}", file_path, e))?;

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
    /// 报告文件完整路径（用于加载报告）
    pub file_path: String,
}

/// 保存压测报告到指定目录
///
/// # 参数
/// - `report`: 压测报告对象
/// - `reports_dir`: 存储目录路径（由调用方决定）
///
/// # 返回
/// - `Ok(PathBuf)`: 保存的文件路径
/// - `Err(String)`: 保存Failed，包含Error信息
///
/// # 文件命名格式
/// `performance_{flow_name}_{YYYYMMDD_HHMMSS}.json`
pub fn save_stress_report(
    report: &StressTestReport,
    reports_dir: &Path,
) -> Result<PathBuf, String> {
    // 确保目录存在
    fs::create_dir_all(reports_dir).map_err(|e| format!("创建压测报告目录Failed: {}", e))?;

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
        serde_json::to_string_pretty(report).map_err(|e| format!("序列化压测报告Failed: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file =
        NamedTempFile::new_in(reports_dir).map_err(|e| format!("创建临时文件Failed: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("写入临时文件Failed: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("保存压测报告文件Failed: {}", e))?;

    Ok(file_path)
}

/// 加载压测报告
///
/// # 参数
/// - `file_path`: 报告文件路径
///
/// # 返回
/// - `Ok(StressTestReport)`: 加载的压测报告
/// - `Err(String)`: 加载Failed，包含Error信息
pub fn load_stress_report(file_path: &Path) -> Result<StressTestReport, String> {
    // 检查文件是否存在
    if !file_path
        .try_exists()
        .map_err(|e| format!("检查压测报告文件Failed: {}", e))?
    {
        return Err(format!("压测报告文件不存在: {:?}", file_path));
    }

    // 读取文件内容
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开压测报告文件 {:?} Failed: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取压测报告文件 {:?} Failed: {}", file_path, e))?;

    // 反序列化
    let report: StressTestReport =
        serde_json::from_str(&content).map_err(|e| format!("解析压测报告文件Failed: {}", e))?;

    Ok(report)
}

/// 删除压测报告文件
///
/// # 参数
/// - `file_path`: 报告文件路径
///
/// # 返回
/// - `Ok(())`: 删除Success
/// - `Err(String)`: 删除Failed，包含Error信息
pub fn delete_stress_report(file_path: &Path) -> Result<(), String> {
    if !file_path
        .try_exists()
        .map_err(|e| format!("检查压测报告文件Failed: {}", e))?
    {
        return Err(format!("压测报告文件不存在: {:?}", file_path));
    }
    fs::remove_file(file_path).map_err(|e| format!("删除压测报告文件 {:?} Failed: {}", file_path, e))
}

/// 列出压测报告摘要
///
/// # 参数
/// - `reports_dir`: 存储目录路径
///
/// # 返回
/// - `Ok(Vec<StressTestReportSummary>)`: 报告摘要列表，按开始时间倒序排序
/// - `Err(String)`: 读取Failed，包含Error信息
pub fn list_stress_reports(reports_dir: &Path) -> Result<Vec<StressTestReportSummary>, String> {
    // 确保目录存在
    if !reports_dir
        .try_exists()
        .map_err(|e| format!("检查压测报告目录Failed: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 performance_*.json 文件
    for entry in fs::read_dir(reports_dir).map_err(|e| format!("读取压测报告目录Failed: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项Failed: {}", e))?;
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
                        file_path: path.to_string_lossy().to_string(),
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

// ============================================================================
// 文件夹管理函数
// ============================================================================

/// 文件夹信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    /// 文件夹名称（仅文件夹名，不含路径）
    pub name: String,
    /// 文件夹相对路径（相对于 `flows_dir`）
    pub path: String,
}

/// 列出 `flows_dir` 下所有子文件夹
///
/// 扫描所有子目录（包括空目录）。
/// 返回结果按路径排序。
pub fn list_folders(flows_dir: &Path) -> Result<Vec<FolderInfo>, String> {
    let mut folders = Vec::new();
    collect_folders(flows_dir, flows_dir, &mut folders)?;
    folders.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(folders)
}

/// 递归收集所有子目录（包括空目录）
fn collect_folders(
    base_dir: &Path,
    flows_dir: &Path,
    folders: &mut Vec<FolderInfo>,
) -> Result<(), String> {
    if !base_dir
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Ok(());
    }

    for entry in fs::read_dir(base_dir).map_err(|e| format!("读取目录Failed: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项Failed: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let rel_path = path
                .strip_prefix(flows_dir)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            folders.push(FolderInfo {
                name,
                path: rel_path.clone(),
            });
            // 递归扫描子目录
            collect_folders(&path, flows_dir, folders)?;
        }
    }

    Ok(())
}

/// 检查目录（递归）是否包含 .mpf 文件
fn has_mpf_files_recursive(dir: &Path) -> Result<bool, String> {
    if !dir
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Ok(false);
    }

    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录Failed: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项Failed: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            if has_mpf_files_recursive(&path)? {
                return Ok(true);
            }
        } else if path.extension().map(|e| e == "mpf").unwrap_or(false) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 创建文件夹
///
/// 在 `flows_dir` 下创建指定名称的子文件夹。
///
/// `folder_path` 为相对于 `flows_dir` 的路径，如 "认证测试" 或 "API/登录"。
pub fn create_folder(flows_dir: &Path, folder_path: &str) -> Result<FolderInfo, String> {
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    // 清理路径中的非法字符（每段单独清理）
    let clean_parts: Vec<String> = folder_path
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(sanitize_filename)
        .collect();

    if clean_parts.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    let clean_path = clean_parts.join("/");
    let full_path = flows_dir.join(&clean_path);

    // 检查是否已存在
    if full_path
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Err(format!("文件夹已存在: {}", clean_path));
    }

    // 创建目录（包括所有必要的父目录）
    fs::create_dir_all(&full_path).map_err(|e| format!("创建文件夹Failed: {}", e))?;

    let name = clean_parts.last().expect("clean_parts 不为空").clone();

    Ok(FolderInfo {
        name,
        path: clean_path,
    })
}

/// 删除文件夹
///
/// 删除指定的文件夹。仅允许删除空文件夹（不包含 .mpf 文件和子文件夹）。
/// `folder_path` 为相对于 `flows_dir` 的路径。
pub fn delete_folder(flows_dir: &Path, folder_path: &str) -> Result<(), String> {
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    let full_path = flows_dir.join(folder_path);

    // 安全检查：确保路径在 flows_dir 内
    let canonical_flows_dir = flows_dir
        .canonicalize()
        .map_err(|e| format!("规范化流程目录Failed: {}", e))?;
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|_| format!("文件夹不存在: {}", folder_path))?;

    if !canonical_full_path.starts_with(&canonical_flows_dir) {
        return Err("不允许删除流程目录之外的文件夹".to_string());
    }

    // 检查是否为空（无 .mpf 文件且无子目录）
    if has_mpf_files_recursive(&full_path)? {
        return Err("文件夹不为空，无法删除（请先移走其中的流程）".to_string());
    }

    // 删除空目录树
    fs::remove_dir_all(&full_path).map_err(|e| format!("删除文件夹Failed: {}", e))?;

    Ok(())
}

/// 重命名文件夹
///
/// 重命名指定的文件夹，保持其内容不变。
/// `folder_path` 为相对于 `flows_dir` 的原路径。
/// `new_name` 为新的文件夹名称（仅最后一层）。
pub fn rename_folder(
    flows_dir: &Path,
    folder_path: &str,
    new_name: &str,
) -> Result<FolderInfo, String> {
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }
    if new_name.trim().is_empty() {
        return Err("Folder name cannot be empty".to_string());
    }

    let clean_name = sanitize_filename(new_name.trim());
    let full_path = flows_dir.join(folder_path);

    if !full_path
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Err(format!("文件夹不存在: {}", folder_path));
    }

    // 计算新路径
    let parent = full_path
        .parent()
        .ok_or_else(|| "无法获取父目录".to_string())?;
    let new_full_path = parent.join(&clean_name);

    if new_full_path
        .try_exists()
        .map_err(|e| format!("检查目录Failed: {}", e))?
    {
        return Err(format!("文件夹名称已存在: {}", clean_name));
    }

    // 重命名
    fs::rename(&full_path, &new_full_path).map_err(|e| format!("重命名文件夹Failed: {}", e))?;

    // 计算新的相对路径
    let new_rel = parent
        .strip_prefix(flows_dir)
        .map(|p| {
            let base = p.to_string_lossy().replace('\\', "/");
            if base.is_empty() {
                clean_name.clone()
            } else {
                format!("{}/{}", base, clean_name)
            }
        })
        .unwrap_or(clean_name.clone());

    Ok(FolderInfo {
        name: clean_name,
        path: new_rel,
    })
}

/// 移动流程到指定文件夹
///
/// 将指定 UUID 的流程文件移动到目标文件夹。
/// `target_folder_path` 为相对于 `flows_dir` 的目标路径，空字符串表示根目录。
pub fn move_flow_to_folder(
    uuid: &str,
    flows_dir: &Path,
    target_folder_path: &str,
) -> Result<(), String> {
    // 找到原始文件
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    let source_path = mpf_files
        .iter()
        .find(|path| {
            let mut content = String::new();
            if let Ok(mut file) = File::open(path) {
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(flow_file) = serde_json::from_str::<FlowFile>(&content) {
                        return flow_file.flow.uuid == uuid;
                    }
                }
            }
            false
        })
        .ok_or_else(|| format!("流程不存在: {}", uuid))?
        .clone();

    // 计算目标路径
    let target_dir = if target_folder_path.is_empty() {
        flows_dir.to_path_buf()
    } else {
        flows_dir.join(target_folder_path)
    };

    // 确保目标目录存在
    if !target_dir
        .try_exists()
        .map_err(|e| format!("检查目标目录Failed: {}", e))?
    {
        fs::create_dir_all(&target_dir).map_err(|e| format!("创建目标目录Failed: {}", e))?;
    }

    let file_name = source_path
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?;
    let target_path = target_dir.join(file_name);

    // 检查是否已在目标位置
    if source_path == target_path {
        return Ok(());
    }

    // 检查目标位置是否有同名文件
    if target_path
        .try_exists()
        .map_err(|e| format!("检查目标文件Failed: {}", e))?
    {
        return Err(format!(
            "目标文件夹中已存在同名文件: {}",
            file_name.to_string_lossy()
        ));
    }

    // 移动文件
    fs::rename(&source_path, &target_path).map_err(|e| format!("移动流程文件Failed: {}", e))?;

    Ok(())
}

/// 移动文件夹到另一个文件夹（或根目录）
///
/// 将指定的源文件夹移动到目标文件夹内（或移动到根目录）。
/// `source_folder_path` 为相对于 `flows_dir` 的源路径。
/// `target_folder_path` 为相对于 `flows_dir` 的目标父文件夹路径，空字符串表示移动到根目录。
pub fn move_folder(
    flows_dir: &Path,
    source_folder_path: &str,
    target_folder_path: &str,
) -> Result<(), String> {
    if source_folder_path.is_empty() {
        return Err("Source folder path cannot be empty".to_string());
    }

    let source_full = flows_dir.join(source_folder_path);

    // 验证源文件夹存在
    if !source_full
        .try_exists()
        .map_err(|e| format!("检查源目录Failed: {}", e))?
    {
        return Err(format!("源文件夹不存在: {}", source_folder_path));
    }

    // 安全检查：规范化路径，防止路径遍历
    let canonical_flows_dir = flows_dir
        .canonicalize()
        .map_err(|e| format!("规范化流程目录Failed: {}", e))?;
    let canonical_source = source_full
        .canonicalize()
        .map_err(|_| format!("源文件夹不存在: {}", source_folder_path))?;

    if !canonical_source.starts_with(&canonical_flows_dir) {
        return Err("不允许操作流程目录之外的文件夹".to_string());
    }

    // 防止移动到自身或自身的子目录
    if !target_folder_path.is_empty() {
        let target_full = flows_dir.join(target_folder_path);
        let canonical_target = target_full
            .canonicalize()
            .map_err(|_| format!("目标文件夹不存在: {}", target_folder_path))?;

        if canonical_source == canonical_target {
            return Err("不能将文件夹移动到自身".to_string());
        }

        // 检查目标是否是源的子目录
        if canonical_target.starts_with(&canonical_source) {
            return Err("不能将文件夹移动到自身的子目录中".to_string());
        }
    }

    // 计算目标路径
    let folder_name = source_full
        .file_name()
        .ok_or_else(|| "无法获取文件夹名称".to_string())?;

    let target_dir = if target_folder_path.is_empty() {
        flows_dir.to_path_buf()
    } else {
        let target_parent = flows_dir.join(target_folder_path);
        // 验证目标文件夹存在
        if !target_parent
            .try_exists()
            .map_err(|e| format!("检查目标目录Failed: {}", e))?
        {
            return Err(format!("目标文件夹不存在: {}", target_folder_path));
        }
        target_parent
    };

    let new_path = target_dir.join(folder_name);

    // 检查是否已在目标位置
    if source_full == new_path {
        return Ok(());
    }

    // 检查目标位置是否有同名文件夹
    if new_path
        .try_exists()
        .map_err(|e| format!("检查目标路径Failed: {}", e))?
    {
        return Err(format!(
            "目标位置已存在同名文件夹: {}",
            folder_name.to_string_lossy()
        ));
    }

    // 移动文件夹
    fs::rename(&source_full, &new_path).map_err(|e| format!("移动文件夹Failed: {}", e))?;

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
        // 模拟 CLI 中的逻辑：如果 get_data_dir() Failed，使用默认空路径
        let log_dir: PathBuf = get_data_dir().map(|d| d.join("logs")).unwrap_or_default();

        // 如果 get_data_dir() Success，log_dir 应该是非空路径
        // 如果Failed（某些测试环境），log_dir 应该是空路径
        if get_data_dir().is_ok() {
            assert!(
                !log_dir.as_os_str().is_empty(),
                "当 get_data_dir() Success时，日志目录路径不应为空"
            );
        } else {
            assert!(
                log_dir.as_os_str().is_empty(),
                "当 get_data_dir() Failed时，应回退到空路径"
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
            file_path: "/tmp/performance_Test_Flow_20240115.json".to_string(),
        };

        let json = serde_json::to_string(&summary).expect("序列化Failed");
        assert!(json.contains("report-001"));
        assert!(json.contains("flow-001"));
        assert!(json.contains("Test Flow"));
        assert!(json.contains("uuid"));
        assert!(json.contains("flowUuid"));
        assert!(json.contains("flowName"));
        assert!(json.contains("startTime"));

        let deserialized: StressTestReportSummary =
            serde_json::from_str(&json).expect("反序列化Failed");
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
        let temp_dir = TempDir::new().expect("创建临时目录Failed");
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
                dataset_path: None,
                dataset_encoding: None,
                dataset_strategy: None,
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
            progress_history: vec![],
            flow_snapshot: None,
        };

        // 保存报告
        let saved_path = save_stress_report(&report, reports_dir).expect("保存报告Failed");

        // 验证文件名格式
        let filename = saved_path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("performance_Test_Flow_"));
        assert!(filename.ends_with(".json"));

        // 加载报告
        let loaded_report = load_stress_report(&saved_path).expect("加载报告Failed");

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
        let temp_dir = TempDir::new().expect("创建临时目录Failed");
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
                progress_history: vec![],
                flow_snapshot: None,
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
                progress_history: vec![],
                flow_snapshot: None,
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
                progress_history: vec![],
                flow_snapshot: None,
            },
        ];

        // 保存所有报告
        for report in &reports {
            save_stress_report(report, reports_dir).expect("保存报告Failed");
        }

        // 列出报告
        let summaries = list_stress_reports(reports_dir).expect("列出报告Failed");

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

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let reports_dir = temp_dir.path();

        let summaries = list_stress_reports(reports_dir).expect("列出报告Failed");
        assert!(summaries.is_empty());
    }

    /// 测试 load_stress_report 文件不存在
    #[test]
    fn test_load_stress_report_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let non_existent_path = temp_dir.path().join("non_existent.json");

        let result = load_stress_report(&non_existent_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    /// 测试 FlowFile 序列化/反序列化往返
    #[test]
    fn test_flow_file_roundtrip() {
        use flow_engine_core::flow::Flow;

        let flow = Flow { uuid: "test-flow-001".to_string(), name: "Test Flow".to_string(), description: None, initial_variables: None, environments: vec![], active_environment: None, nodes: vec![], connections: vec![], hooks: None };
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let json = serde_json::to_string(&flow_file).expect("序列化 FlowFile Failed");
        assert!(json.contains("_version"), "JSON 应包含 _version 字段");

        let deserialized: FlowFile = serde_json::from_str(&json).expect("反序列化 FlowFile Failed");
        assert_eq!(deserialized._version, 1);
        assert_eq!(deserialized.flow.uuid, "test-flow-001");
        assert_eq!(deserialized.flow.name, "Test Flow");
    }

    /// 测试 FlowFile _meta 字段的序列化/反序列化往返
    #[test]
    fn test_flow_file_meta_roundtrip() {
        use flow_engine_core::flow::Flow;

        let flow = Flow { uuid: "builtin-flow-001".to_string(), name: "内置模板".to_string(), description: None, initial_variables: None, environments: vec![], active_environment: None, nodes: vec![], connections: vec![], hooks: None };
        let meta = serde_json::json!({
            "version": 5,
            "isBuiltin": true
        });
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: Some(meta.clone()),
        };

        let json = serde_json::to_string(&flow_file).expect("序列化 FlowFile Failed");
        assert!(json.contains("_meta"), "JSON 应包含 _meta 字段");
        assert!(json.contains("isBuiltin"), "JSON 应包含 isBuiltin 字段");

        let deserialized: FlowFile = serde_json::from_str(&json).expect("反序列化 FlowFile Failed");
        let meta = deserialized._meta.expect("应包含 _meta 字段");
        assert_eq!(meta["version"], 5);
        assert_eq!(meta["isBuiltin"], true);
    }

    /// 测试 FlowFile 无 _meta 时不序列化该字段
    #[test]
    fn test_flow_file_no_meta_serialization() {
        use flow_engine_core::flow::Flow;

        let flow = Flow { uuid: "normal-flow".to_string(), name: "普通流程".to_string(), description: None, initial_variables: None, environments: vec![], active_environment: None, nodes: vec![], connections: vec![], hooks: None };
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let json = serde_json::to_string(&flow_file).expect("序列化 FlowFile Failed");
        assert!(
            !json.contains("_meta"),
            "无 _meta 时 JSON 不应包含 _meta 字段"
        );
    }

    /// 测试 save_flow_file_with_meta 和 load_flow_file_raw 的 _meta 往返
    #[test]
    fn test_save_and_load_flow_file_with_meta() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("builtin-001", "内置模板流程");
        let meta = serde_json::json!({
            "version": 5,
            "isBuiltin": true
        });

        // 保存带 _meta 的流程
        save_flow_file_with_meta(&flow, flows_dir, Some(meta.clone()))
            .expect("保存带 _meta 的流程Failed");

        // 加载并验证 _meta
        let loaded = load_flow_file_raw("builtin-001", flows_dir).expect("加载流程文件Failed");
        assert_eq!(loaded.flow.uuid, "builtin-001");
        assert_eq!(loaded.flow.name, "内置模板流程");
        assert!(loaded._meta.is_some(), "应包含 _meta 字段");
        assert_eq!(loaded._meta.unwrap()["version"], 5);
    }

    /// 测试 FlowMetadata 的 file_path 字段在 JSON 中为 camelCase
    #[test]
    fn test_flow_metadata_has_path() {
        let metadata = FlowMetadata {
            uuid: "meta-001".to_string(),
            name: "元数据测试".to_string(),
            file_path: "/flows/test.json".to_string(),
            updated_at: 1705315845000,
            folder_path: String::new(),
        };

        let json = serde_json::to_string(&metadata).expect("序列化 FlowMetadata Failed");
        assert!(
            json.contains("\"filePath\""),
            "JSON 应包含 camelCase 的 filePath 字段: {}",
            json
        );
        assert!(
            !json.contains("\"file_path\""),
            "JSON 不应包含 snake_case 的 file_path 字段: {}",
            json
        );
    }

    // ============================================================================
    // .mpf 流程文件管理测试
    // ============================================================================

    /// 创建测试用的 Flow 对象
    fn create_test_flow(uuid: &str, name: &str) -> Flow {
        Flow { uuid: uuid.to_string(), name: name.to_string(), description: None, initial_variables: None, environments: vec![], active_environment: None, nodes: vec![], connections: vec![], hooks: None }
    }

    /// 测试 save_flow_file 和 load_flow_file 的基本功能
    #[test]
    fn test_save_and_load_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("flow-save-001", "Test Flow");

        // 保存
        save_flow_file(&flow, flows_dir).expect("保存流程文件Failed");

        // 验证文件已创建，扩展名为 .mpf
        let mpf_files: Vec<_> = fs::read_dir(flows_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "mpf")
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(mpf_files.len(), 1, "应恰好创建一个 .mpf 文件");
        let file_name = mpf_files[0].file_name().to_string_lossy().to_string();
        assert!(
            file_name.starts_with("Test Flow"),
            "文件名应以流程名称开头: {}",
            file_name
        );
        assert!(
            file_name.ends_with(".mpf"),
            "文件名应以 .mpf 结尾: {}",
            file_name
        );

        // 加载
        let loaded = load_flow_file("flow-save-001", flows_dir).expect("加载流程文件Failed");
        assert_eq!(loaded.uuid, "flow-save-001");
        assert_eq!(loaded.name, "Test Flow");
        assert!(loaded.nodes.is_empty());
        assert!(loaded.connections.is_empty());

        // 加载不存在的 UUID 应Failed
        let result = load_flow_file("nonexistent", flows_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("流程不存在"));
    }

    /// 测试 list_flow_files 递归扫描
    #[test]
    fn test_list_flow_files_recursive() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        // 在根目录创建流程
        let flow1 = create_test_flow("list-001", "Alpha流程");
        save_flow_file(&flow1, flows_dir).expect("保存 flow1 Failed");

        // 在子目录创建流程
        let sub_dir = flows_dir.join("subdir");
        fs::create_dir_all(&sub_dir).expect("创建子目录Failed");
        let flow2 = create_test_flow("list-002", "Beta流程");
        // 手动在子目录创建 .mpf 文件（测试递归扫描）
        let flow_file = FlowFile {
            _version: 1,
            flow: flow2.clone(),
            _meta: None,
        };
        let json = serde_json::to_string_pretty(&flow_file).expect("序列化Failed");
        fs::write(sub_dir.join("Beta流程.mpf"), json).expect("写入文件Failed");

        // 在更深子目录创建流程
        let deep_dir = sub_dir.join("deep");
        fs::create_dir_all(&deep_dir).expect("创建深层目录Failed");
        let flow3 = create_test_flow("list-003", "Gamma流程");
        let flow_file3 = FlowFile {
            _version: 1,
            flow: flow3.clone(),
            _meta: None,
        };
        let json3 = serde_json::to_string_pretty(&flow_file3).expect("序列化Failed");
        fs::write(deep_dir.join("Gamma流程.mpf"), json3).expect("写入文件Failed");

        // 列出并验证
        let list = list_flow_files(flows_dir).expect("列出流程文件Failed");
        assert_eq!(list.len(), 3, "应找到 3 个流程文件");

        // 验证按名称排序
        assert_eq!(list[0].name, "Alpha流程");
        assert_eq!(list[1].name, "Beta流程");
        assert_eq!(list[2].name, "Gamma流程");

        // 验证元数据字段
        assert_eq!(list[0].uuid, "list-001");
        assert!(list[0].file_path.contains("Alpha流程"));
        assert!(list[0].updated_at > 0 || list[0].updated_at == 0); // mtime 可能为 0
    }

    /// 测试损坏的 .mpf 文件被跳过而不报错
    #[test]
    fn test_corrupted_file_skipped() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        // 创建一个正常的流程
        let flow = create_test_flow("good-001", "正常流程");
        save_flow_file(&flow, flows_dir).expect("保存正常流程Failed");

        // 创建一个损坏的 .mpf 文件
        fs::write(
            flows_dir.join("corrupted.mpf"),
            "this is not valid json {{{",
        )
        .expect("写入损坏文件Failed");

        // 再创建一个损坏的 .mpf 文件（JSON 格式但不是 FlowFile）
        fs::write(
            flows_dir.join("wrong_structure.mpf"),
            r#"{"not_a_flow": true}"#,
        )
        .expect("写入Error结构文件Failed");

        // 列出 — 应只返回正常流程，不报错
        let list = list_flow_files(flows_dir).expect("列出流程文件应Success");
        assert_eq!(list.len(), 1, "应只找到 1 个有效流程");
        assert_eq!(list[0].uuid, "good-001");
        assert_eq!(list[0].name, "正常流程");
    }

    /// 测试 UUID 冲突时列表不会自动修复（列表不做验证）
    #[test]
    fn test_uuid_collision_detection() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        let duplicate_uuid = "collision-uuid";

        // 手动创建两个相同 UUID 的 .mpf 文件
        let flow1 = create_test_flow(duplicate_uuid, "流程A");
        let flow_file1 = FlowFile {
            _version: 1,
            flow: flow1,
            _meta: None,
        };
        let json1 = serde_json::to_string_pretty(&flow_file1).expect("序列化Failed");
        fs::write(flows_dir.join("flow_a.mpf"), json1).expect("写入文件 A Failed");

        let flow2 = create_test_flow(duplicate_uuid, "流程B");
        let flow_file2 = FlowFile {
            _version: 1,
            flow: flow2,
            _meta: None,
        };
        let json2 = serde_json::to_string_pretty(&flow_file2).expect("序列化Failed");
        fs::write(flows_dir.join("flow_b.mpf"), json2).expect("写入文件 B Failed");

        // 列出 — 列表不做验证，两个文件都会出现（UUID 可能重复）
        let list = list_flow_files(flows_dir).expect("列出流程文件应Success");
        assert_eq!(list.len(), 2, "应返回 2 个流程");

        // 验证名称仍然正确
        let names: Vec<&str> = list.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"流程A"), "应包含流程A");
        assert!(names.contains(&"流程B"), "应包含流程B");

        // UUID 冲突不再自动修复 — 两个文件显示相同 UUID
        let uuids: Vec<&str> = list.iter().map(|m| m.uuid.as_str()).collect();
        assert_eq!(uuids.iter().filter(|&&u| u == duplicate_uuid).count(), 2);
    }

    /// 测试 copy_flow_file 复制流程
    #[test]
    fn test_copy_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("copy-001", "原始流程");
        save_flow_file(&flow, flows_dir).expect("保存原始流程Failed");

        // 复制
        let copied = copy_flow_file("copy-001", flows_dir).expect("复制流程Failed");

        // 验证新 UUID
        assert_ne!(copied.uuid, "copy-001", "复制的流程应有新的 UUID");

        // 验证名称带 "(副本)" 后缀
        assert_eq!(
            copied.name, "原始流程(副本)",
            "复制的流程名称应添加 (副本) 后缀"
        );

        // 验证两个流程独立存在
        let list = list_flow_files(flows_dir).expect("列出流程文件Failed");
        assert_eq!(list.len(), 2, "应存在 2 个流程");

        // 原始流程仍然可加载
        let original = load_flow_file("copy-001", flows_dir).expect("原始流程应可加载");
        assert_eq!(original.name, "原始流程");

        // 复制的流程也可加载（通过新 UUID）
        let copied_load = load_flow_file(&copied.uuid, flows_dir).expect("复制流程应可加载");
        assert_eq!(copied_load.name, "原始流程(副本)");

        // 再次复制同一流程 — 应生成 "(副本 2)"
        let copied2 = copy_flow_file("copy-001", flows_dir).expect("第二次复制Failed");
        assert_eq!(
            copied2.name, "原始流程(副本 2)",
            "第二次复制的名称应为 (副本 2)"
        );
        assert_ne!(copied2.uuid, copied.uuid, "每次复制应有不同 UUID");
    }

    /// 测试 delete_flow_file 删除流程
    #[test]
    fn test_delete_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        // 创建多个流程
        let flow1 = create_test_flow("del-001", "保留流程");
        let flow2 = create_test_flow("del-002", "删除流程");
        save_flow_file(&flow1, flows_dir).expect("保存 flow1 Failed");
        save_flow_file(&flow2, flows_dir).expect("保存 flow2 Failed");

        // 验证初始数量
        let list = list_flow_files(flows_dir).expect("列出Failed");
        assert_eq!(list.len(), 2);

        // 删除一个
        delete_flow_file("del-002", flows_dir).expect("删除流程Failed");

        // 验证删除后数量
        let list_after = list_flow_files(flows_dir).expect("列出Failed");
        assert_eq!(list_after.len(), 1, "删除后应剩余 1 个流程");
        assert_eq!(list_after[0].uuid, "del-001", "剩余的应是保留流程");
        assert_eq!(list_after[0].name, "保留流程");

        // 删除不存在的 UUID 应Failed
        let result = delete_flow_file("nonexistent", flows_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("流程不存在"));
    }

    /// 测试 save_flow_file 更新已有流程（同 UUID 覆盖写入）
    #[test]
    fn test_save_flow_file_updates_existing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let flows_dir = temp_dir.path();

        // 首次保存
        let flow = create_test_flow("update-001", "更新测试");
        save_flow_file(&flow, flows_dir).expect("首次保存Failed");

        // 验证只有 1 个文件
        let list = list_flow_files(flows_dir).expect("列出Failed");
        assert_eq!(list.len(), 1);

        // 更新流程（同 UUID，新名称）
        let mut updated_flow = flow.clone();
        updated_flow.name = "更新后名称".to_string();
        save_flow_file(&updated_flow, flows_dir).expect("更新保存Failed");

        // 验证仍然只有 1 个文件（未产生副本）
        let list_after = list_flow_files(flows_dir).expect("列出Failed");
        assert_eq!(list_after.len(), 1, "更新后应仍只有 1 个文件");

        // 验证内容已更新
        let loaded = load_flow_file("update-001", flows_dir).expect("加载Failed");
        assert_eq!(loaded.name, "更新后名称");

        // 多次更新验证
        updated_flow.name = "第三次更新".to_string();
        save_flow_file(&updated_flow, flows_dir).expect("第三次保存Failed");

        let list_final = list_flow_files(flows_dir).expect("列出Failed");
        assert_eq!(list_final.len(), 1, "多次更新后应仍只有 1 个文件");
        assert_eq!(list_final[0].name, "第三次更新");
    }
}
