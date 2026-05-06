# 项目知识库

**生成时间:** 2026-04-30
**Commit:** ddd1819
**Branch:** master

## 概述

多协议流程编排与执行引擎。Tauri 2.x 桌面应用 + CLI 工具。支持 HTTP、WebSocket、数据库（PostgreSQL/MySQL/SQLite/Redis）、gRPC、MCP、TCP/UDP、MQTT 协议，具备调试模式和压力测试功能。
- **前端:** React 19 + TypeScript + Vite 6 + TailwindCSS 4 + Zustand 5 + React Flow
- **后端:** Rust Workspace (19 crates) + Tauri 2.x

## 结构

```
multi-protocol-flow-executor/
├── src/                    # 前端源码 (React/TypeScript)
│   ├── components/         # UI 组件 (~111 文件)
│   ├── stores/             # Zustand 状态管理 (10 stores)
│   ├── services/           # Tauri IPC 服务 + Mock + Storage
│   ├── types/              # TypeScript 类型 (36 种节点)
│   ├── hooks/              # 自定义 React Hooks (3 个)
│   ├── layouts/            # 布局组件 (MainLayout)
│   ├── lib/                # 工具函数 + 注册表 (7 个模块)
│   ├── templates/          # 内置流程模板
│   ├── test-utils/         # 测试工厂函数
│   └── __tests__/          # Vitest 单元测试 (~76 文件)
├── src-tauri/              # Rust 后端 (Workspace)
│   ├── src/                # Tauri 适配层 + IPC 命令 (49 命令)
│   ├── crates/             # 19 个子 crate
│   │   ├── flow-engine/    # 门面 crate (re-export)
│   │   ├── flow-engine-core/       # 基础类型、traits、插件注册
│   │   ├── flow-engine-crypto/     # AES-256-GCM 加密 + OS Keychain
│   │   ├── flow-engine-http/       # HTTP 客户端 (hyper 1.x) + 执行器
│   │   ├── flow-engine-ws/         # WebSocket 连接管理 + 3 个执行器
│   │   ├── flow-engine-db/         # 数据库连接池 + 6 个执行器
│   │   ├── flow-engine-script/     # QuickJS 运行时 + 脚本执行器
│   │   ├── flow-engine-assertion/  # 15 种断言操作符 + 执行器
│   │   ├── flow-engine-grpc/       # gRPC 客户端 (tonic) + 3 个执行器
│   │   ├── flow-engine-mcp/        # MCP 客户端 + 5 个执行器
│   │   ├── flow-engine-tcp/        # TCP/UDP 客户端 + 3 TCP + 4 UDP 执行器
│   │   ├── flow-engine-mqtt/        # MQTT 5.0 客户端 (rumqttc-next) + 7 个执行器
│   │   ├── flow-engine-executor/   # 主执行器、调试模式、压力测试、插件注册
│   │   ├── flow-engine-amqp/       # AMQP 0-9-1 + 1.0 客户端 + 2 个子模块
│   │   ├── flow-engine-graphql/    # GraphQL 客户端 (支持内省/查询/订阅)
│   │   ├── flow-engine-imap/       # IMAP 客户端 (支持收发/搜索/文件夹管理)
│   │   ├── flow-engine-kafka/      # Kafka 客户端 (支持生产/消费/Schema Registry)
│   │   ├── flow-engine-smtp/       # SMTP 客户端 (支持 SASL/OAuth2 认证)
│   │   └── flow-engine-sse/        # SSE 客户端 (支持连接/监听/断开)
│   └── bin/mpe/            # CLI 工具 (6 子命令, mpe run/validate/debug/stress/flow/report)
├── tests/                  # CLI 测试数据 + fixtures
└── docs/                   # 架构文档
```

### Crate 依赖图

