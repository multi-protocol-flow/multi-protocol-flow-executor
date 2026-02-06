# Multi-Protocol Flow Executor

> 多协议流程编排与执行引擎 - 可视化工作流设计器

一个强大的工作流编排与执行引擎，支持可视化设计、实时执行和命令行操作。

## ✨ 主要特性

### 可视化编辑器
- 🎨 基于 React Flow 的现代化 UI 设计
- 🔀 拖拽式节点编辑器，轻松创建复杂工作流
- 📊 实时连接节点，直观展示流程逻辑
- 🎯 右键菜单快捷操作（重命名、复制、查看详情、更改类型）

### 流程执行引擎
- ⚡ 高性能 Rust 后端执行引擎
- 🔄 支持多种节点类型：开始节点、HTTP 请求、数据转换、条件判断、邮件发送、数据库操作
- 📝 实时执行日志记录
- ✅ 流程验证与错误检测

### 双模式支持
- 🖥️ **桌面应用模式**：Tauri 构建的跨平台桌面应用
- 💻 **CLI 模式**：独立的命令行工具，支持批量执行和集成

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                    前端 (React + TypeScript)             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ 工作流编辑器 │  │ 执行日志面板 │  │ 编排列表管理 │      │
│  │  React Flow │  │ 实时日志展示 │  │  多流程切换  │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────┘
                          ↕ Tauri IPC
┌─────────────────────────────────────────────────────────┐
│              后端 (Rust + Tauri)                        │
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

- **Node.js** >= 18.x
- **Rust** >= 1.77.2
- **pnpm** / **npm** / **yarn**

### 安装依赖

```bash
npm install
```

### 开发模式

#### 启动桌面应用（Tauri）

```bash
npm run tauri dev
```

这将同时启动前端开发服务器和 Tauri 应用窗口。

#### 仅启动前端开发服务器

```bash
npm run dev
```

访问 `http://localhost:5173` 查看界面。

## 📖 使用指南

### 桌面应用模式

#### 1. 创建工作流

1. 应用启动后会自动创建一个默认工作流
2. 点击左侧 "+ 按钮添加新工作流"
3. 在工作流列表中切换不同的流程

#### 2. 编辑节点

- **添加任务节点**：点击顶部 "+ 任务节点" 按钮，选择节点类型
- **移动节点**：拖拽节点到任意位置
- **连接节点**：从节点底部端口拖拽连接线到另一个节点
- **右键菜单**：
  - 重命名节点
  - 复制节点
  - 更改节点类型（仅任务节点）
  - 查看节点详情

#### 3. 节点类型

| 节点类型 | 说明 | 颜色 |
|---------|------|------|
| 开始节点 | 工作流的起点，每个流程必须有一个 | 绿色 |
| HTTP 请求 | 发送 HTTP 请求 | 蓝色 |
| 数据转换 | 数据格式转换和处理 | 紫色 |
| 条件判断 | 根据条件分支执行 | 橙色 |
| 邮件发送 | 发送邮件通知 | 粉色 |
| 数据库操作 | 数据库增删改查操作 | 靛青色 |

#### 4. 执行流程

1. 连接好所有节点
2. 点击顶部 "运行流程" 按钮
3. 在右侧执行日志面板查看实时执行结果

### CLI 模式

CLI 模式允许你通过命令行执行 JSON 格式的流程文件，适合批量执行和自动化集成。

#### 编译 CLI 工具

```bash
# 开发版本
cd src-tauri
cargo build --bin cli

# 生产版本
cd src-tauri
cargo build --release --bin cli
```

#### 使用 CLI

```bash
# 执行流程文件
cli --execute --flow <path-to-flow.json>

# 或使用短格式
cli -e -f <path-to-flow.json>
```

#### 详细文档

更多 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 测试

### 单元测试

```bash
npm run test
```

### E2E 测试

```bash
# 无头模式
npm run test:e2e

# 带界面模式
npm run test:e2e:ui

# 有头模式（显示浏览器窗口）
npm run test:e2e:headed
```

### 类型检查

```bash
npm run check-types
```

### Lint

```bash
npm run lint
```

## 📁 项目结构

```
multi-protocol-flow-executor/
├── src/                      # 前端源代码 (React + TypeScript)
│   ├── components/           # React 组件
│   │   ├── ui/              # 基础 UI 组件（按钮、对话框等）
│   │   ├── modals/          # 模态框组件
│   │   ├── FlowCanvas.tsx   # 流程画布组件
│   │   ├── ConfigPanel.tsx  # 配置面板
│   │   └── ContextMenu.tsx # 右键菜单
│   ├── nodes/               # 自定义节点组件
│   ├── types/               # TypeScript 类型定义
│   ├── lib/                 # 工具函数
│   └── App.tsx             # 主应用组件
│
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
│   ├── e2e/                # E2E 测试
│   └── integration/        # 集成测试
│
├── public/                 # 静态资源
├── package.json            # Node.js 依赖配置
├── tsconfig.json           # TypeScript 配置
├── vite.config.ts          # Vite 配置
├── playwright.config.ts     # Playwright E2E 测试配置
├── vitest.config.mjs       # Vitest 单元测试配置
├── README.md               # 项目说明（本文件）
└── CLI_USAGE.md            # CLI 使用文档
```

## 🔧 开发指南

### 前端开发

- 使用 React Hooks 进行状态管理
- 组件采用 Radix UI + Tailwind CSS 构建
- 流程可视化使用 @xyflow/react（React Flow）

### 后端开发

- Rust 2021 edition
- 使用 Tauri 进行前后端通信
- 执行引擎采用模块化设计，易于扩展新节点类型

### 添加新节点类型

1. 在 `src-tauri/src/node.rs` 中添加新的 `NodeType`
2. 创建对应的执行器（参考 `src-tauri/src/start_executor.rs`）
3. 在 `src-tauri/src/executor.rs` 中注册新执行器
4. 在前端 `src/App.tsx` 中添加节点类型配置

## 📄 License

MIT

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**详细架构设计请参考 [AGENT.md](./AGENT.md)**
