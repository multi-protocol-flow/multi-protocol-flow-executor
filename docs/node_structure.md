# 后端节点设计结构

**版本**：v2.0

## 概述

后端采用 Rust 实现，使用 Tagged Union 模式实现强类型节点配置。整体采用有向无环图（DAG）模型。

---

## ID 管理机制

### ID 生成机制说明

系统使用 UUID 作为唯一标识符，由前端生成：

| 字段 | 类型 | 生成方 | 用途 | 示例 |
|------|------|--------|------|------|
| `uuid` | String | 前端（UUID v4） | 前端关联、连接引用、唯一标识 | `"550e8400-e29b-41d4-a716-446655440000"` |

### ID 使用规则

**节点**：`uuid` 用于前端状态管理和连接关联  
**连接**：`source_node_uuid`/`target_node_uuid` 引用节点的 `uuid`

---

## 核心数据结构

### 1. Flow（流程）

```rust
pub struct Flow {
    /// 流程唯一标识符（前端生成）
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 流程级别的初始变量
    #[serde(default)]
    pub initial_variables: Option<HashMap<String, serde_json::Value>>,
    /// 流程中的节点列表
    pub nodes: Vec<Node>,
    /// 流程中的连接列表
    pub connections: Vec<Connection>,
}
```
 

### 2. Node（节点）

```rust
pub struct Node {
    /// 节点唯一标识符（前端生成UUID，用于连接关联）
    pub uuid: String,
    /// 节点类型
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// 节点名称
    pub name: String,
    /// 节点位置
    pub position: NodePosition,
    /// 节点特定数据（强类型配置）
    pub data: NodeData,
    /// 端口数组（可选）
    pub ports: Option<Vec<Port>>,
}
```

**节点类型枚举：**
```rust
pub enum NodeType {
    Entry,        // 入口节点 - 流程入口（必需）
    Http,         // HTTP 请求
    Conditional,  // 条件分支
}
```

---

### 3. NodeData（节点配置）

使用 Tagged Union 实现强类型配置：

```rust
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NodeData {
    Entry(EntryConfig),
    Http(HttpConfig),
    Conditional(ConditionalConfig),
}
```

#### Entry 节点配置

```rust
pub struct EntryConfig {}
```

#### Http 节点配置

```rust
pub struct HttpConfig {
    pub url: String,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// 请求体类型
    #[serde(default)]
    pub body_type: HttpBodyType,
    /// Content-Type（仅 body_type=body 时使用）
    #[serde(default)]
    pub content_type: ContentType,
    /// 表单字段（仅 form-data/x-www-form-urlencoded 时使用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_fields: Option<Vec<FormField>>,
    /// 二进制文件路径（仅 body_type=binary 时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 原始请求体（仅 body_type=body 时使用）
    pub body: Option<String>,
}

pub enum HttpMethod {
    Get, Post, Put, Delete, Patch,
}

/// HTTP 请求体类型
pub enum HttpBodyType {
    None,                           // 无请求体
    Body,                           // 原始请求体
    FormData,                       // multipart/form-data
    XWwwFormUrlencoded,             // application/x-www-form-urlencoded
    Binary,                         // 二进制文件
}

/// Content-Type 枚举
pub enum ContentType {
    Json, Text, Html, Xml, Javascript,
}

/// 表单字段
pub struct FormField {
    pub key: String,                // 字段键名
    pub value: String,              // 字段值
    pub field_type: FormFieldType,  // 字段类型 (text/file)
    pub file_path: Option<String>,  // 文件路径（仅 file 类型）
}

pub enum FormFieldType {
    Text, File,
}
```

#### HTTP 节点字段说明

| 字段 | 类型 | 说明 | 适用条件 |
|------|------|------|----------|
| `body_type` | `HttpBodyType` | 请求体类型 | 必需，默认 `none` |
| `content_type` | `ContentType` | 内容类型 | 仅 `body_type=body` 时使用 |
| `form_fields` | `Vec<FormField>` | 表单字段列表 | 仅 `form-data` 或 `x-www-form-urlencoded` 时使用 |
| `file_path` | `String` | 二进制文件路径 | 仅 `body_type=binary` 时使用 |
| `body` | `String` | 原始请求体 | 仅 `body_type=body` 时使用 |

#### Content-Type 自动设置规则

