//! 工作区感知与工程分析模块
//!
//! 提供本地项目的高性能文件扫描、代码正则检索、代码切片按需读取以及 API 规范 (OpenAPI/Proto) 解析能力。

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// 忽略的目录名称（巨型目录或非业务代码目录）
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".idea",
    ".vscode",
    "venv",
    ".venv",
    "__pycache__",
    ".next",
    ".nuxt",
    "vendor",
    "bin",
    "obj",
    ".turbo",
    ".output",
];

/// 工作区扫描结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceScanResult {
    /// 探测到的技术栈（如 "Node.js (TypeScript)", "Rust", "Go", "Python"）
    pub tech_stack: Option<String>,
    /// 关键工程配置文件相对路径列表
    pub config_files: Vec<String>,
    /// 接口规范文件相对路径列表 (`OpenAPI` / `Swagger` / `Proto` / `Postman`)
    pub spec_files: Vec<String>,
    /// 关键业务与路由代码目录相对路径列表
    pub key_directories: Vec<String>,
}

/// 代码匹配结果条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeMatch {
    /// 相对文件路径
    pub file: String,
    /// 1-based 行号
    pub line_number: usize,
    /// 匹配所在行内容
    pub line_content: String,
}

/// 扫描本地工程目录，提取技术栈、配置文件、接口规范与核心代码目录
///
/// # 参数
/// - `root`: 工作区本地绝对路径
/// - `max_depth`: 最大遍历递归深度 (通常建议 4 ~ 6)
pub fn scan_workspace(root: &Path, max_depth: usize) -> Result<WorkspaceScanResult, String> {
    if !root.exists() {
        return Err(format!("Workspace root does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("Workspace root is not a directory: {}", root.display()));
    }

    let mut config_files = Vec::new();
    let mut spec_files = Vec::new();
    let mut key_directories = Vec::new();

    collect_workspace_info(
        root,
        root,
        0,
        max_depth,
        &mut config_files,
        &mut spec_files,
        &mut key_directories,
    )?;

    config_files.sort();
    spec_files.sort();
    key_directories.sort();
    key_directories.dedup();

    let tech_stack = detect_tech_stack(&config_files, root);

    Ok(WorkspaceScanResult {
        tech_stack,
        config_files,
        spec_files,
        key_directories,
    })
}

fn collect_workspace_info(
    root: &Path,
    current: &Path,
    current_depth: usize,
    max_depth: usize,
    config_files: &mut Vec<String>,
    spec_files: &mut Vec<String>,
    key_directories: &mut Vec<String>,
) -> Result<(), String> {
    if current_depth > max_depth {
        return Ok(());
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if path.is_dir() {
            if IGNORED_DIRS.contains(&name_str.as_ref()) {
                continue;
            }

            let lower_name = name_str.to_lowercase();
            if is_key_directory(&lower_name) {
                if let Ok(rel) = path.strip_prefix(root) {
                    key_directories.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }

            collect_workspace_info(
                root,
                &path,
                current_depth + 1,
                max_depth,
                config_files,
                spec_files,
                key_directories,
            )?;
        } else if path.is_file() {
            let lower_name = name_str.to_lowercase();
            let rel_path = match path.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            if is_config_file(&lower_name) {
                config_files.push(rel_path.clone());
            }

            if is_spec_file(&lower_name) {
                spec_files.push(rel_path);
            }
        }
    }

    Ok(())
}

fn is_key_directory(name: &str) -> bool {
    matches!(
        name,
        "routes"
            | "routers"
            | "controller"
            | "controllers"
            | "api"
            | "apis"
            | "handler"
            | "handlers"
            | "endpoints"
            | "services"
            | "proto"
            | "protos"
    )
}

fn is_config_file(name: &str) -> bool {
    matches!(
        name,
        "package.json"
            | "cargo.toml"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "requirements.txt"
            | "pyproject.toml"
            | "pipfile"
            | "composer.json"
            | "gemfile"
            | "tsconfig.json"
    ) || name.ends_with(".csproj")
        || name.ends_with(".sln")
}

fn is_spec_file(name: &str) -> bool {
    name.ends_with(".proto")
        || name.ends_with(".postman_collection.json")
        || name.contains("openapi")
        || name.contains("swagger")
}

fn detect_tech_stack(config_files: &[String], root: &Path) -> Option<String> {
    let lower_configs: Vec<String> = config_files.iter().map(|f| f.to_lowercase()).collect();

    if lower_configs.iter().any(|c| c.ends_with("cargo.toml")) {
        return Some("Rust".to_string());
    }
    if lower_configs.iter().any(|c| c.ends_with("go.mod")) {
        return Some("Go".to_string());
    }
    if lower_configs.iter().any(|c| c.ends_with("package.json")) {
        let pkg_path = root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            let is_ts = lower_configs.iter().any(|c| c.ends_with("tsconfig.json"));
            let lang = if is_ts { "TypeScript" } else { "JavaScript" };
            if content.contains("\"express\"") {
                return Some(format!("Node.js ({lang} / Express)"));
            }
            if content.contains("\"@nestjs/core\"") {
                return Some(format!("Node.js ({lang} / NestJS)"));
            }
            if content.contains("\"fastify\"") {
                return Some(format!("Node.js ({lang} / Fastify)"));
            }
            if content.contains("\"koa\"") {
                return Some(format!("Node.js ({lang} / Koa)"));
            }
            return Some(format!("Node.js ({lang})"));
        }
        return Some("Node.js".to_string());
    }
    if lower_configs
        .iter()
        .any(|c| c.ends_with("requirements.txt") || c.ends_with("pyproject.toml"))
    {
        return Some("Python".to_string());
    }
    if lower_configs
        .iter()
        .any(|c| c.ends_with("pom.xml") || c.ends_with("build.gradle") || c.ends_with("build.gradle.kts"))
    {
        return Some("Java / JVM".to_string());
    }
    if lower_configs.iter().any(|c| c.ends_with("composer.json")) {
        return Some("PHP".to_string());
    }

    None
}

/// 基于正则表达式检索工作区中的源代码，按文件与行号返回匹配结果
///
/// # 参数
/// - `root`: 工作区本地根目录
/// - `pattern`: 正则表达式字符串
/// - `ext`: 可选的文件扩展名过滤 (例如 "ts", "rs", "go")
/// - `max_results`: 最大匹配数量，内部硬截断为最大 50 条
pub fn search_workspace_code(
    root: &Path,
    pattern: &str,
    ext: Option<&str>,
    max_results: usize,
) -> Result<Vec<CodeMatch>, String> {
    if !root.exists() {
        return Err(format!("Workspace root does not exist: {}", root.display()));
    }

    let re = Regex::new(pattern).map_err(|e| format!("Invalid regex pattern '{pattern}': {e}"))?;
    let limit = max_results.clamp(1, 50);

    let clean_ext = ext.map(|e| e.trim_start_matches('.').to_lowercase());
    let mut matches = Vec::new();

    search_dir_code(root, root, 0, 8, &re, clean_ext.as_deref(), limit, &mut matches)?;

    Ok(matches)
}

fn search_dir_code(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    re: &Regex,
    target_ext: Option<&str>,
    limit: usize,
    matches: &mut Vec<CodeMatch>,
) -> Result<(), String> {
    if depth > max_depth || matches.len() >= limit {
        return Ok(());
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        if matches.len() >= limit {
            break;
        }

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if IGNORED_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            search_dir_code(root, &path, depth + 1, max_depth, re, target_ext, limit, matches)?;
        } else if path.is_file() {
            if let Some(target) = target_ext {
                let file_ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if file_ext != target {
                    continue;
                }
            }

            // 限制文件尺寸不超过 2MB，防止扫描超大二进制或数据文件
            if let Ok(meta) = path.metadata() {
                if meta.len() > 2 * 1024 * 1024 {
                    continue;
                }
            }

            search_single_file(root, &path, re, limit, matches)?;
        }
    }

    Ok(())
}

fn search_single_file(
    root: &Path,
    file_path: &Path,
    re: &Regex,
    limit: usize,
    matches: &mut Vec<CodeMatch>,
) -> Result<(), String> {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };

    let rel_path = match file_path.strip_prefix(root) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        Err(_) => file_path.to_string_lossy().to_string(),
    };

    let reader = BufReader::new(file);
    for (idx, line_res) in reader.lines().enumerate() {
        if matches.len() >= limit {
            break;
        }
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break, // 可能为二进制文件，直接跳过
        };

        if re.is_match(&line) {
            matches.push(CodeMatch {
                file: rel_path.clone(),
                line_number: idx + 1,
                line_content: line.trim().to_string(),
            });
        }
    }

    Ok(())
}

