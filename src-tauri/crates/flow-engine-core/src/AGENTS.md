# flow-engine-core

**Layer 0 (叶子)** | 23 文件 | 6,504 行 | 无 workspace 依赖

## 概述

基础类型、traits、共享数据结构。所有其他 crate 均依赖此 crate。**修改此 crate 会触发全部 crate 重编译。**

## 结构

```
flow-engine-core/src/
├── node.rs                # Node/NodeType/Port 定义
├── node_config.rs         # NodeData 配置枚举 (14 种节点, 1,339 行)
├── flow.rs                # Flow 结构体
├── connection.rs          # Connection 定义
├── context.rs             # ExecutionContext + VariableStorage (789 行)
├── variable.rs            # 变量系统 (模板替换 {{path.to.value}})
├── error.rs               # ErrorDetail + ErrorKind (致命/可恢复分类)
├── executor.rs            # NodeExecutor trait 定义
├── report.rs              # 执行报告结构
├── statistics.rs          # 流程统计
├── log_event.rs           # 日志事件类型
├── ws_stream.rs           # WsMessage/WsConnectionStream (802 行)
├── db_types.rs            # DbConnection/DbDriver/DbConnectionStatus
├── debug_types.rs         # DebugEvent/DebugCommand/DebugSessionState
├── utils.rs               # 共享工具函数
├── storage.rs             # 文件存储 trait
└── traits/                # 抽象接口 (解耦 Tauri)
    ├── mod.rs
    ├── app_dir.rs         # AppDirProvider
    ├── directory_opener.rs # DirectoryOpener
    ├── log_sender.rs      # LogSender
    ├── ws_provider.rs     # WebSocketProvider
    ├── db_provider.rs     # DatabaseProvider
    └── debug_event_sender.rs # DebugEventSender
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `node.rs` (NodeType) + `node_config.rs` (NodeData) |
| 修改变量替换 | `variable.rs` |
| 修改错误分类 | `error.rs` → ErrorKind |
| 添加新 Trait | `traits/mod.rs` |
| 执行报告格式 | `report.rs` |
| 数据库类型 | `db_types.rs` |
| WebSocket 消息流 | `ws_stream.rs` |
| 调试类型 | `debug_types.rs` |

## 关键类型

- **NodeExecutor trait**: 所有执行器的接口，定义在 `executor.rs`
- **ErrorKind**: 区分致命错误 (`Configuration`, `ConditionSyntax`) 和可恢复错误
- **ExecutionContext**: 流程执行上下文，包含 VariableStorage
- **NodeData**: 14 种节点配置的判别联合

## 约定

- 此 crate 不得依赖任何其他 workspace crate（保持叶子状态）
- 类型定义需与前端 `src/types/` 同步
- 新增公共类型必须添加 `#[serde(...)]` 派生