| body_type | 自动设置的 Content-Type | 备注 |
|-----------|------------------------|------|
| `none` | 无 | GET 请求通常使用 |
| `body` | 根据 `content_type` 字段 | `json`/`text`/`html`/`xml`/`javascript` |
| `form-data` | `multipart/form-data` | 支持文件上传 |
| `x-www-form-urlencoded` | `application/x-www-form-urlencoded` | 简单表单 |
| `binary` | `application/octet-stream` | 二进制文件 |

**优先级说明**: 用户手动设置的 `Content-Type` header 优先于自动设置。

#### HTTP 节点使用示例

**JSON 请求体**:
```json
{
  "type": "http",
  "url": "https://api.example.com/users",
  "method": "post",
  "body_type": "body",
  "content_type": "json",
  "body": "{\"name\": \"{{initial_variables.username}}\"}"
}
```

**Form 表单（带文件上传）**:
```json
{
  "type": "http",
  "url": "https://api.example.com/upload",
  "method": "post",
  "body_type": "form-data",
  "form_fields": [
    { "key": "username", "value": "test", "field_type": "text" },
    { "key": "avatar", "value": "", "field_type": "file", "file_path": "./avatar.png" }
  ]
}
```

**URL-encoded 表单**:
```json
{
  "type": "http",
  "url": "https://api.example.com/login",
  "method": "post",
  "body_type": "x-www-form-urlencoded",
  "form_fields": [
    { "key": "username", "value": "{{initial_variables.user}}", "field_type": "text" },
    { "key": "password", "value": "{{initial_variables.pass}}", "field_type": "text" }
  ]
}
```

**二进制文件上传**:
```json
{
  "type": "http",
  "url": "https://api.example.com/files",
  "method": "put",
  "body_type": "binary",
  "file_path": "./data/export.csv"
}
```

**变量替换支持**: 所有字段值（`body`、`form_fields[].value`、`file_path`）均支持 `{{path.to.value}}` 语法引用变量。

#### Conditional 节点配置

```rust
pub struct ConditionalConfig {
    #[serde(default)]
    pub field_path: Option<String>,
    #[serde(default)]
    pub operator: Option<ConditionOperator>,
    #[serde(default)]
    pub expected_value: Option<serde_json::Value>,
    #[serde(default)]
    pub advanced_expression: Option<String>,
}

pub enum ConditionOperator {
    Equal, NotEqual, Greater, GreaterOrEqual,
    Less, LessOrEqual, Contains, StartsWith, EndsWith, Exists,
}
```

**字段组合规则与默认行为**：

| 条件类型 | 字段组合 | 说明 |
|---------|---------|------|
| **简单条件** | `field_path` + `operator` + `expected_value` | 使用简单字段比较：`input.{field_path} {operator} {expected_value}`<br/>示例：`user_age >= 18` |
| **高级表达式** | 仅 `advanced_expression` | 使用自定义表达式引擎评估，支持复杂逻辑（如 `user.age >= 18 AND user.status == "active"`） |
| **无效配置** | 任意字段缺失或组合错误 | 条件评估失败，节点返回 `Failed` 状态，流程终止 |

**优先级**：
- 如果 `advanced_expression` 有值，忽略简单条件字段（`field_path`、`operator`、`expected_value`）
- 如果 `advanced_expression` 为空，必须提供完整的简单条件（`field_path` + `operator` + `expected_value`）

**验证规则**：
- 简单条件：`field_path`、`operator`、`expected_value` 必须同时提供
- 高级表达式：`advanced_expression` 单独提供
- 不允许同时使用简单条件和高级表达式


#### SubFlow 节点配置

```rust
pub struct SubFlowConfig {
    /// 目标流程 UUID（GUI 模式工作区流程引用）
    pub target_flow_id: Option<String>,
    /// 目标流程文件路径（CLI / 相对文件路径引用）
    pub target_flow_path: Option<String>,
    /// 是否继承父流程所有变量（默认 true）
    pub inherit_all_variables: bool,
    /// 入参显式映射规则列表（父变量名/表达式 -> 子流程变量名）
    pub variable_mappings: Vec<VariableMapping>,
    /// 执行完毕后是否将子流程变量导出回父流程（默认 true）
    pub export_all_variables: bool,
    /// 出参显式映射规则列表（子流程变量名 -> 父流程变量名）
    pub output_mappings: Vec<VariableMapping>,
}

pub struct VariableMapping {
    /// 源变量名或表达式
    pub source: String,
    /// 目标变量名
    pub target: String,
}
```

