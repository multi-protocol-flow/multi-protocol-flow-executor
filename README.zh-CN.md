# Multi-Protocol Flow Executor

> 多协议流程编排与执行引擎

[English](./README.md) | **简体中文**

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

### 流程生命周期阶段
- 🧩 **Setup / Main / Teardown** 是一等公民阶段
- 🔗 Setup 导出的变量直接合并进 Main 上下文
- 📄 Teardown 可读取 Main 报告为只读变量 `$report`
- 🧪 压测与数据集执行只跑一次 Setup/Teardown，QPS/P99 保持为纯 Main 指标

### CLI 模式
- 💻 独立的命令行工具，支持批量执行和自动化集成
- 📄 支持执行 JSON 与 YAML 格式的流程文件
- 🔧 灵活的命令行参数配置
- 📤 结构化 JSON 输出，便于解析和集成
- 🎯 支持 JUnit XML、GitHub Step Summary 与自包含 HTML 报告

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

- **Rust** >= 1.85.0
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

### 编译 GUI 应用（Tauri 桌面版）

```bash
# 构建生产版本（包含前端资源 + Tauri 后端）
npx tauri build
```

编译产物位置：`src-tauri/target/release/bundle/`（平台对应安装包）

### 编译 CLI 工具

项目提供二种二进制入口：

| 二进制 | 适用场景 | 依赖 |
|--------|----------|------|
| `mpe` | GUI + CLI 二合一 | 需要 Tauri + 前端资源 |
| `mpe-cli` | 纯 CLI（服务器/CI/Docker） | 纯 Rust，无 GUI 依赖 |

#### 构建桌面版 CLI（二合一 `mpe`）

```bash
# 开发版本
cd src-tauri
cargo build

# 生产版本（体积优化）
cd src-tauri
cargo build --release
```

产物位置：
- Windows: `src-tauri\target\debug\mpe.exe`（或 `release\mpe.exe`）
- Linux/macOS: `src-tauri/target/debug/mpe`（或 `release/mpe`）

> 注意：二合一 `mpe` 编译依赖完整前端构建，需要先 `npm install`。

#### 构建独立 CLI（纯 Rust，推荐给 Linux 服务器使用）

```bash
# Linux / macOS
cd src-tauri
cargo build -p mpe-cli --release
cp target/release/mpe-cli /usr/local/bin/mpe    # 安装到系统路径

# 按需选择协议（减少编译时间/二进制体积）。
# plugin-manager 不可省略：CLI 无条件读取插件连接配置，缺该 feature 无法编译。
cargo build -p mpe-cli --release --no-default-features \
  -F plugin-manager -F protocol-http -F protocol-script
```

产物：`src-tauri/target/release/mpe-cli`

##### musl 静态编译（兼容所有 Linux 系统）

如需一个不依赖 glibc 版本、可在任何 Linux 上直接运行的二进制，用 Docker 进行 musl 静态编译：

```bash
# 构建 Docker 编译镜像（首次需等待 rustup 安装）
docker build -t mpe-musl-builder -f Dockerfile.musl .

# 编译 musl 静态 mpe-cli
docker run --rm \
  -v $PWD:/src \
  -v musl-cargo:/root/.cargo/registry \
  mpe-musl-builder \
  sh -c "cd src-tauri && cargo build -p mpe-cli --release"

# 产物在 src-tauri/target/release/mpe-cli
file src-tauri/target/release/mpe-cli
# 输出：ELF ... static-pie linked（不依赖 glibc）
```

> GitHub Release 中 `mpe-v*-linux-x86_64.tar.gz` 即为 musl 静态编译版本。

#### Windows 交叉编译 Linux CLI

从 Windows 构建 Linux 独立 CLI 有两种方式：

**方式一：WSL（推荐）**

```bash
# 在 WSL（Ubuntu/Debian）中
sudo apt install build-essential pkg-config libssl-dev
cd /path/to/multi-protocol-flow-executor/src-tauri
cargo build -p mpe-cli --release
# 产物在 target/release/mpe-cli，直接 cp 到 Linux 机器使用
```

**方式二：Rust 交叉编译**

```bash
# 1. 安装 Linux 目标
rustup target add x86_64-unknown-linux-gnu

# 2. 安装交叉编译器（需要 MSYS2 或 mingw64）
# 或直接使用 WSL 更简单（见方式一）

# 3. 构建
cd src-tauri
cargo build -p mpe-cli --release --target x86_64-unknown-linux-gnu
```

#### 安装到 Linux 系统路径

```bash
cp src-tauri/target/release/mpe-cli /usr/local/bin/mpe   # 或 /root/bin/mpe
mpe --help
```

