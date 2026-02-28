# HTTP 节点请求体类型增强方案

**版本**: v1.3  
**日期**: 2026-02-28  
**状态**: 设计中

---

## 1. 概述

### 1.1 背景

当前 HTTP 节点仅支持 JSON 格式的请求体，无法满足实际场景中多样化的请求格式需求，如表单提交、文件上传等。

### 1.2 目标

扩展 HTTP 节点，支持以下请求体类型：

| 类型 | 说明 | 适用场景 |
|------|------|----------|
| `none` | 无请求体 | GET 请求 |
| `body` | 文本数据（含 JSON） | API 调用、XML、HTML、纯文本 |
| `form-data` | 多部分表单数据 | 文件上传、混合数据 |
| `x-www-form-urlencoded` | URL 编码表单 | 简单表单提交 |
| `binary` | 二进制文件 | 文件直接上传 |
---

## 2. 现状分析

### 2.1 前端 (src/components/panels/HttpConfig.tsx)

- 仅提供 Textarea 输入 JSON body
- 无请求体类型选择
- 无文件选择功能

### 2.2 后端 (src-tauri/src/node_config.rs)

```rust
pub struct HttpConfig {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,  // 仅支持 JSON
}
```

### 2.3 执行器 (src-tauri/src/http_executor.rs)

- 直接序列化 body 为字符串发送
- 未根据 Content-Type 调整发送方式

---

## 3. 数据结构设计

### 3.1 请求体类型枚举

```
none | body | form-data | x-www-form-urlencoded | binary
```
### 3.2 请求体配置结构

| body_type | 使用字段 | 说明 |
|-----------|----------|------|
| `none` | - | 无请求体 |
| `body` | `body` + `content_type` | 文本数据，通过 content_type 区分格式 |
| `form-data` | `form_fields` | 多部分表单，支持文件 |
| `x-www-form-urlencoded` | `form_fields` | URL 编码表单 |
| `binary` | `file_path` | 二进制文件 |
### 3.3 Content-Type 枚举（body_type=body 时使用）

| 值 | Content-Type | UI 编辑器 |
|----|--------------|----------|
| `json` | application/json | JSON 格式化 + 语法高亮 + 校验 |
| `text` | text/plain | 纯文本 |
| `html` | text/html | HTML 语法高亮 |
| `xml` | application/xml | XML 格式化 + 语法高亮 |
| `javascript` | application/javascript | JS 语法高亮 |

### 3.4 FormField 结构（form-data / x-www-form-urlencoded）

| 字段 | 类型 | 说明 |
|------|------|------|
| `key` | String | 字段名 |
| `value` | String | 字段值 |
| `type` | `text` \| `file` | 字段类型（file 仅 form-data） |
| `file_path` | Option\<String\> | 文件路径（type=file 时） |

---

## 4. 前端改动

### 4.1 类型定义 (src/types/node.ts)

- 扩展 `HttpConfig` 接口，新增 body_type、content_type、form_fields、file_path
- 新增 `HttpBodyType`、`FormField`、`ContentType` 类型
- **保留** `body` 字段（body_type=body 时使用，类型改为 `string`）

### 4.2 表单 Schema (src/components/panels/schemas.ts)

- 更新 `httpConfigSchema` 支持新字段
- 新增 `formFieldSchema`

### 4.3 配置面板 (src/components/panels/HttpConfig.tsx)

**新增组件结构：**

```
├── BodyTypeSelector        # 请求体类型选择器
├── TextBodyEditor          # body 编辑器
│   ├── ContentTypeSelector # 内容类型选择（json/text/html/xml/js）
│   └── CodeEditor          # 根据 content_type 提供格式化/高亮/校验
├── FormDataTableEditor     # form-data / x-www-form-urlencoded 编辑器
│   ├── 添加/删除字段
│   ├── 字段类型切换（text/file）
│   └── 文件选择按钮（Tauri API）
└── BinaryFileSelector      # 二进制文件选择器
```

### 4.4 文件选择集成

- 使用 `@tauri-apps/plugin-dialog` 的 `open` API 选择文件

### 4.5 代码编辑器策略

| Phase | 实现方式 | 说明 |
|-------|----------|------|
| Phase 2 | Textarea + JSON 校验 | 轻量级，利用现有组件 |
| 后续迭代 | 按需加载 Monaco | 仅 content_type=json 时懒加载 |

