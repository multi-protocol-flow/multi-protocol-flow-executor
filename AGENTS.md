# 项目知识库

**生成时间:** 2026-04-16
**Commit:** 158c34c
**Branch:** master

## 概述

多协议流程编排与执行引擎。Tauri 2.x 桌面应用 + CLI 工具。支持 HTTP、WebSocket、数据库（PostgreSQL/MySQL/SQLite/Redis）协议，具备调试模式和压力测试功能。
- **前端:** React 19 + TypeScript + Vite 6 + TailwindCSS 4 + Zustand 5 + React Flow
- **后端:** Rust Workspace (9 crates) + Tauri 2.x

## 结构

```
multi-protocol-flow-executor/
├── src/                    # 前端源码 (React/TypeScript)
│   ├── components/         # UI 组件 (97 文件，详见 src/components/AGENTS.md)
│   ├── stores/             # Zustand 状态管理 (9 stores)
│   ├── services/           # Tauri IPC 服务 + Mock + Storage
│   ├── types/              # TypeScript 类型 (与 Rust 对齐)
│   ├── templates/          # 内置流程模板
│   └── __tests__/          # Vitest 单元测试
├── src-tauri/              # Rust 后端 (Workspace，详见 src-tauri/crates/AGENTS.md)
│   ├── src/                # Tauri 适配层 + IPC 命令 (详见 src-tauri/src/AGENTS.md)
│   ├── crates/             # 9 个子 crate (详见下方依赖图)
│   │   ├── flow-engine/    # 门面 crate (re-export 全部子 crate)
│   │   ├── flow-engine-core/       # 基础类型、traits、共享数据结构
│   │   ├── flow-engine-crypto/     # AES-256-GCM 加密 + OS Keychain
│   │   ├── flow-engine-http/       # HTTP 客户端 (hyper 1.x) + 执行器
│   │   ├── flow-engine-ws/         # WebSocket 连接管理 + 3 个执行器
│   │   ├── flow-engine-db/         # 数据库连接池 + 6 个执行器 (PG/MySQL/SQLite/Redis)
│   │   ├── flow-engine-script/     # QuickJS 运行时 + 脚本执行器
│   │   ├── flow-engine-assertion/  # 15 种断言操作符 + 执行器
│   │   └── flow-engine-executor/   # 主执行器、调试模式、压力测试、业务命令
│   └── bin/mpe.rs          # CLI 二进制 (mpe 命令)
├── tests/                  # CLI 测试数据 + fixtures
└── docs/                   # 架构文档
```

### Crate 依赖图

```
Layer 0 (叶子):  flow-engine-core, flow-engine-crypto
Layer 1:         flow-engine-http, flow-engine-ws, flow-engine-db,
                 flow-engine-script, flow-engine-assertion    (→ core)
Layer 2:         flow-engine-executor                          (→ core + 全部 Layer 1 + crypto)
Layer 3:         flow-engine                                   (门面，re-export 全部)
Layer 4:         app (src-tauri/)                              (→ flow-engine)
```

**增量编译关键**: 修改 Layer N 的 crate 会触发 Layer N+1 及以上重编译。core 是最关键的基础——修改它会级联全部 crate。

## 快速定位

| 任务 | 位置 |
|------|------|
| 添加新节点类型 | `src-tauri/crates/flow-engine-core/src/` (类型) + `src-tauri/crates/flow-engine-executor/src/` (执行器) + `src/types/node.ts` + `src/components/canvas/nodes/` |
| 修改执行逻辑 | `src-tauri/crates/flow-engine-executor/src/executor.rs` |
| 添加前端组件 | `src/components/` (97 组件) |
| 修改状态管理 | `src/stores/` (9 stores) |
| HTTP 请求节点 | `src-tauri/crates/flow-engine-http/` (详见该 crate 的 AGENTS.md) |
| 条件分支节点 | `src-tauri/crates/flow-engine-executor/src/conditional_executor.rs` |
| 脚本节点 | `src-tauri/crates/flow-engine-script/` (详见该 crate 的 AGENTS.md) |
| 断言节点 | `src-tauri/crates/flow-engine-assertion/` (详见该 crate 的 AGENTS.md) |
| WebSocket 节点 | `src-tauri/crates/flow-engine-ws/` (详见该 crate 的 AGENTS.md) |
| WebSocket 消息流 | `src-tauri/crates/flow-engine-core/src/ws_stream.rs` + `src/types/ws_stream.ts` |
| 数据库节点 | `src-tauri/crates/flow-engine-db/` (详见该 crate 的 AGENTS.md) |
| 调试模式 | `src-tauri/crates/flow-engine-executor/src/debug/` + `src/stores/debug.ts` |
| 压力测试 | `src-tauri/crates/flow-engine-executor/src/stress_test/` + `src/stores/stressTest.ts` |
| IPC 命令 | `src-tauri/src/tauri_commands.rs` (26+ 命令) |
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