**字段说明与校验规则**：

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `target_flow_id` | `Option<String>` | `None` | 工作区中目标流程 UUID |
| `target_flow_path` | `Option<String>` | `None` | 外部 `.mpf` / `.json` 文件路径 |
| `inherit_all_variables` | `bool` | `true` | 是否自动继承父流程上下文的所有全局/局部变量 |
| `variable_mappings` | `Vec<VariableMapping>` | `[]` | 显式变量重命名或参数传入（优先级高于自动继承） |
| `export_all_variables` | `bool` | `true` | 是否将子流程产生的变量全部回传给父流程 |
| `output_mappings` | `Vec<VariableMapping>` | `[]` | 显式出参回传命名规则（用于避免命名冲突） |

**验证规则**：
- `target_flow_id` 与 `target_flow_path` 至少提供一个有效引用。
- 流程加载与编译期执行 DAG 环路检测，直接自引用或间接循环引用将报错拦截。
- 调试期间支持单步步入（`F11` / `StepInto`）、单步步出（`Shift+F11` / `StepOut`）与 `DebugCallFrame` 调用栈帧追踪。
---

### 4. Connection（连接）

```rust
pub struct Connection {
    /// 源节点的 UUID（引用 node.uuid）
    pub source_node_uuid: String,
    /// 目标节点的 UUID（引用 node.uuid）
    pub target_node_uuid: String,
    /// 源端口 ID
    pub source_port_id: String,
    /// 目标端口 ID
    pub target_port_id: String,
}
```

**重要说明：**
- `source_node_uuid` 和 `target_node_uuid` 引用的是节点的 `uuid` 字段
- 这是为了让前端在创建临时节点时就能建立连接关系
- 连接关系完全由节点 UUID 维护

---

### 5. Port（端口）

```rust
pub struct Port {
    /// 端口唯一标识符
    pub id: String,
    /// 端口类型
    #[serde(rename = "type")]
    pub port_type: PortType,
    /// 端口标签（可选）
    pub label: Option<String>,
}

pub enum PortType {
    Input,   // 输入端口
    Output,  // 输出端口
}

// 注意：PortType 枚举值（Input/Output）与端口 ID 字符串表示是两个不同的概念：
// - PortType 枚举值：用于类型定义，大写形式（Input, Output）
// - 端口 ID 字符串：用于连接关联，小写形式（"in", "out"）或特殊名称（"true", "false"）
```

**端口类型说明：**

| 节点类型 | 输入端口 | 输出端口 |
|---------|---------|---------|
| Entry | 0 个 | 1 个（ID: `"out"`，类型: `PortType::Output`） |
| Http | 1 个（ID: `"in"`，类型: `PortType::Input`） | 1 个（ID: `"out"`，类型: `PortType::Output`） |
| Conditional | 1 个（ID: `"in"`，类型: `PortType::Input`） | 2 个（ID: `"true"`, `"false"`，类型: `PortType::Output`） |

**端口功能**：
- **Entry**：仅输出 `out`，流程起始点
- **Http**：输入 `in` 接收上游数据，输出 `out` 传递 HTTP 响应
- **Conditional**：输入 `in` 接收数据，根据条件选择 `true`/`false` 端口输出

---

## 节点关系

```
Flow
├── uuid: String        // 唯一标识符（前端生成）
├── nodes: Vec<Node>
│   ├── Entry (必需，唯一入口)
│   │   ├── uuid: String    // 前端生成UUID
│   │   └── ...
│   ├── Http
│   └── Conditional
└── connections: Vec<Connection>
    ├── source_node_uuid        // 引用源节点的 uuid
    └── target_node_uuid        // 引用目标节点的 uuid
```

**连接规则：**
1. 每个流程必须有且仅有一个 Entry 节点
2. Entry 节点必须至少有一个输出连接（支持多连接，返回所有下游节点）
3. 连接必须从 Output 端口指向 Input 端口（端口类型验证）
   - `source_port_id` 必须对应目标节点的 Output 类型端口
   - `target_port_id` 必须对应目标节点的 Input 类型端口
4. 不允许自环（节点不能连接到自己）
5. 连接通过节点的 `uuid` 建立关联，而非 `id`
6. Conditional 节点有两个输出端口（true/false），根据条件评估结果选择分支：
   - 条件为 true 时使用 true 端口的连接（true 端口无连接时流程终止）
   - 条件为 false 时使用 false 端口的连接（false 端口无连接时流程终止）
   - 与普通节点保持一致：匹配到的分支（true/false）无下游连接时流程终止



## Flow JSON 示例

### 简单流程（Entry → Http）

