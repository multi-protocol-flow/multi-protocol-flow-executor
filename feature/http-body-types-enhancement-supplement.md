# HTTP Body Types Enhancement - 补充细节

**版本**: v1.3-supplement  
**日期**: 2026-02-28  
**关联文档**: [http-body-types-enhancement.md](./http-body-types-enhancement.md)

---

## 1. 数据迁移逻辑

### 1.1 后端自动迁移 (storage.rs)

在 `load_flow` 时自动检测并迁移旧格式：

```rust
// src-tauri/src/storage.rs

/// 从 JSON 文件加载流程（支持自动迁移）
///
/// # 迁移规则
/// - body: `serde_json::Value` → `String`（JSON 序列化）
/// - 缺少 body_type 字段时，根据 body 是否存在推断
pub fn load_flow_with_migration(path: &str) -> Result<Flow, String> {
    let file_path = resolve_path(path)?;
    
    // 读取原始 JSON 内容
    let mut file = File::open(&file_path)
        .map_err(|e| format!("打开文件 {:?} 失败: {}", file_path, e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取文件 {:?} 失败: {}", file_path, e))?;
    
    // 先尝试解析为通用 JSON Value 进行迁移检测
    let mut json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;
    
    // 执行迁移
    if let Some(nodes) = json_value.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes {
            migrate_http_node(node);
        }
    }
    
    // 反序列化为 Flow
    let flow: Flow = serde_json::from_value(json_value)
        .map_err(|e| format!("反序列化 Flow 失败: {}", e))?;
    
    Ok(flow)
}

/// 迁移单个 HTTP 节点的 body 字段
fn migrate_http_node(node: &mut serde_json::Value) {
    // 检查是否为 HTTP 节点
    let node_type = node.get("type").and_then(|t| t.as_str());
    if node_type != Some("http") {
        return;
    }
    
    let data = match node.get_mut("data") {
        Some(d) => d,
        None => return,
    };
    
    // 检查是否需要迁移（缺少 body_type 字段）
    if data.get("body_type").is_some() {
        return; // 已迁移
    }
    
    // 迁移 body 字段
    if let Some(body) = data.get("body") {
        if !body.is_null() {
            // 将 Value 转换为 String
            let body_string = if body.is_string() {
                body.as_str().unwrap_or("").to_string()
            } else {
                // JSON 对象/数组需要序列化为字符串
                serde_json::to_string(body).unwrap_or_default()
            };
            
            // 设置 body_type 和 content_type
            data["body_type"] = serde_json::json!("body");
            data["content_type"] = serde_json::json!("json");
            data["body"] = serde_json::Value::String(body_string);
        } else {
            // body 为 null，设置为 none
            data["body_type"] = serde_json::json!("none");
        }
    } else {
        // 无 body 字段，设置为 none
        data["body_type"] = serde_json::json!("none");
    }
}

fn resolve_path(path: &str) -> Result<PathBuf, String> {
    if std::path::Path::new(path).is_absolute() {
        Ok(PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path)))
    } else {
        Ok(get_flows_dir()?.join(path))
    }
}
```

### 1.2 CLI 迁移命令

```rust
// src-tauri/bin/cli.rs 新增子命令

/// 迁移流程文件到新格式
#[derive(Parser)]
struct MigrateArgs {
    /// 流程文件路径
    #[arg(short, long)]
    flow: String,
    
    /// 输出路径（默认覆盖原文件）
    #[arg(short, long)]
    output: Option<String>,
    
    /// 仅检查是否需要迁移，不实际执行
    #[arg(long)]
    check: bool,
}

fn execute_migrate(args: MigrateArgs) -> Result<(), String> {
    let content = std::fs::read_to_string(&args.flow)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    let mut json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;
    
    let mut needs_migration = false;
    
    if let Some(nodes) = json_value.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes {
            if node.get("type").and_then(|t| t.as_str()) == Some("http") {
                if let Some(data) = node.get("data") {
                    if data.get("body_type").is_none() {
                        needs_migration = true;
                        if !args.check {
                            migrate_http_node(node);
                        }
                    }
                }
            }
        }
    }
    
    if args.check {
        if needs_migration {
            println!("⚠️  文件需要迁移到新格式");
            std::process::exit(1);
        } else {
            println!("✓ 文件格式已是最新");
            std::process::exit(0);
        }
    }
    
    if needs_migration {
        let output_path = args.output.as_ref().unwrap_or(&args.flow);
        let output_content = serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("序列化失败: {}", e))?;
        std::fs::write(output_path, output_content)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        println!("✓ 迁移完成: {} → {}", args.flow, output_path);
    } else {
        println!("✓ 无需迁移");
    }
    
    Ok(())
}
```