/// 按行范围切片读取代码文件内容，带防溢出安全截断保护
///
/// # 参数
/// - `file_path`: 本地代码文件路径
/// - `start_line`: 起始行号 (从 1 开始)
/// - `max_lines`: 最大读取行数 (硬锁定上限为 200 行)
pub fn read_workspace_slice(
    file_path: &Path,
    start_line: usize,
    max_lines: usize,
) -> Result<String, String> {
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_path.display()));
    }
    if !file_path.is_file() {
        return Err(format!("Path is not a regular file: {}", file_path.display()));
    }

    // 单文件尺寸安全检查 (限制最大 5MB)
    if let Ok(meta) = file_path.metadata() {
        if meta.len() > 5 * 1024 * 1024 {
            return Err(format!("File is too large to read safely (> 5MB): {}", file_path.display()));
        }
    }

    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {e}"))?;
    let reader = BufReader::new(file);

    let start = start_line.max(1);
    let limit = max_lines.clamp(1, 200);

    let mut selected_lines = Vec::new();
    let mut total_lines = 0;

    for (idx, line_res) in reader.lines().enumerate() {
        let current_line_num = idx + 1;
        total_lines += 1;

        let line = line_res.map_err(|e| format!("Failed to read line {current_line_num}: {e}"))?;

        if current_line_num >= start && selected_lines.len() < limit {
            selected_lines.push(format!("{current_line_num:4} | {line}"));
        }
    }

    if selected_lines.is_empty() {
        if start > total_lines {
            return Ok(format!(
                "// [Start line {start} exceeds total file lines ({total_lines})]"
            ));
        }
        return Ok(String::new());
    }

    let mut result = selected_lines.join("\n");
    if start + selected_lines.len() <= total_lines {
        result.push_str(&format!(
            "\n// ... [Truncated: showing lines {start} to {}, total file lines: {total_lines}]",
            start + selected_lines.len() - 1
        ));
    }

    Ok(result)
}

