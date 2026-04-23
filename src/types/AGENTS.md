# TypeScript 类型定义

**生成时间:** 2026-04-23

## 概述

前端 TypeScript 类型定义，与 Rust 后端结构体严格对齐。所有类型定义必须与后端保持同步。

## 文件结构

```
src/types/
├── index.ts          # 统一导出入口
├── node.ts           # 节点类型 + 配置 (判别联合，17 种节点)
├── nodeRegistry.ts   # 后端节点元数据 (PortDef, NodeMetadata)
├── flow.ts           # 流程结构
├── connection.ts     # 节点连接
├── report.ts         # 执行报告 + 统计
├── variable.ts       # 变量系统
├── error.ts          # 错误类型
├── debug.ts          # 调试系统类型
├── ws_stream.ts      # WebSocket 消息流类型
├── ipc.ts            # Tauri IPC 命令类型
├── settings.ts       # 全局设置
├── port.ts           # 端口定义
├── database.ts       # 数据库连接类型
├── stressTest.ts     # 压力测试类型
├── events.ts         # 日志事件 (LogLevel, LogEvent)
└── execution_event.ts # 执行事件 (ExecutionEvent discriminated union)
```

## 快速定位

| 任务 | 文件 | 对应 Rust 文件 |
|------|------|---------------|
| 修改节点类型 | `node.ts` | `node.rs` + `node_config.rs` |
| 修改节点元数据 | `nodeRegistry.ts` | core traits |
| 修改流程结构 | `flow.ts` | `flow.rs` |
| 修改连接定义 | `connection.ts` | `connection.rs` |
| 修改执行报告 | `report.ts` | `report.rs` + `statistics.rs` |
| 修改变量系统 | `variable.ts` | `variable.rs` |
| 添加 IPC 命令 | `ipc.ts` | `command.rs` |
| 修改调试类型 | `debug.ts` | `debug/types.rs` |
| 修改 WebSocket 消息流 | `ws_stream.ts` | `ws_stream.rs` |
| 修改数据库类型 | `database.ts` | `db/types.rs` |
| 修改压力测试类型 | `stressTest.ts` | `stress_test/types.rs` |
| 修改日志事件 | `events.ts` | core `log_event.rs` |
| 修改执行事件 | `execution_event.ts` | executor event types |

## 约定

### 前后端类型同步 (强制)

**任何类型修改必须同时更新前后端。**

对应关系（注意: 后端已拆分为多个 crate）：
- `node.ts` ↔ `src-tauri/crates/flow-engine-core/src/node.rs` + `node_config.rs`
- `nodeRegistry.ts` ↔ `src-tauri/crates/flow-engine-core/src/` (PortDef, NodeMetadata)
- `flow.ts` ↔ `src-tauri/crates/flow-engine-core/src/flow.rs`
- `connection.ts` ↔ `src-tauri/crates/flow-engine-core/src/connection.rs`
- `report.ts` ↔ `src-tauri/crates/flow-engine-core/src/report.rs` + `statistics.rs`
- `variable.ts` ↔ `src-tauri/crates/flow-engine-core/src/variable.rs`
- `error.ts` ↔ `src-tauri/crates/flow-engine-core/src/error.rs`
- `debug.ts` ↔ `src-tauri/crates/flow-engine-executor/src/debug/types.rs` + `src-tauri/crates/flow-engine-core/src/debug_types.rs`
- `ws_stream.ts` ↔ `src-tauri/crates/flow-engine-core/src/ws_stream.rs`
- `database.ts` ↔ `src-tauri/crates/flow-engine-core/src/db_types.rs`
- `stressTest.ts` ↔ `src-tauri/crates/flow-engine-executor/src/stress_test/types.rs`
- gRPC 类型 (`node.ts` 中) ↔ `src-tauri/crates/flow-engine-core/src/node_config.rs` + `core/traits/grpc_provider.rs`

### 类型安全红线

- **禁止** `as any` 类型断言
- **禁止** `@ts-ignore` / `@ts-expect-error`
- 使用判别联合类型 (`NodeData`) 代替松散类型
- 类型守卫函数用于运行时判断 (`isHttpConfig` 等)

### 判别联合类型

`NodeData` 使用判别联合，通过 `type` 字段区分：
```typescript
type NodeData = EntryConfig | HttpConfig | ConditionalConfig | ScriptConfig | AssertionConfig
  | WsConnectConfig | WsSendWaitConfig | WsCloseConfig
  | DbConnectConfig | DbQueryConfig | DbBeginConfig | DbCommitConfig | DbRollbackConfig | DbCloseConfig
  | GrpcConnectConfig | GrpcCallConfig | GrpcCloseConfig;
```

### 可选字段

- 可选字段使用 `?:` 标记
- 必须与 Rust 端 `Option<T>` 对应
- 默认值在 Rust 端实现，前端仅声明类型

## 节点类型 (17 种)

| 类型 | 配置类型 | 用途 |
|------|----------|------|
| `entry` | EntryConfig | 流程入口 |
| `http` | HttpConfig | HTTP 请求 |
| `conditional` | ConditionalConfig | 条件分支 |
| `script` | ScriptConfig | JavaScript 脚本 |
| `assertion` | AssertionConfig | 断言验证 |
| `ws_connect` | WsConnectConfig | WebSocket 连接 |
| `ws_send_wait` | WsSendWaitConfig | WebSocket 发送等待 |
| `ws_close` | WsCloseConfig | WebSocket 关闭 |
| `db_connect` | DbConnectConfig | 数据库连接 |
| `db_query` | DbQueryConfig | 数据库查询 |
| `db_begin` | DbBeginConfig | 事务开始 |
| `db_commit` | DbCommitConfig | 事务提交 |
| `db_rollback` | DbRollbackConfig | 事务回滚 |
| `db_close` | DbCloseConfig | 数据库关闭 |
| `grpc_connect` | GrpcConnectConfig | gRPC 连接 |
| `grpc_call` | GrpcCallConfig | gRPC 调用 |
| `grpc_close` | GrpcCloseConfig | gRPC 关闭 |

## 类型漂移风险

| 位置 | 内容 | 优先级 |
|------|------|--------|
| 无 | 前后端类型已完全对齐 | — |