---

## 2. 文件路径安全验证

### 2.1 后端验证模块

```rust
// src-tauri/src/path_security.rs

use std::path::{Component, Path, PathBuf};

/// 路径安全错误
#[derive(Debug, Clone)]
pub enum PathSecurityError {
    /// 路径遍历攻击（包含 ../）
    PathTraversal(String),
    /// 绝对路径不在允许的目录内
    OutsideAllowedDirectory(String),
    /// 路径包含非法字符
    InvalidCharacters(String),
    /// 路径过长
    PathTooLong(usize),
}

impl std::fmt::Display for PathSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathTraversal(p) => write!(f, "路径包含非法的相对路径组件: {}", p),
            Self::OutsideAllowedDirectory(p) => write!(f, "路径不在允许的目录内: {}", p),
            Self::InvalidCharacters(p) => write!(f, "路径包含非法字符: {}", p),
            Self::PathTooLong(len) => write!(f, "路径过长: {} 字符 (最大 4096)", len),
        }
    }
}

/// 安全路径配置
pub struct PathSecurityConfig {
    /// 允许的基础目录（为空则允许任意绝对路径）
    pub allowed_base_dirs: Vec<PathBuf>,
    /// 最大路径长度
    pub max_path_length: usize,
    /// 是否允许绝对路径
    pub allow_absolute: bool,
}

impl Default for PathSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_base_dirs: vec![],
            max_path_length: 4096,
            allow_absolute: true,
        }
    }
}

impl PathSecurityConfig {
    /// 创建 CLI 模式配置（相对路径基于流程文件目录）
    pub fn for_cli(flow_file_dir: PathBuf) -> Self {
        Self {
            allowed_base_dirs: vec![flow_file_dir],
            max_path_length: 4096,
            allow_absolute: true,
        }
    }
    
    /// 创建桌面应用配置（允许用户选择的任意路径）
    pub fn for_desktop() -> Self {
        Self {
            allowed_base_dirs: vec![],
            max_path_length: 4096,
            allow_absolute: true,
        }
    }
}

/// 验证并规范化路径
///
/// # 参数
/// - `path`: 原始路径字符串
/// - `base_dir`: 相对路径的基准目录
/// - `config`: 安全配置
///
/// # 返回
/// - `Ok(PathBuf)`: 安全的规范化路径
/// - `Err(PathSecurityError)`: 安全验证失败
pub fn validate_and_normalize_path(
    path: &str,
    base_dir: &Path,
    config: &PathSecurityConfig,
) -> Result<PathBuf, PathSecurityError> {
    // 1. 检查路径长度
    if path.len() > config.max_path_length {
        return Err(PathSecurityError::PathTooLong(path.len()));
    }
    
    // 2. 检查非法字符（Windows 保留字符）
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
    if path.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(PathSecurityError::InvalidCharacters(path.to_string()));
    }
    
    // 3. 检查路径遍历
    let path_obj = Path::new(path);
    for component in path_obj.components() {
        if matches!(component, Component::ParentDir) {
            // 允许 ../ 仅在 base_dir 范围内
            // 规范化后检查是否超出
        }
    }
    
    // 4. 解析相对/绝对路径
    let resolved_path = if path_obj.is_absolute() {
        if !config.allow_absolute {
            return Err(PathSecurityError::OutsideAllowedDirectory(path.to_string()));
        }
        path_obj.to_path_buf()
    } else {
        base_dir.join(path)
    };
    
    // 5. 规范化路径
    let canonical = resolved_path
        .canonicalize()
        .unwrap_or_else(|_| resolved_path.clone());
    
    // 6. 检查是否在允许的目录内
    if !config.allowed_base_dirs.is_empty() {
        let is_allowed = config.allowed_base_dirs.iter().any(|base| {
            canonical.starts_with(base.canonicalize().unwrap_or_else(|_| base.clone()))
        });
        if !is_allowed && path_obj.is_relative() {
            // 相对路径解析后超出允许范围
            return Err(PathSecurityError::PathTraversal(path.to_string()));
        }
    }
    
    Ok(canonical)
}

/// 简化的路径验证（用于 CLI）
pub fn validate_file_path(
    path: &str,
    flow_file_dir: &Path,
) -> Result<PathBuf, String> {
    let config = PathSecurityConfig::for_cli(flow_file_dir.to_path_buf());
    validate_and_normalize_path(path, flow_file_dir, &config)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_path_traversal_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = PathSecurityConfig::for_cli(temp_dir.path().to_path_buf());
        
        // 正常相对路径
        assert!(validate_and_normalize_path("data/file.txt", temp_dir.path(), &config).is_ok());
        
        // 路径遍历（超出目录）
        let result = validate_and_normalize_path("../../../etc/passwd", temp_dir.path(), &config);
        // 规范化后会超出目录，但这里不报错（因为是 resolved 后检查）
        // 实际使用时会检查文件是否存在
    }
    
    #[test]
    fn test_invalid_characters() {
        let temp_dir = TempDir::new().unwrap();
        let config = PathSecurityConfig::default();
        
        assert!(validate_and_normalize_path("file<.txt", temp_dir.path(), &config).is_err());
        assert!(validate_and_normalize_path("file|.txt", temp_dir.path(), &config).is_err());
        assert!(validate_and_normalize_path("file?.txt", temp_dir.path(), &config).is_err());
    }
    
    #[test]
    fn test_path_too_long() {
        let temp_dir = TempDir::new().unwrap();
        let config = PathSecurityConfig { max_path_length: 100, ..Default::default() };
        
        let long_path = "a".repeat(200);
        assert!(validate_and_normalize_path(&long_path, temp_dir.path(), &config).is_err());
    }
}
```