```
Layer 0 (叶子):  flow-engine-core, flow-engine-crypto
Layer 1:         flow-engine-http, flow-engine-ws, flow-engine-db,
                 flow-engine-script, flow-engine-assertion,
                 flow-engine-grpc, flow-engine-mcp, flow-engine-tcp,
                 flow-engine-mqtt, flow-engine-amqp, flow-engine-graphql,
                 flow-engine-imap, flow-engine-kafka, flow-engine-smtp,
                 flow-engine-sse  (→ core)
Layer 2:         flow-engine-executor                                (→ core + 全部 Layer 1 + crypto)
Layer 3:         flow-engine                                         (门面，re-export 全部)
Layer 4:         app (src-tauri/)                                    (→ flow-engine)
```

**增量编译关键**: 修改 Layer N 的 crate 会触发 Layer N+1 及以上重编译。core 是最关键的基础——修改它会级联全部 crate。

## 快速定位

| 任务 | 位置 |
|------|------|
| 添加新节点类型 | `src-tauri/crates/flow-engine-core/src/` (类型+插件) + `src-tauri/crates/flow-engine-executor/src/` (执行器) + `src/types/node.ts` + `src/components/canvas/nodes/` + `src/lib/NodeRegistry.ts` |
| 修改执行逻辑 | `src-tauri/crates/flow-engine-executor/src/executor.rs` |
| 添加前端组件 | `src/components/` (~111 组件) |
| 修改状态管理 | `src/stores/` (10 stores) |
| HTTP 请求节点 | `src-tauri/crates/flow-engine-http/` (详见该 crate 的 AGENTS.md) |
| 条件分支节点 | `src-tauri/crates/flow-engine-executor/src/conditional_executor.rs` |
| 脚本节点 | `src-tauri/crates/flow-engine-script/` (详见该 crate 的 AGENTS.md) |
| 断言节点 | `src-tauri/crates/flow-engine-assertion/` (详见该 crate 的 AGENTS.md) |
| WebSocket 节点 | `src-tauri/crates/flow-engine-ws/` (详见该 crate 的 AGENTS.md) |
| WebSocket 消息流 | `src-tauri/crates/flow-engine-core/src/ws_stream.rs` + `src/types/ws_stream.ts` |
| 数据库节点 | `src-tauri/crates/flow-engine-db/` (详见该 crate 的 AGENTS.md) |
| gRPC 节点 | `src-tauri/crates/flow-engine-grpc/` (详见该 crate 的 AGENTS.md) |
| gRPC 服务发现 | `src-tauri/crates/flow-engine-grpc/src/proto_parser.rs` + `grpc_connection_manager.rs` |
| MCP 节点 | `src-tauri/crates/flow-engine-mcp/` (详见该 crate 的 AGENTS.md) |
| TCP/UDP 节点 | `src-tauri/crates/flow-engine-tcp/` (详见该 crate 的 AGENTS.md) |
| MQTT 节点 | `src-tauri/crates/flow-engine-mqtt/` (详见该 crate 的 AGENTS.md) |
| AMQP 节点 | `src-tauri/crates/flow-engine-amqp/` (详见该 crate 的 AGENTS.md) |
| GraphQL 节点 | `src-tauri/crates/flow-engine-graphql/` (详见该 crate 的 AGENTS.md) |
| IMAP 节点 | `src-tauri/crates/flow-engine-imap/` (详见该 crate 的 AGENTS.md) |
| Kafka 节点 | `src-tauri/crates/flow-engine-kafka/` (详见该 crate 的 AGENTS.md) |
| SMTP 节点 | `src-tauri/crates/flow-engine-smtp/` (详见该 crate 的 AGENTS.md) |
| SSE 节点 | `src-tauri/crates/flow-engine-sse/` (详见该 crate 的 AGENTS.md) |
| 插件注册表 | `src-tauri/crates/flow-engine-core/src/registry.rs` + `plugin.rs` |
| 前端注册表 | `src/lib/NodeRegistry.ts` + `configPanelRegistry.ts` + `viewerRegistry.tsx` |
| 调试模式 | `src-tauri/crates/flow-engine-executor/src/debug/` + `src/stores/debug.ts` |
| 压力测试 | `src-tauri/crates/flow-engine-executor/src/stress_test/` + `src/stores/stressTest.ts` |
| IPC 命令 | `src-tauri/src/tauri_commands.rs` (49 命令) |
| 加密模块 | `src-tauri/crates/flow-engine-crypto/` |
| 节点类型定义 | `src-tauri/crates/flow-engine-core/src/node.rs` + `node_config.rs` |
| 执行上下文/变量 | `src-tauri/crates/flow-engine-core/src/context.rs` + `variable.rs` |
| 错误分类 | `src-tauri/crates/flow-engine-core/src/error.rs` |

