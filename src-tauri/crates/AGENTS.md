# Rust Workspace 总览

**生成时间:** 2026-05-21
**Commit:** dd3608d

## 概述

19 个 crate 组成的 Rust Workspace。`flow-engine` 为门面 crate (re-export 全部)，`app` (src-tauri/src/) 仅依赖 `flow-engine`。

## 依赖图

```
Layer 0 (叶子):   flow-engine-core, flow-engine-crypto
Layer 1 (协议):   flow-engine-http, flow-engine-ws, flow-engine-db,
                  flow-engine-script, flow-engine-assertion,
                  flow-engine-grpc, flow-engine-mcp, flow-engine-tcp,
                  flow-engine-amqp, flow-engine-graphql, flow-engine-imap,
                  flow-engine-kafka, flow-engine-smtp, flow-engine-sse  (→ core)
Layer 2 (编排):   flow-engine-executor                          (→ core + 全部 Layer 1 + crypto)
Layer 3 (门面):   flow-engine                                   (re-export 全部)
Layer 4 (应用):   app (src-tauri/)                              (→ flow-engine)
```

**增量编译关键**: 修改 Layer N 的 crate 会触发 Layer N+1 及以上重编译。

| Crate | Layer | 依赖的 Workspace Crate | 源文件数 | 行数 |
|-------|-------|----------------------|---------|------|
| flow-engine-core | 0 | *(无)* | 30 | ~8,300 |
| flow-engine-crypto | 0 | *(无)* | 2 | 461 |
| flow-engine-http | 1 | core | 12 | ~5,100 |
| flow-engine-ws | 1 | core | 6 | ~5,500 |
| flow-engine-db | 1 | core | 16 | ~8,400 |
| flow-engine-script | 1 | core | 5 | ~3,500 |
| flow-engine-assertion | 1 | core | 7 | ~3,000 |
| flow-engine-grpc | 1 | core | 7 | ~4,365 |
| flow-engine-mcp | 1 | core | 9 | ~4,000 |
| flow-engine-tcp | 1 | core | 15 | ~5,000 |
| flow-engine-amqp | 1 | core | 26 | ~8,500 |
| flow-engine-graphql | 1 | core | 10 | ~3,200 |
| flow-engine-imap | 1 | core | 16 | ~4,800 |
| flow-engine-kafka | 1 | core | 12 | ~3,600 |
| flow-engine-smtp | 1 | core | 8 | ~2,400 |
| flow-engine-sse | 1 | core | 7 | ~2,100 |
| flow-engine-executor | 2 | core + 全部 Layer 1 + crypto | 28 | ~15,717 |
| flow-engine | 3 | 全部 18 个 crate | 2 | ~205 |

## 快速定位

| 任务 | Crate | 位置 |
|------|-------|------|
| 基础类型 (Node, Flow, Error) | flow-engine-core | `core/src/` |
| HTTP 执行器 | flow-engine-http | `http/src/` |
| WebSocket 执行器 | flow-engine-ws | `ws/src/` |
| 数据库执行器 | flow-engine-db | `db/src/` |
| 脚本节点 | flow-engine-script | `script/src/` |
| 断言节点 | flow-engine-assertion | `assertion/src/` |
| gRPC 执行器 | flow-engine-grpc | `grpc/src/` |
| MCP 执行器 | flow-engine-mcp | `mcp/src/` |
| TCP/UDP 执行器 | flow-engine-tcp | `tcp/src/` |
| AMQP 执行器 | flow-engine-amqp | `amqp/src/` |
| GraphQL 执行器 | flow-engine-graphql | `graphql/src/` |
| IMAP 执行器 | flow-engine-imap | `imap/src/` |
| Kafka 执行器 | flow-engine-kafka | `kafka/src/` |
| SMTP 执行器 | flow-engine-smtp | `smtp/src/` |
| SSE 执行器 | flow-engine-sse | `sse/src/` |
| 主执行器 / 调试 / 压测 | flow-engine-executor | `executor/src/` |
| 加密 / Keychain | flow-engine-crypto | `crypto/src/` |

## 增量编译

```bash
cd src-tauri
cargo build -p flow-engine-http          # 仅 core + http
cargo test -p flow-engine-grpc           # 仅测 gRPC
cargo clippy -p flow-engine-db           # 仅 Lint DB
```

## 修改影响分析

| 修改的 crate | 重编译范围 | 编译量 |
|-------------|-----------|--------|
| `flow-engine-core` | 全部 crate (级联) | **全量** |
| `flow-engine-crypto` | executor → flow-engine → app | ~3 crate |
| `flow-engine-http/ws/db/script/assertion/grpc/mcp/tcp/amqp/graphql/imap/kafka/smtp/sse` | executor → flow-engine → app | ~3 crate |
| `flow-engine-executor` | flow-engine → app | ~2 crate |
| `flow-engine` (门面) | app | ~1 crate |

## 子目录知识库

- [flow-engine/src/AGENTS.md](./flow-engine/src/AGENTS.md) - 门面 crate
- [flow-engine-core/src/AGENTS.md](./flow-engine-core/src/AGENTS.md) - 基础类型
- [flow-engine-crypto/src/AGENTS.md](./flow-engine-crypto/src/AGENTS.md) - 加密/Keychain
- [flow-engine-executor/src/AGENTS.md](./flow-engine-executor/src/AGENTS.md) - 执行器
- [flow-engine-http/src/AGENTS.md](./flow-engine-http/src/AGENTS.md) - HTTP
- [flow-engine-db/src/AGENTS.md](./flow-engine-db/src/AGENTS.md) - 数据库
- [flow-engine-ws/src/AGENTS.md](./flow-engine-ws/src/AGENTS.md) - WebSocket
- [flow-engine-script/src/AGENTS.md](./flow-engine-script/src/AGENTS.md) - 脚本
- [flow-engine-assertion/src/AGENTS.md](./flow-engine-assertion/src/AGENTS.md) - 断言
- [flow-engine-grpc/src/AGENTS.md](./flow-engine-grpc/src/AGENTS.md) - gRPC
- [flow-engine-mcp/src/AGENTS.md](./flow-engine-mcp/src/AGENTS.md) - MCP
- [flow-engine-tcp/src/AGENTS.md](./flow-engine-tcp/src/AGENTS.md) - TCP/UDP
- [flow-engine-amqp/src/AGENTS.md](./flow-engine-amqp/src/AGENTS.md) - AMQP
- [flow-engine-graphql/src/AGENTS.md](./flow-engine-graphql/src/AGENTS.md) - GraphQL
- [flow-engine-imap/src/AGENTS.md](./flow-engine-imap/src/AGENTS.md) - IMAP
- [flow-engine-kafka/src/AGENTS.md](./flow-engine-kafka/src/AGENTS.md) - Kafka
- [flow-engine-smtp/src/AGENTS.md](./flow-engine-smtp/src/AGENTS.md) - SMTP
- [flow-engine-sse/src/AGENTS.md](./flow-engine-sse/src/AGENTS.md) - SSE
