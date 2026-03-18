# Flow Engine 核心库

**生成时间:** 2026-03-18
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
├── db/                    # 数据库模块 (74 源文件)
│   ├── mod.rs
│   ├── traits.rs          # DatabasePool/DbConnection trait
│   ├── types.rs           # IsolationLevel, TransactionState, QueryResult
│   ├── pool.rs            # 连接池管理
│   ├── postgres.rs        # PostgreSQL 实现
│   ├── mysql.rs           # MySQL 实现
│   ├── sqlite.rs          # SQLite 实现
│   └── redis.rs           # Redis 实现
├── assertion/             # 断言模块
│   ├── mod.rs
│   ├── operator.rs        # 15 种断言操作符
│   ├── evaluator.rs       # 断言求值器
│   └── preset.rs          # 预设断言
├── stress_test/           # 压力测试模块
│   ├── mod.rs
│   ├── config.rs          # 压测配置
│   ├── executor.rs        # 压测执行器
│   ├── metrics.rs         # 指标收集
│   ├── report.rs          # 压测报告
│   ├── command.rs         # IPC 命令
│   └── types.rs           # 类型定义
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
├── ws_stream.rs           # WebSocket 消息流数据结构 (调试模式)
├── log_event.rs           # 日志事件
├── command.rs             # 命令输入/输出类型
├── encryption.rs          # AES-256-GCM 加密 (OS Keychain)
├── script_api.rs          # 脚本节点 API (QuickJS)
└── *_executor.rs          # 节点执行器实现 (14 种)
    ├── entry_executor.rs
    ├── http_executor.rs
    ├── conditional_executor.rs
    ├── script_executor.rs
    ├── assertion_executor.rs
    ├── ws_connect_executor.rs
    ├── ws_send_wait_executor.rs
    ├── ws_close_executor.rs
    ├── db_connect_executor.rs
    ├── db_query_executor.rs
    ├── db_begin_executor.rs
    ├── db_commit_executor.rs
    ├── db_rollback_executor.rs
    └── db_close_executor.rs
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
| WebSocket 消息流 | `ws_stream.rs` |
| 调试执行器 | `debug/executor.rs` |
| 数据库连接池 | `db/pool.rs` + `db/traits.rs` |
| PostgreSQL 实现 | `db/postgres.rs` |
| MySQL 实现 | `db/mysql.rs` |
| SQLite 实现 | `db/sqlite.rs` |
| Redis 实现 | `db/redis.rs` |
| 断言操作符 | `assertion/operator.rs` |
| 断言求值 | `assertion/evaluator.rs` |
| 压力测试执行 | `stress_test/executor.rs` |
| 压力测试指标 | `stress_test/metrics.rs` |
| 脚本节点 API | `script_api.rs` |
| 密码加密 | `encryption.rs` |

## 节点类型 (14 种)

| 类型 | 执行器文件 | 端口 | 用途 |
|------|-----------|------|------|
| Entry | `entry_executor.rs` | `out` | 流程入口，处理 initial_variables |
| Http | `http_executor.rs` | `true`/`false` | HTTP 请求，2xx = true |
| Conditional | `conditional_executor.rs` | `true`/`false` | 条件判断 (evalexpr) |
| Script | `script_executor.rs` | `true`/`false` | QuickJS 脚本执行 |
| Assertion | `assertion_executor.rs` | `true`/`false` | 断言验证 |
| WsConnect | `ws_connect_executor.rs` | `true`/`false` | WebSocket 连接 |
| WsSendWait | `ws_send_wait_executor.rs` | `true`/`false` | 发送并等待响应 |
| WsClose | `ws_close_executor.rs` | `out` | 关闭连接 |
| DbConnect | `db_connect_executor.rs` | `true`/`false` | 数据库连接 |
| DbQuery | `db_query_executor.rs` | `true`/`false` | SQL 查询/Redis 命令 |
| DbBegin | `db_begin_executor.rs` | `true`/`false` | 事务开始 |
| DbCommit | `db_commit_executor.rs` | `true`/`false` | 事务提交 |
| DbRollback | `db_rollback_executor.rs` | `true`/`false` | 事务回滚 |
| DbClose | `db_close_executor.rs` | `out` | 关闭数据库连接 |

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
- `Database` - 数据库操作错误

## 约定

1. **独立运行**: 所有核心逻辑通过 `traits/` 抽象，Tauri 依赖通过 trait 注入
2. **日志发送**: `LogSender` trait 支持多实现，CLI 用 `NoopLogSender`
3. **变量语法**: `{{path.to.value}}`，不支持索引和函数调用
4. **错误分类**: `ErrorKind` 区分致命/可恢复错误，决定流程是否终止
5. **端口路由**: HTTP/WebSocket/Conditional/Assertion/Database 节点使用 true/false 端口
6. **连接池**: `WebSocketConnectionManager` 和 `DatabasePool` 在流程级别共享
7. **事务管理**: 数据库事务支持隔离级别配置
8. **脚本安全**: QuickJS 运行时隔离，支持超时控制