## 代码规范

### TypeScript
- `strict: true`, `noUnusedLocals`, `noUnusedParameters`
- 路径别名: `@/*` → `src/*`
- **禁止** `as any`, `@ts-ignore`, `@ts-expect-error`

### Rust
- **禁止** `unsafe_code` (forbid)
- `correctness = "deny"` - 正确性问题报错
- 格式化: `cargo fmt`, Lint: `cargo clippy`
- 无 `unwrap()` 调用（使用 `?` 或 `expect`）

### 通用
- 禁止空 catch 块
- 禁止删除失败测试来"通过"
- 注释使用中文

### 架构约定
- NodePlugin trait + NodeRegistry 替代旧的 match 分发模式，新增节点类型只需实现 trait 并注册
- 前端 3 个注册表: NodeRegistry (后端元数据), configPanelRegistry (配置面板), viewerRegistry (结果查看器)

## 增量编译 (减少编译时间)

**核心原则: 修改哪个模块就编译/测试哪个模块，避免全量重编译。**

### 单 Crate 编译/测试

```bash
cd src-tauri

# 编译单个 crate (仅触发该 crate 及其依赖重编译)
cargo build -p flow-engine-http          # Layer 1 - 仅 core + http
cargo build -p flow-engine-grpc          # Layer 1 - 仅 core + grpc
cargo build -p flow-engine-executor      # Layer 2 - core + 全部协议 crate + executor
cargo build -p flow-engine               # Layer 3 - 全部 (等同全量)
cargo build -p app                       # Layer 4 - 全部 + Tauri 链接

# 测试单个 crate
cargo test -p flow-engine-http           # 只测 HTTP crate
cargo test -p flow-engine-grpc           # 只测 gRPC crate
cargo test -p flow-engine-core           # 只测基础类型

# Lint 单个 crate
cargo clippy -p flow-engine-db           # 只检查 DB crate
cargo clippy -p flow-engine-grpc         # 只检查 gRPC crate
```

### 修改影响分析

| 修改的 crate | 重编译范围 | 编译量 |
|-------------|-----------|--------|
| `flow-engine-core` | 全部 crate (级联) | **全量** |
| `flow-engine-crypto` | executor → flow-engine → app | ~3 crate |
| `flow-engine-http/ws/db/script/assertion/grpc/mcp/tcp/mqtt/amqp/graphql/imap/kafka/smtp/sse` | executor → flow-engine → app | ~3 crate |
| `flow-engine-executor` | flow-engine → app | ~2 crate |
| `flow-engine` (门面) | app | ~1 crate |
| `src-tauri/src/*` (Tauri 层) | app | ~1 crate |

### 开发循环建议

```bash
# 场景: 修改 HTTP 执行器
cd src-tauri
cargo test -p flow-engine-http           # 快速测试
cargo clippy -p flow-engine-http         # 快速 lint
# 满意后再全量验证
cargo test && cargo clippy               # 完整回归

# 场景: 修改 gRPC 执行器
cargo test -p flow-engine-grpc           # 快速测试
cargo clippy -p flow-engine-grpc         # 快速 lint

# 场景: 修改基础类型 (core)
cargo test -p flow-engine-core           # 先测 core
cargo test -p flow-engine-executor       # 再测依赖 core 的执行器
# core 修改影响全量，务必跑完整回归

# 场景: 只改前端
npm run typecheck && npm run test        # 无需编译 Rust
```

## 回归测试 (强制)

**任何功能变更必须执行回归测试。**

