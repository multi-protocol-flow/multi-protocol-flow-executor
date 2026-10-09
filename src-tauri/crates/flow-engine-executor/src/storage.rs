// 存储模块 - JSON 文件读写实现

use crate::stress_test::StressTestReport;
use flow_engine_core::flow::{EnvironmentSet, Flow};
use flow_engine_core::report::{
    ExecutionMode, ExecutionStatus, FlowExecutionReport, ReportMetadata,
};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::UNIX_EPOCH;
use tempfile::NamedTempFile;

/// 模块级流程元数据缓存：`(file_path)` -> `(mtime_ms, uuid, name)`
static FLOW_METADATA_CACHE: LazyLock<Mutex<HashMap<PathBuf, (u64, String, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 流程 UUID 到文件绝对路径的内存索引缓存：`uuid` -> `file_path`
static UUID_PATH_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 使得指定路径的流程元数据与 UUID 缓存失效
pub(crate) fn invalidate_flow_metadata_cache(path: &Path) {
    let mut cache = FLOW_METADATA_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    cache.remove(path);
    let mut uuid_cache = UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    uuid_cache.retain(|_, p| p != path);
}

/// 按 UUID 查找流程文件路径（优先使用内存索引与 mtime 缓存，极大加速冷热检索）
pub(crate) fn find_flow_path_by_uuid(uuid: &str, flows_dir: &Path) -> Result<PathBuf, String> {
    // 1. 优先查 UUID 缓存
    if let Some(cached_path) = UUID_PATH_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(uuid)
        .cloned()
    {
        if cached_path.is_file() {
            return Ok(cached_path);
        }
    }

    // 2. 扫描流程目录，利用 FLOW_METADATA_CACHE 的 mtime 减少磁盘读取与 JSON 反序列化
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    let mut found = None;
    for path in &mpf_files {
        let mtime = fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let cached_match = {
            let meta_cache = FLOW_METADATA_CACHE
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, cached_uuid, _)) = meta_cache.get(path) {
                if *cached_mtime == mtime {
                    Some(cached_uuid == uuid)
                } else {
                    None
                }
            } else {
                None
            }
        };

        match cached_match {
            Some(true) => {
                let mut uuid_cache = UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
                uuid_cache.insert(uuid.to_string(), path.clone());
                found = Some(path.clone());
                break;
            }
            Some(false) => continue,
            None => {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(meta) = deserialize_flow_file_meta(&content) {
                        let meta_uuid = meta.flow.uuid;
                        let meta_name = meta.flow.name;
                        {
                            let mut meta_cache = FLOW_METADATA_CACHE
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            meta_cache.insert(path.clone(), (mtime, meta_uuid.clone(), meta_name));
                        }
                        {
                            let mut uuid_cache =
                                UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
                            uuid_cache.insert(meta_uuid.clone(), path.clone());
                        }
                        if meta_uuid == uuid {
                            found = Some(path.clone());
                            break;
                        }
                    }
                }
            }
        }
    }

    found.ok_or_else(|| format!("Flow not found: {}", uuid))
}

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
    #[allow(dead_code)]
    name: String,
}

/// 流程文件序列化格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FlowFormat {
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
}

/// 探测流程文件内容格式（根据有效首字符是否为 '{' 进行区分）
pub fn detect_flow_format(content: &str) -> FlowFormat {
    if content.trim_start().starts_with('{') {
        FlowFormat::Json
    } else {
        FlowFormat::Yaml
    }
}

/// 从字符串统一反序列化为 FlowFile（自动适配 JSON 与 YAML）
pub fn deserialize_flow_file(content: &str) -> Result<FlowFile, String> {
    match detect_flow_format(content) {
        FlowFormat::Json => serde_json::from_str::<FlowFile>(content).map_err(|e| {
            format!(
                "Deserialize FlowFile Failed (JSON): {} (Flow file must use FlowFile wrapper format with _version and flow keys)",
                e
            )
        }),
        FlowFormat::Yaml => serde_yml::from_str::<FlowFile>(content).map_err(|e| {
            format!(
                "Deserialize FlowFile Failed (YAML): {} (Flow file must use FlowFile wrapper format with _version and flow keys)",
                e
            )
        }),
    }
}

/// 从字符串快速提取流程元数据（自动适配 JSON 与 YAML）
fn deserialize_flow_file_meta(content: &str) -> Result<FlowFileMeta, String> {
    match detect_flow_format(content) {
        FlowFormat::Json => serde_json::from_str::<FlowFileMeta>(content)
            .map_err(|e| format!("failed to parse flow metadata (JSON): {}", e)),
        FlowFormat::Yaml => serde_yml::from_str::<FlowFileMeta>(content)
            .map_err(|e| format!("failed to parse flow metadata (YAML): {}", e)),
    }
}

/// 序列化 `FlowFile` 为指定格式的文本（YAML 格式会自动对节点浮点坐标取整规范化）
pub fn serialize_flow_file(flow_file: &FlowFile, format: FlowFormat) -> Result<String, String> {
    match format {
        FlowFormat::Json => serde_json::to_string_pretty(flow_file)
            .map_err(|e| format!("failed to serialize flow (JSON): {}", e)),
        FlowFormat::Yaml => {
            let mut normalized = flow_file.clone();
            for node in &mut normalized.flow.nodes {
                node.position.x = node.position.x.round();
                node.position.y = node.position.y.round();
            }
            serde_yml::to_string(&normalized)
                .map_err(|e| format!("failed to serialize flow (YAML): {}", e))
        }
    }
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
            .map_err(|e| format!("failed to get APPDATA environment variable: {}", e))?
            .into()
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map_err(|e| format!("failed to get HOME environment variable: {}", e))
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))?
    } else {
        // Linux 和其他类 Unix 系统
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map_err(|e| format!("failed to get HOME environment variable: {}", e))
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            })?
    };

    Ok(base_dir.join(app_name))
}

/// 获取 flows 子目录路径
///
/// 优先返回宿主登记的覆盖目录（见 [`set_flows_dir_override`]），未登记时回退到
/// 默认数据目录 `<data_dir>/flows`。
pub fn get_flows_dir() -> Result<PathBuf, String> {
    if let Some(dir) = flows_dir_override() {
        return Ok(dir);
    }
    Ok(get_data_dir()?.join("flows"))
}

/// 运行时流程目录覆盖
///
/// GUI 的流程目录可由设置项 `flowsDir` 指向任意目录（工作区/项目仓库），
/// 与默认的 `<data_dir>/flows` 不同。执行期按 ID 解析子流程、按相对路径读写流程
/// 文件都必须落在这个"当前生效目录"上，否则会出现「流程列表可见、运行时找不到」。
/// 宿主（Tauri 命令层 / CLI）在边界处登记该目录。
static FLOWS_DIR_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// 覆盖值的测试串行锁：进程级状态，避免用例间互相踩踏
#[cfg(test)]
static FLOWS_DIR_OVERRIDE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// 读取当前登记的流程目录覆盖值（`None` 表示使用默认目录）
fn flows_dir_override() -> Option<PathBuf> {
    FLOWS_DIR_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// 登记当前生效的流程目录（`None` 恢复默认目录）
///
/// 由宿主在每次涉及流程存储/执行的入口处调用，保证执行期解析与前端列表使用同一目录。
pub fn set_flows_dir_override(dir: Option<PathBuf>) {
    *FLOWS_DIR_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = dir;
}

/// 全局环境保存目录覆盖：可由设置项 `environmentsDir` 指定独立路径，
/// 默认若未配置则跟随流程目录下的 `environments/` 子目录。
static ENVIRONMENTS_DIR_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> =
    LazyLock::new(|| Mutex::new(None));

/// 登记当前生效的环境目录（`None` 恢复默认跟随流程目录）
pub fn set_environments_dir_override(dir: Option<PathBuf>) {
    *ENVIRONMENTS_DIR_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = dir;
}

/// 获取当前生效的环境保存目录
///
/// 优先级：
/// 1. `ENVIRONMENTS_DIR_OVERRIDE`（若显式配置且存在）
/// 2. 默认跟随当前生效流程目录下的 `environments/` 子目录
pub fn get_environments_dir() -> Result<PathBuf, String> {
    if let Some(dir) = ENVIRONMENTS_DIR_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return Ok(dir);
    }
    Ok(get_flows_dir()?.join("environments"))
}

/// 确保 environments 子目录存在
pub fn ensure_environments_dir() -> Result<PathBuf, String> {
    let env_dir = get_environments_dir()?;
    if !env_dir.exists() {
        fs::create_dir_all(&env_dir)
            .map_err(|e| format!("Failed to create environments directory: {e}"))?;
    }
    Ok(env_dir)
}

/// 确保 flows 子目录存在
///
/// 如果目录不存在则创建，包括所有必要的父目录
pub fn ensure_flows_dir() -> Result<PathBuf, String> {
    let flows_dir = get_flows_dir()?;

    // 检查目录是否已存在（使用 try_exists 避免竞态条件）
    if !flows_dir
        .try_exists()
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        // 创建目录（包括所有必要的父目录）
        fs::create_dir_all(&flows_dir)
            .map_err(|e| format!("Failed to create flows directory: {}", e))?;
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
    let parent_dir = if let Some(parent) = file_path.parent() {
        if !parent
            .try_exists()
            .map_err(|e| format!("failed to check parent directory: {}", e))?
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent directory {:?}: {}", parent, e))?;
        }
        parent
    } else {
        Path::new(".")
    };

    // 包装为 FlowFile 并判定格式
    let flow_file = FlowFile {
        _version: 1,
        flow: flow.clone(),
        _meta: None,
    };
    let format = if file_path.exists() {
        if let Ok(content) = fs::read_to_string(&file_path) {
            detect_flow_format(&content)
        } else {
            FlowFormat::Yaml
        }
    } else {
        FlowFormat::Yaml
    };
    let output_string = serialize_flow_file(&flow_file, format)?;

    // 原子写入：在同目录下创建临时文件并持久化到目标路径
    let mut temp_file = NamedTempFile::new_in(parent_dir)
        .map_err(|e| format!("failed to create temporary file in {:?}: {}", parent_dir, e))?;

    temp_file
        .write_all(output_string.as_bytes())
        .map_err(|e| format!("failed to write temporary file: {}", e))?;

    temp_file
        .persist(&file_path)
        .map_err(|e| format!("failed to persist file {:?}: {}", file_path, e))?;

    invalidate_flow_metadata_cache(&file_path);

    Ok(())
}

/// 从文件加载流程（支持 JSON 与 YAML 双格式透明反序列化）
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
    } else if std::path::Path::new(path).exists() {
        PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path))
    } else {
        // 相对路径，使用 flows 目录
        get_flows_dir()?.join(path)
    };

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;

    let flow_file = deserialize_flow_file(&content)?;
    let flow = flow_file.flow;

    Ok(flow)
}

// === .mpf 流程文件管理 ===

/// 递归收集目录下所有 .mpf 文件路径
fn collect_mpf_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir
        .try_exists()
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|e| format!("failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
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
    save_flow_file_with_meta_and_path(flow, flows_dir, meta, None)
}

