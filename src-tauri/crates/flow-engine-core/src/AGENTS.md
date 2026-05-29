# flow-engine-core

**Layer 0 (叶子)** | ~30 文件 | ~8,300 行 | 无 workspace 依赖

## 概述

基础类型、traits、共享数据结构。所有其他 crate 均依赖此 crate。**修改此 crate 会触发全部 crate 重编译。**

## 结构

```
flow-engine-core/src/
├── lib.rs                 # 模块导出
├── node.rs                # Node/NodeType/Port 定义 (16 variants)
├── node_config.rs         # NodeData 配置枚举 (30 种节点 + Plugin 兜底, ~1,771 行)
├── node_types.rs          # 辅助节点类型
├── flow.rs                # Flow 结构体
├── connection.rs          # Connection 定义
├── context.rs             # ExecutionContext + VariableStorage + grpc_manager (~967 行)
├── variable.rs            # 变量系统 (模板替换 {{path.to.value}})
├── error.rs               # ErrorDetail + ErrorKind (致命/可恢复 + 5 gRPC 变体)
├── executor.rs            # NodeExecutor trait 定义
├── plugin.rs              # NodePlugin trait (插件架构)
├── registry.rs            # NodeRegistry (插件注册表)
├── report.rs              # 执行报告结构
├── statistics.rs          # 流程统计
├── execution_event.rs     # ExecutionEvent 执行事件
├── execution_event_sender.rs # ExecutionEventSender trait
├── log_event.rs           # 日志事件类型
├── ws_stream.rs           # WsMessage/WsConnectionStream (~808 行)
├── db_types.rs            # DbConnection/DbDriver/DbConnectionStatus
├── debug_types.rs         # DebugEvent/DebugCommand/DebugSessionState
├── utils.rs               # 共享工具函数
├── app_dir.rs             # AppDir 实现
└── traits/                # 抽象接口 (解耦 Tauri)
    ├── mod.rs
    ├── app_dir.rs         # AppDirProvider
    ├── directory_opener.rs # DirectoryOpener
    ├── log_sender.rs      # LogSender
    ├── execution_event_sender.rs # ExecutionEventSender
    ├── ws_provider.rs     # WebSocketProvider
    ├── db_provider.rs     # DatabaseProvider
    ├── grpc_provider.rs   # GrpcProvider trait + gRPC 类型
    └── debug_event_sender.rs # DebugEventSender
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `node.rs` (NodeType) + `node_config.rs` (NodeData) + `plugin.rs` (NodePlugin) |
| 修改变量替换 | `variable.rs` |
| 修改错误分类 | `error.rs` → ErrorKind |
| 添加新 Trait | `traits/mod.rs` |
| 插件注册表 | `registry.rs` (NodeRegistry) |
| NodePlugin trait | `plugin.rs` |
| gRPC Provider | `traits/grpc_provider.rs` |
| 执行报告格式 | `report.rs` |
| 数据库类型 | `db_types.rs` |
| WebSocket 消息流 | `ws_stream.rs` |
| 调试类型 | `debug_types.rs` |
| 执行事件 | `execution_event.rs` |

## 关键类型

- **NodeExecutor trait**: 所有执行器的接口，定义在 `executor.rs`
- **NodePlugin trait**: 插件架构接口，定义在 `plugin.rs`，替代旧 match 分发
- **NodeRegistry**: 插件注册表，定义在 `registry.rs`
- **ErrorKind**: 区分致命错误 (`Configuration`, `ConditionSyntax`) 和可恢复错误，含 5 个 gRPC 变体
- **ExecutionContext**: 流程执行上下文，包含 VariableStorage + grpc_manager
- **NodeData**: 29 种节点配置 + Plugin 兜底的判别联合
- **NodeType**: 27 个枚举变体 (注意: NodeData 29 种 + Plugin 兜底可处理未知类型)

## 约定

- 此 crate 不得依赖任何其他 workspace crate（保持叶子状态）
- 类型定义需与前端 `src/types/` 同步
- 新增公共类型必须添加 `#[serde(...)]` 派生
