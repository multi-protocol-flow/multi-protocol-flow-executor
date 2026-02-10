# Multi-Protocol Flow Executor

> 多协议流程编排与执行引擎

一个强大的工作流编排与执行引擎，支持命令行执行流程文件。

## ✨ 主要特性

### 流程执行引擎
- ⚡ 高性能 Rust 后端执行引擎
- 🔄 支持多种节点类型：开始节点、HTTP 请求、数据转换、条件判断、邮件发送、数据库操作
- 📝 实时执行日志记录
- ✅ 流程验证与错误检测

### CLI 模式
- 💻 独立的命令行工具，支持批量执行和集成
- 📄 支持执行 JSON 格式的流程文件
- 🔧 灵活的命令行参数配置

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────┐
│              后端 (Rust)                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ 执行引擎    │  │ 流程验证器  │  │ 存储管理    │      │
│  │   Executor  │  │  Validator  │  │  Storage    │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────┘
                          ↕
┌─────────────────────────────────────────────────────────┐
│                  CLI 独立二进制                         │
│         支持命令行执行 JSON 格式流程文件                  │
└─────────────────────────────────────────────────────────┘
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

### 使用 CLI

```bash
# 执行流程文件
cli --execute --flow <path-to-flow.json>

# 或使用短格式
cli -e -f <path-to-flow.json>
```

## 📖 使用指南

CLI 模式允许你通过命令行执行 JSON 格式的流程文件，适合批量执行和自动化集成。

#### 详细文档

更多 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 测试

```bash
# 进入后端目录
cd src-tauri

# 运行测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture
```

## 📁 项目结构

```
multi-protocol-flow-executor/
├── src-tauri/              # 后端源代码 (Rust)
│   ├── src/
│   │   ├── command.rs      # 命令基础类型
│   │   ├── commands.rs     # Tauri 命令处理器
│   │   ├── executor.rs     # 执行引擎核心
│   │   ├── flow.rs         # 流程定义
│   │   ├── node.rs         # 节点定义
│   │   ├── connection.rs   # 连接定义
│   │   ├── validator.rs    # 流程验证器
│   │   ├── storage.rs      # 数据存储
│   │   ├── start_executor.rs  # 开始节点执行器
│   │   └── lib.rs          # 库入口
│   ├── bin/
│   │   └── cli.rs          # CLI 独立二进制入口
│   ├── Cargo.toml          # Rust 依赖配置
│   └── tauri.conf.json     # Tauri 配置
│
├── tests/                  # 测试文件
│   ├── fixtures/           # 测试数据
│   │   └── flows/         # 流程测试用例
│   └── integration/        # 集成测试
│
├── README.md               # 项目说明（本文件）
└── CLI_USAGE.md            # CLI 使用文档
```

## 🔧 开发指南

### 后端开发

- Rust 2021 edition
- 执行引擎采用模块化设计，易于扩展新节点类型

### 添加新节点类型

1. 在 `src-tauri/src/node.rs` 中添加新的 `NodeType`
2. 创建对应的执行器（参考 `src-tauri/src/start_executor.rs`）
3. 在 `src-tauri/src/executor.rs` 中注册新执行器

## 📄 License

MIT

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**详细架构设计请参考 [AGENT.md](./AGENT.md)**