## 增量编译 (减少编译时间)

**核心原则: 修改哪个模块就编译/测试哪个模块，避免全量重编译。**

### 单 Crate 编译/测试

```bash
cd src-tauri

# 编译单个 crate (仅触发该 crate 及其依赖重编译)
cargo build -p flow-engine-http          # Layer 1 - 仅 core + http
cargo build -p flow-engine-executor      # Layer 2 - core + 全部协议 crate + executor
cargo build -p flow-engine               # Layer 3 - 全部 (等同全量)
cargo build -p app                       # Layer 4 - 全部 + Tauri 链接

# 测试单个 crate
cargo test -p flow-engine-http           # 只测 HTTP crate
cargo test -p flow-engine-core           # 只测基础类型

# Lint 单个 crate
cargo clippy -p flow-engine-db           # 只检查 DB crate
```

### 修改影响分析

| 修改的 crate | 重编译范围 | 编译量 |
|-------------|-----------|--------|
| `flow-engine-core` | 全部 crate (级联) | **全量** |
| `flow-engine-crypto` | executor → flow-engine → app | ~3 crate |
| `flow-engine-http/ws/db/script/assertion` | executor → flow-engine → app | ~3 crate |
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
| CSS `!important` | FlowCanvas.tsx | 仅用于覆盖 ReactFlow 样式 |

## Bug 修复规范

**修复问题时必须遵守以下原则：**

1. **最小化修改**: 仅修改与问题直接相关的代码，不要改动其他代码，避免引入新问题
2. **测试验证**: 修复必须有对应的测试验证，确保功能正常
3. **回归测试**: 修复完成后必须执行回归测试（见上方"回归测试"表格），确保其他功能未受影响
4. **中文输出**: 全程使用中文输出，方便查阅

## 技术债务

| 位置 | 内容 | 优先级 |
|------|------|--------|
| `src/components/canvas/utils.ts:101` | TODO: 添加更多连接验证逻辑 | 中 |
| `src/types/error.ts` | 缺少 WebSocket 错误类型定义 (与 Rust 不对齐) | 中 |
| 无 CI/CD | 无 `.github/workflows`，需手动运行测试 | 中 |
| Workspace 依赖管理 | 仅 core crate 使用 `workspace = true`，其余 7 个 crate 硬编码版本号 | 中 |
| `src-tauri/compile_errors.txt` | 调试产物被 git 跟踪，应 gitignore | 低 |
| `components.json` | 引用不存在的 `tailwind.config.js` (TailwindCSS 4 使用 CSS 配置) | 低 |

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

# Tauri 桌面应用
npm run tauri dev    # 开发模式
npm run tauri build  # 构建安装包
```

## 注意事项

1. **双二进制输出**: `src-tauri/Cargo.toml` 定义了 `app` (Tauri 桌面应用) 和 `mpe` (独立 CLI 工具，支持 `mpe run -f flow.mpf`)

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
- [src/stores/AGENTS.md](./src/stores/AGENTS.md) - Zustand 状态管理
- [src/services/AGENTS.md](./src/services/AGENTS.md) - Tauri IPC 服务
- [src/types/AGENTS.md](./src/types/AGENTS.md) - TypeScript 类型定义
- [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md) - 前端测试
