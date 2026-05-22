# Multi-Protocol Flow Executor

> 多协议流程编排与执行引擎

一个强大的工作流编排与执行引擎，提供可视化编辑器和命令行执行能力。基于 **Rust + Tauri 2.x + React** 构建的跨平台桌面应用。

## ✨ 主要特性

### 可视化流程编辑器
- 🎨 拖拽式流程设计，基于 React Flow
- 📦 节点库面板，快速添加各类节点
- ⚙️ 实时节点配置面板
- 📋 流程列表管理，支持创建、删除、切换

### 流程执行引擎
- ⚡ 高性能 Rust 后端执行引擎
- 🔄 支持多种核心节点类型：Entry（入口）、HTTP（请求）、Conditional（条件分支）
- 📝 实时执行日志与详细报告
- ✅ 流程验证与错误检测
- 📊 执行统计与历史记录
- 🚀 内置流程模板，首次启动自动加载示例流程

### CLI 模式
- 💻 独立的命令行工具，支持批量执行和自动化集成
- 📄 支持执行 JSON 格式的流程文件
- 🔧 灵活的命令行参数配置
- 📤 结构化 JSON 输出，便于解析和集成

### 全局设置
- ⚙️ 可自定义报告和日志存储位置
- 💾 设置自动持久化保存
- 📂 支持选择目录和打开目录功能

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        前端 (React + TypeScript)                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  FlowCanvas │  │ NodeLibrary │  │ ConfigPanel │  │ Execution   │     │
│  │  流程画布   │  │  节点库面板 │  │  配置面板   │  │  执行报告   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                              ↕ Tauri IPC
┌─────────────────────────────────────────────────────────────────────────┐
│                    后端 (Rust + Tauri 2.x)                               │
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                      │
│  │  Executor   │  │  Validator  │  │   Storage   │                      │
│  │  执行引擎   │  │  流程验证器 │  │  存储管理   │                      │
│  └─────────────┘  └─────────────┘  └─────────────┘                      │
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    NodeExecutor Implementations                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────┐ ┌──────────┐            │  │
│  │  │  Entry   │ │   HTTP   │ │Conditional │ │   End    │            │  │
│  │  │ Executor │ │ Executor │ │  Executor  │ │ Executor │            │  │
│  │  └──────────┘ └──────────┘ └────────────┘ └──────────┘            │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────────────────┐
│                         CLI 独立二进制                                    │
│               支持命令行执行 JSON 格式流程文件                              │
└─────────────────────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 环境要求

- **Rust** >= 1.77.2
- **Node.js** >= 18 (推荐使用 bun)
- **pnpm** / **npm** / **bun** 包管理器

### 安装依赖

```bash
# 安装前端依赖
npm install
# 或使用 bun
bun install
```

### 开发模式

```bash
# 启动 Tauri 开发模式（前端 + 后端热重载）
npx tauri dev
```

### 构建应用

```bash
# 构建生产版本
npx tauri build
```

### 编译 CLI 工具

```bash
# 开发版本
cd src-tauri
cargo build

# 生产版本
cd src-tauri
cargo build --release
```

编译产物位置：
- Windows: `src-tauri\target\debug\mpe.exe` (或 `release\mpe.exe`)
- Linux/macOS: `src-tauri/target/debug/mpe` (或 `release/mpe`)

### 使用 CLI

```bash
# 执行流程文件
mpe run -f <path-to-flow.json>

# 验证流程
mpe validate -f <path-to-flow.json>

# 压力测试
mpe stress run -f <path-to-flow.json>

# 显示帮助
mpe --help
```

## 📖 使用指南

### 可视化编辑器

1. **创建流程**: 点击左侧「新建流程」按钮
2. **添加节点**: 从节点库拖拽节点到画布
3. **连接节点**: 从节点输出端口拖拽到另一节点的输入端口
4. **配置节点**: 选中节点后在右侧面板配置参数
5. **执行流程**: 点击顶部「执行」按钮
6. **查看报告**: 执行完成后查看详细报告

### 流程文件示例

流程文件使用 **FlowFile 包装格式**，包含 `_version`（版本号）、`flow`（流程数据）和 `_meta`（元数据）三个顶层键：