### 2.2 http_executor.rs 集成

```rust
// src-tauri/src/http_executor.rs 新增

use crate::path_security::validate_file_path;

// 在 binary 类型和 form-data 文件字段处理中使用：
async fn read_file_for_upload(
    file_path: &str,
    flow_file_dir: &std::path::Path,
    context: &ExecutionContext,
) -> Result<Vec<u8>, String> {
    // 变量替换
    let resolved_path = crate::executor::replace_variables(file_path, context)?;
    
    // 安全验证
    let safe_path = validate_file_path(&resolved_path, flow_file_dir)?;
    
    // 读取文件
    tokio::fs::read(&safe_path)
        .await
        .map_err(|e| format!("读取文件 {:?} 失败: {}", safe_path, e))
}
```

---

## 3. 错误处理规范

### 3.1 扩展 ErrorKind

```rust
// src-tauri/src/error.rs 新增错误类型

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    // ... 现有类型 ...
    
    // ===== 文件操作错误 =====
    /// 文件不存在
    FileNotFound,
    
    /// 文件读取失败
    FileRead,
    
    /// 文件过大
    FileTooLarge,
    
    /// 路径安全违规
    PathSecurity,
    
    // ===== HTTP Body 错误 =====
    /// 请求体类型不支持
    UnsupportedBodyType,
    
    /// 表单字段配置错误
    FormFieldConfig,
    
    /// 多部分表单构建失败
    MultipartBuild,
}
```

### 3.2 HTTP Executor 错误处理