**说明**：避免引入 Monaco/CodeMirror 导致包体积膨胀（Monaco ~2MB），Phase 2 使用现有 Textarea 组件配合简单的 JSON 格式校验即可满足需求。
---

## 5. 后端改动

### 5.1 node_config.rs

**HttpConfig 结构调整：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `body_type` | `HttpBodyType` | 请求体类型（默认 none） |
| `body` | `Option<String>` | 文本数据（body_type=body 时） |
| `content_type` | `Option<ContentType>` | 内容类型（body_type=body 时，默认 json） |
| `file_path` | `Option<String>` | 文件路径（body_type=binary 时） |

**body 字段变更**：`Option<serde_json::Value>` → `Option<String>`

### 5.2 http_executor.rs

**执行逻辑调整：**

```
根据 body_type 分发：
  none                → 不添加请求体
  body                → 发送 body，根据 content_type 设置 Content-Type
  form-data           → 构建 multipart/form-data，处理文件字段
  x-www-form-urlencoded → 构建 application/x-www-form-urlencoded
  binary              → 读取文件，发送二进制数据
```

**文件处理：**
- 使用 `std::fs::read()` 读取文件
- 文件路径支持变量替换
- 读取失败返回执行错误

### 5.3 Content-Type 处理

| body_type | Content-Type 来源 |
|-----------|-------------------|
| body | 用户选择（json/text/html/xml/javascript） |
| form-data | 自动生成 multipart/form-data; boundary=... |
| x-www-form-urlencoded | application/x-www-form-urlencoded |
| binary | application/octet-stream |

用户可通过 headers 手动覆盖（高级用法）。

### 5.4 CLI 文件路径解析

#### 背景

当 `body_type` 为 `binary` 或 `form-data` 包含文件字段时，文件路径可能为相对路径。
CLI 模式下需要相对于流程文件所在目录解析。

#### context.rs 改动

在 `ExecutionContext` 中添加 `flow_file_path` 字段：

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub flow_uuid: String,
    pub current_node_uuid: Option<String>,
    pub variables: VariableStorage,
    pub node_reports: Vec<NodeExecutionReport>,
    pub flow_trigger_time: u64,
    
    /// 流程文件路径（CLI 模式下用于解析相对路径）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_file_path: Option<PathBuf>,
}
```

新增路径解析方法：

```rust
impl ExecutionContext {
    /// 解析文件路径（相对路径基于流程文件目录）
    pub fn resolve_file_path(&self, file_path: &str) -> PathBuf {
        let path = PathBuf::from(file_path);
        if path.is_absolute() {
            path
        } else if let Some(flow_path) = &self.flow_file_path {
            flow_path.parent()
                .unwrap_or(std::path::Path::new("."))
                .join(file_path)
        } else {
            path
        }
    }
}
```

#### cli.rs 改动

在调用 `execute_flow` 时传递流程文件路径：

```rust
use std::path::PathBuf;

async fn run() -> Result<(), String> {
    // ... 现有代码 ...

    // 加载流程
    let flow_path_buf = PathBuf::from(&flow_path);
    let flow = load_flow(&flow_path).map_err(|e| format!("加载流程失败: {}", e))?;

    // 创建带文件路径的执行器
    let executor = FlowExecutor::new().with_flow_file_path(flow_path_buf);
    let result: ExecutionResult = executor.execute_flow(&flow, None).await;
    
    // ... 现有代码 ...
}
```

#### executor.rs 改动

`FlowExecutor` 需要新增字段和方法：

```rust
pub struct FlowExecutor {
    flow_file_path: Option<PathBuf>,
}

impl FlowExecutor {
    pub fn new() -> Self {
        Self { flow_file_path: None }
    }
    
    /// 设置流程文件路径（CLI 模式使用）
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }
    
    /// 创建执行上下文时传递文件路径
    fn create_context(&self, flow: &Flow) -> ExecutionContext {
        let mut ctx = ExecutionContext::new(flow.uuid.clone(), /* ... */);
        ctx.flow_file_path = self.flow_file_path.clone();
        ctx
    }
}
```

#### http_executor.rs 改动

在处理 `binary` 和 `form-data` 文件时使用 `resolve_file_path`：

```rust
// binary 类型
let resolved_path = context.resolve_file_path(&file_path);
let file_content = tokio::fs::read(&resolved_path).await
    .map_err(|e| format!("读取文件失败 ({}): {}", resolved_path.display(), e))?;

