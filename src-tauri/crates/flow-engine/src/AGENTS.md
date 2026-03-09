# Flow Engine 核心库

**生成时间:** 2026-03-09
**Commit:** cc2bf47
**类型:** 独立 Rust crate

## 概述

流程执行引擎核心库，完全独立于 Tauri 框架。CLI 工具和测试环境可直接使用，无需 GUI 依赖。

## 结构

```
flow-engine/src/
├── lib.rs                 # 模块入口 + re-exports
├── traits/                # 抽象接口（解耦 Tauri 依赖）
│   ├── mod.rs
│   ├── app_dir.rs         # AppDirProvider - 应用目录
│   ├── directory_opener.rs # DirectoryOpener - 打开目录
│   └── log_sender.rs      # LogSender - 日志发送（Tauri/Console/Noop）
├── commands/              # 业务命令（非 Tauri command）
│   ├── mod.rs
│   ├── execute_flow.rs    # 流程执行入口
│   ├── create_flow.rs     # 创建流程
│   ├── update_node.rs     # 更新节点
│   ├── delete_node.rs     # 删除节点
│   └── report_commands.rs # 报告相关命令
├── http/                  # HTTP 客户端模块 (hyper 1.x)
│   ├── mod.rs
│   ├── client.rs          # HttpClient 实现
│   ├── request.rs         # 请求构建
│   ├── response.rs        # 响应处理
│   ├── body.rs            # 请求体处理
│   ├── timing.rs          # 性能计时
│   ├── connector.rs       # 连接器
│   └── error.rs           # HTTP 错误
├── debug/                 # 调试执行器
│   ├── mod.rs
│   ├── executor.rs        # DebugFlowExecutor
│   └── types.rs           # DebugEvent, DebugCommand
├── executor.rs            # FlowExecutor + NodeExecutor trait
├── context.rs             # ExecutionContext + VariableStorage
├── node.rs                # Node/NodeType/Port 定义
├── node_config.rs         # NodeData 配置枚举
├── flow.rs                # Flow 结构体
├── connection.rs          # Connection 定义
├── error.rs               # ErrorDetail + ErrorKind
├── report.rs              # 执行报告结构
├── statistics.rs          # 流程统计
├── storage.rs             # 文件存储
├── validator.rs           # 流程验证器
├── variable.rs            # 变量系统
├── websocket_manager.rs   # WebSocket 连接池管理
├── log_event.rs           # 日志事件
├── command.rs             # 命令输入/输出类型
└── *_executor.rs          # 节点执行器实现
    ├── entry_executor.rs
    ├── http_executor.rs
    ├── conditional_executor.rs
    ├── ws_connect_executor.rs
    ├── ws_send_wait_executor.rs
    └── ws_close_executor.rs
```

## 快速定位

| 任务 | 位置 |
|------|------|
| 添加新节点执行器 | `*_executor.rs` + `executor.rs` |
| 修改变量替换逻辑 | `executor.rs:replace_variables()` |
| 添加新 Trait | `traits/mod.rs` |
| 错误分类 | `error.rs:ErrorKind` |
| 执行报告格式 | `report.rs` |
| CLI 调用入口 | `commands/execute_flow.rs` |
| HTTP 客户端 | `http/client.rs` |
| WebSocket 管理 | `websocket_manager.rs` |
| 调试执行器 | `debug/executor.rs` |

## 节点类型 (6种)

| 类型 | 执行器文件 | 端口 | 用途 |
|------|-----------|------|------|
| Entry | `entry_executor.rs` | `out` | 流程入口，处理 initial_variables |
| Http | `http_executor.rs` | `true`/`false` | HTTP 请求，2xx = true |
| Conditional | `conditional_executor.rs` | `true`/`false` | 条件判断 (evalexpr) |
| WsConnect | `ws_connect_executor.rs` | `true`/`false` | WebSocket 连接 |
| WsSendWait | `ws_send_wait_executor.rs` | `true`/`false` | 发送并等待响应 |
| WsClose | `ws_close_executor.rs` | `out` | 关闭连接 |

## NodeExecutor Pattern

```rust
#[async_trait::async_trait]
pub trait NodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), String>;
}
```

返回值:
- `Ok((next_node_ids, output_data))`: 成功，next_node_ids 为空则终止
- `Err(msg)`: 失败，触发错误处理流程

## 错误分类系统

**致命错误 (立即终止):**
- `Configuration` - 配置缺失/无效
- `ConditionSyntax` - 条件表达式语法错误

**可恢复错误 (路由到 false 端口):**
- `Execution` - 通用执行错误
- `Connection` - 网络连接失败
- `Timeout` - 操作超时
- `ConditionEvaluation` - 条件运行时求值错误
- `Validation` - 参数验证失败
- `Parse` - JSON/数据解析错误
- `WebSocketConnection` - WebSocket 连接错误
- `WebSocketTimeout` - WebSocket 超时
- `WebSocketMessage` - WebSocket 消息错误
- `WebSocketClose` - WebSocket 关闭错误
- `WebSocketProtocol` - WebSocket 协议错误

## 约定

1. **独立运行**: 所有核心逻辑通过 `traits/` 抽象，Tauri 依赖通过 trait 注入
2. **日志发送**: `LogSender` trait 支持多实现，CLI 用 `NoopLogSender`
3. **变量语法**: `{{path.to.value}}`，不支持索引和函数调用
4. **错误分类**: `ErrorKind` 区分致命/可恢复错误，决定流程是否终止
5. **端口路由**: HTTP/WebSocket/Conditional 节点使用 true/false 端口
6. **连接池**: `WebSocketConnectionManager` 在流程级别共享