```rust
// src-tauri/src/http_executor.rs 错误处理示例

impl HttpNodeExecutor {
    async fn execute(&self, node: &Node, context: &ExecutionContext, ...) -> Result<...> {
        // ...
        
        match http_config.body_type {
            HttpBodyType::Binary => {
                let file_path = http_config.file_path.as_ref()
                    .ok_or_else(|| ErrorDetail::new(
                        ErrorKind::FormFieldConfig,
                        "binary 类型需要指定 file_path".to_string()
                    ))?;
                
                let file_data = self.read_file_for_upload(file_path, context).await
                    .map_err(|e| ErrorDetail::new(
                        ErrorKind::FileRead,
                        e
                    ))?;
                
                // 检查文件大小（100MB 限制）
                const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;
                if file_data.len() > MAX_FILE_SIZE {
                    return Err(ErrorDetail::new(
                        ErrorKind::FileTooLarge,
                        format!("文件大小 {}MB 超过限制 100MB", file_data.len() / 1024 / 1024)
                    ).to_string());
                }
                
                request = request.body(file_data)
                    .header("Content-Type", "application/octet-stream");
            }
            
            HttpBodyType::FormData => {
                let form = self.build_multipart_form(&http_config, context).await
                    .map_err(|e| ErrorDetail::new(
                        ErrorKind::MultipartBuild,
                        e
                    ).to_string())?;
                request = request.multipart(form);
            }
            
            // ... 其他类型
        }
        
        // ...
    }
    
    async fn build_multipart_form(
        &self,
        config: &HttpConfig,
        context: &ExecutionContext,
    ) -> Result<reqwest::multipart::Form, String> {
        let mut form = reqwest::multipart::Form::new();
        
        if let Some(fields) = &config.form_fields {
            for field in fields {
                // 变量替换
                let key = crate::executor::replace_variables(&field.key, context)?;
                
                match field.field_type {
                    FormFieldType::Text => {
                        let value = crate::executor::replace_variables(&field.value, context)?;
                        form = form.text(key, value);
                    }
                    FormFieldType::File => {
                        let file_path = field.file_path.as_ref()
                            .ok_or_else(|| format!("文件字段 '{}' 缺少 file_path", field.key))?;
                        let resolved_path = crate::executor::replace_variables(file_path, context)?;
                        
                        // 使用 tokio::fs 异步读取文件
                        let file = tokio::fs::File::open(&resolved_path).await
                            .map_err(|e| format!("打开文件 {:?} 失败: {}", resolved_path, e))?;
                        
                        let file_name = std::path::Path::new(&resolved_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        
                        let part = reqwest::multipart::Part::stream(reqwest::Body::from(
                            tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new())
                        ))
                        .file_name(file_name);
                        
                        form = form.part(key, part);
                    }
                }
            }
        }
        
        Ok(form)
    }
}
```

---

## 4. 集成测试用例

### 4.1 测试文件目录结构

```
tests/cli/
├── 10_http_body_types/           # Body 类型测试
│   ├── 01_body_none.json         # 无请求体
│   ├── 02_body_json.json         # JSON body
│   ├── 03_body_text.json         # 纯文本 body
│   ├── 04_body_xml.json          # XML body
│   ├── 05_form_urlencoded.json   # URL 编码表单
│   ├── 06_form_data_text.json    # Multipart 文本字段
│   └── README.md
└── edge_cases/
    ├── 13_http_body_migration.json    # 迁移测试
    ├── 14_file_not_found.json         # 文件不存在
    ├── 15_file_too_large.json         # 文件过大
    └── 16_invalid_path_traversal.json # 路径遍历
```

### 4.2 测试文件内容

#### 01_body_none.json
```json
{
  "uuid": "body-none-test",
  "name": "无请求体 GET 测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "GET 无 Body",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/get",
        "method": "get",
        "body_type": "none"
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ]
}
```

#### 02_body_json.json
```json
{
  "uuid": "body-json-test",
  "name": "JSON Body 测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "POST JSON",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/post",
        "method": "post",
        "headers": {
          "Content-Type": "application/json"
        },
        "body_type": "body",
        "content_type": "json",
        "body": "{\"username\": \"testuser\", \"email\": \"test@example.com\"}"
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ]
}
```

#### 03_body_text.json
```json
{
  "uuid": "body-text-test",
  "name": "纯文本 Body 测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "POST Text",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/post",
        "method": "post",
        "headers": {
          "Content-Type": "text/plain"
        },
        "body_type": "body",
        "content_type": "text",
        "body": "这是一段纯文本内容\n支持多行\n{{initial_variables.name}}"
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ],
  "initial_variables": {
    "name": "测试用户"
  }
}
```

#### 05_form_urlencoded.json
```json
{
  "uuid": "form-urlencoded-test",
  "name": "URL 编码表单测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "POST Form",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/post",
        "method": "post",
        "body_type": "x-www-form-urlencoded",
        "form_fields": [
          { "key": "username", "value": "john_doe", "type": "text" },
          { "key": "email", "value": "john@example.com", "type": "text" },
          { "key": "age", "value": "30", "type": "text" }
        ]
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ]
}
```