| 变更类型 | 命令 |
|----------|------|
| 前端 | `npm run typecheck && npm run test` |
| 单个 Rust crate | `cd src-tauri && cargo test -p <crate> && cargo clippy -p <crate>` |
| 后端 (全量) | `cd src-tauri && cargo test && cargo clippy` |
| CLI | `cd src-tauri && cargo test cli_tests` |
| 完整回归 | `npm run typecheck && npm run test && cd src-tauri && cargo test && cargo clippy` |

**禁止:** 跳过测试直接提交、删除失败测试、忽略 clippy 警告

## 测试

### 测试命令

| 环境 | 命令 |
|------|------|
| 前端单元测试 | `npm run test` |
| 前端覆盖率 | `npm run test:coverage` |
| 单个 crate 测试 | `cd src-tauri && cargo test -p <crate-name>` |
| 后端单元测试 (全量) | `cd src-tauri && cargo test` |
| 后端覆盖率 | `cd src-tauri && cargo llvm-cov` |
| 后端 Lint | `cd src-tauri && cargo clippy` |

### 测试文件位置

| 类型 | 位置 |
|------|------|
| 前端单元测试 | `src/__tests__/` (详见 [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md)) |
| 后端单元测试 | `src-tauri/src/*/tests.rs` + `src-tauri/tests/` |
| CLI 集成测试 | `src-tauri/tests/cli_tests.rs` |
| 测试数据 | `tests/` |

### TDD 工作流

1. **RED**: 先写失败的测试
2. **GREEN**: 写最少代码使测试通过
3. **REFACTOR**: 重构代码保持测试通过

### 测试模式

| 场景 | 模式 |
|------|------|
| Zustand Store | 使用 `getState().reset()` 隔离测试状态 |
| Tauri IPC | 使用 `src/services/tauri/mock.ts` 或 `@tauri-apps/api/mocks` 的 `mockIPC` |
| React Flow | Mock `useReactFlow`/`useNodes`/`useEdges` |
| 用户交互 | 优先使用 `userEvent` 而非 `fireEvent` |
| 浏览器 API | 在 `src/__tests__/setup.ts` 添加 polyfill |

### 测试约定

1. jsdom 环境已在 vitest.config.ts 全局配置，组件测试无需单独标记 `/** @vitest-environment jsdom */`
2. Store 测试在 `beforeEach` 中调用 `reset()` 重置状态
3. 测试文件末尾调用 `vi.restoreAllMocks()` 清理 mock
4. E2E 测试文件命名 `*.e2e.test.ts`

## 反模式 (本项目禁止)

| 模式 | 位置 | 说明 |
|------|------|------|
| `unsafe_code` | Rust | forbid 级别禁止 |
| `as any` | TypeScript | 类型安全红线 |
| `@ts-ignore` | TypeScript | 禁止压制类型错误 |
| `unwrap()` 生产代码 | `core/src/executor.rs:32-33` | 2 个违规 (regex captures) |
| `eslint-disable` 生产组件 | `panels/*.tsx` | 7 处 react-hooks/exhaustive-deps |

## Bug 修复规范

**修复问题时必须遵守以下原则：**

1. **最小化修改**: 仅修改与问题直接相关的代码，不要改动其他代码，避免引入新问题
2. **测试验证**: 修复必须有对应的测试验证，确保功能正常
3. **回归测试**: 修复完成后必须执行回归测试（见上方"回归测试"表格），确保其他功能未受影响
4. **中文输出**: 全程使用中文输出，方便查阅

## 技术债务

| 位置 | 内容 | 优先级 |
|------|------|--------|
| 无 CI/CD | 无 `.github/workflows`，需手动运行测试 | 高 |
| `src/types/error.ts` | 缺少 WebSocket 错误类型定义 (与 Rust 不对齐) | 中 |
| Workspace 依赖管理 | 部分 crate 硬编码版本号，未统一使用 workspace = true | 中 |
| 生产代码 unwrap() | `core/executor.rs:32-33` 违反无 unwrap 规则 | 中 |
| 无 ESLint/Prettier | 前端无 lint/format 配置，但代码中有 eslint-disable | 中 |
| CSP 未配置 | tauri.conf.json csp: null，生产安全风险 | 中 |
| `components.json` | 引用不存在的 `tailwind.config.js` | 低 |