```json
{
  "_version": 1,
  "flow": {
    "uuid": "flow-001",
    "name": "示例流程",
    "initial_variables": {
      "api_base": "https://api.example.com"
    },
    "nodes": [
      {
        "uuid": "entry-001",
        "type": "entry",
        "name": "入口",
        "position": { "x": 0, "y": 0 },
        "data": { "type": "entry" }
      },
      {
        "uuid": "http-001",
        "type": "http",
        "name": "获取数据",
        "position": { "x": 200, "y": 0 },
        "data": {
          "type": "http",
          "url": "{{api_base}}/users",
          "method": "get",
          "timeout_ms": 30000
        }
      }
    ],
    "connections": [
      {
        "id": "conn-001",
        "source_node_uuid": "entry-001",
        "target_node_uuid": "http-001",
        "source_port_id": "out",
        "target_port_id": "in"
      }
    ]
  },
  "_meta": null
}
```

### 节点类型

| 类型 | 说明 | 用途 |
|------|------|------|
| `entry` | 入口节点 | 流程起点，每个流程必须有且仅有一个 |
| `http` | HTTP 请求节点 | 发送 HTTP 请求，支持 GET/POST/PUT/DELETE/PATCH，多种请求体类型（JSON、表单、文件） |
| `conditional` | 条件分支节点 | 根据条件选择执行路径 |
| `script` | 脚本节点 | 执行 JavaScript 代码，支持数据提取、变量操作、断言验证 |

#### 脚本节点

脚本节点允许在流程中运行 JavaScript 代码，提供丰富的 `fn.*` API：

- `fn.variables.*` - 变量操作
- `fn.response.*` - 响应数据访问
- `fn.console.*` - 日志输出
- `fn.flow.*` - 流程控制
- `fn.util.*` - 工具函数（编码、加密、时间、UUID 等）
- `fn.expect()` - 链式断言

详细 API 文档请参考 [docs/script-node.md](./docs/script-node.md)

### 变量引用

使用 `{{path.to.value}}` 语法引用变量：

| 语法 | 说明 |
|------|------|
| `{{initial_variables.xxx}}` | 引用流程初始变量 |
| `{{status}}` | 引用上一个 HTTP 响应的状态码 |
| `{{body.xxx}}` | 引用上一个 HTTP 响应的 JSON 数据 |
| `{{headers.xxx}}` | 引用上一个 HTTP 响应的响应头 |

#### 详细文档

更多 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 测试

### 后端测试

```bash
# 进入后端目录
cd src-tauri

# 运行所有测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture

# 仅运行 CLI 测试
cargo test cli_tests

# 仅运行属性测试
cargo test proptest
```

### 前端测试

```bash
# 运行前端单元测试
npm run test

# 类型检查
npm run typecheck
```

## 📁 项目结构