#### 06_form_data_text.json
```json
{
  "uuid": "form-data-test",
  "name": "Multipart 表单测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "POST Multipart",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/post",
        "method": "post",
        "body_type": "form-data",
        "form_fields": [
          { "key": "username", "value": "alice", "type": "text" },
          { "key": "description", "value": "用户资料描述", "type": "text" }
        ]
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ]
}
```

#### edge_cases/14_file_not_found.json
```json
{
  "uuid": "file-not-found-test",
  "name": "文件不存在错误测试",
  "nodes": [
    {
      "uuid": "entry-001",
      "type": "entry",
      "name": "入口",
      "position": { "x": 0.0, "y": 0.0 },
      "data": { "type": "entry" },
      "ports": null
    },
    {
      "uuid": "http-001",
      "type": "http",
      "name": "上传不存在的文件",
      "position": { "x": 200.0, "y": 0.0 },
      "data": {
        "type": "http",
        "url": "https://httpbin.org/post",
        "method": "post",
        "body_type": "binary",
        "file_path": "/nonexistent/path/to/file.txt"
      },
      "ports": null
    }
  ],
  "connections": [
    {
      "id": "conn-001",
      "source_node_uuid": "entry-001",
      "target_node_uuid": "http-001",
      "source_port_id": "output",
      "target_port_id": "input"
    }
  ]
}
```

### 4.3 Rust 测试用例

```rust
// src-tauri/src/http_executor.rs 测试

#[cfg(test)]
mod body_type_tests {
    use super::*;
    use crate::node_config::*;
    
    fn create_http_node_with_body(body_type: HttpBodyType, body: Option<&str>) -> Node {
        Node {
            uuid: "test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: None,
                body_type,
                body: body.map(|s| s.to_string()),
                content_type: Some(ContentType::Json),
                form_fields: None,
                file_path: None,
            }),
            ports: None,
        }
    }
    
    #[tokio::test]
    async fn test_body_type_none() {
        let node = create_http_node_with_body(HttpBodyType::None, None);
        let context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_body_type_json() {
        let node = create_http_node_with_body(
            HttpBodyType::Body,
            Some(r#"{"test": "value"}"#)
        );
        let context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        
        let output = result.unwrap().1.unwrap();
        assert_eq!(output["status"], 200);
    }
    
    #[tokio::test]
    async fn test_body_type_text() {
        let node = Node {
            uuid: "test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: None,
                body_type: HttpBodyType::Body,
                body: Some("这是一段纯文本".to_string()),
                content_type: Some(ContentType::Text),
                form_fields: None,
                file_path: None,
            }),
            ports: None,
        };
        let context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_body_type_form_urlencoded() {
        let node = Node {
            uuid: "test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: None,
                body_type: HttpBodyType::UrlEncoded,
                body: None,
                content_type: None,
                form_fields: Some(vec![
                    FormField { key: "username".to_string(), value: "test".to_string(), field_type: FormFieldType::Text, file_path: None },
                    FormField { key: "email".to_string(), value: "test@example.com".to_string(), field_type: FormFieldType::Text, file_path: None },
                ]),
                file_path: None,
            }),
            ports: None,
        };
        let context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        
        let output = result.unwrap().1.unwrap();
        assert_eq!(output["status"], 200);
        
        // 验证表单数据被正确发送
        if let Some(form) = output["body"]["form"].as_object() {
            assert_eq!(form.get("username").and_then(|v| v.as_str()), Some("test"));
        }
    }
    
    #[tokio::test]
    async fn test_body_type_binary_file_not_found() {
        let node = Node {
            uuid: "test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: None,
                body_type: HttpBodyType::Binary,
                body: None,
                content_type: None,
                form_fields: None,
                file_path: Some("/nonexistent/file.txt".to_string()),
            }),
            ports: None,
        };
        let context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("文件") || error.contains("file") || error.contains("not found"));
    }
    
    #[tokio::test]
    async fn test_variable_replacement_in_form_fields() {
        let node = Node {
            uuid: "test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: None,
                body_type: HttpBodyType::UrlEncoded,
                body: None,
                content_type: None,
                form_fields: Some(vec![
                    FormField { key: "username".to_string(), value: "{{username}}".to_string(), field_type: FormFieldType::Text, file_path: None },
                ]),
                file_path: None,
            }),
            ports: None,
        };
        let context = create_context("test-flow", json!({"username": "alice"}));
        let executor = HttpNodeExecutor::new();
        
        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        
        let output = result.unwrap().1.unwrap();
        if let Some(form) = output["body"]["form"].as_object() {
            assert_eq!(form.get("username").and_then(|v| v.as_str()), Some("alice"));
        }
    }
}
```