/// 保存流程到指定路径（若指定）或基于 flows_dir 自动定位/生成文件名
pub fn save_flow_file_with_meta_and_path(
    flow: &Flow,
    flows_dir: &Path,
    meta: Option<serde_json::Value>,
    target_file_path: Option<&Path>,
) -> Result<(), String> {
    // 确保目录存在
    fs::create_dir_all(flows_dir)
        .map_err(|e| format!("Failed to create flows directory: {}", e))?;

    // 包装为 FlowFile
    let flow_file = FlowFile {
        _version: 1,
        flow: flow.clone(),
        _meta: meta,
    };

    let (target_path, old_path_to_remove, existing_format) = if let Some(explicit_path) =
        target_file_path
    {
        let format = if explicit_path.exists() {
            fs::read_to_string(explicit_path)
                .ok()
                .map(|c| detect_flow_format(&c))
        } else {
            None
        };
        (explicit_path.to_path_buf(), None, format)
    } else {
        // 检查是否已存在同 UUID 的文件（更新场景，优先利用内存索引加速定位）
        let (existing_path, existing_format) = match find_flow_path_by_uuid(&flow.uuid, flows_dir) {
            Ok(path) => {
                let format = fs::read_to_string(&path)
                    .ok()
                    .map(|c| detect_flow_format(&c));
                (Some(path), format)
            }
            Err(_) => (None, None),
        };
        // 确定目标目录：保持原有文件夹位置，否则使用根目录
        let target_dir = existing_path
            .as_ref()
            .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| flows_dir.to_path_buf());

        // 确保目标目录存在
        if target_dir != flows_dir {
            fs::create_dir_all(&target_dir)
                .map_err(|e| format!("Failed to create target directory: {}", e))?;
        }

        // 生成文件名，处理冲突
        let safe_name = sanitize_filename(&flow.name);
        let mut filename = format!("{}.mpf", safe_name);
        let mut counter = 2u32;

        loop {
            let candidate_path = target_dir.join(&filename);
            // 如果目标路径就是旧文件路径（名称未变），直接跳出
            let is_old_path = existing_path.as_ref() == Some(&candidate_path);
            if is_old_path
                || !candidate_path
                    .try_exists()
                    .map_err(|e| format!("failed to check file: {}", e))?
            {
                break;
            }
            filename = format!("{}_{}.mpf", safe_name, counter);
            counter += 1;
        }

        let computed_path = target_dir.join(&filename);
        let to_remove = existing_path.filter(|p| *p != computed_path);
        (computed_path, to_remove, existing_format)
    };

    let format = existing_format.unwrap_or(FlowFormat::Yaml);
    let output_string = serialize_flow_file(&flow_file, format)?;

    let parent_dir = target_path.parent().unwrap_or(flows_dir);
    if !parent_dir.exists() {
        let _ = fs::create_dir_all(parent_dir);
    }

    // 原子写入：先写临时文件，再重命名
    let mut temp_file = NamedTempFile::new_in(parent_dir)
        .map_err(|e| format!("failed to create temporary file: {}", e))?;

    temp_file
        .write_all(output_string.as_bytes())
        .map_err(|e| format!("failed to write temporary file: {}", e))?;

    temp_file
        .persist(&target_path)
        .map_err(|e| format!("failed to save flow file: {}", e))?;

    // 如果文件名/位置变更，删除旧文件
    if let Some(ref old_path) = old_path_to_remove {
        let _ = fs::remove_file(old_path);
        invalidate_flow_metadata_cache(old_path);
    }
    invalidate_flow_metadata_cache(&target_path);
    {
        let mut uuid_cache = UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        uuid_cache.insert(flow.uuid.clone(), target_path.clone());
    }

    Ok(())
}

/// 从 .mpf 文件加载流程文件（支持按相对/绝对路径或 UUID 查找）
///
/// 优先按文件路径直接定位打开（O(1) 毫秒级）；若不存在则回退流式扫描所有 `.mpf` 匹配 UUID。
pub fn load_flow_file_with_path(
    uuid_or_path: &str,
    flows_dir: &Path,
) -> Result<(FlowFile, PathBuf), String> {
    // 1. 优先尝试直接作为相对路径（基于 flows_dir）或绝对路径解析
    let candidate_path = {
        let relative = flows_dir.join(uuid_or_path);
        if relative.is_file() {
            Some(relative)
        } else {
            let direct = PathBuf::from(uuid_or_path);
            if direct.is_file() {
                Some(direct)
            } else {
                None
            }
        }
    };

    let target_path = match candidate_path {
        Some(p) => p,
        None => find_flow_path_by_uuid(uuid_or_path, flows_dir)?,
    };

    let content =
        fs::read_to_string(&target_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let flow_file = deserialize_flow_file(&content)
        .map_err(|e| format!("Flow file corrupted: {} - {}", target_path.display(), e))?;

    {
        let mut uuid_cache = UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        uuid_cache.insert(flow_file.flow.uuid.clone(), target_path.clone());
    }
    Ok((flow_file, target_path))
}

/// 从 .mpf 文件加载流程文件（按路径或 UUID 查找）
pub fn load_flow_file_raw(uuid: &str, flows_dir: &Path) -> Result<FlowFile, String> {
    load_flow_file_with_path(uuid, flows_dir).map(|(ff, _)| ff)
}

/// 从 .mpf 文件加载流程（按路径或 UUID 查找），仅返回 Flow 部分
pub fn load_flow_file(uuid: &str, flows_dir: &Path) -> Result<Flow, String> {
    let flow_file = load_flow_file_raw(uuid, flows_dir)?;
    Ok(flow_file.flow)
}

/// 列出所有 .mpf 流程文件的元数据（纯文件系统元数据，不读文件内容）
///
/// 极速扫描：仅检查 `.mpf` 扩展名与文件属性，直接以文件名（去掉 `.mpf`）作为展示名称，
/// 不进行任何 JSON 反序列化与流程内容检查。损坏的文件在打开时再由校验器提示。
/// 结果按文件夹路径排序，同文件夹内按名称排序。
pub fn list_flow_files(flows_dir: &Path) -> Result<Vec<FlowMetadata>, String> {
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    let mut metadata_list = Vec::new();

    for path in &mpf_files {
        let updated_at = fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let name = file_stem.to_string();

        let folder_path = path
            .parent()
            .and_then(|parent| parent.strip_prefix(flows_dir).ok())
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            std::path::absolute(path).unwrap_or_else(|_| path.clone())
        };

        // 统一使用相对路径作为列表项稳定唯一标识 (例如 "folder/demo.mpf" 或 "demo.mpf")
        let rel_path = path
            .strip_prefix(flows_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| abs_path.to_string_lossy().replace('\\', "/"));

        metadata_list.push(FlowMetadata {
            uuid: rel_path,
            name,
            file_path: abs_path.to_string_lossy().to_string(),
            updated_at,
            folder_path,
        });
    }

    metadata_list.sort_by(|a, b| {
        a.folder_path
            .cmp(&b.folder_path)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(metadata_list)
}

/// 删除指定路径或 UUID 的流程文件
pub fn delete_flow_file(uuid_or_path: &str, flows_dir: &Path) -> Result<(), String> {
    let relative = flows_dir.join(uuid_or_path);
    if relative.is_file() {
        fs::remove_file(&relative)
            .map_err(|e| format!("Failed to delete flow file {:?}: {}", relative, e))?;
        invalidate_flow_metadata_cache(&relative);
        return Ok(());
    }

    let direct = PathBuf::from(uuid_or_path);
    if direct.is_file() {
        fs::remove_file(&direct)
            .map_err(|e| format!("Failed to delete flow file {:?}: {}", direct, e))?;
        invalidate_flow_metadata_cache(&direct);
        return Ok(());
    }

    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;

    for path in mpf_files {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(meta) = deserialize_flow_file_meta(&content) {
                if meta.flow.uuid == uuid_or_path {
                    fs::remove_file(&path)
                        .map_err(|e| format!("Failed to delete flow file {:?}: {}", path, e))?;
                    invalidate_flow_metadata_cache(&path);
                    return Ok(());
                }
            }
        }
    }

    Err(format!("Flow not found: {}", uuid_or_path))
}

/// 全量重映射流程中的所有 Node 与 Connection UUID，并级联更新内部引用
pub fn remap_flow_uuids(flow: &mut Flow) {
    let mut id_map: HashMap<String, String> = HashMap::new();

    // 1. 为每个节点分配全新 UUID，建立旧 ID -> 新 ID 映射表
    for node in &mut flow.nodes {
        let old_id = node.uuid.clone();
        let new_id = uuid::Uuid::new_v4().to_string();
        id_map.insert(old_id, new_id.clone());
        node.uuid = new_id;
    }

    // 2. 为每条连线分配全新 ID，并重映射 source_node_uuid 和 target_node_uuid 节点引用
    for conn in &mut flow.connections {
        conn.id = uuid::Uuid::new_v4().to_string();
        if let Some(new_source) = id_map.get(&conn.source_node_uuid) {
            conn.source_node_uuid = new_source.clone();
        }
        if let Some(new_target) = id_map.get(&conn.target_node_uuid) {
            conn.target_node_uuid = new_target.clone();
        }
    }

    // 3. 级联重映射节点配置内部的跨节点引用（如 connection_id, connection_node_id, source_node_id, connection_uuid）
    for node in &mut flow.nodes {
        if let Ok(mut val) = serde_json::to_value(&node.data) {
            if let Some(obj) = val.as_object_mut() {
                let ref_keys = [
                    "connection_id",
                    "connection_node_id",
                    "source_node_id",
                    "connection_uuid",
                ];
                let mut changed = false;
                for key in ref_keys {
                    if let Some(serde_json::Value::String(ref_id)) = obj.get(key) {
                        if let Some(new_ref) = id_map.get(ref_id) {
                            obj.insert(key.to_string(), serde_json::Value::String(new_ref.clone()));
                            changed = true;
                        }
                    }
                }
                if changed {
                    if let Ok(updated_data) = serde_json::from_value(val) {
                        node.data = updated_data;
                    }
                }
            }
        }
    }
}

/// 复制流程文件（生成新 UUID 和副本名称）
pub fn copy_flow_file(uuid_or_path: &str, flows_dir: &Path) -> Result<Flow, String> {
    let original_flow = load_flow_file(uuid_or_path, flows_dir)?;

    let new_uuid = uuid::Uuid::new_v4().to_string();

    let mut existing_names = HashSet::new();
    let mut mpf_files = Vec::new();
    collect_mpf_files(flows_dir, &mut mpf_files)?;
    for path in &mpf_files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            existing_names.insert(stem.to_string());
        }
    }

    let base_name = &original_flow.name;
    let mut candidate_name = format!("{}(副本)", base_name);
    let mut counter = 2u32;
    while existing_names.contains(&candidate_name) {
        candidate_name = format!("{}(副本 {})", base_name, counter);
        counter += 1;
    }

    let mut copied_flow = original_flow.clone();
    copied_flow.uuid = new_uuid;
    copied_flow.name = candidate_name;
    remap_flow_uuids(&mut copied_flow);
    save_flow_file(&copied_flow, flows_dir)?;

    Ok(copied_flow)
}