## 命令

```bash
# 前端开发
npm run dev          # Vite 开发服务器 (0.0.0.0:5173)
npm run build        # tsc + vite build
npm run test         # Vitest 测试
npm run typecheck    # TypeScript 类型检查

# 后端开发
cd src-tauri
cargo build          # 调试构建
cargo build --release --bin mpe  # 生产 CLI
cargo test           # Rust 测试
cargo clippy         # Lint 检查

# 单 crate 测试/Lint
cargo test -p flow-engine-grpc && cargo clippy -p flow-engine-grpc

# Tauri 桌面应用
npm run tauri dev    # 开发模式
npm run tauri build  # 构建安装包
```

## IPC 命令 (49)

| 分类 | 命令 |
|------|------|
| Flow (5) | create_flow, execute_flow, cancel_flow, update_node, delete_node |
| Reports (4) | save/load/list/delete_flow_report |
| Stress Reports (4) | save/load/list/delete_stress_test_report |
| Debug (5) | start_debug, send_debug_command, end_debug, get_debug_status, force_end_debug |
| Stress Test (3) | start/stop/get_stress_test_status |
| Database (1) | test_db_connection |
| gRPC (2) | grpc_discover_services, grpc_generate_request_skeleton |
| MCP (1) | mcp_discover_tools |
| Settings (2) | get_app_data_dir, open_directory |
| Flow Files (5) | list/load/save/delete/copy_flow_file |
| Folders (6) | list/create/delete/rename_folder, move_flow_to_folder, move_folder |
| Registry (3) | get_registered_node_types, get_node_config_schema, list_registered_plugins |
| Dataset (6) | validate_dataset_csv, execute_flow_with_dataset, cancel_dataset_execution, list/load/delete_dataset_report, load_dataset_row_detail |
| MQTT (1) | mqtt_test_connection |

## 注意事项

1. **双二进制输出**: `src-tauri/Cargo.toml` 定义了 `app` (Tauri 桌面应用) 和 `mpe` (独立 CLI 工具，6 子命令: run/validate/debug/stress/flow/report)

2. **TailwindCSS 4**: 使用 CSS-based 配置 (`@import "tailwindcss"` + `@theme`)，无 `tailwind.config.js`

3. **类型同步**: 前端 `src/types/` 与后端 `src-tauri/src/` 结构体必须保持一致:
   - `node.ts` ↔ `node.rs` + `node_config.rs`
   - `flow.ts` ↔ `flow.rs`
   - `connection.ts` ↔ `connection.rs`

4. **Tauri IPC Mock**: `src/services/tauri/mock.ts` 用于非 Tauri 环境测试

5. **缺失 CI/CD**: 无 `.github/workflows`，需手动运行测试

