# Workspace Crates
**Generated:** 2026-08-05 | **Branch:** feature/plugins | **Crates:** 25

## OVERVIEW
25 crate dirs across 4 layers + plugin system + CLI:
- **L0 (Leaves):** `flow-engine-core`, `flow-engine-crypto`
- **L1 (Protocols):** 16 optional crates (http, ws, db, script, assertion, grpc, mcp, tcp, mqtt, kafka, sse, graphql, amqp, smtp, imap, redis)
- **L2 (Orchestration):** `flow-engine-executor`
- **L3 (Facade):** `flow-engine`
- **CLI:** `mpe-cli` (纯 Rust, 无 Tauri 依赖)
- **Extra:** `workspace-hack` (cargo-hakari)
- **插件系统 (独立分层):** `mpe-protocol` (共享 wire 契约) + `flow-engine-plugin` (宿主侧) + `mpe-plugin-sdk` (作者 SDK)

All L1 crates are feature-gated (`protocol-*`). Default `all-protocols` enables all 16.

## DEPENDENCY GRAPH
```
app/mpe -> flow-engine -> flow-engine-executor -> 16 L1 crates -> flow-engine-core
                     \-> flow-engine-crypto
mpe-cli -> flow-engine (default-features = false, 按需 feature)
mpe-protocol (无依赖) <- flow-engine-plugin <- flow-engine-executor
                        <- mpe-plugin-sdk (仅依赖 mpe-protocol)
```
`smtp` and `imap` also depend on `flow-engine-crypto`.

## CRATE + WHERE TO LOOK
| Crate | L | .rs | LOC | Purpose | Key deps |
|-------|---|-----|-----|---------|----------|
| flow-engine-core | 0 | 46 | 17,485 | Node types, traits, flow structs | serde, regex, jsonpath |
| flow-engine-crypto | 0 | 2 | 461 | Encryption, keychain | aes-gcm, keyring |
| flow-engine-http | 1 | 13 | 5,571 | HTTP request nodes | hyper, rustls |
| flow-engine-ws | 1 | 6 | 5,461 | WebSocket client nodes | tokio-tungstenite |
| flow-engine-db | 1 | 15 | 8,224 | DB (SQLite/MySQL/PG) nodes | sqlx |
| flow-engine-script | 1 | 5 | 3,710 | JS script execution | rquickjs |
| flow-engine-assertion | 1 | 7 | 3,105 | Assertion/validation | regex, jsonpath |
| flow-engine-grpc | 1 | 12 | 7,141 | gRPC client + reflection | tonic, prost |
| flow-engine-mcp | 1 | 9 | 3,963 | MCP client nodes | rmcp |
| flow-engine-tcp | 1 | 18 | 10,702 | TCP/UDP socket nodes | tokio, rustls |
| flow-engine-mqtt | 1 | 11 | 6,633 | MQTT pub/sub | rumqttc-next |
| flow-engine-kafka | 1 | 12 | 6,383 | Kafka producer/consumer | rdkafka |
| flow-engine-sse | 1 | 7 | 3,003 | Server-Sent Events | reqwest |
| flow-engine-graphql | 1 | 9 | 6,111 | GraphQL query/sub | reqwest, tungstenite |
| flow-engine-amqp | 1 | 26 | 16,380 | AMQP 0-9-1 / 1.0 | lapin, fe2o3 |
| flow-engine-smtp | 1 | 8 | 4,056 | SMTP email send | lettre |
| flow-engine-imap | 1 | 16 | 3,810 | IMAP email fetch | async-imap |
| flow-engine-redis | 1 | 6 | 1,328 | Redis commands | redis-rs |
| flow-engine-executor | 2 | 39 | 18,557 | Execution engine, plugin sys | evalexpr, csv |
| flow-engine | 3 | 2 | 286 | Facade / re-exports | - |
| mpe-cli | CLI | 14 | ~2,500 | 纯 CLI 二进制 (run/validate/debug/stress/flow/report) | clap, flow-engine |
| mpe-protocol | 插件 | 7 | ~900 | 插件 wire 契约 (JSON-RPC 2.0 subset + manifest + framing) | serde |
| flow-engine-plugin | 插件 | 32 | ~4,200 | 宿主侧插件管理: discovery/lifecycle/transport/registration | mpe-protocol, executor |
| mpe-plugin-sdk | 插件 | 11 | ~950 | 插件作者 SDK: Plugin trait + mpe_plugin_main! 宏 | mpe-protocol |
| workspace-hack | X | 1 | 1 | Dep unification | - |
**Total: ~330 .rs files.**

## BUILD COMMANDS
```bash
cargo build -p flow-engine-http         # single crate
cargo test -p flow-engine-executor      # single crate test
cargo build -p mpe-cli --release        # 纯 CLI 二进制
cargo build -p flow-engine --no-default-features  # no protocols
cargo check --workspace                 # full workspace
```

## COMPILE IMPACT
| Modified | Recompiled |
|----------|-----------|
| core | all 25 crates + app |
| crypto | executor, smtp, imap, facade, app |
| Any L1 | executor, facade, app |
| executor | facade, app, flow-engine-plugin |
| facade | app, mpe-cli |
| mpe-protocol | flow-engine-plugin, mpe-plugin-sdk |
| workspace-hack | full rebuild |

## SUBDIRECTORY AGENTS.MD
每个 crate 的 `src/AGENTS.md` 记录其细节（协议 crate 均已有）。新 crate（flow-engine-plugin / mpe-plugin-sdk / mpe-protocol / mpe-cli）也各有一份。Entry point: each crate's `src/lib.rs`.