/// 重命名流程文件
pub fn rename_flow_file(
    uuid_or_path: &str,
    new_name: &str,
    flows_dir: &Path,
) -> Result<FlowMetadata, String> {
    let (mut flow_file, target_path) = load_flow_file_with_path(uuid_or_path, flows_dir)?;

    let parent_dir = target_path.parent().unwrap_or(flows_dir);
    let safe_new_name = sanitize_filename(new_name);
    if safe_new_name.is_empty() {
        return Err("New flow name is invalid".to_string());
    }

    let new_file_path = parent_dir.join(format!("{}.mpf", safe_new_name));
    if new_file_path != target_path && new_file_path.exists() {
        return Err(format!(
            "File with name '{}.mpf' already exists",
            safe_new_name
        ));
    }

    let format = if target_path.exists() {
        if let Ok(content) = fs::read_to_string(&target_path) {
            detect_flow_format(&content)
        } else {
            FlowFormat::Yaml
        }
    } else {
        FlowFormat::Yaml
    };

    flow_file.flow.name = new_name.to_string();
    let output_string = serialize_flow_file(&flow_file, format)?;

    let mut temp_file = NamedTempFile::new_in(flows_dir)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    temp_file
        .write_all(output_string.as_bytes())
        .map_err(|e| format!("Failed to write temporary file: {}", e))?;
    temp_file
        .persist(&new_file_path)
        .map_err(|e| format!("Failed to persist flow file: {}", e))?;

    if new_file_path != target_path {
        let _ = fs::remove_file(&target_path);
        invalidate_flow_metadata_cache(&target_path);
    }
    invalidate_flow_metadata_cache(&new_file_path);

    let updated_at = fs::metadata(&new_file_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let folder_path = new_file_path
        .parent()
        .and_then(|parent| parent.strip_prefix(flows_dir).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();

    let abs_path = if new_file_path.is_absolute() {
        new_file_path.clone()
    } else {
        std::path::absolute(&new_file_path).unwrap_or_else(|_| new_file_path.clone())
    };

    let rel_path = new_file_path
        .strip_prefix(flows_dir)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| abs_path.to_string_lossy().replace('\\', "/"));

    Ok(FlowMetadata {
        uuid: rel_path,
        name: safe_new_name,
        file_path: abs_path.to_string_lossy().to_string(),
        updated_at,
        folder_path,
    })
}

// ============================================================================
// 环境文件存储函数 (Environment Storage)
// ============================================================================

/// 环境文件元数据
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentMetadata {
    /// 环境名称
    pub name: String,
    /// 绝对文件路径
    pub file_path: String,
    /// 定义的变量数量
    pub variable_count: usize,
    /// 是否存在本地私有覆盖文件（.local.yaml）
    pub has_local_override: bool,
    /// 文件最后修改时间戳（毫秒）
    pub updated_at: u64,
}

/// 清理环境名称中的非法字符
fn sanitize_env_name(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// 校验并清洗环境名称：空名（含纯空白）一律拒绝
fn checked_env_name(name: &str) -> Result<String, String> {
    let safe = sanitize_env_name(name);
    if safe.is_empty() {
        return Err("Environment name cannot be empty".to_string());
    }
    Ok(safe)
}

/// 探测并解析环境文件内容
///
/// 支持两种形态：
/// - 完整环境对象：`{ name, description?, variables: { ... } }`
/// - 纯键值映射：整张表即变量表（宽容兼容用户手写的最小 YAML）
///
/// 注意：必须先按通用值解析并判断真实形状。直接把内容反序列化成 `EnvironmentSet`
/// 会因为 serde 默认忽略未知字段，把 `name: x` + 若干标量的纯键值文件静默解析成
/// 「空变量集」，造成变量丢失。环境名始终以文件名为准，避免文件内容里的 name
/// 与磁盘文件名不一致（否则后续保存会写到另一个环境文件上）。
fn deserialize_environment_content(
    content: &str,
    default_name: &str,
) -> Result<EnvironmentSet, String> {
    let parsed = serde_json::from_str::<serde_json::Value>(content)
        .or_else(|_| serde_yml::from_str::<serde_json::Value>(content))
        .map_err(|e| format!("Failed to parse environment file for '{default_name}': {e}"))?;

    let serde_json::Value::Object(mut map) = parsed else {
        return Err(format!(
            "Failed to parse environment file for '{default_name}': expected a mapping of variables"
        ));
    };

    // 只有 variables 确实是映射时才按「完整环境对象」处理
    let is_full_env = map.get("variables").is_some_and(|v| v.is_object());
    if !is_full_env {
        return Ok(EnvironmentSet {
            name: default_name.to_string(),
            description: None,
            variables: map.into_iter().collect(),
        });
    }

    let description = map
        .get("description")
        .and_then(|d| d.as_str())
        .map(str::to_string);
    let variables = match map.remove("variables") {
        Some(serde_json::Value::Object(vars)) => vars.into_iter().collect(),
        _ => HashMap::new(),
    };

    Ok(EnvironmentSet {
        name: default_name.to_string(),
        description,
        variables,
    })
}

/// 在指定环境下查找对应环境的文件路径（优先 .yaml，备选 .yml）
fn find_environment_file_path(name: &str, is_local: bool, dir: &Path) -> Option<PathBuf> {
    let safe_name = sanitize_env_name(name);
    if is_local {
        let p1 = dir.join(format!("{safe_name}.local.yaml"));
        if p1.exists() {
            return Some(p1);
        }
        let p2 = dir.join(format!("{safe_name}.local.yml"));
        if p2.exists() {
            return Some(p2);
        }
    } else {
        let p1 = dir.join(format!("{safe_name}.yaml"));
        if p1.exists() {
            return Some(p1);
        }
        let p2 = dir.join(format!("{safe_name}.yml"));
        if p2.exists() {
            return Some(p2);
        }
    }
    None
}

/// 列出指定目录下的所有环境文件元数据
///
/// 同一环境若同时存在 `.yaml` 与 `.yml`，一律以 `.yaml` 为准
/// （与 `find_environment_file_path` 的加载优先级保持一致，避免列表展示的文件
/// 与实际加载/执行的文件不是同一个）。
pub fn list_environment_files(environments_dir: &Path) -> Result<Vec<EnvironmentMetadata>, String> {
    if !environments_dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(environments_dir).map_err(|e| {
        format!(
            "Failed to read environments directory {:?}: {}",
            environments_dir, e
        )
    })?;

    // 先收集环境名（read_dir 顺序不确定，需自行归一）
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        // 排除 *.local.yaml / *.local.yml，它们属于覆盖层而非独立环境
        if file_name.ends_with(".local.yaml") || file_name.ends_with(".local.yml") {
            continue;
        }
        let name = if let Some(base) = file_name.strip_suffix(".yaml") {
            base.to_string()
        } else if let Some(base) = file_name.strip_suffix(".yml") {
            base.to_string()
        } else {
            continue;
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }

    let mut result = Vec::with_capacity(names.len());
    for name in names {
        let Some(path) = find_environment_file_path(&name, false, environments_dir) else {
            continue;
        };

        let has_local_override =
            find_environment_file_path(&name, true, environments_dir).is_some();
        let updated_at = fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let variable_count = fs::read_to_string(&path)
            .ok()
            .and_then(|content| deserialize_environment_content(&content, &name).ok())
            .map(|env| env.variables.len())
            .unwrap_or(0);

        let abs_path = if path.is_absolute() {
            path
        } else {
            std::path::absolute(&path).unwrap_or_else(|_| path.clone())
        };

        result.push(EnvironmentMetadata {
            name,
            file_path: abs_path.to_string_lossy().to_string(),
            variable_count,
            has_local_override,
            updated_at,
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// 直接读取单个环境文件原始内容（不自动合并 local 覆盖）
pub fn load_environment_file_raw(
    name: &str,
    is_local: bool,
    environments_dir: &Path,
) -> Result<EnvironmentSet, String> {
    let safe_name = checked_env_name(name)?;
    let path =
        find_environment_file_path(&safe_name, is_local, environments_dir).ok_or_else(|| {
            format!(
                "Environment file '{}' (is_local={}) not found in {:?}",
                safe_name, is_local, environments_dir
            )
        })?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read environment file {:?}: {}", path, e))?;
    deserialize_environment_content(&content, &safe_name)
}

/// 加载环境配置，若存在对应的 `.local.yaml` 本地覆盖文件则自动增量合并
pub fn load_environment_file(
    name: &str,
    environments_dir: &Path,
) -> Result<EnvironmentSet, String> {
    let mut base_env = load_environment_file_raw(name, false, environments_dir)?;
    if let Ok(local_env) = load_environment_file_raw(name, true, environments_dir) {
        base_env.variables.extend(local_env.variables);
    }
    Ok(base_env)
}

/// 保存环境到 YAML 文件（原子写入，支持主环境与 local 覆盖文件）
pub fn save_environment_file(
    env: &EnvironmentSet,
    is_local: bool,
    environments_dir: &Path,
) -> Result<(), String> {
    let safe_name = checked_env_name(&env.name)?;

    if !environments_dir.exists() {
        fs::create_dir_all(environments_dir).map_err(|e| {
            format!(
                "Failed to create environments directory {:?}: {}",
                environments_dir, e
            )
        })?;
    }

    let file_name = if is_local {
        format!("{safe_name}.local.yaml")
    } else {
        format!("{safe_name}.yaml")
    };
    let target_path = environments_dir.join(file_name);

    let yaml_content = serde_yml::to_string(env)
        .map_err(|e| format!("Failed to serialize environment to YAML: {}", e))?;

    let mut temp_file = NamedTempFile::new_in(environments_dir).map_err(|e| {
        format!(
            "Failed to create temporary file in {:?}: {}",
            environments_dir, e
        )
    })?;
    temp_file
        .write_all(yaml_content.as_bytes())
        .map_err(|e| format!("Failed to write temporary file: {}", e))?;
    temp_file.persist(&target_path).map_err(|e| {
        format!(
            "Failed to persist environment file {:?}: {}",
            target_path, e
        )
    })?;

    Ok(())
}

/// 删除环境文件（同时删除对应的 `.local.yaml` 覆盖文件）
pub fn delete_environment_file(name: &str, environments_dir: &Path) -> Result<(), String> {
    let safe_name = checked_env_name(name)?;
    let mut deleted_any = false;

    for ext in &["yaml", "yml"] {
        for path in [
            environments_dir.join(format!("{safe_name}.{ext}")),
            environments_dir.join(format!("{safe_name}.local.{ext}")),
        ] {
            if !path.exists() {
                continue;
            }
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete environment file {:?}: {}", path, e))?;
            deleted_any = true;
        }
    }

    if !deleted_any {
        return Err(format!(
            "Environment '{name}' not found in {:?}",
            environments_dir
        ));
    }
    Ok(())
}

/// 仅删除环境的本地私有覆盖文件（`<name>.local.yaml`），保留主环境文件
///
/// 用于「取消本地覆盖」：清空开发机私有变量，但不影响团队共享的主环境定义。
pub fn delete_environment_local_override(
    name: &str,
    environments_dir: &Path,
) -> Result<(), String> {
    let safe_name = checked_env_name(name)?;
    let mut deleted_any = false;

    for ext in &["yaml", "yml"] {
        let local_path = environments_dir.join(format!("{safe_name}.local.{ext}"));
        if !local_path.exists() {
            continue;
        }
        fs::remove_file(&local_path)
            .map_err(|e| format!("Failed to delete local override {:?}: {}", local_path, e))?;
        deleted_any = true;
    }

    if !deleted_any {
        return Err(format!(
            "Local override for environment '{name}' not found in {:?}",
            environments_dir
        ));
    }
    Ok(())
}

/// 重命名环境（同时重命名关联的 .local 覆盖文件）
pub fn rename_environment_file(
    old_name: &str,
    new_name: &str,
    environments_dir: &Path,
) -> Result<EnvironmentMetadata, String> {
    // 两端都按同一套规则清洗，否则 " dev " 之类的旧名会因为比较失败而被误判为「已存在」
    let safe_old_name = checked_env_name(old_name)?;
    let safe_new_name = checked_env_name(new_name)?;

    if safe_new_name == safe_old_name {
        return list_environment_files(environments_dir)?
            .into_iter()
            .find(|m| m.name == safe_new_name)
            .ok_or_else(|| format!("Environment '{safe_old_name}' not found"));
    }

    if find_environment_file_path(&safe_new_name, false, environments_dir).is_some() {
        return Err(format!("Environment '{safe_new_name}' already exists"));
    }
    // 目标名已存在本地覆盖时同样必须拒绝：否则下面的写入会覆盖掉目标环境已有的私有配置
    if find_environment_file_path(&safe_new_name, true, environments_dir).is_some() {
        return Err(format!(
            "Local override for environment '{safe_new_name}' already exists"
        ));
    }

    let mut env = load_environment_file_raw(&safe_old_name, false, environments_dir)?;
    let local_env = load_environment_file_raw(&safe_old_name, true, environments_dir).ok();

    // 先写新文件、确认成功后再删旧文件，避免中途失败造成环境丢失
    env.name = safe_new_name.clone();
    save_environment_file(&env, false, environments_dir)?;

    let has_local = if let Some(mut local) = local_env {
        local.name = safe_new_name.clone();
        save_environment_file(&local, true, environments_dir)?;
        true
    } else {
        false
    };

    delete_environment_file(&safe_old_name, environments_dir)?;

    let target_path = environments_dir.join(format!("{safe_new_name}.yaml"));
    let updated_at = fs::metadata(&target_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let abs_path = if target_path.is_absolute() {
        target_path
    } else {
        std::path::absolute(&target_path).unwrap_or_else(|_| target_path.clone())
    };

    Ok(EnvironmentMetadata {
        name: safe_new_name,
        file_path: abs_path.to_string_lossy().to_string(),
        variable_count: env.variables.len(),
        has_local_override: has_local,
        updated_at,
    })
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
        .map_err(|e| format!("failed to create report directory: {}", e))?;

    // 根据执行模式决定文件名前缀
    let filename = match report.execution_mode {
        ExecutionMode::Debug => format!("debug_{}.json", report.uuid),
        ExecutionMode::Normal => format!("{}.json", report.uuid),
    };

    // 构建文件路径
    let file_path = reports_dir.join(filename);

    // 序列化为 JSON
    let json_string = serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to serialize report: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file = NamedTempFile::new_in(reports_dir)
        .map_err(|e| format!("failed to create temporary file: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("failed to write temporary file: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("failed to save report file: {}", e))?;

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
        .map_err(|e| format!("failed to check report file: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("failed to check report file: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("report file not found: {}", uuid));
    };

    // 读取文件内容
    let mut file = File::open(&file_path)
        .map_err(|e| format!("Failed to open report file {:?}: {}", file_path, e))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("failed to read report file {:?}: {}", file_path, e))?;

    // 反序列化
    let report: FlowExecutionReport = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse report file: {}", e))?;

    Ok(report)
}

/// 轻量级报告列表条目，仅包含列表展示所需的字段
///
/// 用于 `list_reports` 中快速反序列化报告文件，
/// 避免完整解析 `FlowExecutionReport` `中的大量嵌套数据（node_reports、ws_streams` 等）。
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
/// 仅读取每个文件头部的元数字段，跳过 `node_reports、ws_streams、flow_snapshot` 等大数据字段。
///
/// # 参数
/// - `reports_dir`: 存储目录路径
pub fn list_reports(reports_dir: &Path) -> Result<Vec<ReportMetadata>, String> {
    // 确保目录存在
    if !reports_dir
        .try_exists()
        .map_err(|e| format!("failed to check report directory: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 JSON 文件
    for entry in
        fs::read_dir(reports_dir).map_err(|e| format!("failed to read report directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
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
    let mut file = File::open(path).map_err(|e| format!("failed to open report file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("failed to read report file: {}", e))?;

    serde_json::from_str::<ReportListEntry>(&content)
        .map_err(|e| format!("failed to parse report metadata: {}", e))
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
        .map_err(|e| format!("failed to check report file: {}", e))?
    {
        normal_path
    } else if debug_path
        .try_exists()
        .map_err(|e| format!("failed to check report file: {}", e))?
    {
        debug_path
    } else {
        return Err(format!("report file not found: {}", uuid));
    };

    // 删除文件
    fs::remove_file(&file_path)
        .map_err(|e| format!("failed to delete report file {:?}: {}", file_path, e))?;

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
    fs::create_dir_all(reports_dir)
        .map_err(|e| format!("failed to create stress test report directory: {}", e))?;

    // 生成时间戳：YYYYMMDD_HHMMSS
    let timestamp = format_timestamp(report.start_time);

    // 清理流程名称中的非法文件名字符
    let safe_flow_name = sanitize_filename(&report.flow_name);

    // 构建文件名
    let filename = format!("performance_{}_{}.json", safe_flow_name, timestamp);

    // 构建文件路径
    let file_path = reports_dir.join(filename);

    // 序列化为 JSON
    let json_string = serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to serialize stress test report: {}", e))?;

    // 使用原子写入：先写临时文件，再重命名
    let mut temp_file = NamedTempFile::new_in(reports_dir)
        .map_err(|e| format!("failed to create temporary file: {}", e))?;

    // 写入内容到临时文件
    temp_file
        .write_all(json_string.as_bytes())
        .map_err(|e| format!("failed to write temporary file: {}", e))?;

    // 原子重命名到最终路径
    temp_file
        .persist(&file_path)
        .map_err(|e| format!("failed to save stress test report file: {}", e))?;

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
        .map_err(|e| format!("failed to check stress test report file: {}", e))?
    {
        return Err(format!(
            "stress test report file not found: {:?}",
            file_path
        ));
    }

    // 读取文件内容
    let mut file = File::open(file_path).map_err(|e| {
        format!(
            "failed to open stress test report file {:?}: {}",
            file_path, e
        )
    })?;

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| {
        format!(
            "failed to read stress test report file {:?}: {}",
            file_path, e
        )
    })?;

    // 反序列化
    let report: StressTestReport = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse stress test report file: {}", e))?;

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
        .map_err(|e| format!("failed to check stress test report file: {}", e))?
    {
        return Err(format!(
            "stress test report file not found: {:?}",
            file_path
        ));
    }
    fs::remove_file(file_path).map_err(|e| {
        format!(
            "failed to delete stress test report file {:?}: {}",
            file_path, e
        )
    })
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
        .map_err(|e| format!("failed to check stress test report directory: {}", e))?
    {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();

    // 读取目录中的所有 performance_*.json 文件
    for entry in fs::read_dir(reports_dir)
        .map_err(|e| format!("failed to read stress test report directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
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
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Ok(());
    }

    for entry in fs::read_dir(base_dir).map_err(|e| format!("failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
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
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Ok(false);
    }

    for entry in fs::read_dir(dir).map_err(|e| format!("failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
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
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Err(format!("folder already exists: {}", clean_path));
    }

    // 创建目录（包括所有必要的父目录）
    fs::create_dir_all(&full_path).map_err(|e| format!("failed to create folder: {}", e))?;

    let name = clean_parts
        .last()
        .ok_or_else(|| "Internal error: clean_parts should not be empty".to_string())?
        .clone();

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
        .map_err(|e| format!("failed to canonicalize flows directory: {}", e))?;
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|_| format!("folder does not exist: {}", folder_path))?;

    if !canonical_full_path.starts_with(&canonical_flows_dir) {
        return Err("cannot delete folder outside the flows directory".to_string());
    }

    // 检查是否为空（无 .mpf 文件且无子目录）
    if has_mpf_files_recursive(&full_path)? {
        return Err("folder is not empty, cannot delete (move flows out first)".to_string());
    }

    // 删除空目录树
    fs::remove_dir_all(&full_path).map_err(|e| format!("failed to delete folder: {}", e))?;

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
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Err(format!("folder does not exist: {}", folder_path));
    }

    // 计算新路径
    let parent = full_path
        .parent()
        .ok_or_else(|| "cannot get parent directory".to_string())?;
    let new_full_path = parent.join(&clean_name);

    if new_full_path
        .try_exists()
        .map_err(|e| format!("Failed to check directory: {}", e))?
    {
        return Err(format!("folder name already exists: {}", clean_name));
    }

    // 重命名
    fs::rename(&full_path, &new_full_path)
        .map_err(|e| format!("failed to rename folder: {}", e))?;

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
/// 将指定 UUID 或相对/绝对路径的流程文件移动到目标文件夹。
/// `target_folder_path` 为相对于 `flows_dir` 的目标路径，空字符串表示根目录。
pub fn move_flow_to_folder(
    uuid_or_path: &str,
    flows_dir: &Path,
    target_folder_path: &str,
) -> Result<(), String> {
    // 1. 尝试直接按路径定位（优先基于 flows_dir 的相对路径，其次绝对路径）
    let candidate_relative = flows_dir.join(uuid_or_path);
    let candidate_direct = Path::new(uuid_or_path);

    let source_path = if candidate_relative.is_file() {
        candidate_relative
    } else if candidate_direct.is_file() {
        candidate_direct.to_path_buf()
    } else {
        // 2. 回退：按流程内部 UUID 定位
        find_flow_path_by_uuid(uuid_or_path, flows_dir)?
    };

    // 计算目标路径（安全清理首尾斜杠，避免被 Path::join 解析为绝对根路径）
    let clean_target_folder = target_folder_path.trim_matches(['/', '\\']);
    let target_dir = if clean_target_folder.is_empty() {
        flows_dir.to_path_buf()
    } else {
        flows_dir.join(clean_target_folder)
    };

    // 确保目标目录存在
    if !target_dir
        .try_exists()
        .map_err(|e| format!("failed to check target directory: {}", e))?
    {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    let file_name = source_path
        .file_name()
        .ok_or_else(|| "cannot get file name".to_string())?;
    let target_path = target_dir.join(file_name);

    // 检查是否已在目标位置
    if source_path == target_path {
        return Ok(());
    }

    // 检查目标位置是否有同名文件
    if target_path
        .try_exists()
        .map_err(|e| format!("failed to check target file: {}", e))?
    {
        return Err(format!(
            "a file with the same name already exists in the target folder: {}",
            file_name.to_string_lossy()
        ));
    }

    // 移动文件（优先原子重命名，跨文件系统时回退至 copy + remove）
    if let Err(rename_err) = fs::rename(&source_path, &target_path) {
        fs::copy(&source_path, &target_path).map_err(|copy_err| {
            format!(
                "failed to move flow file: rename failed ({}), copy fallback failed ({})",
                rename_err, copy_err
            )
        })?;
        if let Err(rm_err) = fs::remove_file(&source_path) {
            let _ = fs::remove_file(&target_path);
            return Err(format!(
                "failed to remove source flow file after copy: {}",
                rm_err
            ));
        }
    }

    // 缓存失效与更新
    invalidate_flow_metadata_cache(&source_path);
    invalidate_flow_metadata_cache(&target_path);

    if let Ok(content) = fs::read_to_string(&target_path) {
        if let Ok(meta) = deserialize_flow_file_meta(&content) {
            let mut uuid_cache = UUID_PATH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            uuid_cache.insert(meta.flow.uuid, target_path);
        }
    }

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
        .map_err(|e| format!("failed to check source directory: {}", e))?
    {
        return Err(format!(
            "source folder does not exist: {}",
            source_folder_path
        ));
    }

    // 安全检查：规范化路径，防止路径遍历
    let canonical_flows_dir = flows_dir
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize flows directory: {}", e))?;
    let canonical_source = source_full
        .canonicalize()
        .map_err(|_| format!("source folder does not exist: {}", source_folder_path))?;

    if !canonical_source.starts_with(&canonical_flows_dir) {
        return Err("cannot operate on folders outside the flows directory".to_string());
    }

    // 防止移动到自身或自身的子目录
    if !target_folder_path.is_empty() {
        let target_full = flows_dir.join(target_folder_path);
        let canonical_target = target_full
            .canonicalize()
            .map_err(|_| format!("target folder does not exist: {}", target_folder_path))?;

        if canonical_source == canonical_target {
            return Err("cannot move a folder into itself".to_string());
        }

        // 检查目标是否是源的子目录
        if canonical_target.starts_with(&canonical_source) {
            return Err("cannot move a folder into its own subdirectory".to_string());
        }
    }

    // 计算目标路径
    let folder_name = source_full
        .file_name()
        .ok_or_else(|| "cannot get folder name".to_string())?;

    let target_dir = if target_folder_path.is_empty() {
        flows_dir.to_path_buf()
    } else {
        let target_parent = flows_dir.join(target_folder_path);
        // 验证目标文件夹存在
        if !target_parent
            .try_exists()
            .map_err(|e| format!("failed to check target directory: {}", e))?
        {
            return Err(format!(
                "target folder does not exist: {}",
                target_folder_path
            ));
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
        .map_err(|e| format!("failed to check target path: {}", e))?
    {
        return Err(format!(
            "a folder with the same name already exists at the target location: {}",
            folder_name.to_string_lossy()
        ));
    }

    // 移动文件夹
    fs::rename(&source_full, &new_path).map_err(|e| format!("failed to move folder: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 流程目录覆盖：登记后 `get_flows_dir()` 返回覆盖值，复位后回到默认目录
    ///
    /// 覆盖值是进程级状态，用例内单独串行访问该状态（见 `FLOWS_DIR_OVERRIDE_TEST_LOCK`）。
    #[test]
    fn test_flows_dir_override_switches_effective_dir() {
        let _guard = FLOWS_DIR_OVERRIDE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let default_dir = get_data_dir().map(|d| d.join("flows"));
        let workspace = PathBuf::from("/tmp/mpe-flows-dir-override-test");

        set_flows_dir_override(Some(workspace.clone()));
        assert_eq!(get_flows_dir().unwrap_or_else(|e| panic!("{e}")), workspace);

        set_flows_dir_override(None);
        if let Ok(default_dir) = default_dir {
            assert_eq!(
                get_flows_dir().unwrap_or_else(|e| panic!("{e}")),
                default_dir
            );
        }
    }

    /// 测试 `get_data_dir()` 返回正确的路径格式
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

    /// 测试 `get_data_dir()` 返回的路径是有效的目录路径
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

    /// 测试 `get_flows_dir()` 返回数据目录下的 flows 子目录
    #[test]
    fn test_get_flows_dir() {
        if let Ok(data_dir) = get_data_dir() {
            let flows_dir = get_flows_dir();

            assert!(flows_dir.is_ok(), "get_flows_dir() 应返回 Ok");

            let flows_path = flows_dir.unwrap_or_else(|e| panic!("{e}"));
            let expected = data_dir.join("flows");

            assert_eq!(
                flows_path, expected,
                "flows 目录应为数据目录下的 'flows' 子目录"
            );
        }
    }

    /// 测试 `get_reports_dir()` 返回数据目录下的 reports 子目录
    #[test]
    fn test_get_reports_dir() {
        if let Ok(data_dir) = get_data_dir() {
            let reports_dir = get_reports_dir();

            assert!(reports_dir.is_ok(), "get_reports_dir() 应返回 Ok");

            let reports_path = reports_dir.unwrap_or_else(|e| panic!("{e}"));
            let expected = data_dir.join("reports");

            assert_eq!(
                reports_path, expected,
                "reports 目录应为数据目录下的 'reports' 子目录"
            );
        }
    }

    /// 测试日志目录组合逻辑（模拟 CLI 中的用法）
    ///
    /// CLI 中使用: `get_data_dir().map(|d`| `d.join("logs")).unwrap_or_default()`
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
                let data_dir = get_data_dir().unwrap_or_else(|e| panic!("{e}"));
                assert_eq!(parent, &data_dir, "日志目录的父目录应为数据目录");
            }
        }
    }

    /// 测试默认值回退逻辑（模拟 CLI 中 `unwrap_or_default` 的行为）
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

    /// 测试 `ensure_flows_dir()` 创建目录
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

    /// 测试 `format_timestamp` 函数
    #[test]
    fn test_format_timestamp() {
        // 使用 chrono 验证时间戳转换
        use chrono::{TimeZone, Utc};

        // 创建一个已知的时间
        let datetime = Utc
            .with_ymd_and_hms(2024, 1, 15, 10, 30, 45)
            .single()
            .unwrap_or_else(|| panic!("expected Some"));
        let timestamp_ms = datetime.timestamp_millis() as u64;

        let formatted = format_timestamp(timestamp_ms);

        // 验证格式正确（YYYYMMDD_HHMMSS）
        assert!(formatted.starts_with("20240115_"));
        assert_eq!(formatted.len(), 15); // "YYYYMMDD_HHMMSS" = 15 字符

        // 验证格式化后的时间可以解析
        let year: i32 = formatted[0..4].parse().unwrap_or_else(|e| panic!("{e}"));
        let month: u32 = formatted[4..6].parse().unwrap_or_else(|e| panic!("{e}"));
        let day: u32 = formatted[6..8].parse().unwrap_or_else(|e| panic!("{e}"));
        let hour: u32 = formatted[9..11].parse().unwrap_or_else(|e| panic!("{e}"));
        let minute: u32 = formatted[11..13].parse().unwrap_or_else(|e| panic!("{e}"));
        let second: u32 = formatted[13..15].parse().unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(year, 2024);
        assert_eq!(month, 1);
        assert_eq!(day, 15);
        assert_eq!(hour, 10);
        assert_eq!(minute, 30);
        assert_eq!(second, 45);
    }

    /// 测试 `sanitize_filename` 函数
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

    /// 测试 `StressTestReportSummary` 序列化
    #[test]
    fn test_stress_test_report_summary_serialization() {
        let summary = StressTestReportSummary {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1705315845000,
            file_path: "/tmp/performance_Test_Flow_20240115.json".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("report-001"));
        assert!(json.contains("flow-001"));
        assert!(json.contains("Test Flow"));
        assert!(json.contains("uuid"));
        assert!(json.contains("flowUuid"));
        assert!(json.contains("flowName"));
        assert!(json.contains("startTime"));

        let deserialized: StressTestReportSummary =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized, summary);
    }

    /// 测试 `save_stress_report` 和 `load_stress_report`
    #[test]
    fn test_save_and_load_stress_report() {
        use crate::stress_test::{
            ResponseTimeDistribution, StressTestConfig, StressTestReport, StressTestStatistics,
        };
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
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
            lifecycle: None,
        };

        // 保存报告
        let saved_path = save_stress_report(&report, reports_dir)
            .unwrap_or_else(|e| panic!("保存报告Failed: {e}"));

        // 验证文件名格式
        let filename = saved_path
            .file_name()
            .unwrap_or_else(|| panic!("expected Some"))
            .to_str()
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(filename.starts_with("performance_Test_Flow_"));
        assert!(filename.ends_with(".json"));

        // 加载报告
        let loaded_report =
            load_stress_report(&saved_path).unwrap_or_else(|e| panic!("加载报告Failed: {e}"));

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

    /// 测试 `list_stress_reports`
    #[test]
    fn test_list_stress_reports() {
        use crate::stress_test::{
            ResponseTimeDistribution, StressTestConfig, StressTestReport, StressTestStatistics,
        };
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
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
                lifecycle: None,
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
                lifecycle: None,
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
                lifecycle: None,
            },
        ];

        // 保存所有报告
        for report in &reports {
            save_stress_report(report, reports_dir)
                .unwrap_or_else(|e| panic!("保存报告Failed: {e}"));
        }

        // 列出报告
        let summaries =
            list_stress_reports(reports_dir).unwrap_or_else(|e| panic!("列出报告Failed: {e}"));

        // 验证数量
        assert_eq!(summaries.len(), 3);

        // 验证按时间倒序排序（最新的在前）
        assert_eq!(summaries[0].uuid, "report-002"); // start_time: 1705315945000
        assert_eq!(summaries[1].uuid, "report-003"); // start_time: 1705315895000
        assert_eq!(summaries[2].uuid, "report-001"); // start_time: 1705315845000
    }

    /// 测试 `list_stress_reports` 空目录
    #[test]
    fn test_list_stress_reports_empty_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let reports_dir = temp_dir.path();

        let summaries =
            list_stress_reports(reports_dir).unwrap_or_else(|e| panic!("列出报告Failed: {e}"));
        assert!(summaries.is_empty());
    }

    /// 测试 `load_stress_report` 文件不存在
    #[test]
    fn test_load_stress_report_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let non_existent_path = temp_dir.path().join("non_existent.json");

        let result = load_stress_report(&non_existent_path);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("not found"));
    }

    /// 测试 `FlowFile` 序列化/反序列化往返
    #[test]
    fn test_flow_file_roundtrip() {
        use flow_engine_core::flow::Flow;

        let flow = Flow {
            uuid: "test-flow-001".to_string(),
            name: "Test Flow".to_string(),
            description: None,
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
            setup: None,
            teardown: None,
        };
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let json = serde_json::to_string(&flow_file)
            .unwrap_or_else(|e| panic!("序列化 FlowFile Failed: {e}"));
        assert!(json.contains("_version"), "JSON 应包含 _version 字段");

        let deserialized: FlowFile =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化 FlowFile Failed: {e}"));
        assert_eq!(deserialized._version, 1);
        assert_eq!(deserialized.flow.uuid, "test-flow-001");
        assert_eq!(deserialized.flow.name, "Test Flow");
    }

    /// 测试 `FlowFile` _meta 字段的序列化/反序列化往返
    #[test]
    fn test_flow_file_meta_roundtrip() {
        use flow_engine_core::flow::Flow;

        let flow = Flow {
            uuid: "builtin-flow-001".to_string(),
            name: "内置模板".to_string(),
            description: None,
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
            setup: None,
            teardown: None,
        };
        let meta = serde_json::json!({
            "version": 5,
            "isBuiltin": true
        });
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: Some(meta.clone()),
        };

        let json = serde_json::to_string(&flow_file)
            .unwrap_or_else(|e| panic!("序列化 FlowFile Failed: {e}"));
        assert!(json.contains("_meta"), "JSON 应包含 _meta 字段");
        assert!(json.contains("isBuiltin"), "JSON 应包含 isBuiltin 字段");

        let deserialized: FlowFile =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化 FlowFile Failed: {e}"));
        let meta = deserialized
            ._meta
            .unwrap_or_else(|| panic!("应包含 _meta 字段"));
        assert_eq!(meta["version"], 5);
        assert_eq!(meta["isBuiltin"], true);
    }

    /// 测试 `FlowFile` 无 _meta 时不序列化该字段
    #[test]
    fn test_flow_file_no_meta_serialization() {
        use flow_engine_core::flow::Flow;

        let flow = Flow {
            uuid: "normal-flow".to_string(),
            name: "普通流程".to_string(),
            description: None,
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
            setup: None,
            teardown: None,
        };
        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let json = serde_json::to_string(&flow_file)
            .unwrap_or_else(|e| panic!("序列化 FlowFile Failed: {e}"));
        assert!(
            !json.contains("_meta"),
            "无 _meta 时 JSON 不应包含 _meta 字段"
        );
    }

    /// 测试 `save_flow_file_with_meta` 和 `load_flow_file_raw` 的 _meta 往返
    #[test]
    fn test_save_and_load_flow_file_with_meta() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("builtin-001", "内置模板流程");
        let meta = serde_json::json!({
            "version": 5,
            "isBuiltin": true
        });

        // 保存带 _meta 的流程
        save_flow_file_with_meta(&flow, flows_dir, Some(meta.clone()))
            .unwrap_or_else(|e| panic!("保存带 _meta 的流程Failed: {e}"));

        // 加载并验证 _meta
        let loaded = load_flow_file_raw("builtin-001", flows_dir)
            .unwrap_or_else(|e| panic!("加载流程文件Failed: {e}"));
        assert_eq!(loaded.flow.uuid, "builtin-001");
        assert_eq!(loaded.flow.name, "内置模板流程");
        assert!(loaded._meta.is_some(), "应包含 _meta 字段");
        assert_eq!(
            loaded._meta.unwrap_or_else(|| panic!("expected Some"))["version"],
            5
        );
    }

    /// 测试 `save_flow_file_with_meta_and_path` 显式写入目标路径（Remap 场景覆写原文件）
    #[test]
    fn test_save_flow_file_with_explicit_target_path() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();
        let target_file = flows_dir.join("my_custom_flow_copy.mpf");

        let flow = create_test_flow("remapped-uuid-999", "重映射后的流程");
        save_flow_file_with_meta_and_path(&flow, flows_dir, None, Some(&target_file))
            .unwrap_or_else(|e| panic!("保存到指定路径Failed: {e}"));

        assert!(target_file.exists(), "指定路径的文件必须存在");

        let loaded = load_flow_file_raw(target_file.to_str().unwrap(), flows_dir)
            .unwrap_or_else(|e| panic!("加载覆写文件Failed: {e}"));
        assert_eq!(loaded.flow.uuid, "remapped-uuid-999");
        assert_eq!(loaded.flow.name, "重映射后的流程");
    }

    /// 测试 `FlowMetadata` 的 `file_path` 字段在 JSON 中为 camelCase
    #[test]
    fn test_flow_metadata_has_path() {
        let metadata = FlowMetadata {
            uuid: "meta-001".to_string(),
            name: "元数据测试".to_string(),
            file_path: "/flows/test.json".to_string(),
            updated_at: 1705315845000,
            folder_path: String::new(),
        };

        let json = serde_json::to_string(&metadata)
            .unwrap_or_else(|e| panic!("序列化 FlowMetadata Failed: {e}"));
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
        Flow {
            uuid: uuid.to_string(),
            name: name.to_string(),
            description: None,
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
            setup: None,
            teardown: None,
        }
    }

    /// 测试 `save_flow_file` 和 `load_flow_file` 的基本功能
    #[test]
    fn test_save_and_load_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("flow-save-001", "Test Flow");

        // 保存
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存流程文件Failed: {e}"));

        // 验证文件已创建，扩展名为 .mpf
        let mpf_files: Vec<_> = fs::read_dir(flows_dir)
            .unwrap_or_else(|e| panic!("{e}"))
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
            file_name.starts_with("Test_Flow"),
            "文件名应以流程名称开头: {}",
            file_name
        );
        assert!(
            file_name.ends_with(".mpf"),
            "文件名应以 .mpf 结尾: {}",
            file_name
        );

        // 加载
        let loaded = load_flow_file("flow-save-001", flows_dir)
            .unwrap_or_else(|e| panic!("加载流程文件Failed: {e}"));
        assert_eq!(loaded.uuid, "flow-save-001");
        assert_eq!(loaded.name, "Test Flow");
        assert!(loaded.nodes.is_empty());
        assert!(loaded.connections.is_empty());

        // 加载不存在的 UUID 应Failed
        let result = load_flow_file("nonexistent", flows_dir);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("Flow not found"));
    }

    /// 测试 `list_flow_files` 递归扫描
    #[test]
    fn test_list_flow_files_recursive() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 在根目录创建流程
        let flow1 = create_test_flow("list-001", "Alpha流程");
        save_flow_file(&flow1, flows_dir).unwrap_or_else(|e| panic!("保存 flow1 Failed: {e}"));

        // 在子目录创建流程
        let sub_dir = flows_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap_or_else(|e| panic!("创建子目录Failed: {e}"));
        let flow2 = create_test_flow("list-002", "Beta流程");
        // 手动在子目录创建 .mpf 文件（测试递归扫描）
        let flow_file = FlowFile {
            _version: 1,
            flow: flow2.clone(),
            _meta: None,
        };
        let json = serde_json::to_string_pretty(&flow_file)
            .unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        fs::write(sub_dir.join("Beta流程.mpf"), json)
            .unwrap_or_else(|e| panic!("写入文件Failed: {e}"));

        // 在更深子目录创建流程
        let deep_dir = sub_dir.join("deep");
        fs::create_dir_all(&deep_dir).unwrap_or_else(|e| panic!("创建深层目录Failed: {e}"));
        let flow3 = create_test_flow("list-003", "Gamma流程");
        let flow_file3 = FlowFile {
            _version: 1,
            flow: flow3.clone(),
            _meta: None,
        };
        let json3 = serde_json::to_string_pretty(&flow_file3)
            .unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        fs::write(deep_dir.join("Gamma流程.mpf"), json3)
            .unwrap_or_else(|e| panic!("写入文件Failed: {e}"));

        // 列出并验证
        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出流程文件Failed: {e}"));
        assert_eq!(list.len(), 3, "应找到 3 个流程文件");

        // 验证按名称排序
        assert_eq!(list[0].name, "Alpha流程");
        assert_eq!(list[1].name, "Beta流程");
        assert_eq!(list[2].name, "Gamma流程");

        // 验证元数据字段（uuid 为相对路径）
        assert_eq!(list[0].uuid, "Alpha流程.mpf");
        assert!(list[0].file_path.contains("Alpha流程"));
    }

    /// 测试损坏的 .mpf 文件正常列出（不检查内容），打开时才报错
    #[test]
    fn test_corrupted_file_listed_and_error_on_load() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 创建一个正常的流程
        let flow = create_test_flow("good-001", "正常流程");
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存正常流程Failed: {e}"));

        // 创建一个损坏的 .mpf 文件
        fs::write(
            flows_dir.join("corrupted.mpf"),
            "this is not valid json {{{",
        )
        .unwrap_or_else(|e| panic!("写入损坏文件Failed: {e}"));

        // 再创建一个损坏的 .mpf 文件（JSON 格式但不是 FlowFile）
        fs::write(
            flows_dir.join("wrong_structure.mpf"),
            r#"{"not_a_flow": true}"#,
        )
        .unwrap_or_else(|e| panic!("写入Error结构文件Failed: {e}"));

        // 列出 — 快速扫描文件名，3 个文件全部秒级列出，不进行耗时内容反序列化
        let list =
            list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出流程文件应Success: {e}"));
        assert_eq!(list.len(), 3, "应列出 3 个文件");

        // 打开损坏文件时报错
        let load_err = load_flow_file("corrupted.mpf", flows_dir);
        assert!(load_err.is_err(), "打开损坏文件应报错");
        assert!(load_err.unwrap_err().contains("Flow file corrupted"));
    }

    /// 测试重命名流程文件
    #[test]
    fn test_rename_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("rename-001", "旧流程");
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存流程Failed: {e}"));

        let meta = rename_flow_file("旧流程.mpf", "新流程名称", flows_dir)
            .unwrap_or_else(|e| panic!("重命名流程Failed: {e}"));
        assert_eq!(meta.name, "新流程名称");
        assert_eq!(meta.uuid, "新流程名称.mpf");

        let loaded = load_flow_file("新流程名称.mpf", flows_dir)
            .unwrap_or_else(|e| panic!("加载新流程Failed: {e}"));
        assert_eq!(loaded.name, "新流程名称");
        assert_eq!(loaded.uuid, "rename-001");
    }

    /// 测试 `copy_flow_file` 复制流程
    #[test]
    fn test_copy_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("copy-001", "原始流程");
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存原始流程Failed: {e}"));

        // 复制
        let copied =
            copy_flow_file("copy-001", flows_dir).unwrap_or_else(|e| panic!("复制流程Failed: {e}"));

        // 验证新 UUID
        assert_ne!(copied.uuid, "copy-001", "复制的流程应有新的 UUID");

        // 验证名称带 "(副本)" 后缀
        assert_eq!(
            copied.name, "原始流程(副本)",
            "复制的流程名称应添加 (副本) 后缀"
        );

        // 验证两个流程独立存在
        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出流程文件Failed: {e}"));
        assert_eq!(list.len(), 2, "应存在 2 个流程");

        // 原始流程仍然可加载
        let original = load_flow_file("copy-001", flows_dir)
            .unwrap_or_else(|e| panic!("原始流程应可加载: {e}"));
        assert_eq!(original.name, "原始流程");

        // 复制的流程也可加载（通过新 UUID）
        let copied_load = load_flow_file(&copied.uuid, flows_dir)
            .unwrap_or_else(|e| panic!("复制流程应可加载: {e}"));
        assert_eq!(copied_load.name, "原始流程(副本)");

        // 再次复制同一流程 — 应生成 "(副本 2)"
        let copied2 = copy_flow_file("copy-001", flows_dir)
            .unwrap_or_else(|e| panic!("第二次复制Failed: {e}"));
        assert_eq!(
            copied2.name, "原始流程(副本 2)",
            "第二次复制的名称应为 (副本 2)"
        );
        assert_ne!(copied2.uuid, copied.uuid, "每次复制应有不同 UUID");
    }

    /// 测试 `copy_flow_file` 复制流程时节点与连线全量重映射
    #[test]
    fn test_copy_flow_file_remaps_nodes_and_connections() {
        use flow_engine_core::connection::Connection;
        use flow_engine_core::node::Node;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let mut flow = create_test_flow("copy-with-nodes", "包含节点的流程");
        let mut node1 = Node::default();
        node1.uuid = "node-origin-1".to_string();
        let mut node2 = Node::default();
        node2.uuid = "node-origin-2".to_string();
        let conn = Connection {
            id: "conn-origin-1".to_string(),
            source_node_uuid: "node-origin-1".to_string(),
            target_node_uuid: "node-origin-2".to_string(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        flow.nodes = vec![node1, node2];
        flow.connections = vec![conn];

        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存原始流程Failed: {e}"));

        let copied = copy_flow_file("copy-with-nodes", flows_dir)
            .unwrap_or_else(|e| panic!("复制失败: {e}"));
        assert_eq!(copied.nodes.len(), 2);
        assert_eq!(copied.connections.len(), 1);

        // 节点 UUID 必须被替换，不能相同
        assert_ne!(copied.nodes[0].uuid, "node-origin-1");
        assert_ne!(copied.nodes[1].uuid, "node-origin-2");
        assert_ne!(copied.nodes[0].uuid, copied.nodes[1].uuid);

        // 连线 ID 必须被替换，且 source_node_uuid 与 target_node_uuid 必须指向新的节点 UUID
        assert_ne!(copied.connections[0].id, "conn-origin-1");
        assert_eq!(copied.connections[0].source_node_uuid, copied.nodes[0].uuid);
        assert_eq!(copied.connections[0].target_node_uuid, copied.nodes[1].uuid);
    }

    /// 测试 `copy_flow_file` 复制流程时数据库节点内部 connection_uuid 级联重映射
    #[test]
    fn test_copy_flow_file_remaps_connection_uuid() {
        use flow_engine_core::node::Node;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let mut flow = create_test_flow("copy-db-flow", "包含数据库节点的流程");
        let mut node1 = Node::default();
        node1.uuid = "db-conn-origin".to_string();

        let mut node2 = Node::default();
        node2.uuid = "db-query-origin".to_string();
        node2.data = serde_json::from_value(serde_json::json!({
            "type": "db:query",
            "connection_uuid": "db-conn-origin",
            "sql": "SELECT 1"
        }))
        .unwrap_or_else(|e| panic!("反序列化节点数据失败: {e}"));

        flow.nodes = vec![node1, node2];

        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存原始流程Failed: {e}"));

        let copied =
            copy_flow_file("copy-db-flow", flows_dir).unwrap_or_else(|e| panic!("复制失败: {e}"));
        assert_eq!(copied.nodes.len(), 2);

        let new_conn_uuid = &copied.nodes[0].uuid;
        assert_ne!(new_conn_uuid, "db-conn-origin");

        let query_data_val =
            serde_json::to_value(&copied.nodes[1].data).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(query_data_val["connection_uuid"], *new_conn_uuid);
    }

    /// 测试 `delete_flow_file` 删除流程
    #[test]
    fn test_delete_flow_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 创建多个流程
        let flow1 = create_test_flow("del-001", "保留流程");
        let flow2 = create_test_flow("del-002", "删除流程");
        save_flow_file(&flow1, flows_dir).unwrap_or_else(|e| panic!("保存 flow1 Failed: {e}"));
        save_flow_file(&flow2, flows_dir).unwrap_or_else(|e| panic!("保存 flow2 Failed: {e}"));

        // 验证初始数量
        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list.len(), 2);

        // 删除一个
        delete_flow_file("del-002", flows_dir).unwrap_or_else(|e| panic!("删除流程Failed: {e}"));

        // 验证删除后数量
        let list_after = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list_after.len(), 1, "删除后应剩余 1 个流程");
        assert_eq!(
            list_after[0].uuid, "保留流程.mpf",
            "剩余的应是保留流程相对路径"
        );
        assert_eq!(list_after[0].name, "保留流程");
        let remaining = load_flow_file("del-001", flows_dir)
            .unwrap_or_else(|e| panic!("加载保留流程Failed: {e}"));
        assert_eq!(remaining.uuid, "del-001");

        // 删除不存在的 UUID 应Failed
        let result = delete_flow_file("nonexistent", flows_dir);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("Flow not found"));
    }

    /// 测试 `move_flow_to_folder` 移动流程（支持相对路径与 UUID 两种传参方式）
    #[test]
    fn test_move_flow_to_folder() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 1. 创建流程
        let flow1 = create_test_flow("flow-move-1", "待移动流程1");
        let flow2 = create_test_flow("flow-move-2", "待移动流程2");
        save_flow_file(&flow1, flows_dir).unwrap_or_else(|e| panic!("保存 flow1 Failed: {e}"));
        save_flow_file(&flow2, flows_dir).unwrap_or_else(|e| panic!("保存 flow2 Failed: {e}"));

        // 2. 按相对路径移动 flow1 到目标子文件夹 "target-sub"
        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        let flow1_meta = list.iter().find(|m| m.name == "待移动流程1").unwrap();
        assert_eq!(flow1_meta.uuid, "待移动流程1.mpf");

        move_flow_to_folder(&flow1_meta.uuid, flows_dir, "target-sub")
            .unwrap_or_else(|e| panic!("按相对路径移动Failed: {e}"));

        let flow1_new_path = flows_dir.join("target-sub").join("待移动流程1.mpf");
        assert!(flow1_new_path.is_file(), "文件应已移动至 target-sub");
        assert!(!flows_dir.join("待移动流程1.mpf").exists(), "根目录下原文件应已移除");

        // 3. 按内部 UUID 移动 flow2 到目标子文件夹
        move_flow_to_folder("flow-move-2", flows_dir, "target-sub")
            .unwrap_or_else(|e| panic!("按 UUID 移动Failed: {e}"));

        let flow2_new_path = flows_dir.join("target-sub").join("待移动流程2.mpf");
        assert!(flow2_new_path.is_file(), "文件应已移动至 target-sub");

        // 4. 再次移回根目录（target_folder_path = ""）
        move_flow_to_folder("target-sub/待移动流程1.mpf", flows_dir, "")
            .unwrap_or_else(|e| panic!("移回根目录Failed: {e}"));
        assert!(flows_dir.join("待移动流程1.mpf").is_file());

        // 5. 目标文件夹同名冲突检查
        let conflict_flow = create_test_flow("flow-move-conflict", "待移动流程1");
        let conflict_path = flows_dir.join("target-sub").join("待移动流程1.mpf");
        save_flow_file_with_meta_and_path(&conflict_flow, flows_dir, None, Some(&conflict_path))
            .unwrap_or_else(|e| panic!("创建冲突文件Failed: {e}"));

        let err = move_flow_to_folder("待移动流程1.mpf", flows_dir, "target-sub");
        assert!(err.is_err(), "同名文件已存在时应报错");
        assert!(err.unwrap_err().contains("already exists in the target folder"));
    }

    /// 测试 `save_flow_file` 更新已有流程（同 UUID 覆盖写入）
    #[test]
    fn test_save_flow_file_updates_existing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 首次保存
        let flow = create_test_flow("update-001", "更新测试");
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("首次保存Failed: {e}"));

        // 验证只有 1 个文件
        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list.len(), 1);

        // 更新流程（同 UUID，新名称）
        let mut updated_flow = flow.clone();
        updated_flow.name = "更新后名称".to_string();
        save_flow_file(&updated_flow, flows_dir).unwrap_or_else(|e| panic!("更新保存Failed: {e}"));

        // 验证仍然只有 1 个文件（未产生副本）
        let list_after = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list_after.len(), 1, "更新后应仍只有 1 个文件");

        // 验证内容已更新
        let loaded =
            load_flow_file("update-001", flows_dir).unwrap_or_else(|e| panic!("加载Failed: {e}"));
        assert_eq!(loaded.name, "更新后名称");

        // 多次更新验证
        updated_flow.name = "第三次更新".to_string();
        save_flow_file(&updated_flow, flows_dir)
            .unwrap_or_else(|e| panic!("第三次保存Failed: {e}"));

        let list_final = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list_final.len(), 1, "多次更新后应仍只有 1 个文件");
        assert_eq!(list_final[0].name, "第三次更新");
    }

    /// 测试基于 mtime 的元数据缓存一致性与自动刷新
    #[test]
    fn test_flow_metadata_cache_consistency() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        let flow = create_test_flow("cache-001", "缓存测试流程");
        save_flow_file(&flow, flows_dir).unwrap_or_else(|e| panic!("保存Failed: {e}"));

        // 首次列表（填充缓存）
        let list1 = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("首次列出Failed: {e}"));
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].name, "缓存测试流程");

        // 二次列表（命中缓存）
        let list2 = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("二次列出Failed: {e}"));
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].name, "缓存测试流程");
        assert_eq!(list1[0].updated_at, list2[0].updated_at);

        // 更新流程并再次列出
        let mut updated = flow.clone();
        updated.name = "缓存更新流程".to_string();
        save_flow_file(&updated, flows_dir).unwrap_or_else(|e| panic!("更新Failed: {e}"));

        let list3 = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("三次列出Failed: {e}"));
        assert_eq!(list3.len(), 1);
        assert_eq!(list3[0].name, "缓存更新流程");

        // 删除流程
        delete_flow_file("cache-001", flows_dir).unwrap_or_else(|e| panic!("删除Failed: {e}"));
        let list_empty = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列出Failed: {e}"));
        assert_eq!(list_empty.len(), 0);
    }

    #[test]
    fn test_detect_flow_format() {
        assert_eq!(detect_flow_format("{\"_version\": 1}"), FlowFormat::Json);
        assert_eq!(
            detect_flow_format("  \n  { \"_version\": 1 }"),
            FlowFormat::Json
        );
        assert_eq!(detect_flow_format("_version: 1\nflow:\n"), FlowFormat::Yaml);
        assert_eq!(detect_flow_format("---\n_version: 1\n"), FlowFormat::Yaml);
    }

    #[test]
    fn test_deserialize_historical_json_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/flows/HTTP_Node_Example_Flow.mpf");
        let content = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("读取历史测试夹具Failed: {e}"));

        assert_eq!(detect_flow_format(&content), FlowFormat::Json);

        let flow_file = deserialize_flow_file(&content)
            .unwrap_or_else(|e| panic!("反序列化历史 JSON 流程Failed: {e}"));
        assert_eq!(flow_file._version, 1);
        assert!(!flow_file.flow.uuid.is_empty());
        assert!(!flow_file.flow.nodes.is_empty());
        assert!(!flow_file.flow.connections.is_empty());
    }

    #[test]
    fn test_yaml_serialization_and_coordinate_rounding() {
        let mut flow = create_test_flow("yaml-round-01", "坐标取整测试");
        let mut test_node = flow_engine_core::node::Node::default();
        test_node.uuid = "node-round-01".to_string();
        test_node.position.x = 100.499;
        test_node.position.y = 200.501;
        flow.nodes.push(test_node);

        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let yaml_str = serialize_flow_file(&flow_file, FlowFormat::Yaml)
            .unwrap_or_else(|e| panic!("YAML 序列化Failed: {e}"));
        assert_eq!(detect_flow_format(&yaml_str), FlowFormat::Yaml);
        assert!(yaml_str.contains("100.0") || yaml_str.contains("100"));
        assert!(yaml_str.contains("201.0") || yaml_str.contains("201"));

        let deserialized =
            deserialize_flow_file(&yaml_str).unwrap_or_else(|e| panic!("YAML 反序列化Failed: {e}"));
        assert_eq!(deserialized.flow.nodes[0].position.x, 100.0);
        assert_eq!(deserialized.flow.nodes[0].position.y, 201.0);
    }

    #[test]
    fn test_yaml_multiline_script_preservation() {
        use flow_engine_core::node_config::{NodeData, ScriptNodeConfig};
        let mut flow = create_test_flow("yaml-script-01", "多行脚本YAML测试");
        let script_code = "const a = 1;\nconst b = 2;\nreturn a + b;\n";
        let node = flow_engine_core::node::Node {
            uuid: "script-node-01".to_string(),
            node_type: "script".to_string(),
            name: "JS代码".to_string(),
            position: flow_engine_core::node::NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::Script(ScriptNodeConfig {
                script: script_code.to_string(),
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        flow.nodes.push(node);

        let flow_file = FlowFile {
            _version: 1,
            flow,
            _meta: None,
        };

        let yaml_str = serialize_flow_file(&flow_file, FlowFormat::Yaml).unwrap();
        let deserialized = deserialize_flow_file(&yaml_str).unwrap();
        if let NodeData::Script(cfg) = &deserialized.flow.nodes[0].data {
            assert_eq!(cfg.script, script_code);
        } else {
            panic!("expected Script node data");
        }
    }

    #[test]
    fn test_save_flow_file_preserves_format_and_defaults_to_yaml() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap_or_else(|e| panic!("创建临时目录Failed: {e}"));
        let flows_dir = temp_dir.path();

        // 1. 新建流程默认使用 YAML 格式保存
        let flow1 = create_test_flow("flow-yaml-default", "新建YAML流程");
        save_flow_file(&flow1, flows_dir).unwrap_or_else(|e| panic!("保存新流程Failed: {e}"));

        let list = list_flow_files(flows_dir).unwrap_or_else(|e| panic!("列表Failed: {e}"));
        assert_eq!(list.len(), 1);
        let saved_path = Path::new(&list[0].file_path);
        let content1 = fs::read_to_string(saved_path).unwrap_or_else(|e| panic!("读取Failed: {e}"));
        assert_eq!(detect_flow_format(&content1), FlowFormat::Yaml);

        // 2. 模拟老流程：以 JSON 格式手动写入并更新，验证保存时保持 JSON 格式
        let flow2 = create_test_flow("flow-json-legacy", "历史JSON流程");
        let flow_file2 = FlowFile {
            _version: 1,
            flow: flow2.clone(),
            _meta: None,
        };
        let legacy_json = serde_json::to_string_pretty(&flow_file2).unwrap();
        let legacy_file_path = flows_dir.join("历史JSON流程.mpf");
        fs::write(&legacy_file_path, legacy_json).unwrap();

        // 验证读取
        let loaded_legacy = load_flow_file("flow-json-legacy", flows_dir).unwrap();
        assert_eq!(loaded_legacy.uuid, "flow-json-legacy");

        // 更新该老流程
        let mut updated_legacy = flow2.clone();
        updated_legacy.name = "历史JSON流程已修改".to_string();
        save_flow_file(&updated_legacy, flows_dir).unwrap();

        let list2 = list_flow_files(flows_dir).unwrap();
        let legacy_meta = list2
            .iter()
            .find(|m| m.name.contains("历史JSON流程已修改"))
            .unwrap();
        let content_legacy_updated = fs::read_to_string(&legacy_meta.file_path).unwrap();
        assert_eq!(
            detect_flow_format(&content_legacy_updated),
            FlowFormat::Json,
            "历史 JSON 流程更新后应保持 JSON 格式"
        );
    }

    // ========================================================================
    // 环境文件存储测试 (Environment Storage Tests)
    // ========================================================================

    #[test]
    fn test_environment_storage_crud() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        // 1. 初始化列表为空
        let list = list_environment_files(env_dir).unwrap();
        assert!(list.is_empty());

        // 2. 创建并保存环境 dev
        let mut vars = HashMap::new();
        vars.insert("HOST".to_string(), serde_json::json!("dev.example.com"));
        vars.insert("PORT".to_string(), serde_json::json!(8080));
        let env = EnvironmentSet {
            name: "dev".to_string(),
            description: Some("开发环境".to_string()),
            variables: vars,
        };
        save_environment_file(&env, false, env_dir).unwrap();

        // 3. 验证列表
        let list = list_environment_files(env_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "dev");
        assert_eq!(list[0].variable_count, 2);
        assert!(!list[0].has_local_override);
        assert!(list[0].updated_at > 0);

        // 4. 读取原始环境
        let loaded = load_environment_file_raw("dev", false, env_dir).unwrap();
        assert_eq!(loaded.name, "dev");
        assert_eq!(loaded.description, Some("开发环境".to_string()));
        assert_eq!(
            loaded.variables.get("HOST"),
            Some(&serde_json::json!("dev.example.com"))
        );
        assert_eq!(loaded.variables.get("PORT"), Some(&serde_json::json!(8080)));

        // 5. 删除环境
        delete_environment_file("dev", env_dir).unwrap();
        let list_after = list_environment_files(env_dir).unwrap();
        assert!(list_after.is_empty());
    }

    #[test]
    fn test_environment_local_override_merge() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        // 1. 主环境 dev.yaml
        let mut base_vars = HashMap::new();
        base_vars.insert("HOST".to_string(), serde_json::json!("dev.example.com"));
        base_vars.insert("PORT".to_string(), serde_json::json!(8080));
        let base_env = EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: base_vars,
        };
        save_environment_file(&base_env, false, env_dir).unwrap();

        // 2. 本地私有覆盖 dev.local.yaml
        let mut local_vars = HashMap::new();
        local_vars.insert("HOST".to_string(), serde_json::json!("localhost"));
        local_vars.insert("SECRET".to_string(), serde_json::json!("my-local-secret"));
        let local_env = EnvironmentSet {
            name: "dev".to_string(),
            description: None,
            variables: local_vars,
        };
        save_environment_file(&local_env, true, env_dir).unwrap();

        // 3. 元数据应标记 has_local_override = true
        let list = list_environment_files(env_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "dev");
        assert!(list[0].has_local_override);

        // 4. raw 加载不合并
        let loaded_raw = load_environment_file_raw("dev", false, env_dir).unwrap();
        assert_eq!(
            loaded_raw.variables.get("HOST"),
            Some(&serde_json::json!("dev.example.com"))
        );
        assert!(loaded_raw.variables.get("SECRET").is_none());

        // 5. 普通加载自动合并（local 覆盖 base）
        let loaded_merged = load_environment_file("dev", env_dir).unwrap();
        assert_eq!(
            loaded_merged.variables.get("HOST"),
            Some(&serde_json::json!("localhost"))
        );
        assert_eq!(
            loaded_merged.variables.get("PORT"),
            Some(&serde_json::json!(8080))
        );
        assert_eq!(
            loaded_merged.variables.get("SECRET"),
            Some(&serde_json::json!("my-local-secret"))
        );
    }

    #[test]
    fn test_environment_rename() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        let mut vars = HashMap::new();
        vars.insert("K".to_string(), serde_json::json!("V"));
        let env = EnvironmentSet {
            name: "old_env".to_string(),
            description: None,
            variables: vars,
        };
        save_environment_file(&env, false, env_dir).unwrap();

        let mut local_vars = HashMap::new();
        local_vars.insert("LOCAL_K".to_string(), serde_json::json!("LOCAL_V"));
        let local_env = EnvironmentSet {
            name: "old_env".to_string(),
            description: None,
            variables: local_vars,
        };
        save_environment_file(&local_env, true, env_dir).unwrap();

        // 重命名 old_env -> new_env
        let meta = rename_environment_file("old_env", "new_env", env_dir).unwrap();
        assert_eq!(meta.name, "new_env");
        assert!(meta.has_local_override);

        // 验证旧文件已删除，新文件可加载
        assert!(load_environment_file_raw("old_env", false, env_dir).is_err());
        let new_loaded = load_environment_file("new_env", env_dir).unwrap();
        assert_eq!(new_loaded.variables.get("K"), Some(&serde_json::json!("V")));
        assert_eq!(
            new_loaded.variables.get("LOCAL_K"),
            Some(&serde_json::json!("LOCAL_V"))
        );
    }

    /// 仅删除本地私有覆盖时，主环境文件必须完好无损且不再叠加覆盖值
    #[test]
    fn test_environment_delete_local_override_only() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        let mut base_vars = HashMap::new();
        base_vars.insert("HOST".to_string(), serde_json::json!("shared.host"));
        save_environment_file(
            &EnvironmentSet {
                name: "prod".to_string(),
                description: Some("共享环境".to_string()),
                variables: base_vars,
            },
            false,
            env_dir,
        )
        .unwrap();

        let mut local_vars = HashMap::new();
        local_vars.insert("HOST".to_string(), serde_json::json!("private.host"));
        save_environment_file(
            &EnvironmentSet {
                name: "prod".to_string(),
                description: None,
                variables: local_vars,
            },
            true,
            env_dir,
        )
        .unwrap();

        assert!(load_environment_file("prod", env_dir)
            .unwrap()
            .variables
            .get("HOST")
            .is_some_and(|v| v == &serde_json::json!("private.host")));

        delete_environment_local_override("prod", env_dir).unwrap();

        // 主环境文件保留，覆盖值消失
        let remaining = list_environment_files(env_dir).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "prod");
        assert!(!remaining[0].has_local_override);
        assert_eq!(
            load_environment_file("prod", env_dir)
                .unwrap()
                .variables
                .get("HOST"),
            Some(&serde_json::json!("shared.host"))
        );

        // 重复删除应报错（幂等性由调用方提示处理）
        assert!(delete_environment_local_override("prod", env_dir).is_err());
    }

    /// 用户手写的纯键值 YAML（无 variables 包裹）必须完整解析，不能被静默解析成空变量集
    #[test]
    fn test_environment_flat_mapping_not_silently_dropped() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        std::fs::write(
            env_dir.join("dev.yaml"),
            "name: my-service\nHOST: dev.example.com\nPORT: 8080\n",
        )
        .unwrap();

        let loaded = load_environment_file_raw("dev", false, env_dir).unwrap();
        // name 以文件名为准，避免文件内容里的 name 让后续保存写错文件
        assert_eq!(loaded.name, "dev");
        assert_eq!(
            loaded.variables.get("HOST"),
            Some(&serde_json::json!("dev.example.com"))
        );
        assert_eq!(loaded.variables.get("PORT"), Some(&serde_json::json!(8080)));

        // 列表也必须反映真实变量数
        let list = list_environment_files(env_dir).unwrap();
        assert_eq!(list[0].variable_count, 3);

        // 完整对象形态仍然走 variables 分支
        std::fs::write(
            env_dir.join("full.yaml"),
            "name: whatever\ndescription: 完整环境\nvariables:\n  A: 1\n",
        )
        .unwrap();
        let full = load_environment_file_raw("full", false, env_dir).unwrap();
        assert_eq!(full.name, "full");
        assert_eq!(full.description, Some("完整环境".to_string()));
        assert_eq!(full.variables.get("A"), Some(&serde_json::json!(1)));

        // 非法内容（标量文档）应报错而不是产出空环境
        std::fs::write(env_dir.join("bad.yaml"), "just-a-scalar\n").unwrap();
        assert!(load_environment_file_raw("bad", false, env_dir).is_err());
    }

    /// 同时存在 .yaml 与 .yml 时，列表与加载必须指向同一个文件（.yaml 优先）
    #[test]
    fn test_environment_listing_prefers_yaml() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        std::fs::write(env_dir.join("dup.yaml"), "A: from-yaml\n").unwrap();
        std::fs::write(env_dir.join("dup.yml"), "A: from-yml\n").unwrap();

        let list = list_environment_files(env_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].file_path.ends_with("dup.yaml"));
        assert_eq!(list[0].variable_count, 1);
        assert_eq!(
            load_environment_file("dup", env_dir)
                .unwrap()
                .variables
                .get("A"),
            Some(&serde_json::json!("from-yaml"))
        );
    }

    /// 重命名必须拒绝覆盖已存在的环境（含仅存在本地覆盖的目标名）
    #[test]
    fn test_environment_rename_rejects_existing_targets() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let env_dir = temp_dir.path();

        for name in ["a", "b"] {
            save_environment_file(
                &EnvironmentSet {
                    name: name.to_string(),
                    description: None,
                    variables: HashMap::new(),
                },
                false,
                env_dir,
            )
            .unwrap();
        }
        save_environment_file(
            &EnvironmentSet {
                name: "c".to_string(),
                description: None,
                variables: HashMap::new(),
            },
            true,
            env_dir,
        )
        .unwrap();

        // 目标已有主环境 -> 拒绝，且原环境不被破坏
        assert!(rename_environment_file("a", "b", env_dir).is_err());
        assert!(load_environment_file_raw("a", false, env_dir).is_ok());

        // 目标已有本地覆盖（无主环境）-> 同样拒绝，避免覆盖他人私有配置
        assert!(rename_environment_file("a", "c", env_dir).is_err());
        assert!(load_environment_file_raw("a", false, env_dir).is_ok());
        assert_eq!(
            list_environment_files(env_dir)
                .unwrap()
                .iter()
                .filter(|m| m.name == "c")
                .count(),
            0
        );

        // 空名（含纯空白）必须被拒绝
        assert!(rename_environment_file("a", "   ", env_dir).is_err());
        assert!(load_environment_file_raw("", false, env_dir).is_err());
        assert!(delete_environment_file("  ", env_dir).is_err());

        // 带空白的目标名按同一套清洗规则落到 "d"
        let meta = rename_environment_file("a", " d ", env_dir).unwrap();
        assert_eq!(meta.name, "d");
        assert!(load_environment_file_raw("d", false, env_dir).is_ok());
    }
}
