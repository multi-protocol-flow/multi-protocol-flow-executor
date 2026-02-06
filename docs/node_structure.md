# 后端节点设计结构

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

**节点（Node）:**
- `uuid`: 前端创建节点时生成，用于前端状态管理和连接关联

**连接（Connection）:**
- `source_node_uuid`: 引用源节点的 `uuid`
- `target_node_uuid`: 引用目标节点的 `uuid`

### 典型流程

```
1. 前端创建节点 → 生成 uuid（如：uuid-123）
2. 前端连接节点 → 使用 node.uuid（如：uuid-123 → uuid-456）
3. 前端保存流程 → 发送完整数据（包含 uuid）
4. 后端保存流程 → 保存完整数据
5. 后端返回流程 → 返回完整数据
6. 前端接收数据 → 使用 uuid 恢复前端状态
```

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

流程是顶层容器，包含所有节点和连接，以及流程级别的初始变量。

---

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
    Start,        // 开始节点 - 流程入口（必需）
    Http,         // HTTP 请求
    Conditional,  // 条件分支
    End,          // 结束节点
}
```

---

### 3. NodeData（节点配置）

使用 Tagged Union 实现强类型配置：

```rust
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NodeData {
    Start(StartConfig),
    Http(HttpConfig),
    Conditional(ConditionalConfig),
    End(EndConfig),
}
```

#### Start 节点配置

```rust
pub struct StartConfig {}
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
    pub condition_type: ConditionType,
    #[serde(default)]
    pub field_path: Option<String>,
    #[serde(default)]
    pub operator: Option<ConditionOperator>,
    #[serde(default)]
    pub expected_value: Option<serde_json::Value>,
    #[serde(default)]
    pub advanced_expression: Option<String>,
}

pub enum ConditionType {
    Simple,    // 简单条件
    Advanced,  // 高级表达式
}

pub enum ConditionOperator {
    Equal, NotEqual, Greater, GreaterOrEqual,
    Less, LessOrEqual, Contains, StartsWith, EndsWith, Exists,
}
```

#### End 节点配置

```rust
pub struct EndConfig {}
```

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
```

---

## 节点关系

```
Flow
├── uuid: String        // 唯一标识符（前端生成）
├── nodes: Vec<Node>
│   ├── Start (必需，唯一入口)
│   │   ├── uuid: String    // 前端生成UUID
│   │   └── ...
│   ├── Http
│   ├── Conditional
│   └── End
└── connections: Vec<Connection>
    ├── source_node_uuid        // 引用源节点的 uuid
    └── target_node_uuid        // 引用目标节点的 uuid
```

**连接规则：**
1. 每个流程必须有且仅有一个 Start 节点
2. Start 节点必须至少有一个输出连接
3. 连接必须从 Output 端口指向 Input 端口
4. 不允许自环（节点不能连接到自己）
5. 连接通过节点的 `uuid` 建立关联，而非 `id`

---

## 执行引擎

### ExecutionContext

```rust
pub struct ExecutionContext {
    /// 流程 UUID
    pub flow_uuid: String,
    /// 当前执行节点的 UUID
    pub current_node_uuid: Option<String>,
    /// 执行变量
    pub variables: serde_json::Value,
}
```

### ExecutionResult

```rust
pub struct ExecutionResult {
    /// 执行是否成功
    pub success: bool,
    /// 返回数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 已执行节点的 UUID 列表
    pub executed_nodes: Vec<String>,
}
```

### 执行流程

1. 验证流程（检查 Start 节点、连接完整性）
2. 初始化执行上下文
3. 通过 uuid 查找 Start 节点
4. 按连接顺序遍历节点（使用 uuid 定位）
5. 每个节点通过 `NodeExecutor` trait 执行
6. 返回执行结果

---

## 完整 Flow JSON 示例

```json
{
  "uuid": "flow-550e8400-e29b-41d4-a716-446655440000",
  "name": "用户注册流程",
  "initial_variables": {
    "api_key": "sk-xxx",
    "base_url": "https://api.example.com"
  },
  "nodes": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "type": "start",
      "name": "开始",
      "position": { "x": 100, "y": 100 },
      "data": { "type": "start" }
    },
    {
      "uuid": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "type": "http",
      "name": "发送请求",
      "position": { "x": 400, "y": 100 },
      "data": {
        "type": "http",
        "url": "https://api.example.com/users",
        "method": "post",
        "body": { "name": "张三" }
      }
    },
    {
      "uuid": "93283471-6ba7-11d1-9b6b-00c04fd430c8",
      "type": "end",
      "name": "结束",
      "position": { "x": 700, "y": 100 },
      "data": { "type": "end" }
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
      "target_node_uuid": "93283471-6ba7-11d1-9b6b-00c04fd430c8",
      "source_port_id": "out",
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

### 前端开发注意
1. 节点创建时必须生成 `uuid`（使用 UUID v4）
2. 连接建立时使用节点的 `uuid`
3. 保存和加载流程时，`uuid` 是前端状态恢复的唯一标识
4. 所有节点和连接都通过 `uuid` 进行关联和引用

### 后端开发注意
1. 接收前端数据时，必须包含 `uuid` 字段
2. 保存时保留 `uuid` 不变
3. 连接的 `source_node_uuid` 和 `target_node_uuid` 必须引用节点的 `uuid`
4. 执行引擎使用 `uuid` 定位节点