```
multi-protocol-flow-executor/
├── src/                        # 前端源代码 (React + TypeScript)
│   ├── components/
│   │   ├── canvas/             # 流程画布组件
│   │   │   ├── nodes/          # 自定义节点组件
│   │   │   └── edges/          # 自定义边组件
│   │   ├── panels/             # 配置面板组件
│   │   ├── sidebar/            # 侧边栏组件
│   │   ├── execution/          # 执行相关组件
│   │   ├── dialogs/            # 对话框组件
│   │   └── ui/                 # 基础 UI 组件 (shadcn/ui)
│   ├── stores/                 # Zustand 状态管理
│   ├── services/               # 服务层 (Tauri IPC, Storage)
│   ├── types/                  # TypeScript 类型定义
│   ├── hooks/                  # 自定义 Hooks
│   ├── layouts/                # 布局组件
│   └── lib/                    # 工具函数
│
├── src-tauri/                  # 后端源代码 (Rust)
│   ├── src/
│   │   ├── lib.rs              # 库入口
│   │   ├── main.rs             # Tauri 应用入口
│   │   ├── executor.rs         # 执行引擎核心 (FlowExecutor)
│   │   ├── context.rs          # 执行上下文 + 变量存储
│   │   ├── node.rs             # 节点定义 (Node/NodeType/Port)
│   │   ├── node_config.rs      # 节点配置枚举
│   │   ├── connection.rs       # 连接定义
│   │   ├── flow.rs             # 流程定义
│   │   ├── validator.rs        # 流程验证器
│   │   ├── error.rs            # 错误类型定义
│   │   ├── report.rs           # 执行报告
│   │   ├── statistics.rs       # 统计信息
│   │   ├── storage.rs          # 文件存储
│   │   ├── command.rs          # 命令输入/输出类型
│   │   ├── commands.rs         # Tauri IPC 命令
│   │   ├── entry_executor.rs   # 入口节点执行器
│   │   ├── http_executor.rs    # HTTP 请求节点执行器
│   │   ├── conditional_executor.rs # 条件分支节点执行器
│   │   └── end_executor.rs     # 终止节点执行器
│   ├── bin/
│   │   └── cli.rs              # CLI 独立二进制入口
│   ├── tests/
│   │   ├── cli_tests.rs        # CLI 集成测试
│   │   └── integration.rs      # 流程集成测试
│   ├── Cargo.toml              # Rust 依赖配置
│   └── tauri.conf.json         # Tauri 配置
│
├── tests/                      # 测试文件
│   ├── cli/                    # CLI 测试用例
│   │   ├── 01_simple_entry_http.json
│   │   ├── 02_http_all_methods.json
│   │   ├── ...
│   │   └── edge_cases/         # 边界情况测试
│   └── fixtures/               # 测试夹具
│       └── flows/              # 流程测试用例
│
├── docs/                       # 文档目录
│   ├── executor_architecture.md # 架构设计文档
│   ├── node_structure.md       # 节点结构文档
│   └── executor/               # 执行器详细文档
│
├── README.md                   # 项目说明（本文件）
├── CLI_USAGE.md                # CLI 使用文档
└── AGENTS.md                   # 项目知识库
```

## 🔧 开发指南

### 前端开发

- React 19 + TypeScript
- 状态管理：Zustand
- UI 组件：shadcn/ui + Tailwind CSS
- 流程画布：@xyflow/react
- 表单：react-hook-form + zod

```bash
# 启动开发服务器
npm run dev

# 类型检查
npm run typecheck

# 运行测试
npm run test
```

### 后端开发

- Rust 2021 edition
- 执行引擎采用模块化设计，基于 `NodeExecutor` trait 扩展节点类型
- 使用 Clippy 进行代码质量检查：`cargo clippy`

### 添加新节点类型

1. **后端**：在 `src-tauri/src/node.rs` 中添加新的 `NodeType` 枚举值
2. **后端**：在 `src-tauri/src/node_config.rs` 中添加对应的 `NodeData` 配置
3. **后端**：创建对应的执行器文件 `xxx_executor.rs`，实现 `NodeExecutor` trait：

```rust
#[async_trait::async_trait]
impl NodeExecutor for MyNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), String>;
}
```

4. **后端**：在 `src-tauri/src/executor.rs` 的 `execute_node()` 方法中注册新执行器
5. **前端**：在 `src/components/canvas/nodes/` 中创建对应的节点组件
6. **前端**：在 `src/components/panels/` 中创建对应的配置面板
7. 添加对应的单元测试和集成测试

### 代码规范

**前端：**
- 格式化：使用 ESLint + Prettier
- 类型安全：禁止 `as any`、`@ts-ignore`、`@ts-expect-error`

**后端：**
- 格式化：`cargo fmt`
- Lint 检查：`cargo clippy`
- 禁止 `unsafe_code`

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| 前端框架 | React 19 + TypeScript |
| 状态管理 | Zustand |
| UI 组件 | shadcn/ui + TailwindCSS 4 |
| 流程画布 | @xyflow/react |
| 桌面框架 | Tauri 2.x |
| 后端语言 | Rust |
| HTTP 客户端 | reqwest |
| 表达式求值 | evalexpr |

## 📄 License

MIT

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**详细架构设计请参考 [docs/executor_architecture.md](./docs/executor_architecture.md)**  
**项目知识库请参考 [AGENTS.md](./AGENTS.md)**
