# 项目知识库

**生成时间:** 2026-06-12 | **Branch:** dev

## 概述

多协议流程编排与执行引擎。Tauri 2.x 桌面应用 + CLI 模式（同一二进制 `mpe`）。
- **前端:** React 19 + TypeScript + Vite 6 + TailwindCSS 4 + Zustand 5 + @xyflow/react
- **后端:** Rust Workspace (20 crates) + Tauri 2.x

## 结构

```
root/
├── src/                  # 前端 (React 19 + TypeScript)
├── src-tauri/            # 后端 (Rust workspace, 20 crates)
├── tests/                # CLI 测试夹具 (JSON flow files)
├── echo/                 # 全协议 echo 测试服务 (Docker Compose)
├── docs/                 # 架构文档
├── website/              # 营销网站
└── public/               # Vite 静态资源
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 前端组件 | `src/components/` |
| 状态管理 | `src/stores/` |
| IPC 服务层 | `src/services/tauri/` |
| 类型定义 | `src/types/` |
| i18n 翻译 | `src/i18n/` |
| Rust crate 总览 | `src-tauri/crates/AGENTS.md` |
| IPC 命令 | `src-tauri/src/tauri_commands.rs` |
| 执行引擎 | `src-tauri/crates/flow-engine-executor/` |
| 协议执行器 | `src-tauri/crates/flow-engine-*/` |
| CLI 入口 | `src-tauri/src/cli/` |

## 架构

### 单二进制入口

`main.rs` 检测参数: 有参数 → CLI (clap dispatch), 无参数 → GUI (Tauri)。Windows GUI 调用 `FreeConsole()` 分离控制台。

### Crate 分层

```
Layer 0 (叶子):     flow-engine-core, flow-engine-crypto
Layer 1 (协议):     16 个协议 crate (http, ws, db, script, assertion, grpc, mcp, tcp, mqtt, amqp, graphql, imap, kafka, smtp, sse, redis)
Layer 2 (编排):     flow-engine-executor (依赖 Layer 0 + 可选 Layer 1)
Layer 3 (门面):     flow-engine (re-export 全部)
Layer 4 (应用):     app (src-tauri/) 仅依赖 flow-engine
Extra:              workspace-hack (cargo-hakari 自动管理)
```

**增量编译**: 修改 Layer N → Layer N+1 及以上重编译。修改 core → 全量。

### IPC 命令 (72 个)

注册在 `lib.rs` 的 `invoke_handler` 中。`tauri_commands.rs` 包装，委托 `flow-engine::commands::*`。

| 分类 | 数量 | 说明 |
|------|------|------|
| Flow | 6 | CRUD + execute + cancel + validate |
| Reports/Debug | 4/5 | 报告 CRUD / 调试会话控制 |
| Stress | 7 | 压测 CRUD + 执行 |
| Flow Files | 5 | .mpf 文件 CRUD |
| Folders | 6 | 文件夹管理 |
| gRPC | 8 | 服务发现、骨架生成、反射 |
| Kafka | 4 | 连接、topic、group、schema |
| OAuth2 | 3 | authorize/refresh/revoke |
| Dataset | 8 | 数据集执行 |
| 其他 | 16 | MCP/GraphQL/TCP/CSV/Registry 等 |

### 节点类型 (89+ variants)

定义在 `flow-engine-core/src/node.rs`。涵盖：entry, end, http, conditional, ws(3), db(6), script, assertion, grpc(3), mcp(5), tcp(4), udp(4), mqtt(7), kafka(6), sse(3), graphql(5), amqp(13), smtp(3), imap(12), redis(3), variable_extractor, proxy。

新增节点类型需: NodeType + NodeData + NodePlugin trait 实现 + 前端 NodeRegistry + configPanel + viewer。

## 约定

### 前端

- **i18n**: 用 `getLL()`，不用 `useI18nContext()`。新增 key 同步修改 zh-CN + en-US + i18n-types.ts
- **Store**: Zustand 5，每个 store 一个文件，必须实现 `reset()`。`useXxxStore.getState().reset()` 用于测试隔离
- **注册表**: 三重注册 — `NodeRegistry.ts`(元数据) + `configPanelRegistry.ts`(配置面板) + `viewerRegistry.tsx`(查看器)
- **TypeScript**: `strict: true`, `noUnusedLocals`, `noUnusedParameters`。禁止 `as any` / `@ts-ignore` / `@ts-expect-error`
- **路径别名**: `@/*` → `src/*`
- **类型同步**: `src/types/*.ts` 与 `flow-engine-core/src/*.rs` 必须严格对齐

### Rust

- **禁止** `unsafe_code` (workspace forbid，app crate 因 FreeConsole 需 allow)
- 无 `unwrap()` 生产代码（用 `?` 或 `expect`）
- `missing_docs = "warn"`, `correctness = "deny"`
- Clippy: all/style/complexity/perf = warn, `too_many_arguments`/`type_complexity` = allow
- 格式化: `cargo fmt`, edition 2021, min rust 1.77.2
- 16 个协议 crate 均为可选 feature (`protocol-*`)，默认 `all-protocols` 启用

## 反模式 (本项目禁止)

- 非 Bridge 组件使用 `useI18nContext()` → 翻译不随 locale 切换
- 跳过任一注册表 (`NodeRegistry`/`configPanelRegistry`/`viewerRegistry`) → 节点无法完整渲染
- 类型断言 (`as any`) → 破坏 TypeScript 严格性
- `unwrap()` 在生产 Rust 代码中 → 应传播错误
- 新增翻译 key 不更新 `i18n-types.ts` → 编译错误

## 命令

```bash
# 前端
npm run dev                     # Vite dev server (0.0.0.0:5173)
npm run typecheck               # tsc --noEmit
npm run test                    # Vitest (jsdom, 80% coverage)
npm run build                   # typecheck + vite build

# 后端 (cd src-tauri)
cargo build                     # 调试构建
cargo build --release --bin mpe # 生产构建
cargo build -p flow-engine-http # 单 crate 增量编译
cargo test                      # 全量测试
cargo clippy                    # Lint

# Tauri
npm run tauri dev               # 开发模式 (热重载)
npm run tauri build             # 构建安装包
```

## 子目录知识库

- [src/components/AGENTS.md](./src/components/AGENTS.md) — 前端组件
- [src/components/execution/AGENTS.md](./src/components/execution/AGENTS.md) — 执行组件
- [src/stores/AGENTS.md](./src/stores/AGENTS.md) — 状态管理
- [src/services/AGENTS.md](./src/services/AGENTS.md) — 服务层
- [src/types/AGENTS.md](./src/types/AGENTS.md) — 类型定义
- [src/i18n/AGENTS.md](./src/i18n/AGENTS.md) — 国际化
- [src/lib/AGENTS.md](./src/lib/AGENTS.md) — 工具函数 & 注册表
- [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md) — 测试
- [echo/AGENTS.md](./echo/AGENTS.md) — Echo 测试服务
- [src-tauri/src/AGENTS.md](./src-tauri/src/AGENTS.md) — Tauri 应用层
- [src-tauri/crates/AGENTS.md](./src-tauri/crates/AGENTS.md) — Rust Workspace 总览
- [src-tauri/crates/flow-engine/src/AGENTS.md](./src-tauri/crates/flow-engine/src/AGENTS.md) — 门面 crate
- [src-tauri/crates/flow-engine-core/src/AGENTS.md](./src-tauri/crates/flow-engine-core/src/AGENTS.md) — 基础类型
- [src-tauri/crates/flow-engine-crypto/src/AGENTS.md](./src-tauri/crates/flow-engine-crypto/src/AGENTS.md) — 加密/Keychain
- [src-tauri/crates/flow-engine-executor/src/AGENTS.md](./src-tauri/crates/flow-engine-executor/src/AGENTS.md) — 执行器
- [src-tauri/crates/flow-engine-http/src/AGENTS.md](./src-tauri/crates/flow-engine-http/src/AGENTS.md) — HTTP
- [src-tauri/crates/flow-engine-ws/src/AGENTS.md](./src-tauri/crates/flow-engine-ws/src/AGENTS.md) — WebSocket
- [src-tauri/crates/flow-engine-db/src/AGENTS.md](./src-tauri/crates/flow-engine-db/src/AGENTS.md) — 数据库
- [src-tauri/crates/flow-engine-script/src/AGENTS.md](./src-tauri/crates/flow-engine-script/src/AGENTS.md) — 脚本
- [src-tauri/crates/flow-engine-assertion/src/AGENTS.md](./src-tauri/crates/flow-engine-assertion/src/AGENTS.md) — 断言
- [src-tauri/crates/flow-engine-grpc/src/AGENTS.md](./src-tauri/crates/flow-engine-grpc/src/AGENTS.md) — gRPC
- [src-tauri/crates/flow-engine-redis/src/AGENTS.md](./src-tauri/crates/flow-engine-redis/src/AGENTS.md) — Redis
- [src-tauri/crates/flow-engine-mcp/src/AGENTS.md](./src-tauri/crates/flow-engine-mcp/src/AGENTS.md) — MCP
- [src-tauri/crates/flow-engine-tcp/src/AGENTS.md](./src-tauri/crates/flow-engine-tcp/src/AGENTS.md) — TCP/UDP
- [src-tauri/crates/flow-engine-mqtt/src/AGENTS.md](./src-tauri/crates/flow-engine-mqtt/src/AGENTS.md) — MQTT
- [src-tauri/crates/flow-engine-amqp/src/AGENTS.md](./src-tauri/crates/flow-engine-amqp/src/AGENTS.md) — AMQP
- [src-tauri/crates/flow-engine-graphql/src/AGENTS.md](./src-tauri/crates/flow-engine-graphql/src/AGENTS.md) — GraphQL
- [src-tauri/crates/flow-engine-imap/src/AGENTS.md](./src-tauri/crates/flow-engine-imap/src/AGENTS.md) — IMAP
- [src-tauri/crates/flow-engine-kafka/src/AGENTS.md](./src-tauri/crates/flow-engine-kafka/src/AGENTS.md) — Kafka
- [src-tauri/crates/flow-engine-smtp/src/AGENTS.md](./src-tauri/crates/flow-engine-smtp/src/AGENTS.md) — SMTP
- [src-tauri/crates/flow-engine-sse/src/AGENTS.md](./src-tauri/crates/flow-engine-sse/src/AGENTS.md) — SSE
