# Workspace Crates
**Generated:** 2026-08-05 | **Branch:** feature/plugins | **Crates:** 14

## OVERVIEW
14 crate dirs across 4 layers + CLI + workspace-hack（插件系统已独立为公开仓库，不在 workspace 内）:
- **L0 (Leaves):** `flow-engine-core`
- **L1 (Protocols):** 7 optional crates (http, ws, script, assertion, tcp, sse, graphql) — redis/db/mongo/kafka/smtp/mcp/grpc 已移出宿主，为独立仓库插件（清单见根 `CONTRIBUTING.md`）。mqtt/imap/amqp 无公开仓库，MQTT 与 IMAP 能力没有对应实现
- **L2 (Orchestration):** `flow-engine-executor`
- **L3 (Facade):** `flow-engine`
- **CLI:** `mpe-cli` (纯 Rust, 无 Tauri 依赖)
- **Extra:** `workspace-hack` (cargo-hakari)
- **插件系统 (独立分层):** `mpe-plugin-sdk` (独立公开仓库: 共享 wire 契约 + 作者 SDK) + `flow-engine-plugin` (宿主侧)

All L1 crates are feature-gated (`protocol-*`). Default `all-protocols` enables all 7.

## DEPENDENCY GRAPH
```
app/mpe -> flow-engine -> flow-engine-executor -> 7 L1 crates -> flow-engine-core
mpe-cli -> flow-engine (default-features = false, 按需 feature)
mpe-plugin-sdk (独立公开仓库, git+tag; 宿主以 default-features = false 取 wire 类型) <- flow-engine-plugin <- flow-engine-executor
```

## CRATE + WHERE TO LOOK
| Crate | L | .rs | LOC | Purpose | Key deps |
|-------|---|-----|-----|---------|----------|
| flow-engine-core | 0 | 46 | 17,485 | Node types, traits, flow structs | serde, regex, jsonpath |
| flow-engine-http | 1 | 13 | 5,571 | HTTP request nodes | hyper, rustls |
| flow-engine-ws | 1 | 6 | 5,461 | WebSocket client nodes | tokio-tungstenite |
| flow-engine-script | 1 | 5 | 3,710 | JS script execution | rquickjs |
| flow-engine-assertion | 1 | 7 | 3,105 | Assertion/validation | regex, jsonpath |
| flow-engine-tcp | 1 | 18 | 10,702 | TCP/UDP socket nodes | tokio, rustls |
| flow-engine-sse | 1 | 7 | 3,003 | Server-Sent Events | reqwest |
| flow-engine-graphql | 1 | 9 | 6,111 | GraphQL query/sub | reqwest, tungstenite |
| flow-engine-executor | 2 | 39 | 18,557 | Execution engine, plugin sys | evalexpr, csv |
| flow-engine | 3 | 2 | 286 | Facade / re-exports | - |
| mpe-cli | CLI | 14 | ~2,500 | 纯 CLI 二进制 (run/validate/debug/stress/flow/report) | clap, flow-engine |
| flow-engine-plugin | 插件 | 32 | ~4,200 | 宿主侧插件管理: discovery/lifecycle/transport/registration | mpe-plugin-sdk (git), executor |
| workspace-hack | X | 1 | 1 | Dep unification | - |
**Total: ~250 .rs files.**

## BUILD COMMANDS
```bash
cargo build -p flow-engine-http         # single crate
cargo nextest run -p flow-engine-executor # single crate test
cargo build -p mpe-cli --release        # 纯 CLI 二进制
cargo build -p flow-engine --no-default-features  # no protocols
cargo check --workspace                 # full workspace
```

## COMPILE IMPACT
| Modified | Recompiled |
|----------|-----------|
| core | all 14 workspace crates + app |
| Any L1 | executor, facade, app |
| executor | facade, app, flow-engine-plugin |
| facade | app, mpe-cli |
| mpe-plugin-sdk (git 依赖) | 宿主侧不参与增量重编译（tag 固定；升级 tag 才重拉） |
| workspace-hack | full rebuild |

## SUBDIRECTORY AGENTS.MD
每个 crate 的 `src/AGENTS.md` 记录其细节（协议 crate 均已有）。新 crate（flow-engine-plugin / mpe-cli）也各有一份。`mpe-plugin-sdk` 的知识库在独立公开仓库 `multi-protocol-flow/mpe-plugin-sdk` 根目录。Entry point: each crate's `src/lib.rs`.
