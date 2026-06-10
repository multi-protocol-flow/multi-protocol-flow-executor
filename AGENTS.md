# 项目知识库

**生成时间:** 2026-06-09 | **Branch:** dev

## 概述

多协议流程编排与执行引擎。Tauri 2.x 桌面应用 + CLI 模式（同一二进制 `mpe`，无参数启动 GUI，有参数启动 CLI）。
- **前端:** React 19 + TypeScript + Vite 6 + TailwindCSS 4 + Zustand 5 + @xyflow/react
- **后端:** Rust Workspace (20 crates) + Tauri 2.x

## 开发命令

```bash
# 前端
npm run dev                     # Vite dev server (0.0.0.0:5173, strictPort)
npm run typecheck               # tsc --noEmit 类型检查
npm run test                    # Vitest 测试
npm run build                   # npm-run-all typecheck + vite build
npm run typesafe-i18n           # 手动触发 i18n 类型生成

# 后端 (cd src-tauri)
cargo build                     # 调试构建
cargo build --release --bin mpe # 生产构建
cargo test                      # 全量测试
cargo clippy                    # Lint 检查
cargo build -p flow-engine-http # 单 crate 编译 (增量)

# Tauri
npm run tauri dev               # 开发模式 (前端+后端热重载)
npm run tauri build             # 构建安装包
```

## 关键架构

### 单二进制入口 (src-tauri/src/main.rs)
```
无参数 → GUI (Tauri 桌面应用)
有参数 → CLI (mpe run/validate/debug/stress/flow/report)
```
Windows GUI 模式下调用 `FreeConsole()` 分离控制台。

### Crate 分层 (20 crates + 1 app)

```
Layer 0 (叶子):     flow-engine-core, flow-engine-crypto
Layer 1 (协议):     15 个协议 crate → 依赖 core
Layer 2 (编排):     flow-engine-executor → 依赖 Layer 0 + 全部 Layer 1
Layer 3 (门面):     flow-engine → re-export 全部
Layer 4 (应用):     app (src-tauri/) → 依赖 flow-engine
Extra (工具):       workspace-hack → 所有 crate 依赖（自动管理）
```

**增量编译关键**: 修改 Layer N 触发 Layer N+1 及以上重编译。修改 core 触发全量。

### IPC 命令 (72 个)

实际注册在 `lib.rs` 的 `invoke_handler` 中，按分类：

| 分类 | 数量 | 说明 |
|------|------|------|
| Flow | 6 | create/execute/cancel/update/delete/validate_flow |
| Reports | 4 | save/load/list/delete_flow_report |
| Debug | 5 | start/send/end/get_status/force_end_debug |
| Stress Test | 3 | start/stop/get_stress_test_status |
| Stress Reports | 4 | save/load/list/delete_stress_test_report |
| Flow Files | 5 | list/load/save/delete/copy_flow_file |
| Folders | 6 | list/create/delete/rename_folder, move_flow/move_folder |
| Registry | 3 | get_registered_node_types/get_config_schema/list_plugins |
| Dataset | 8 | validate/execute/cancel/save/list/load/delete/load_row_detail |
| gRPC | 8 | discover/generate_skeleton/generate_from_reflection/validate/scan/read/parse/channelz — **not just 2** |
| Kafka | 4 | test_connection/list_topics/list_consumer_groups/discover_schema |
| Email OAuth2 | 3 | authorize/refresh/revoke |
| Protocol Test | 6 | test_db_connection, mqtt/sse/amqp/smtp/imap_test_connection |
| Other | 8 | mcp_discover_tools, graphql_introspect_schema, tcp_list_templates, tcp_get_template, save_csv_template, get_app_data_dir, open_directory, validate_flow |
| | | total: 72 |

### 节点类型 (89 variants)

定义在 `flow-engine-core/src/node.rs`。覆盖：entry, end, http, conditional, ws(3), db(6), script, assertion, grpc(3), mcp(5), tcp(4), udp(4), mqtt(7), kafka(6), sse(3), graphql(5), amqp(13), smtp(3), imap(12), variable_extractor, proxy。

新增节点类型需：NodeType + NodeData + NodePlugin trait 实现 + 前端 NodeRegistry + configPanel + viewer。

## 前端关键约定

### i18n (typesafe-i18n v5.26.2)

- **获取翻译**: 只能用 `getLL()`，禁止 `useI18nContext()` / `contextLL`（不兼容 locale 切换）
- **模块**: 9 个翻译文件：common / nodes / panels / execution / settings / flow / validation / welcome / shortcuts
- **新增 key**: 同步修改 `zh-CN/<module>.ts` + `en-US/<module>.ts` + `src/i18n/i18n-types.ts` 类型块（手动维护，无自动生成）
- **locale 切换**: 由 `App.tsx` 的 `I18nBridge` 触发全树重渲染

### 注册表 (3 个前端注册表)

