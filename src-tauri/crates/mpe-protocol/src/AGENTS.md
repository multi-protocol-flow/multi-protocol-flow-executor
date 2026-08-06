# mpe-protocol/src

**Generated:** 2026-08-05 | **Branch:** feature/plugins | 无依赖叶 crate, 7 文件, ~900 LOC

## OVERVIEW

MPE 插件系统的共享 JSON-RPC 2.0 子集契约：request / 判别联合 response / error 对象 + plugin.json 清单 schema + stdio framing 常量。被宿主 `flow-engine-plugin` 与 SDK `mpe-plugin-sdk` 同时依赖，wire 契约因此永不漂移。完整规范见 `docs/plugin-protocol.md`。

## STRUCTURE

```
src/
├── lib.rs          # 模块声明 + framing 常量 (MAX_LINE_BYTES, LINE_DELIMITER)
├── request.rs      # Request: host → plugin, id 为 u64 (禁止 batch/notification)
├── response.rs     # Response: 判别联合, 恰好含 result 或 error 之一
├── error.rs        # JsonRpcError 错误对象
├── notification.rs # server → client 通知
├── describe.rs     # 节点描述
└── manifest.rs     # plugin.json 清单 schema
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 新增 RPC 方法 (宿主/插件双向) | `request.rs` + `response.rs` + `describe.rs` |
| 修改 plugin.json 清单字段 | `manifest.rs` |
| 错误码 / 错误对象契约 | `error.rs` |
| framing 上限或分隔符 | `lib.rs` (`MAX_LINE_BYTES`, `LINE_DELIMITER`) |

## CONVENTIONS

- **Framing**: `LINE_DELIMITER = b'\n'`，写入方发裸 `\n`；读取方容忍尾部 `\r` / CRLF（Python `print()` 产出）。`MAX_LINE_BYTES = 2 MiB` 双向适用，超限即协议违规，由 framing reader 拒绝，绝不无界缓冲。
- **Request 约束**: `id` 为裸 `u64`，null（notification）与数组（batch）在类型层面不可表示。notification 永不序列化 `id` 成员，故 notification 永不会被解析为 Request。
- **Response 约束**: 判别联合 + 自定义 `Deserialize`，同时含 `result` 与 `error` 的混合形态在 wire 边界被拒绝。解析时容忍缺失 `jsonrpc` 字段（按构造恒为 "2.0"）。
- **本 crate 是唯一权威契约**：任何改动同时破坏宿主与全部插件，改动前必须同步更新 `flow-engine-plugin`、`mpe-plugin-sdk` 与 `docs/plugin-protocol.md`。

## ANTI-PATTERNS

- 在 response 中允许 `result` 与 `error` 共存 → 破坏判别联合，混合形态必须在类型层不可表示
- 给 notification 加 `id` → 会被 framing 侧误判为 Request
- 单边改动协议（只改宿主或只改 SDK）→ 两侧契约漂移，运行时静默失配
- 放宽 `MAX_LINE_BYTES` 或改用可变分隔符 → 破坏 framing 对恶意/异常输出的上限保证