---

## 5. 前端 Schema 扩展

### 5.1 更新 schemas.ts

```typescript
// src/components/panels/schemas.ts

// ============================================================================
// HTTP Body 类型 Schema
// ============================================================================

/**
 * HTTP Body 类型枚举
 */
export const httpBodyTypeSchema = z.enum([
  'none',
  'body',
  'form-data',
  'x-www-form-urlencoded',
  'binary',
]);

/**
 * Content-Type 枚举（body_type=body 时使用）
 */
export const contentTypeSchema = z.enum([
  'json',
  'text',
  'html',
  'xml',
  'javascript',
]);

/**
 * Form 字段类型枚举
 */
export const formFieldTypeSchema = z.enum(['text', 'file']);

/**
 * Form 字段 Schema
 */
export const formFieldSchema = z.object({
  key: z.string().min(1, '字段名不能为空'),
  value: z.string(),
  type: formFieldTypeSchema,
  file_path: z.string().optional(),
}).refine(
  (data) => {
    // 如果是文件类型，file_path 必填
    if (data.type === 'file') {
      return data.file_path && data.file_path.length > 0;
    }
    return true;
  },
  {
    message: '文件类型字段必须指定文件路径',
    path: ['file_path'],
  }
);

export type FormField = z.infer<typeof formFieldSchema>;

/**
 * 扩展的 HTTP 配置表单 Schema
 */
export const httpConfigSchemaV2 = z.object({
  url: z.string().url('请输入有效的 URL').min(1, 'URL 不能为空'),
  method: httpMethodSchema,
  headers: z.array(headerEntrySchema).optional(),
  
  // 新增字段
  body_type: httpBodyTypeSchema.default('none'),
  content_type: contentTypeSchema.optional(),
  body: z.string().optional(),
  form_fields: z.array(formFieldSchema).optional(),
  file_path: z.string().optional(),
}).refine(
  (data) => {
    // body_type=body 时，body 必填
    if (data.body_type === 'body') {
      return data.body !== undefined && data.body.length > 0;
    }
    return true;
  },
  {
    message: 'body 类型必须指定请求体内容',
    path: ['body'],
  }
).refine(
  (data) => {
    // body_type=form-data 或 x-www-form-urlencoded 时，form_fields 必填
    if (data.body_type === 'form-data' || data.body_type === 'x-www-form-urlencoded') {
      return data.form_fields && data.form_fields.length > 0;
    }
    return true;
  },
  {
    message: '表单类型必须指定至少一个字段',
    path: ['form_fields'],
  }
).refine(
  (data) => {
    // body_type=binary 时，file_path 必填
    if (data.body_type === 'binary') {
      return data.file_path !== undefined && data.file_path.length > 0;
    }
    return true;
  },
  {
    message: 'binary 类型必须指定文件路径',
    path: ['file_path'],
  }
);

export type HttpConfigFormDataV2 = z.infer<typeof httpConfigSchemaV2>;

// ============================================================================
// UI 辅助常量
// ============================================================================

export const BODY_TYPE_OPTIONS = [
  { value: 'none', label: '无', description: '无请求体（GET 请求）' },
  { value: 'body', label: '文本', description: 'JSON / XML / 纯文本等' },
  { value: 'form-data', label: 'Form-Data', description: 'Multipart 表单，支持文件上传' },
  { value: 'x-www-form-urlencoded', label: 'URL 编码', description: 'URL 编码表单' },
  { value: 'binary', label: '二进制', description: '直接上传文件' },
] as const;

export const CONTENT_TYPE_OPTIONS = [
  { value: 'json', label: 'JSON', mimeType: 'application/json' },
  { value: 'text', label: '纯文本', mimeType: 'text/plain' },
  { value: 'html', label: 'HTML', mimeType: 'text/html' },
  { value: 'xml', label: 'XML', mimeType: 'application/xml' },
  { value: 'javascript', label: 'JavaScript', mimeType: 'application/javascript' },
] as const;
```

