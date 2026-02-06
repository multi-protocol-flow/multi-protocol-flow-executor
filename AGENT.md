# AGENT.md - 架构设计文档

> Multi-Protocol Flow Executor 架构设计详解

## 📐 目录

- [系统架构](#系统架构)
- [前端架构](#前端架构)
- [后端架构](#后端架构)
- [执行流程](#执行流程)
- [通信协议](#通信协议)
- [数据模型](#数据模型)
- [扩展指南](#扩展指南)

## 系统架构

### 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        Desktop Application                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Frontend Layer (React + TypeScript)                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │  │
│  │  │  UI 组件  │  │ 状态管理  │  │ 节点编辑  │  │ 执行日志  │  │  │
│  │  │ Radix UI │  │  React   │  │ ReactFlow│  │  实时更新 │  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          ↕ Tauri IPC                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Backend Layer (Rust)                                    │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │  │
│  │  │ 命令处理器│  │ 执行引擎  │  │ 流程验证  │  │ 存储管理  │  │  │
│  │  │ Commands │  │ Executor │  │Validator │  │ Storage  │  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │  │
│  │                                                            │  │
│  │  节点执行器:                                               │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                │  │
│  │  │ 开始节点  │  │ HTTP节点  │  │ 条件节点  │  ...          │  │
│  │  └──────────┘  └──────────┘  └──────────┘                │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      CLI Mode (Standalone)                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  CLI Binary (Rust)                                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │  │
│  │  │ 参数解析  │  │ 执行引擎  │  │ 结果输出  │                 │  │
│  │  │   Args   │  │ Executor │  │  Output   │                 │  │
│  │  └──────────┘  └──────────┘  └──────────┘                 │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 架构设计原则

1. **关注点分离**：前端负责 UI 交互，后端负责业务逻辑和执行
2. **模块化设计**：各功能模块独立，便于扩展和维护
3. **类型安全**：前后端均使用强类型系统（TypeScript + Rust）
4. **性能优先**：核心执行逻辑使用 Rust 实现，确保高效执行

## 前端架构

### 技术栈

- **框架**：React 19 + TypeScript
- **构建工具**：Vite 7
- **UI 库**：Radix UI（无样式组件库）
- **样式**：Tailwind CSS 4
- **流程可视化**：@xyflow/react（React Flow）
- **表单处理**：react-hook-form + zod
- **动画**：motion（Framer Motion）
- **图标**：lucide-react

### 核心模块

#### 1. App.tsx - 主应用组件

**职责**：
- 管理全局状态（节点、连接、工作流）
- 协调各个子组件
- 处理用户交互事件

**关键状态**：
```typescript
// 节点管理
const [nodes, setNodes] = useState<Node[]>([]);
const [edges, setEdges] = useState<Edge[]>([]);
const [selectedNode, setSelectedNode] = useState<Node | null>(null);

// 工作流管理
const [workflows, setWorkflows] = useState<Workflow[]>([]);
const [currentWorkflowId, setCurrentWorkflowId] = useState<string>("");

// UI 状态
const [isConfigModalOpen, setIsConfigModalOpen] = useState(false);
const [isTaskTypeOpen, setIsTaskTypeOpen] = useState(false);
const [executionLogs, setExecutionLogs] = useState<LogEntry[]>([]);
```

#### 2. FlowCanvas.tsx - 流程画布组件

**职责**：
- 封装 React Flow 实例
- 处理节点和边的变化
- 提供流程编辑的交互层

**关键功能**：
- 节点拖拽
- 连线创建/删除
- 画布缩放和平移
- 右键菜单触发

#### 3. 组件层次结构

```
App
├── Header (工具栏)
│   ├── Logo
│   ├── 标题
│   └── 操作按钮（保存、运行、添加节点）
├── Sidebar (左侧面板)
│   └── WorkflowList (工作流列表)
├── ReactFlow (中央画布)
│   ├── Background (网格背景)
│   ├── Controls (缩放/重置控制)
│   └── Custom Nodes (自定义节点)
│       ├── StartNode
│       ├── TaskNode (HTTP, Transform, Condition, etc.)
│       └── EndNode
├── ContextMenu (右键菜单)
│   ├── Rename
│   ├── Copy
│   ├── Change Type
│   └── View Info
├── ConfigPanel (右侧日志面板)
│   ├── ExecutionLogs
│   └── UsageGuide
└── Modals
    ├── ConfigModal (配置模态框)
    ├── TaskTypeModal (任务类型选择)
    └── NodeInfoModal (节点信息)
```

### 状态管理策略

当前使用 React 内置状态管理：
- **useState**：简单状态（选中节点、模态框开关等）
- **useCallback**：事件处理函数缓存
- **useMemo**：计算属性优化（节点类型配置）

未来可考虑：
- Zustand：轻量级全局状态
- Jotai：原子化状态管理
- React Query：服务端状态管理

### 数据流向

```
用户操作 → 事件处理 → 状态更新 → React 重新渲染 → UI 更新
                  ↓
           可选：Tauri 命令调用
```

## 后端架构

### 技术栈

- **语言**：Rust 2021
- **框架**：Tauri 2.10.0
- **序列化**：serde + serde_json
- **日志**：log + tauri-plugin-log
- **UUID**：uuid (v4)

### 模块职责

#### 1. flow.rs - 流程定义

```rust
pub struct Flow {
    pub id: Uuid,
    pub name: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

impl Flow {
    pub fn validate(&self) -> ValidationResult { ... }
    pub fn to_json(&self) -> Result<String> { ... }
    pub fn from_json(json: &str) -> Result<Self> { ... }
}
```

**职责**：
- 定义流程数据结构
- 实现序列化/反序列化
- 提供流程验证接口

#### 2. node.rs - 节点定义

```rust
pub enum NodeType {
    Start,
    Task(TaskType),
    End,
}

pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub position: NodePosition,
    pub ports: Vec<Port>,
}

pub struct Port {
    pub id: String,
    pub port_type: PortType,  // Input / Output
}
```

**职责**：
- 定义节点数据结构
- 管理节点端口
- 支持节点类型扩展

#### 3. connection.rs - 连接定义

```rust
pub struct Connection {
    pub id: Uuid,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub from_port: String,
    pub to_port: String,
}
```

**职责**：
- 定义节点连接关系
- 确保连接的合法性

#### 4. executor.rs - 执行引擎核心

```rust
pub struct FlowExecutor {
    flow: Flow,
}

pub trait NodeExecutor {
    fn execute(&self, context: &ExecutionContext) -> ExecutionResult;
}
```

**职责**：
- 统一的执行流程管理
- 节点执行器调度
- 执行上下文传递
- 错误处理和日志记录

**执行流程**：
```
开始节点 → 验证 → 查找下一个节点 → 递归执行 → 结束/错误
```

#### 5. start_executor.rs - 开始节点执行器

```rust
pub struct StartNodeExecutor;

impl NodeExecutor for StartNodeExecutor {
    fn execute(&self, context: &ExecutionContext) -> ExecutionResult {
        // 记录执行时间
        // 查找输出连接
        // 返回执行结果
    }
}
```

**职责**：
- 实现开始节点的执行逻辑
- 作为流程执行的入口点

#### 6. validator.rs - 流程验证器

```rust
pub struct FlowValidator;

impl FlowValidator {
    pub fn validate(flow: &Flow) -> ValidationResult {
        // 检查开始节点是否存在
        // 检查连接的合法性
        // 检测循环依赖
        // 返回验证结果
    }
}
```

**职责**：
- 流程完整性检查
- 循环检测
- 未连接节点检测

**验证规则**：
- 必须有且仅有一个开始节点
- 所有连接必须指向存在的节点
- 不能存在循环引用
- 所有节点必须有输入或输出端口连接

#### 7. storage.rs - 存储管理

```rust
pub fn ensure_flows_dir() -> Result<PathBuf> { ... }
pub fn get_flows_dir() -> Result<PathBuf> { ... }
pub fn save_flow(flow: &Flow) -> Result<()> { ... }
pub fn load_flow(flow_id: &Uuid) -> Result<Flow> { ... }
```

**职责**：
- 文件系统操作
- 流程文件持久化
- 目录管理

**存储位置**：
- 开发模式：`$USERDATA_DIR/flows/`
- 生产模式：应用数据目录

#### 8. commands.rs - Tauri 命令处理器

```rust
#[tauri::command]
pub fn create_flow(name: String) -> Result<Flow, String> { ... }

#[tauri::command]
pub fn execute_flow(flow_id: Uuid) -> Result<ExecutionResult, String> { ... }

#[tauri::command]
pub fn update_node(node_id: Uuid, updates: NodeUpdates) -> Result<(), String> { ... }

#[tauri::command]
pub fn delete_node(node_id: Uuid) -> Result<(), String> { ... }
```

**职责**：
- 暴露 Tauri 命令给前端
- 处理 IPC 调用
- 参数验证和错误转换

### CLI 架构

#### bin/cli.rs - CLI 入口

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "multi-protocol-flow-executor")]
#[command(about = "多协议流程执行器")]
struct Cli {
    #[arg(short, long)]
    execute: bool,

    #[arg(short, long)]
    flow: Option<String>,
}
```

**职责**：
- 命令行参数解析
- 加载流程文件
- 调用执行引擎
- 格式化输出结果

**执行流程**：
```
解析参数 → 加载 JSON 文件 → 验证流程 → 执行引擎 → 输出结果
```

## 执行流程

### 完整执行流程

```
┌─────────────────────────────────────────────────────────────┐
│  1. 前端用户点击"运行流程"按钮                                 │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  2. 前端调用 Tauri 命令 execute_flow(flow_id)              │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  3. 后端 Commands 模块接收请求                                │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  4. Storage 模块加载流程数据                                   │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  5. Validator 验证流程合法性                                  │
│     - 检查开始节点                                            │
│     - 检查连接完整性                                          │
│     - 检测循环依赖                                            │
└─────────────────────────────────────────────────────────────┘
                          ↓
        ┌─────────────────┴─────────────────┐
        ↓                                   ↓
┌─────────────────┐                 ┌─────────────────┐
│ 验证失败         │                 │ 验证成功         │
│ 返回错误信息     │                 │                 │
└─────────────────┘                 └─────────────────┘
                                            ↓
┌─────────────────────────────────────────────────────────────┐
│  6. Executor 创建执行上下文                                   │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  7. 递归执行节点                                              │
│     - 查找开始节点                                            │
│     - 调用对应的 NodeExecutor                                │
│     - 记录执行结果和日志                                      │
│     - 查找下一个节点                                          │
└─────────────────────────────────────────────────────────────┘
                          ↓
        ┌─────────────────┴─────────────────┐
        ↓                                   ↓
┌─────────────────┐                 ┌─────────────────┐
│ 执行失败         │                 │ 执行成功         │
│ 记录错误日志     │                 │ 返回执行结果     │
│ 继续下一个节点   │                 │                 │
└─────────────────┘                 └─────────────────┘
                                            ↓
┌─────────────────────────────────────────────────────────────┐
│  8. 前端接收执行结果并显示日志                                 │
└─────────────────────────────────────────────────────────────┘
```

### 节点执行策略

```rust
// 执行引擎伪代码
fn execute_flow(flow: &Flow) -> ExecutionResult {
    let mut context = ExecutionContext::new();
    let mut visited = HashSet::new();
    let mut current_node = find_start_node(flow)?;

    loop {
        // 检查循环
        if visited.contains(&current_node.id) {
            return Err("循环检测失败".to_string());
        }
        visited.insert(current_node.id);

        // 执行当前节点
        let result = execute_node(&current_node, &context)?;

        // 查找下一个节点
        let next_node = find_next_node(flow, &current_node)?;
        if next_node.is_none() {
            break;
        }
        current_node = next_node.unwrap();
    }

    Ok(ExecutionResult::success())
}
```

## 通信协议

### 前后端通信 (Tauri IPC)

#### 命令定义

```typescript
// 前端调用
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<ExecutionResult>('execute_flow', {
  flowId: '550e8400-e29b-41d4-a716-446655440000'
});
```

```rust
// 后端实现
#[tauri::command]
pub fn execute_flow(flow_id: Uuid) -> Result<ExecutionResult, String> {
    // 实现逻辑
}
```

#### 命令列表

| 命令名 | 参数 | 返回值 | 说明 |
|-------|------|--------|------|
| `create_flow` | `name: String` | `Flow` | 创建新工作流 |
| `execute_flow` | `flow_id: Uuid` | `ExecutionResult` | 执行工作流 |
| `update_node` | `node_id: Uuid, updates: NodeUpdates` | `()` | 更新节点 |
| `delete_node` | `node_id: Uuid` | `()` | 删除节点 |

### CLI 输出格式

```json
{
  "success": true,
  "data": {
    "flow_id": "550e8400-e29b-41d4-a716-446655440000",
    "executed_nodes": ["node-1", "node-2", "node-3"],
    "execution_time": 42
  },
  "error": null,
  "execution_time": 50
}
```

## 数据模型

### 前端数据模型

```typescript
// workflow.ts
interface Position {
  x: number;
  y: number;
}

interface NodeData {
  id: string;
  type: "start" | "task";
  taskType?: "http" | "transform" | "condition" | "email" | "database";
  name: string;
  position: Position;
  createdAt: number;
  error?: string;
  isExecuting?: boolean;
}

interface Connection {
  id: string;
  fromId: string;
  toId: string;
}

interface Workflow {
  id: string;
  name: string;
  nodes: NodeData[];
  connections: Connection[];
  createdAt: number;
  updatedAt: number;
}
```

### 后端数据模型

```rust
// flow.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub id: Uuid,
    pub name: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

// node.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Start,
    Task(TaskType),
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub position: NodePosition,
    pub ports: Vec<Port>,
}
```

## 扩展指南

### 添加新的节点类型

#### 1. 后端实现

**步骤 1**：在 `node.rs` 中定义新的任务类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Http,
    Transform,
    Condition,
    Email,
    Database,
    CustomTask,  // 新增
}
```

**步骤 2**：创建执行器（如 `src-tauri/src/custom_task_executor.rs`）

```rust
pub struct CustomTaskExecutor;

impl NodeExecutor for CustomTaskExecutor {
    fn execute(&self, context: &ExecutionContext) -> ExecutionResult {
        // 实现执行逻辑
        log::info!("执行自定义任务节点");
        ExecutionResult::success()
    }
}
```

**步骤 3**：在 `executor.rs` 中注册执行器

```rust
mod custom_task_executor;
use custom_task_executor::CustomTaskExecutor;

impl FlowExecutor {
    fn get_executor(&self, node_type: &NodeType) -> Box<dyn NodeExecutor> {
        match node_type {
            NodeType::Start => Box::new(StartNodeExecutor),
            NodeType::Task(TaskType::CustomTask) => Box::new(CustomTaskExecutor),
            // ... 其他类型
        }
    }
}
```

#### 2. 前端实现

**步骤 1**：在 `src/types/workflow.ts` 中添加类型定义

```typescript
type TaskNodeType = "http" | "transform" | "condition" | "email" | "database" | "customTask";
```

**步骤 2**：在 `src/App.tsx` 中添加图标和颜色映射

```typescript
const TASK_TYPE_ICONS: Record<TaskNodeType, React.ReactNode> = {
  http: <Globe size={20} />,
  transform: <Code size={20} />,
  // ... 其他类型
  customTask: <CustomIcon size={20} />,
};

const TASK_TYPE_COLORS: Record<TaskNodeType, string> = {
  http: "text-blue-500",
  transform: "text-purple-500",
  // ... 其他类型
  customTask: "text-teal-500",
};

const getTaskTypeName = (taskType: TaskNodeType): string => {
  const names: Record<TaskNodeType, string> = {
    http: 'HTTP 请求',
    transform: '数据转换',
    // ... 其他类型
    customTask: '自定义任务',
  };
  return names[taskType];
};
```

**步骤 3**：更新 `nodeTypes` 配置

```typescript
const nodeTypes = useMemo(
  () => ({
    start: BaseNode,
    http: BaseNode,
    transform: BaseNode,
    condition: BaseNode,
    email: BaseNode,
    database: BaseNode,
    customTask: BaseNode,  // 新增
  }),
  []
);
```

### 添加新的 Tauri 命令

**步骤 1**：在 `src-tauri/src/commands.rs` 中定义命令

```rust
#[tauri::command]
pub fn new_command(param: String) -> Result<String, String> {
    // 实现逻辑
    Ok(format!("处理完成: {}", param))
}
```

**步骤 2**：在 `src-tauri/src/lib.rs` 中注册命令

```rust
.invoke_handler(tauri::generate_handler![
    create_flow,
    execute_flow,
    update_node,
    delete_node,
    new_command,  // 新增
])
```

**步骤 3**：在前端调用命令

```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<string>('new_command', { param: 'test' });
```

---

**详细 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)**