```json
{
  "uuid": "flow-uuid",
  "name": "用户注册流程",
  "nodes": [
    { "uuid": "node-001", "type": "entry", "data": {"type": "entry"} },
    { "uuid": "node-002", "type": "http", "data": {"type": "http", "url": "..."} }
  ],
  "connections": [
    { "source_node_uuid": "node-001", "target_node_uuid": "node-002", "source_port_id": "out", "target_port_id": "in" }
  ]
}
```

### 条件分支流程（Entry → Conditional → Http）

```json
{
  "nodes": [
    { "uuid": "n1", "type": "entry" },
    {
      "uuid": "n2",
      "type": "conditional",
      "data": {"type": "conditional", "field_path": "user_age", "operator": "GreaterOrEqual", "expected_value": 18},
      "ports": [{"id": "in"}, {"id": "true"}, {"id": "false"}]
    },
    { "uuid": "n3", "type": "http", "name": "成年人接口" },
    { "uuid": "n4", "type": "http", "name": "未成年人接口" }
  ],
  "connections": [
    {"source_node_uuid": "n1", "target_node_uuid": "n2", "source_port_id": "out", "target_port_id": "in"},
    {"source_node_uuid": "n2", "target_node_uuid": "n3", "source_port_id": "true", "target_port_id": "in"},
    {"source_node_uuid": "n2", "target_node_uuid": "n4", "source_port_id": "false", "target_port_id": "in"}
  ]
}
```

**Conditional 节点示例（包含两个输出端口）**：

```json
{
  "uuid": "flow-conditional-example",
  "name": "条件分支流程",
  "initial_variables": {
    "user_age": 25
  },
  "nodes": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "type": "entry",
      "name": "入口",
      "position": { "x": 100, "y": 100 },
      "data": { "type": "entry" },
      "ports": [
        { "id": "out", "type": "output" }
      ]
    },
    {
      "uuid": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "type": "conditional",
      "name": "年龄判断",
      "position": { "x": 400, "y": 100 },
      "data": {
        "type": "conditional",
        "field_path": "user_age",
        "operator": "GreaterOrEqual",
        "expected_value": 18
      },
      "ports": [
        { "id": "in", "type": "input" },
        { "id": "true", "type": "output" },
        { "id": "false", "type": "output" }
      ]
    },
    {
      "uuid": "9ba7b810-9dad-11d1-80b4-00c04fd430c9",
      "type": "http",
      "name": "成年人接口",
      "position": { "x": 700, "y": 50 },
      "data": {
        "type": "http",
        "url": "https://api.example.com/adult",
        "method": "post"
      }
    },
    {
      "uuid": "a1a7b810-9dad-11d1-80b4-00c04fd430ca",
      "type": "http",
      "name": "未成年人接口",
      "position": { "x": 700, "y": 150 },
      "data": {
        "type": "http",
        "url": "https://api.example.com/minor",
        "method": "post"
      }
    }
  ],
  "connections": [
    {
      "source_node_uuid": "550e8400-e29b-41d4-a716-446655440000",
      "target_node_uuid": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "source_port_id": "out",
      "target_port_id": "in"
    },
    {
      "source_node_uuid": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "target_node_uuid": "9ba7b810-9dad-11d1-80b4-00c04fd430c9",
      "source_port_id": "true",
      "target_port_id": "in"
    },
    {
      "source_node_uuid": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "target_node_uuid": "a1a7b810-9dad-11d1-80b4-00c04fd430ca",
      "source_port_id": "false",
      "target_port_id": "in"
    }
  ]
}
```

---

## 源代码文件

```
src-tauri/src/
├── node.rs           # Node, NodeType, Port 定义（包含 uuid 字段）
├── node_config.rs    # NodeData 及各节点配置
├── connection.rs     # Connection 定义（使用 node_uuid 关联）
├── flow.rs           # Flow 定义（包含 uuid 字段）
├── executor.rs       # 执行引擎
├── validator.rs      # 流程验证器
└── storage.rs        # 数据存储
```

---

## 注意事项

**前端开发**：节点创建时生成 `uuid`（UUID v4），连接时使用 `node.uuid`

**后端开发**：保存时保留 `uuid`，连接必须引用节点的 `uuid`

---

## 相关文档

- **执行器架构设计**：`docs/executor_architecture.md` - 执行引擎、ExecutionContext、NodeExecutor trait、执行流程
- **报告结构设计**：`docs/report_structure.md`
- **执行流程详细设计**：`docs/executor/execution_flow.md`
- **变量管理详细设计**：`docs/executor/variable_management.md`


