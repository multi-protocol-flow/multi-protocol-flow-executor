// 存储模块 - JSON 文件读写实现

use crate::flow::Flow;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // 测试用的辅助函数：创建一个简单的 Flow
    fn create_test_flow() -> Flow {
        Flow {
            uuid: "test-flow-1".to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        }
    }

    // 清理测试数据目录
    fn cleanup_test_data() {
        if let Ok(flows_dir) = get_flows_dir() {
            // 删除 flows 目录，忽略错误
            let _ = fs::remove_dir_all(&flows_dir);
        }
    }

    #[test]
    fn test_get_data_dir() {
        let data_dir = get_data_dir();
        assert!(data_dir.is_ok());
        let dir = data_dir.unwrap();
        assert!(dir.ends_with("multi-protocol-flow-executor"));
    }

    #[test]
    fn test_ensure_flows_dir() {
        cleanup_test_data();

        let flows_dir = ensure_flows_dir();
        assert!(flows_dir.is_ok(), "ensure_flows_dir 应该成功");
        let dir = flows_dir.unwrap();

        // 验证路径格式
        assert!(dir.ends_with("flows"), "路径应该以 'flows' 结尾");

        cleanup_test_data();
    }

    #[test]
    fn test_save_and_load_flow() {
        // 使用临时测试目录，避免系统目录的权限问题
        let test_dir = std::env::current_dir().unwrap().join("test-flows");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).expect("Failed to create test dir");

        eprintln!(
            "测试 flows dir: {:?}, exists: {}",
            test_dir,
            test_dir.exists()
        );

        let flow = create_test_flow();

        // 使用绝对路径保存流程
        let file_path = test_dir.join("test-flow.json");
        let save_result = save_flow(&flow, file_path.to_str().unwrap());
        if save_result.is_err() {
            eprintln!("保存失败: {:?}", save_result);
        }
        assert!(save_result.is_ok());

        // 验证文件已创建
        assert!(file_path.exists(), "文件 {:?} 应该存在", file_path);

        // 从绝对路径加载流程
        let loaded_flow = load_flow(file_path.to_str().unwrap());
        assert!(loaded_flow.is_ok());

        let loaded = loaded_flow.unwrap();
        assert_eq!(flow.uuid, loaded.uuid);
        assert_eq!(flow.name, loaded.name);
        assert_eq!(flow.nodes.len(), loaded.nodes.len());
        assert_eq!(flow.connections.len(), loaded.connections.len());

        // 清理测试目录
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_load_nonexistent_flow() {
        cleanup_test_data();

        let result = load_flow("nonexistent-flow.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("打开文件"));

        cleanup_test_data();
    }

    #[test]
    fn test_save_to_absolute_path() {
        cleanup_test_data();

        let flow = create_test_flow();

        // 使用当前目录下的临时文件路径
        let temp_file_path = std::env::current_dir().unwrap().join("test-temp-flow.json");

        // 保存到绝对路径
        let save_result = save_flow(&flow, temp_file_path.to_str().unwrap());
        assert!(save_result.is_ok());
        assert!(temp_file_path.exists());

        // 从绝对路径加载
        let loaded_flow = load_flow(temp_file_path.to_str().unwrap());
        assert!(loaded_flow.is_ok());
        assert_eq!(flow.uuid, loaded_flow.unwrap().uuid);

        // 清理临时文件
        let _ = fs::remove_file(&temp_file_path);
    }

    #[test]
    fn test_create_test_flow_uses_uuid_field() {
        let flow = create_test_flow();

        // 验证 uuid 字段存在且不为空
        assert!(!flow.uuid.is_empty(), "flow.uuid 不应该为空");
        assert_eq!(flow.uuid, "test-flow-1", "flow.uuid 应该是 'test-flow-1'");
    }

    #[test]
    fn test_flow_uuid_preserved_across_save_load() {
        // 创建多个具有不同 uuid 的 flow
        let flow1 = Flow {
            uuid: "uuid-12345".to_string(),
            name: "流程1".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        };

        let flow2 = Flow {
            uuid: "uuid-67890".to_string(),
            name: "流程2".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        };

        let test_dir = std::env::current_dir().unwrap().join("test-flows-uuid");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).expect("Failed to create test dir");

        // 保存两个 flow
        let path1 = test_dir.join("flow1.json");
        let path2 = test_dir.join("flow2.json");

        save_flow(&flow1, path1.to_str().unwrap()).expect("Failed to save flow1");
        save_flow(&flow2, path2.to_str().unwrap()).expect("Failed to save flow2");

        // 加载并验证 uuid 保留
        let loaded1 = load_flow(path1.to_str().unwrap()).expect("Failed to load flow1");
        let loaded2 = load_flow(path2.to_str().unwrap()).expect("Failed to load flow2");

        assert_eq!(loaded1.uuid, "uuid-12345", "flow1.uuid 应该被正确保留");
        assert_eq!(loaded2.uuid, "uuid-67890", "flow2.uuid 应该被正确保留");
        assert_ne!(loaded1.uuid, loaded2.uuid, "不同 flow 的 uuid 应该不同");

        // 清理测试目录
        let _ = fs::remove_dir_all(&test_dir);
    }
}