// form-data 文件字段
let resolved_path = context.resolve_file_path(field.file_path.as_ref().unwrap());
let part = reqwest::multipart::Part::file(&resolved_path).await
    .map_err(|e| format!("读取文件失败 ({}): {}", resolved_path.display(), e))?;
```

---

## 6. 变量替换

**支持的变量替换位置：**

| 位置 | 支持 | 说明 |
|------|------|------|
| URL | ✓ | 请求地址中的动态参数 |
| Headers 值 | ✓ | 请求头中的动态值 |
| body（body 类型） | ✓ | 请求体文本中的变量 |
| form_fields[].value | ✓ | 表单字段值 |
| form_fields[].file_path | ✓ | 表单文件字段路径 |
| file_path（binary 类型） | ✓ | 二进制文件路径 |

---

## 7. CLI 兼容性

### 7.1 流程文件格式

- `body` 字段类型从 `Value` 改为 `String`，旧 JSON 格式需转为字符串
- 其他新增字段无兼容问题

### 7.2 文件路径处理

CLI 模式下：
- 文件路径相对于流程文件所在目录解析
- 支持绝对路径

---

## 8. 依赖变更

### 8.1 Cargo.toml

```toml
[dependencies]
# reqwest 添加 multipart 和 stream 支持
reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }

# Tauri 文件选择对话框
tauri-plugin-dialog = "2"
```

### 8.2 capabilities/default.json

Tauri 2.x 使用 capabilities 系统管理权限，而非 `tauri.conf.json` 中的 `plugins` 字段。

修改 `src-tauri/capabilities/default.json`：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default"
  ]
}
```

**注意**：`tauri.conf.json` 无需修改，不需要添加 `plugins` 字段。

### 8.3 package.json

```bash
npm install @tauri-apps/plugin-dialog
```

---

## 9. 影响范围

| 文件/模块 | 改动类型 |
|-----------|----------|
| `src/types/node.ts` | 扩展类型，body 改为 string |
| `src/components/panels/HttpConfig.tsx` | 重构 |
| `src/components/panels/schemas.ts` | 扩展 schema |
| `src-tauri/src/node_config.rs` | 扩展结构体 |
| `src-tauri/src/http_executor.rs` | 重构执行逻辑 |
| `src-tauri/src/context.rs` | 新增 flow_file_path + resolve_file_path |
| `src-tauri/src/executor.rs` | 新增 flow_file_path 支持 |
| `src-tauri/bin/cli.rs` | 传递 flow_file_path |
| `src-tauri/Cargo.toml` | 添加依赖 |
| `src-tauri/capabilities/default.json` | 添加 dialog 权限 |
| `docs/node_structure.md` | 更新文档 |

---
## 10. 风险与限制

### 10.1 文件大小

- 大文件上传可能导致内存问题
- 建议：前端提示文件大小限制（如 100MB）
- 后端使用 `reqwest::multipart::Part::file()` 流式读取，避免一次性加载到内存

### 10.2 Tauri 沙盒

- 桌面应用：文件访问无限制
- 打包后：需确认目标平台文件系统权限

### 10.3 reqwest multipart 注意事项

- `Part::file()` 是异步的，需使用 `tokio::fs::File`
- 文件不存在时返回错误，需妥善处理并返回有意义的错误信息

---
## 11. 实施计划

| 阶段 | 内容 | 优先级 |
|------|------|--------|
| Phase 1 | 后端结构体扩展 + body 类型 + 依赖添加 | 高 |
| Phase 2 | 前端类型选择器 + Textarea body 编辑器 | 高 |
| Phase 3 | form-data + x-www-form-urlencoded 支持 | 中 |
| Phase 4 | binary 支持 + 文件选择 UI（Tauri dialog） | 中 |
| Phase 5 | 文档更新 + 测试补充 | 低 |

---
---

## 12. 相关文档

- `docs/node_structure.md` - 节点结构定义
- `docs/executor_architecture.md` - 执行器架构
- Tauri Dialog Plugin: https://v2.tauri.app/plugin/dialog/
- reqwest multipart: https://docs.rs/reqwest/latest/reqwest/multipart/index.html

---

**文档结束**
