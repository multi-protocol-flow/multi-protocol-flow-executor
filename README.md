# Multi-Protocol Flow Executor

> 多协议流程编排与执行引擎

一个强大的工作流编排与执行引擎，支持命令行执行流程文件。基于 Rust + Tauri 2.x 构建。

## ✨ 主要特性

### 流程执行引擎
- ⚡ 高性能 Rust 后端执行引擎
- 🔄 支持三种核心节点类型：Entry（入口）、HTTP（请求）、Conditional（条件分支）
- 📝 实时执行日志与详细报告
- ✅ 流程验证与错误检测
- 📊 执行统计信息收集

### CLI 模式
- 💻 独立的命令行工具，支持批量执行和自动化集成
- 📄 支持执行 JSON 格式的流程文件
- 🔧 灵活的命令行参数配置
- 📤 结构化 JSON 输出，便于解析和集成

## 🏗️ 架构概览

```
┌───────────────────────────────────────────────────────────────┐
│                    后端 (Rust + Tauri 2.x)                     │
│                                                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │  Executor   │  │  Validator  │  │   Storage   │            │
│  │  执行引擎   │  │  流程验证器 │  │  存储管理   │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              NodeExecutor Implementations                │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────┐ ┌──────────┐  │  │
│  │  │  Entry   │ │   HTTP   │ │Conditional │ │   End    │  │  │
│  │  │ Executor │ │ Executor │ │  Executor  │ │ Executor │  │  │
│  │  └──────────┘ └──────────┘ └────────────┘ └──────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
                              ↕
┌───────────────────────────────────────────────────────────────┐
│                     CLI 独立二进制                             │
│            支持命令行执行 JSON 格式流程文件                      │
└───────────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 环境要求

- **Rust** >= 1.77.2

### 编译 CLI 工具

```bash
# 开发版本
cd src-tauri
cargo build --bin cli

# 生产版本
cd src-tauri
cargo build --release --bin cli
```

编译产物位置：
- Windows: `src-tauri\target\debug\cli.exe` (或 `release\cli.exe`)
- Linux/macOS: `src-tauri/target/debug/cli` (或 `release/cli`)

### 使用 CLI

```bash
# 执行流程文件
cli --execute --flow <path-to-flow.json>

# 或使用短格式
cli -e -f <path-to-flow.json>

# 显示帮助
cli --help
```

## 📖 使用指南

### 流程文件示例

```json
{
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
        "url": "{{initial_variables.api_base}}/users",
        "method": "get"
      }
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

### 节点类型

| 类型 | 说明 | 用途 |
|------|------|------|
| `entry` | 入口节点 | 流程起点，每个流程必须有且仅有一个 |
| `http` | HTTP 请求节点 | 发送 HTTP 请求，支持 GET/POST/PUT/DELETE/PATCH |
| `conditional` | 条件分支节点 | 根据条件选择执行路径 |

### 变量引用

使用 `{{path.to.value}}` 语法引用变量：

| 语法 | 说明 |
|------|------|
| `{{initial_variables.xxx}}` | 引用流程初始变量 |
| `{{http_response.data.xxx}}` | 引用 HTTP 响应数据 |
| `{{http_response.status}}` | 引用 HTTP 状态码 |

#### 详细文档

更多 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 测试

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

## 📁 项目结构

```
multi-protocol-flow-executor/
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
│   └── executor_architecture.md # 架构设计文档
│
├── README.md                   # 项目说明（本文件）
├── CLI_USAGE.md                # CLI 使用文档
└── AGENTS.md                   # 项目知识库
```

## 🔧 开发指南

### 后端开发

- Rust 2021 edition
- 执行引擎采用模块化设计，基于 `NodeExecutor` trait 扩展节点类型
- 使用 Clippy 进行代码质量检查：`cargo clippy`

### 添加新节点类型

1. 在 `src-tauri/src/node.rs` 中添加新的 `NodeType` 枚举值
2. 在 `src-tauri/src/node_config.rs` 中添加对应的 `NodeData` 配置
3. 创建对应的执行器文件 `xxx_executor.rs`，实现 `NodeExecutor` trait：

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

4. 在 `src-tauri/src/executor.rs` 的 `execute_node()` 方法中注册新执行器
5. 添加对应的单元测试和集成测试

### 代码规范

- 格式化：`cargo fmt`
- Lint 检查：`cargo clippy`
- 禁止 `unsafe_code`、`as any`、`@ts-ignore` 等不安全操作

## 📄 License

MIT

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**详细架构设计请参考 [docs/executor_architecture.md](./docs/executor_architecture.md)**  
**项目知识库请参考 [AGENTS.md](./AGENTS.md)**
