# Rust Workspace 总览

**生成时间:** 2026-04-16
**Commit:** 158c34c

## 概述

9 个 crate 组成的 Rust Workspace。`flow-engine` 为门面 crate (re-export 全部)，`app` (src-tauri/src/) 仅依赖 `flow-engine`。

## 依赖图

```
Layer 0 (叶子):   flow-engine-core, flow-engine-crypto
Layer 1 (协议):   flow-engine-http, flow-engine-ws, flow-engine-db,
                  flow-engine-script, flow-engine-assertion    (→ core)
Layer 2 (编排):   flow-engine-executor                          (→ core + 全部 Layer 1 + crypto)
Layer 3 (门面):   flow-engine                                   (re-export 全部)
Layer 4 (应用):   app (src-tauri/)                              (→ flow-engine)
```

**增量编译关键**: 修改 Layer N 的 crate 会触发 Layer N+1 及以上重编译。

| Crate | Layer | 依赖的 Workspace Crate | 源文件数 | 行数 |
|-------|-------|----------------------|---------|------|
| flow-engine-core | 0 | *(无)* | 23 | 6,504 |
| flow-engine-crypto | 0 | *(无)* | 2 | 461 |
| flow-engine-http | 1 | core | 10 | 4,590 |
| flow-engine-ws | 1 | core | 5 | 5,360 |
| flow-engine-db | 1 | core | 15 | 7,760 |
| flow-engine-script | 1 | core | 4 | 3,374 |
| flow-engine-assertion | 1 | core | 6 | 2,946 |
| flow-engine-executor | 2 | core + 全部 Layer 1 + crypto | 24 | 12,341 |
| flow-engine | 3 | 全部 8 个 crate | 2 | 205 |

## 快速定位

| 任务 | Crate | 位置 |
|------|-------|------|
| 基础类型 (Node, Flow, Error) | flow-engine-core | `core/src/` |
| HTTP 执行器 | flow-engine-http | `http/src/` |
| WebSocket 执行器 | flow-engine-ws | `ws/src/` |
| 数据库执行器 | flow-engine-db | `db/src/` |
| 脚本节点 | flow-engine-script | `script/src/` |
| 断言节点 | flow-engine-assertion | `assertion/src/` |
| 主执行器 / 调试 / 压测 | flow-engine-executor | `executor/src/` |
| 加密 / Keychain | flow-engine-crypto | `crypto/src/` |

## 增量编译命令

```bash
cd src-tauri

# 编译单个 crate
cargo build -p flow-engine-http          # 仅 core + http
cargo build -p flow-engine-executor      # core + 全部协议 crate + executor
cargo build -p flow-engine               # 全部 (等同全量)

# 测试单个 crate
cargo test -p flow-engine-http
cargo test -p flow-engine-core

# Lint 单个 crate
cargo clippy -p flow-engine-db
```

## 修改影响分析

| 修改的 crate | 重编译范围 | 编译量 |
|-------------|-----------|--------|
| `flow-engine-core` | 全部 crate (级联) | **全量** |
| `flow-engine-crypto` | executor → flow-engine → app | ~3 crate |
| `flow-engine-http/ws/db/script/assertion` | executor → flow-engine → app | ~3 crate |
| `flow-engine-executor` | flow-engine → app | ~2 crate |
| `flow-engine` (门面) | app | ~1 crate |

## 子目录知识库

- [flow-engine/src/AGENTS.md](./flow-engine/src/AGENTS.md) - 门面 crate
- [flow-engine-core/src/AGENTS.md](./flow-engine-core/src/AGENTS.md) - 基础类型
- [flow-engine-executor/src/AGENTS.md](./flow-engine-executor/src/AGENTS.md) - 执行器
- [flow-engine-http/src/AGENTS.md](./flow-engine-http/src/AGENTS.md) - HTTP
- [flow-engine-db/src/AGENTS.md](./flow-engine-db/src/AGENTS.md) - 数据库
- [flow-engine-ws/src/AGENTS.md](./flow-engine-ws/src/AGENTS.md) - WebSocket
- [flow-engine-script/src/AGENTS.md](./flow-engine-script/src/AGENTS.md) - 脚本
- [flow-engine-assertion/src/AGENTS.md](./flow-engine-assertion/src/AGENTS.md) - 断言
