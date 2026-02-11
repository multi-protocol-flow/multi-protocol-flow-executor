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
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

pub enum HttpMethod {
    Get, Post, Put, Delete, Patch,
}
```

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
2. Entry 节点必须至少有一个输出连接
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