- `NodeRegistry.ts` — 后端元数据
- `configPanelRegistry.ts` — 配置面板
- `viewerRegistry.tsx` — 结果查看器

### Zustand Stores (11 个)

`console`, `datasetExecution`, `debug`, `execution`, `flow`, `flowList`, `settings`, `stressTest`, `ui`, `wsStream`, `zoom` (不含 `index.ts` barrel)

### TypeScript 红线

- `strict: true`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`
- **禁止** `as any`, `@ts-ignore`, `@ts-expect-error`
- 路径别名 `@/*` → `src/*`

## Rust 关键约定

- **禁止** `unsafe_code`（workspace forbid，但 app crate 因 FreeConsole 需要 allow）
- `missing_docs = "warn"`，`correctness = "deny"`
- Clippy: all/style/complexity/perf = warn, too_many_arguments/type_complexity = allow
- 格式化: `cargo fmt`
- 无 `unwrap()` 生产代码（使用 `?` 或 `expect`）
- Edition 2021, min rust 1.77.2

## 编译优化（2026-06-10 实施）

### 1. Linker 优化（Windows）
创建 `.cargo/config.toml`，用 `rust-lld.exe` 替代默认的 MSVC `link.exe`，链接阶段加速 **15-25%**。

```toml
# src-tauri/.cargo/config.toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"
```

### 2. Dev Profile 优化

```toml
[profile.dev]
debug = "line-tables-only"      # 保留行号回溯，去掉变量级调试信息
split-debuginfo = "unpacked"    # Windows 上加速链接

[profile.dev.package."*"]
debug = false                   # 依赖项去掉调试符号
```

重依赖预编译为 O3（共 26 个 crate 覆盖）：tonic, prost, fe2o3-amqp, rdkafka, sqlx, lapin, rmcp, rquickjs, tokio-tungstenite, tauri, tauri-runtime-wry, wry, reqwest, hyper, h2, rustls, rustls-webpki, tokio, tokio-util, tokio-stream, tokio-rustls, syn, proc-macro2, quote, serde, serde_json, regex, chrono。

### 3. Release 编译优化

```toml
lto = true, codegen-units = 1, strip = true, panic = "abort", opt-level = "z"
```

### 4. 统一 workspace 依赖
将公共依赖纳入 `[workspace.dependencies]`，避免多版本重复编译：

| 新增依赖 | 覆盖 crate 数 | 版本 |
|---------|--------------|------|
| base64 | 8 | 0.22 |
| rustls | 7 | 0.23 |
| webpki-roots | 7 | 0.26 |
| tokio-rustls | 4 | 0.26 |
| futures-util | 7 | 0.3 |
| http | 3 | 1 |
| rustls-pemfile | 3 | 2 |
| jsonpath-rust | 3 | 1.0.4 |

### 5. tokio 特性按需声明
从 workspace 全局移除了 `signal` 和 `fs` 特性，改为由实际使用的 crate 自行声明：
- `flow-engine-http`: `features = ["fs"]`
- `app crate`: `features = ["signal"]`
- `flow-engine-mcp`: `features = ["process"]`（补充声明）

### 6. cargo-hakari (workspace-hack)
通过 `cargo-hakari` 生成 `crates/workspace-hack` crate，统一所有外部依赖的 feature 集，避免跨 crate 构建时因 feature 不一致触发的重复编译。

```bash
# 重新生成 workspace-hack（feature 变更后需执行）
cd src-tauri
cargo hakari generate
```

### P3: Protocol Crate Feature Gates (2026-06-10)

16 个协议 crate 从硬依赖改为可选 feature flags，加速日常增量编译。

**Features**: 16 个 `protocol-xxx` feature + `all-protocols` meta-feature

**架构切换**:
- `flow-engine-executor/Cargo.toml` — 16 个协议 crate 改为 `optional = true`
- `flow-engine/Cargo.toml` — 移除直接协议依赖，改为 forwarding features
- `src-tauri/Cargo.toml` — 添加 forwarding features
- `flow-engine-core/src/context.rs` — 13 个类型化 provider 字段 → HashMap + ProviderKey 模式
- `flow-engine-executor/src/provider_builder.rs` — DRY manager 构造

**编译验证**:
cargo test --all-features
cargo test --no-default-features
cargo test --no-default-features --features "protocol-http,protocol-db,protocol-script"
cargo hakari generate --diff

### 当前编译现状

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| Cargo.lock 包数 | 870 | 870 |
| target/debug 体积 | 22.88 GB | — |
| 项目级 .cargo/config | 无 | ✅ |
| sccache | 用户级 | 用户级（不变） |
| workspace 管理的依赖 | 13 | 21 |

### 测试

| 环境 | 命令 |
|------|------|
| 前端 | `npm run test` (Vitest, jsdom 全局, coverage 门槛 80%) |
| 后端全量 | `cargo test && cargo clippy` |
| 单 crate | `cargo test -p flow-engine-http && cargo clippy -p flow-engine-http` |
| CLI 集成 | `cargo test cli_tests` |
| 完整回归 | `npm run typecheck && npm run test && cd src-tauri && cargo test && cargo clippy` |

测试约定: Store 测试用 `getState().reset()` 隔离，Tauri IPC mock 用 `src/services/tauri/mock.ts`，浏览器 API polyfill 在 `src/__tests__/setup.ts`。

## 已知技术债务

| 位置 | 内容 | 优先级 |
|------|------|--------|
| `src/types/error.ts` | 缺少 WebSocket 错误类型 (与 Rust 不对齐) | 中 |
| ~~Workspace 依赖~~ | ~~部分 crate 硬编码版本号，未统一用 workspace = true~~ | ~~中~~ ✅ 已修复 |
| 生产代码 unwrap() | `core/executor.rs:32-33` (regex captures) | 中 |
| 无 ESLint/Prettier | 前端无 lint/format 配置，但有 eslint-disable 残留 | 中 |
| `components.json` | 引用不存在的 `tailwind.config.js` (TailwindCSS 4 用 CSS config) | 低 |
| `src/components/panels/*.tsx` | setTimeout 硬编码魔数 (100ms, 500ms) | 低 |
| `src/services/storage/migration.ts` | 直接使用 console.log/error | 低 |
| `src/components/panels/Mcp*.tsx` | 多处 `as` 类型断言 | 中 |

## 子目录知识库

所有 20 个 crate 和主要前端模块均有 AGENTS.md：

- [src/components/AGENTS.md](./src/components/AGENTS.md)
- [src/stores/AGENTS.md](./src/stores/AGENTS.md)
- [src/services/AGENTS.md](./src/services/AGENTS.md)
- [src/types/AGENTS.md](./src/types/AGENTS.md)
- [src/i18n/AGENTS.md](./src/i18n/AGENTS.md)
- [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md)
- [src-tauri/src/AGENTS.md](./src-tauri/src/AGENTS.md)
- [src-tauri/crates/AGENTS.md](./src-tauri/crates/AGENTS.md)
- [src-tauri/crates/flow-engine/src/AGENTS.md](./src-tauri/crates/flow-engine/src/AGENTS.md)
- [src-tauri/crates/flow-engine-core/src/AGENTS.md](./src-tauri/crates/flow-engine-core/src/AGENTS.md)
- [src-tauri/crates/flow-engine-executor/src/AGENTS.md](./src-tauri/crates/flow-engine-executor/src/AGENTS.md)
- [src-tauri/crates/flow-engine-http/src/AGENTS.md](./src-tauri/crates/flow-engine-http/src/AGENTS.md)
- [src-tauri/crates/flow-engine-db/src/AGENTS.md](./src-tauri/crates/flow-engine-db/src/AGENTS.md)
- [src-tauri/crates/flow-engine-ws/src/AGENTS.md](./src-tauri/crates/flow-engine-ws/src/AGENTS.md)
- [src-tauri/crates/flow-engine-script/src/AGENTS.md](./src-tauri/crates/flow-engine-script/src/AGENTS.md)
- [src-tauri/crates/flow-engine-assertion/src/AGENTS.md](./src-tauri/crates/flow-engine-assertion/src/AGENTS.md)
- [src-tauri/crates/flow-engine-grpc/src/AGENTS.md](./src-tauri/crates/flow-engine-grpc/src/AGENTS.md)
- [src-tauri/crates/flow-engine-mcp/src/AGENTS.md](./src-tauri/crates/flow-engine-mcp/src/AGENTS.md)
- [src-tauri/crates/flow-engine-tcp/src/AGENTS.md](./src-tauri/crates/flow-engine-tcp/src/AGENTS.md)
- [src-tauri/crates/flow-engine-mqtt/src/AGENTS.md](./src-tauri/crates/flow-engine-mqtt/src/AGENTS.md)
- [src-tauri/crates/flow-engine-amqp/src/AGENTS.md](./src-tauri/crates/flow-engine-amqp/src/AGENTS.md)
- [src-tauri/crates/flow-engine-graphql/src/AGENTS.md](./src-tauri/crates/flow-engine-graphql/src/AGENTS.md)
- [src-tauri/crates/flow-engine-imap/src/AGENTS.md](./src-tauri/crates/flow-engine-imap/src/AGENTS.md)
- [src-tauri/crates/flow-engine-kafka/src/AGENTS.md](./src-tauri/crates/flow-engine-kafka/src/AGENTS.md)
- [src-tauri/crates/flow-engine-smtp/src/AGENTS.md](./src-tauri/crates/flow-engine-smtp/src/AGENTS.md)
- [src-tauri/crates/flow-engine-sse/src/AGENTS.md](./src-tauri/crates/flow-engine-sse/src/AGENTS.md)
- [src-tauri/crates/flow-engine-crypto/src/AGENTS.md](./src-tauri/crates/flow-engine-crypto/src/AGENTS.md)