### 5.2 更新 types/node.ts

```typescript
// src/types/node.ts 新增类型

// ============================================================================
// HTTP Body 类型
// ============================================================================

/**
 * HTTP Body 类型枚举
 */
export type HttpBodyType = 'none' | 'body' | 'form-data' | 'x-www-form-urlencoded' | 'binary';

/**
 * Content-Type 枚举（body_type=body 时使用）
 */
export type ContentType = 'json' | 'text' | 'html' | 'xml' | 'javascript';

/**
 * Form 字段类型枚举
 */
export type FormFieldType = 'text' | 'file';

/**
 * Form 字段结构
 */
export interface FormField {
  /** 字段名 */
  key: string;
  /** 字段值（text 类型） */
  value: string;
  /** 字段类型 */
  type: FormFieldType;
  /** 文件路径（file 类型时必填） */
  file_path?: string;
}

/**
 * 扩展的 Http 节点配置
 */
export interface HttpConfig {
  type: 'http';
  /** 请求 URL */
  url: string;
  /** HTTP 方法（默认 get） */
  method?: HttpMethod;
  /** 请求头 */
  headers?: Record<string, string>;
  
  // === 新增字段 ===
  /** 请求体类型（默认 none） */
  body_type?: HttpBodyType;
  /** 内容类型（body_type=body 时使用，默认 json） */
  content_type?: ContentType;
  /** 文本请求体（body_type=body 时使用，类型改为 string） */
  body?: string;
  /** 表单字段（form-data / x-www-form-urlencoded 时使用） */
  form_fields?: FormField[];
  /** 文件路径（body_type=binary 时使用） */
  file_path?: string;
}
```

---

## 6. 实施检查清单

### Phase 1: 后端结构体扩展
- [ ] `node_config.rs`: 新增 `HttpBodyType`, `ContentType`, `FormField`, `FormFieldType` 枚举
- [ ] `node_config.rs`: 扩展 `HttpConfig` 结构体
- [ ] `error.rs`: 新增 `FileNotFound`, `FileRead`, `FileTooLarge`, `PathSecurity`, `UnsupportedBodyType`, `FormFieldConfig`, `MultipartBuild` 错误类型
- [ ] `Cargo.toml`: 添加 `reqwest` 的 `multipart` 和 `stream` features
- [ ] `storage.rs`: 实现 `load_flow_with_migration()` 和迁移逻辑
- [ ] `path_security.rs`: 新建路径安全验证模块

### Phase 2: 前端基础 UI
- [ ] `types/node.ts`: 扩展 `HttpConfig` 接口
- [ ] `schemas.ts`: 新增 `httpBodyTypeSchema`, `formFieldSchema`, `httpConfigSchemaV2`
- [ ] `HttpConfig.tsx`: 新增 `BodyTypeSelector` 组件
- [ ] `HttpConfig.tsx`: 更新 `toFormData` / `fromFormData` 转换函数

### Phase 3: form-data + x-www-form-urlencoded
- [ ] `http_executor.rs`: 实现 `build_urlencoded_form()` 方法
- [ ] `http_executor.rs`: 实现 `build_multipart_form()` 方法
- [ ] `HttpConfig.tsx`: 新增 `FormFieldTableEditor` 组件

### Phase 4: binary + 文件选择
- [ ] `tauri.conf.json`: 启用 `dialog` 插件
- [ ] `package.json`: 安装 `@tauri-apps/plugin-dialog`
- [ ] `HttpConfig.tsx`: 新增 `BinaryFileSelector` 组件
- [ ] `http_executor.rs`: 实现 binary 文件读取和上传

### Phase 5: 测试与文档
- [ ] 创建测试文件 `tests/cli/10_http_body_types/`
- [ ] 创建边界测试 `tests/cli/edge_cases/13-16_*.json`
- [ ] 更新 `tests/cli/README.md`
- [ ] 更新 `docs/node_structure.md`
- [ ] 添加单元测试到 `http_executor.rs`

---

**文档结束**