6. **HTTP Body Types**: 支持 `none`、`body`、`form-data`、x-www-form-urlencoded`、`binary`，Content-Type 自动设置，用户手动设置优先

7. **HTTP 节点增强**: Content-Type 选择器、form-data 文件上传、超时配置 (`timeout_ms`，默认 30000ms，范围 1000-300000ms)

8. **内置流程模板**: 首次启动时自动加载示例流程 (`src/templates/builtinFlows.ts`)，变量引用使用顶级键 `status`、`body`、`headers`

9. **全局设置**: 报告/日志目录配置，持久化使用 tauri-plugin-store（Tauri）/ localStorage（Mock），默认位置 `{appDataDir}/reports/` 和 `{appDataDir}/logs/`

10. **WebSocket 支持**: 三种节点类型 - `ws_connect` (连接)、`ws_send_wait` (发送等待)、`ws_close` (关闭)，支持 wait 模式 (connect/wait/both)

11. **调试模式**: 支持断点、单步执行、继续、跳过节点，通过 Channel 实现实时日志流，状态管理使用 `useDebugStore`

12. **错误分类**: `ErrorKind` 区分致命错误 (`configuration`, `condition_syntax`) 和可恢复错误，可恢复错误路由到 false 端口

13. **数据库支持**: PostgreSQL、MySQL、SQLite、Redis 四种数据库，支持连接池、事务管理（隔离级别），6 种数据库节点：`db_connect`、`db_query`、`db_begin`、`db_commit`、`db_rollback`、`db_close`

14. **脚本节点**: QuickJS 运行时，支持 `fn.*` API (variables, response, console, flow, util, expect)，详见 `docs/script-node.md`

15. **断言节点**: 15 种操作符 (eq, ne, gt, gte, lt, lte, contains, matches, exists, in, between 等)，支持预设断言和 All/Any 模式

16. **压力测试**: 并发用户数配置、渐进启动、持续时长、实时进度（RPS、延迟、迭代数）、最终报告（p50/p90/p95/p99 响应时间分布）

17. **加密模块**: AES-256-GCM 加密存储数据库密码，集成 OS Keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service)

18. **gRPC 支持**: 三种节点类型 - `grpc_connect` (连接+服务发现)、`grpc_call` (调用，支持 Unary/SS/CS/Bidi 四种模式)、`grpc_close` (关闭)。运行时 .proto 解析 (protox + prost-reflect)，无需编译时代码生成。TLS 支持明文/标准/跳过验证三种模式

19. **MCP 支持**: 五种节点类型 - `mcp_connect` (连接)、`mcp_call_tool` (调用工具)、`mcp_read_resource` (读取资源)、`mcp_get_prompt` (获取提示)、`mcp_close` (关闭)。支持 MCP 协议工具发现，通过 `src/services/tauri/mcp.ts` 提供前端服务层

20. **TCP/UDP 支持**: 三种 TCP 节点 - `tcp_connect` (连接)、`tcp_send` (发送)、`tcp_close` (关闭)，四种 UDP 节点 - `udp_bind` (绑定)、`udp_send_to` (发送)、`udp_recv_from` (接收)、`udp_close` (关闭)。支持 TLS、编解码配置、帧协议、消息录制

21. **MQTT 支持**: 七种节点类型 - `mqtt_connect` (连接)、`mqtt_publish` (发布)、`mqtt_subscribe` (订阅)、`mqtt_unsubscribe` (取消订阅)、`mqtt_disconnect` (断开)、`mqtt_wait_message` (等待消息)、`mqtt_request` (请求/响应)。支持 MQTT 5.0 协议，QoS 0/1/2，Retain 消息，遗嘱消息(LWT)，TLS，WebSocket 传输

22. **AMQP 支持**: 两种协议版本 - AMQP 0-9-1 (RabbitMQ 兼容) 和 AMQP 1.0。三种节点类型 - `amqp_connect` (连接)、`amqp_publish` (发布消息)、`amqp_consume` (消费消息)。支持交换器声明、队列绑定、消息确认、事务处理

23. **GraphQL 支持**: 四种节点类型 - `graphql_introspect` (内省)、`graphql_query` (查询)、`graphql_subscribe` (订阅)、`graphql_unsubscribe` (取消订阅)。支持运行时内省获取 Schema、查询/变更操作、实时订阅、变量绑定

24. **IMAP 支持**: 五种节点类型 - `imap_connect` (连接)、`imap_fetch` (获取邮件)、`imap_search` (搜索邮件)、`imap_folder` (文件夹管理)、`imap_idle` (IDLE 监听)。支持 SSL/TLS、PLAIN/LOGIN/XOAUTH2 认证、邮件解析、附件下载

25. **Kafka 支持**: 四种节点类型 - `kafka_produce` (生产消息)、`kafka_consume` (消费消息)、`kafka_batch_consume` (批量消费)、`kafka_admin` (管理操作)。支持 Schema Registry、消费者组、偏移量管理、主题管理、分区控制

26. **SMTP 支持**: 三种节点类型 - `smtp_connect` (连接)、`smtp_send` (发送邮件)、`smtp_close` (关闭)。支持 PLAIN/LOGIN/CRAM-MD5/XOAUTH2 认证、TLS/STARTTLS、多收件人、附件发送、HTML/纯文本内容

27. **SSE 支持**: 三种节点类型 - `sse_connect` (连接)、`sse_listen` (监听事件)、`sse_close` (关闭)。支持自动重连、自定义事件过滤、Last-Event-ID 恢复、多事件流并发

## 子目录知识库

- [src/components/AGENTS.md](./src/components/AGENTS.md) - 前端组件架构
- [src-tauri/src/AGENTS.md](./src-tauri/src/AGENTS.md) - Tauri 应用层
- [src-tauri/crates/AGENTS.md](./src-tauri/crates/AGENTS.md) - Rust Workspace 总览 + 增量编译
- [src-tauri/crates/flow-engine/src/AGENTS.md](./src-tauri/crates/flow-engine/src/AGENTS.md) - Flow Engine 门面 crate
- [src-tauri/crates/flow-engine-core/src/AGENTS.md](./src-tauri/crates/flow-engine-core/src/AGENTS.md) - 基础类型 crate
- [src-tauri/crates/flow-engine-executor/src/AGENTS.md](./src-tauri/crates/flow-engine-executor/src/AGENTS.md) - 执行器 crate
- [src-tauri/crates/flow-engine-http/src/AGENTS.md](./src-tauri/crates/flow-engine-http/src/AGENTS.md) - HTTP crate
- [src-tauri/crates/flow-engine-db/src/AGENTS.md](./src-tauri/crates/flow-engine-db/src/AGENTS.md) - 数据库 crate
- [src-tauri/crates/flow-engine-ws/src/AGENTS.md](./src-tauri/crates/flow-engine-ws/src/AGENTS.md) - WebSocket crate
- [src-tauri/crates/flow-engine-script/src/AGENTS.md](./src-tauri/crates/flow-engine-script/src/AGENTS.md) - 脚本 crate
- [src-tauri/crates/flow-engine-assertion/src/AGENTS.md](./src-tauri/crates/flow-engine-assertion/src/AGENTS.md) - 断言 crate
- [src-tauri/crates/flow-engine-grpc/src/AGENTS.md](./src-tauri/crates/flow-engine-grpc/src/AGENTS.md) - gRPC crate
- [src-tauri/crates/flow-engine-mcp/src/AGENTS.md](./src-tauri/crates/flow-engine-mcp/src/AGENTS.md) - MCP crate
- [src-tauri/crates/flow-engine-tcp/src/AGENTS.md](./src-tauri/crates/flow-engine-tcp/src/AGENTS.md) - TCP/UDP crate
- [src-tauri/crates/flow-engine-mqtt/src/AGENTS.md](./src-tauri/crates/flow-engine-mqtt/src/AGENTS.md) - MQTT crate
- [src-tauri/crates/flow-engine-amqp/src/AGENTS.md](./src-tauri/crates/flow-engine-amqp/src/AGENTS.md) - AMQP crate
- [src-tauri/crates/flow-engine-graphql/src/AGENTS.md](./src-tauri/crates/flow-engine-graphql/src/AGENTS.md) - GraphQL crate
- [src-tauri/crates/flow-engine-imap/src/AGENTS.md](./src-tauri/crates/flow-engine-imap/src/AGENTS.md) - IMAP crate
- [src-tauri/crates/flow-engine-kafka/src/AGENTS.md](./src-tauri/crates/flow-engine-kafka/src/AGENTS.md) - Kafka crate
- [src-tauri/crates/flow-engine-smtp/src/AGENTS.md](./src-tauri/crates/flow-engine-smtp/src/AGENTS.md) - SMTP crate
- [src-tauri/crates/flow-engine-sse/src/AGENTS.md](./src-tauri/crates/flow-engine-sse/src/AGENTS.md) - SSE crate
- [src/stores/AGENTS.md](./src/stores/AGENTS.md) - Zustand 状态管理
- [src/services/AGENTS.md](./src/services/AGENTS.md) - Tauri IPC 服务
- [src/types/AGENTS.md](./src/types/AGENTS.md) - TypeScript 类型定义
- [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md) - 前端测试