### 浏览器模式（前端独立运行，无需 Tauri 后端）

前端支持在浏览器中独立运行，所有 Tauri 后端调用会自动降级为 Mock 实现。

```bash
# 浏览器模式开发（热重载）
npm run dev:web

# 构建为静态站点，产物输出到 website/playground/
npm run build:web

# 如果使用 VSCode 或其他编辑器，直接 npm run dev 效果等同
```

构建产物位于 `website/playground/`，可直接部署到任何静态文件服务器或 GitHub Pages。

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

### 插件 (Plugins)

插件系统允许第三方开发者以**独立进程**方式为 MPE 扩展新的节点类型，无需重新编译宿主应用。插件功能默认开启，启动时自动扫描插件目录。

**安装插件：** 将包含 `plugin.json` 清单和入口脚本（如 `main.py`、`index.js`）的插件文件夹放入插件目录即可，开箱即用。

| 插件目录 | 说明 |
|----------|------|
| 默认 | `get_data_dir()/plugins`（应用数据目录下的 `plugins` 文件夹） |
| 环境变量 | 设置 `MPE_PLUGIN_DIR` 指向其他目录，优先于默认目录 |

官方协议插件（redis / db / mongo / grpc / kafka / smtp / mcp / llm / agent / openapi / postman / eolink）位于 [`multi-protocol-flow`](https://github.com/multi-protocol-flow) 组织下的独立仓库，清单见 [CONTRIBUTING.md](./CONTRIBUTING.md#repository-layout)。

**插件作者**：开发指南见 [docs/plugin-sdk.md](./docs/plugin-sdk.md)，线协议规范见 [docs/plugin-protocol.md](./docs/plugin-protocol.md)，性能设计见 [docs/plugin-perf.md](./docs/plugin-perf.md)，可运行模板见 [`sdk-templates/`](./sdk-templates)。

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
        "type": "http:request",
        "name": "获取数据",
        "position": { "x": 200, "y": 0 },
        "data": {
          "type": "http:request",
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

| 类型 ID | 说明 | 用途 |
|---------|------|------|
| `entry` | 入口节点 | 流程起点，每个流程必须有且仅有一个 |
| `http:request` | HTTP 请求节点 | 发送 HTTP 请求，支持 GET/POST/PUT/DELETE/PATCH，多种请求体类型（JSON、表单、文件） |
| `conditional` | 条件分支节点 | 根据条件选择执行路径 |
| `script` | 脚本节点 | 执行 JavaScript 代码，支持数据提取、变量操作、断言验证 |
| `flow:sub` | 子流程节点 | 内联执行另一个 `.mpf` 流程 |
| `http:ingress` / `http:respond` | Mock 服务入口/出口 | 通过 `mpe serve` 以 HTTP 提供流程 |

类型 ID 采用 `<protocol>:<operation>` 命名空间，与插件节点（`db:query`、`grpc:call` …）同一套规则。
0.4 之前的老 ID（`http`、`ws_connect`）会被校验器判定为未知节点类型。

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

`{{name}}` 按**裸键名**在流程变量表中查找——没有命名空间前缀。支持点号进入对象
（`{{user.profile.name}}`）；不支持索引（`[0]`）与函数调用。

| 语法 | 说明 |
|------|------|
| `{{api_base}}` | 流程携带的变量（初始变量 / 环境文件 / `--var` / 上游节点写入） |
| `{{user.id}}` | 对象型变量内的嵌套路径 |
| `{{$uuid}}`、`{{$timestamp}}`、`{{$randomInt}}` | 内置动态生成器，运行时求值 |
| `{{$report.status}}` | Main 阶段报告，**仅 Teardown 可用** |

变量不存在会让节点失败（`Variable not found: <key>`），不会静默替换为空串。
上一个节点的响应数据**不会**自动注入为变量——请用 `variable:extractor` 节点
（JSONPath → 变量）或 `script` 节点的 `fn.response.*` 读取。

#### 详细文档

更多 CLI 使用说明请参考 [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 测试

> **测试文件分布：**
>
> | 位置 | 类型 | 说明 |
> |------|------|------|
> | 各 crate 源码 `mod tests` | 单元测试 | 业务代码内联的单测 |
> | `src-tauri/tests/*.rs`（15个文件） | 集成测试 | 全流程、CLI、报告等端到端测试 |
> | `src-tauri/crates/**/tests/*.rs` | 集成测试 | 各协议 crate 的集成测试 |
> | `tests/`（根目录） | 测试夹具 | JSON 流程文件、证书、测试服务 |

### 后端测试

> **测试运行器：** 项目标准为 [cargo-nextest](https://nexte.st)（`cargo nextest run`）——进程级并行、输出捕获、失败重跑、按测试分组报告。nextest 不支持 doctest，文档测试用 `cargo test --doc` 补充。

```bash
# 进入后端目录
cd src-tauri

# ── 全量测试 ─────────────────────────────────────────────────
cargo nextest run                  # 运行所有测试（单测 + 集成测试，进程级并行）
cargo nextest run --nocapture      # 实时显示 println 输出（nextest 下隐含串行）
cargo nextest run --test-threads 4 # 控制并行测试进程数（-j 别名）

# ── 文档测试（nextest 不支持，用 cargo 补充） ────────────────
cargo test --doc                   # 运行 doctest

# ── 只跑单元测试（跳过集成测试） ──────────────────────────────
cargo nextest run --lib            # 所有 crate 的单测

# ── 单 crate 测试 ────────────────────────────────────────────
cargo nextest run -p flow-engine-core --lib        # flow-engine-core 单测
cargo nextest run -p flow-engine-executor          # executor 全部测试
cargo nextest run -p flow-engine-http              # HTTP 协议全部测试
cargo nextest run -p flow-engine-assertion         # 断言全部测试
cargo nextest run -p flow-engine-ws                # WebSocket 全部测试
# 协议插件（db/mcp/grpc/kafka/redis/mongo/smtp/llm/agent…）不在本仓库，
# 各自独立克隆后执行: cargo nextest run --manifest-path <plugin>/Cargo.toml
cargo nextest run -p flow-engine-script            # 脚本全部测试
# gRPC 已移出宿主，测试见 mpe-plugin-grpc 仓库
cargo nextest run -p flow-engine-sse               # SSE 全部测试
cargo nextest run -p flow-engine-graphql           # GraphQL 全部测试
cargo nextest run -p flow-engine-tcp               # TCP/UDP 全部测试
cargo nextest run -p mpe-cli                       # CLI 全部测试

# ── 按测试名称过滤 ────────────────────────────────────────────
cargo nextest run cli_tests                # 仅 CLI 集成测试
cargo nextest run proptest                 # 仅属性测试
cargo nextest run variable                 # 变量相关测试
cargo nextest run report                   # 报告相关测试
cargo nextest run ws_debug                 # WebSocket 调试流测试
cargo nextest run http_executor            # HTTP 执行器测试
cargo nextest run assertion_executor       # 断言执行器测试
cargo nextest run dependency_test          # 依赖检测测试
cargo nextest run report_storage           # 报告存储测试

# ── 按模块路径过滤（单测模块级） ──────────────────────────────
cargo nextest run -p flow-engine-core --lib 'variable::tests::'  # core 变量模块单测
cargo nextest run -p flow-engine-core --lib 'node::tests::'      # core 节点模块单测
cargo nextest run -p flow-engine-core --lib 'context::tests::'   # core 上下文模块单测

# ── 指定集成测试文件 ─────────────────────────────────────────
cargo nextest run --test integration                # 主集成测试
cargo nextest run --test cli_tests                  # CLI 集成测试
cargo nextest run --test dependency_test            # 依赖检测测试
cargo nextest run --test variable_tracking_test     # 变量追踪测试
cargo nextest run --test ws_debug_flow_test         # WS 调试流测试
# gRPC 集成测试在 mpe-plugin-grpc 仓库中

# ── 其他组合 ──────────────────────────────────────────────────
cargo nextest run --lib --nocapture              # 所有单测 + 实时输出
cargo nextest run -p flow-engine-core            # core 全部测试（含集成）
cargo nextest run --workspace                    # workspace 所有包的全部测试

# ── 外部服务集成测试 ────────────────────────────────────────
# 部分测试（HTTP、WebSocket 等）需要一个多协议 echo 服务，
# 已在代码中标记 #[ignore]，默认不执行。
# 该 echo 测试服务不包含在本仓库中（/echo/ 已被 .gitignore 排除）。
# 需要时自行启动等价服务，再使用 --include-ignored：
#   cargo nextest run --include-ignored    # 运行所有测试（含 ignored）
#   cargo nextest run -p flow-engine-http --include-ignored  # 仅 HTTP
#   cargo nextest run -p flow-engine-ws --include-ignored    # 仅 WebSocket
#   cargo nextest run cli_tests --include-ignored             # 仅 CLI
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
├── src-tauri/                  # 后端源代码 (Rust workspace)
│   ├── src/                    # Tauri 应用层（仅 app crate）
│   │   ├── lib.rs              # 库入口 + invoke_handler 注册
│   │   ├── main.rs             # 单二进制入口：有参数 → CLI，无参数 → GUI
│   │   ├── tauri_commands.rs   # 88 个 IPC 命令的包装层
│   │   ├── updater.rs          # 更新检测、下载与安装
│   │   └── plugin_protocol.rs  # 插件前端自定义协议
│   ├── crates/                 # workspace crates
│   │   ├── flow-engine-core/       # 基础类型、变量、上下文
│   │   ├── flow-engine-executor/   # 执行引擎（编排、调试、压测、Ingress）
│   │   ├── flow-engine-{http,ws,tcp,sse,graphql,script,assertion}/  # 协议执行器
│   │   ├── flow-engine-plugin/     # 插件宿主（sidecar 生命周期）
│   │   ├── flow-engine-plugin-marketplace/  # 市场客户端 + 安装器
│   │   ├── flow-engine/            # 门面 crate（re-export 全部）
│   │   └── mpe-cli/                # 纯 Rust CLI 二进制
│   ├── tests/                  # 集成测试
│   ├── Cargo.toml              # workspace + app 依赖配置
│   └── tauri.conf.json         # Tauri 配置
│
├── tests/                      # 测试夹具
│   ├── cli/                    # CLI 测试用例
│   │   ├── 01_simple_entry_http.json
│   │   ├── 02_http_all_methods.json
│   │   ├── ...
│   │   └── edge_cases/         # 边界情况测试
│   ├── certs/                  # 自签名 TLS 测试证书（见其 README）
│   └── fixtures/               # 测试夹具
│       └── flows/              # 流程测试用例
│
├── docs/                       # 文档目录
│   ├── executor_architecture.md # 架构设计文档
│   ├── node_structure.md       # 节点结构文档
│   ├── plugin-*.md             # 插件架构 / SDK / wire / 性能
│   └── executor/               # 执行器详细文档
│
├── sdk-templates/              # 插件 SDK 可运行模板 (Python / Node.js)
├── README.md                   # 项目说明（英文，默认）
├── README.zh-CN.md             # 项目说明（简体中文，本文件）
├── CONTRIBUTING.md             # 贡献指南与 DCO
├── SECURITY.md                 # 安全策略
├── CLI_USAGE.md                # CLI 使用文档
├── CHANGELOG.md                # 版本变更（发版唯一事实来源）
├── LICENSE                     # MIT
└── AGENTS.md                   # 项目知识库
```

> 协议插件（redis / db / mongo / grpc / kafka / smtp / mcp / llm / agent / openapi / postman / eolink）
> 各自是独立仓库，不在本仓库工作树内（`/plugins/` 被 `.gitignore` 排除），仓库清单见
> [CONTRIBUTING.md](./CONTRIBUTING.md#repository-layout)。


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

1. **后端**：在 `src-tauri/crates/flow-engine-core/src/node_types.rs` 中添加节点类型 ID 常量
   （ID 采用 `<protocol>:<operation>` 命名空间，如 `myproto:call`）
2. **后端**：在 `src-tauri/crates/flow-engine-core/src/node_config.rs` 中添加对应的 `NodeData` 变体
3. **后端**：新建 `*_executor.rs`，实现 `NodeExecutor` trait：

```rust
#[async_trait::async_trait]
impl NodeExecutor for MyNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail>;
}
```

4. **后端**：在 `src-tauri/crates/flow-engine-executor/src/default_registry.rs` 的 `default_registry()` 中注册 `NodePlugin`
5. **前端**：在 `src/components/canvas/nodes/` 中创建节点组件
6. **前端**：在 `src/components/panels/` 中创建配置面板
7. **前端**：在三个注册表中登记——`src/lib/NodeRegistry.ts`、`src/lib/configPanelRegistry.ts`、`src/lib/viewerRegistry.tsx`
8. 补充单元测试与集成测试

第三方扩展节点无需以上步骤，直接发布[插件](#插件-plugins)即可。

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

[MIT](./LICENSE) © MPE Flow contributors.

本仓库采用 MIT 许可。协议插件位于各自独立仓库，其许可证以各仓库为准。

提交 PR 前请阅读 [CONTRIBUTING.md](./CONTRIBUTING.md)：需要 DCO 签名（`git commit -s`）。
安全问题请勿公开提 issue，走 [SECURITY.md](./SECURITY.md) 的私下上报渠道。

---

**详细架构设计请参考 [docs/executor_architecture.md](./docs/executor_architecture.md)**  
**项目知识库请参考 [AGENTS.md](./AGENTS.md)**
