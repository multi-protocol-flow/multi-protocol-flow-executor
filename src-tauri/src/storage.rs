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
            .map(|p| PathBuf::from(p))
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

    if !flows_dir.exists() {
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
        PathBuf::from(path)
    } else {
        // 相对路径，使用 flows 目录
        ensure_flows_dir()?.join(path)
    };

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
        PathBuf::from(path)
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
            id: "test-flow-1".to_string(),
            name: "测试流程".to_string(),
            nodes: vec![],
            connections: vec![],
        }
    }

    // 清理测试数据目录
    fn cleanup_test_data() {
        if let Ok(data_dir) = get_data_dir() {
            if data_dir.exists() {
                let _ = fs::remove_dir_all(&data_dir);
            }
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

        eprintln!("调用 ensure_flows_dir...");
        let flows_dir = ensure_flows_dir();
        if !flows_dir.is_ok() {
            eprintln!("ensure_flows_dir 失败: {:?}", flows_dir);
        }
        assert!(flows_dir.is_ok());
        let dir = flows_dir.unwrap();
        eprintln!(
            "Flows dir path: {:?}, exists: {}, is_dir: {}",
            dir,
            dir.exists(),
            dir.is_dir()
        );
        assert!(dir.exists());
        assert!(dir.is_dir());
        assert!(dir.ends_with("flows"));

        cleanup_test_data();
    }

    #[test]
    fn test_save_and_load_flow() {
        cleanup_test_data();

        // 检查清理是否成功
        if let Ok(data_dir) = get_data_dir() {
            eprintln!(
                "Data dir after cleanup: {:?}, exists: {}",
                data_dir,
                data_dir.exists()
            );
        }

        let flow = create_test_flow();

        // 保存流程
        eprintln!("准备保存流程...");
        let save_result = save_flow(&flow, "test-flow.json");
        if !save_result.is_ok() {
            eprintln!("保存失败: {:?}", save_result);
        }
        assert!(save_result.is_ok());

        // 验证文件已创建
        let file_path = get_flows_dir().unwrap().join("test-flow.json");
        assert!(file_path.exists());

        // 加载流程
        let loaded_flow = load_flow("test-flow.json");
        assert!(loaded_flow.is_ok());

        let loaded = loaded_flow.unwrap();
        assert_eq!(flow.id, loaded.id);
        assert_eq!(flow.name, loaded.name);
        assert_eq!(flow.nodes.len(), loaded.nodes.len());
        assert_eq!(flow.connections.len(), loaded.connections.len());

        cleanup_test_data();
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
        assert_eq!(flow.id, loaded_flow.unwrap().id);

        // 清理临时文件
        let _ = fs::remove_file(&temp_file_path);
    }
}