/// 解析接口契约规范文件 (`OpenAPI` JSON/YAML 或 `Protobuf` `.proto`)
///
/// 输出结构化的端点或服务接口描述 JSON 对象
pub fn inspect_api_spec(file_path: &Path) -> Result<serde_json::Value, String> {
    if !file_path.exists() {
        return Err(format!("Spec file does not exist: {}", file_path.display()));
    }

    let mut file = File::open(file_path).map_err(|e| format!("Failed to open spec file: {e}"))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read spec file: {e}"))?;

    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "proto" {
        return parse_proto_spec(&content);
    }

    // 优先尝试标准 JSON 解析 (OpenAPI / Swagger / Postman)
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
        if json_val.get("paths").is_some() {
            return parse_openapi_json(&json_val);
        }
        if let Some(item) = json_val.get("item") {
            if item.is_array() {
                return parse_postman_collection(&json_val);
            }
        }
    }

    // 回退到 OpenAPI YAML 行解析器
    parse_openapi_yaml(&content)
}

fn parse_proto_spec(content: &str) -> Result<serde_json::Value, String> {
    let service_re = match Regex::new(r"service\s+([A-Za-z0-9_]+)\s*\{([^}]+)\}") {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let rpc_re = match Regex::new(r"rpc\s+([A-Za-z0-9_]+)\s*\(\s*([A-Za-z0-9_.]+)\s*\)\s*returns\s*\(\s*([A-Za-z0-9_.]+)\s*\)") {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let mut services = Vec::new();

    for caps in service_re.captures_iter(content) {
        let service_name = caps.get(1).map_or("", |m| m.as_str());
        let body = caps.get(2).map_or("", |m| m.as_str());

        let mut methods = Vec::new();
        for rpc_caps in rpc_re.captures_iter(body) {
            let rpc_name = rpc_caps.get(1).map_or("", |m| m.as_str());
            let input_type = rpc_caps.get(2).map_or("", |m| m.as_str());
            let output_type = rpc_caps.get(3).map_or("", |m| m.as_str());

            methods.push(serde_json::json!({
                "method": rpc_name,
                "input_type": input_type,
                "output_type": output_type,
            }));
        }

        services.push(serde_json::json!({
            "service": service_name,
            "rpcs": methods,
        }));
    }

    Ok(serde_json::json!({
        "protocol": "grpc",
        "services": services,
    }))
}

fn parse_openapi_json(json_val: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut endpoints = Vec::new();
    let paths = match json_val.get("paths").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return Ok(serde_json::json!({ "format": "openapi", "endpoints": [] })),
    };

    let valid_methods = ["get", "post", "put", "delete", "patch", "options", "head"];

    for (path_key, path_item) in paths {
        if let Some(path_obj) = path_item.as_object() {
            for (method_key, operation) in path_obj {
                let method_lower = method_key.to_lowercase();
                if !valid_methods.contains(&method_lower.as_str()) {
                    continue;
                }

                let summary = operation
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .or_else(|| operation.get("description").and_then(|s| s.as_str()))
                    .unwrap_or("");

                let operation_id = operation
                    .get("operationId")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                let params = operation
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::Value::Array(Vec::new()));

                let request_body = operation.get("requestBody").cloned();

                endpoints.push(serde_json::json!({
                    "path": path_key,
                    "method": method_lower,
                    "summary": summary,
                    "operation_id": operation_id,
                    "parameters": params,
                    "request_body": request_body,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "format": "openapi",
        "endpoints": endpoints,
    }))
}

fn parse_postman_collection(json_val: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut endpoints = Vec::new();
    if let Some(items) = json_val.get("item").and_then(|i| i.as_array()) {
        collect_postman_items(items, &mut endpoints);
    }
    Ok(serde_json::json!({
        "format": "postman",
        "endpoints": endpoints,
    }))
}

fn collect_postman_items(items: &[serde_json::Value], endpoints: &mut Vec<serde_json::Value>) {
    for item in items {
        if let Some(sub_items) = item.get("item").and_then(|i| i.as_array()) {
            collect_postman_items(sub_items, endpoints);
        } else if let Some(req) = item.get("request") {
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let method = req
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("GET")
                .to_lowercase();
            let url = if let Some(raw_url) = req.get("url").and_then(|u| u.get("raw")).and_then(|r| r.as_str()) {
                raw_url.to_string()
            } else if let Some(url_str) = req.get("url").and_then(|u| u.as_str()) {
                url_str.to_string()
            } else {
                String::new()
            };

            endpoints.push(serde_json::json!({
                "path": url,
                "method": method,
                "summary": name,
            }));
        }
    }
}

fn parse_openapi_yaml(content: &str) -> Result<serde_json::Value, String> {
    let mut endpoints = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_method: Option<String> = None;
    let mut current_summary: Option<String> = None;
    let mut in_paths = false;

    let path_re = match Regex::new(r"^\s{2,4}(/[^:]+):\s*$") {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let method_re = match Regex::new(r"^\s{4,8}(get|post|put|delete|patch|options|head):\s*$") {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let summary_re = match Regex::new(r#"^\s{6,12}(?:summary|description):\s*['"]?([^'"]+)['"]?\s*$"#) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let flush_endpoint = |endpoints: &mut Vec<serde_json::Value>,
                          path: &Option<String>,
                          method: &Option<String>,
                          summary: &Option<String>| {
        if let (Some(p), Some(m)) = (path, method) {
            endpoints.push(serde_json::json!({
                "path": p,
                "method": m,
                "summary": summary.clone().unwrap_or_default(),
            }));
        }
    };

    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed == "paths:" {
            in_paths = true;
            continue;
        }

        if in_paths {
            if !line.starts_with(' ') && !trimmed.is_empty() {
                // 离开 paths 区块
                flush_endpoint(&mut endpoints, &current_path, &current_method, &current_summary);
                break;
            }

            if let Some(caps) = path_re.captures(trimmed) {
                flush_endpoint(&mut endpoints, &current_path, &current_method, &current_summary);
                current_method = None;
                current_summary = None;
                current_path = caps.get(1).map(|m| m.as_str().to_string());
            } else if let Some(caps) = method_re.captures(trimmed) {
                flush_endpoint(&mut endpoints, &current_path, &current_method, &current_summary);
                current_summary = None;
                current_method = caps.get(1).map(|m| m.as_str().to_lowercase());
            } else if let Some(caps) = summary_re.captures(trimmed) {
                if current_summary.is_none() {
                    current_summary = caps.get(1).map(|m| m.as_str().to_string());
                }
            }
        }
    }

    flush_endpoint(&mut endpoints, &current_path, &current_method, &current_summary);

    Ok(serde_json::json!({
        "format": "openapi",
        "endpoints": endpoints,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use tempfile::tempdir;

    #[test]
    fn test_workspace_scan_and_detect_tech() {
        let dir = match tempdir() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create tempdir: {e}"),
        };
        let root = dir.path();

        let pkg_json = r#"{ "name": "mock-app", "dependencies": { "express": "^4.18.0" } }"#;
        if let Err(e) = write(root.join("package.json"), pkg_json) {
            panic!("Failed to write package.json: {e}");
        }
        if let Err(e) = write(root.join("tsconfig.json"), "{}") {
            panic!("Failed to write tsconfig.json: {e}");
        }
        if let Err(e) = create_dir_all(root.join("src/controllers")) {
            panic!("Failed to create controllers dir: {e}");
        }
        if let Err(e) = create_dir_all(root.join("node_modules/express")) {
            panic!("Failed to create node_modules: {e}");
        }
        if let Err(e) = write(root.join("openapi.yaml"), "paths:\n  /users:\n    get:\n      summary: Get all users\n") {
            panic!("Failed to write openapi.yaml: {e}");
        }

        let scan = match scan_workspace(root, 4) {
            Ok(s) => s,
            Err(e) => panic!("scan_workspace failed: {e}"),
        };

        assert_eq!(
            scan.tech_stack.as_deref(),
            Some("Node.js (TypeScript / Express)")
        );
        assert!(scan.config_files.contains(&"package.json".to_string()));
        assert!(scan.config_files.contains(&"tsconfig.json".to_string()));
        assert!(scan.spec_files.contains(&"openapi.yaml".to_string()));
        assert!(scan.key_directories.contains(&"src/controllers".to_string()));
        // node_modules 不应该出现在 key_directories
        assert!(!scan.key_directories.iter().any(|d| d.contains("node_modules")));
    }

    #[test]
    fn test_search_and_slice() {
        let dir = match tempdir() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create tempdir: {e}"),
        };
        let root = dir.path();

        let code = "const express = require('express');\nconst app = express();\napp.get('/api/v1/health', (req, res) => res.send('ok'));\napp.post('/api/v1/login', authHandler);\n";
        let file_path = root.join("routes.js");
        if let Err(e) = write(&file_path, code) {
            panic!("Failed to write routes.js: {e}");
        }

        let matches = match search_workspace_code(root, r"app\.(get|post)", Some("js"), 10) {
            Ok(m) => m,
            Err(e) => panic!("search_workspace_code failed: {e}"),
        };
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 3);
        assert_eq!(matches[1].line_number, 4);

        let slice = match read_workspace_slice(&file_path, 2, 2) {
            Ok(s) => s,
            Err(e) => panic!("read_workspace_slice failed: {e}"),
        };
        assert!(slice.contains("app = express()"));
        assert!(slice.contains("/api/v1/health"));
    }

    #[test]
    fn test_inspect_api_spec_openapi_and_proto() {
        let dir = match tempdir() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create tempdir: {e}"),
        };
        let root = dir.path();

        // 1. OpenAPI JSON
        let json_spec = r#"{
            "openapi": "3.0.0",
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets"
                    }
                }
            }
        }"#;
        let json_file = root.join("openapi.json");
        if let Err(e) = write(&json_file, json_spec) {
            panic!("Failed to write openapi.json: {e}");
        }
        let parsed_json = match inspect_api_spec(&json_file) {
            Ok(p) => p,
            Err(e) => panic!("inspect_api_spec json failed: {e}"),
        };
        assert_eq!(parsed_json["format"], "openapi");
        assert_eq!(parsed_json["endpoints"][0]["path"], "/pets");
        assert_eq!(parsed_json["endpoints"][0]["method"], "get");
        assert_eq!(parsed_json["endpoints"][0]["summary"], "List pets");

        // 2. Proto
        let proto_spec = r#"
        syntax = "proto3";
        service UserService {
            rpc GetUser (UserRequest) returns (UserResponse);
            rpc ListUsers (ListRequest) returns (ListResponse);
        }
        "#;
        let proto_file = root.join("service.proto");
        if let Err(e) = write(&proto_file, proto_spec) {
            panic!("Failed to write service.proto: {e}");
        }
        let parsed_proto = match inspect_api_spec(&proto_file) {
            Ok(p) => p,
            Err(e) => panic!("inspect_api_spec proto failed: {e}"),
        };
        assert_eq!(parsed_proto["protocol"], "grpc");
        assert_eq!(parsed_proto["services"][0]["service"], "UserService");
        assert_eq!(parsed_proto["services"][0]["rpcs"].as_array().map_or(0, |a| a.len()), 2);
    }
}
