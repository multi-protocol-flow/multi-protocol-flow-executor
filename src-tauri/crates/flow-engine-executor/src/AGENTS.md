# flow-engine-executor

**Layer 2** | 24 文件 | 12,341 行 | 依赖 core + 全部 Layer 1 + crypto

## 概述

主执行器、流程编排、调试模式、压力测试、业务命令。最大的 crate。

## 结构

```
flow-engine-executor/src/
├── executor.rs            # FlowExecutor 主执行循环 (1,885 行)
├── entry_executor.rs      # 入口节点执行器
├── conditional_executor.rs # 条件分支执行器 (1,555 行)
├── validator.rs           # 流程验证器
├── storage.rs             # 文件存储 (765 行)
├── command.rs             # 命令输入/输出类型
├── commands/              # 业务命令
│   ├── mod.rs
│   ├── execute_flow.rs    # 流程执行入口
│   ├── create_flow.rs     # 创建流程
│   ├── update_node.rs     # 更新节点
│   ├── delete_node.rs     # 删除节点
│   └── report_commands.rs # 报告相关命令
├── debug/                 # 调试执行模式
│   ├── mod.rs
│   ├── executor.rs        # DebugFlowExecutor (2,104 行)
│   └── types.rs           # DebugEvent, DebugCommand (621 行)
├── debug_event_adapter.rs # Debug 事件适配器
└── stress_test/           # 压力测试
    ├── mod.rs
    ├── config.rs          # 压测配置
    ├── executor.rs        # 压测执行器 (700 行)
    ├── metrics.rs         # 指标收集
    ├── report.rs          # 压测报告
    ├── command.rs         # IPC 命令 (673 行)
    └── types.rs           # 类型定义
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 修改执行循环 | `executor.rs` |
| 条件分支逻辑 | `conditional_executor.rs` |
| 流程验证 | `validator.rs` |
| 调试执行 | `debug/executor.rs` |
| 压力测试 | `stress_test/executor.rs` |
| CLI 调用入口 | `commands/execute_flow.rs` |
| 业务命令 (IPC) | `commands/*.rs` |
| 存储逻辑 | `storage.rs` |

## 关键逻辑

### FlowExecutor (executor.rs)
- `execute()`: 主执行循环，遍历节点图
- `replace_variables()`: 全局变量替换
- `execute_node()`: 按 NodeType 分发到各协议执行器
- 端口路由: HTTP/WS/DB/Conditional/Assertion 节点使用 true/false 端口

### DebugFlowExecutor (debug/executor.rs)
- 断点暂停、单步执行、继续、跳过
- 通过 Channel 实现实时日志流

### 压力测试 (stress_test/)
- 并发用户数、渐进启动、持续时长
- 实时进度 (RPS、延迟、迭代数)
- 最终报告 (p50/p90/p95/p99)

## 编译影响

修改此 crate → `flow-engine` (门面) → `app` 重编译 (~2 crate)

```bash
cd src-tauri
cargo test -p flow-engine-executor      # 测试此 crate
cargo clippy -p flow-engine-executor    # Lint 此 crate
```
