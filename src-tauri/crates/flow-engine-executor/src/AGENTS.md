# flow-engine-executor

**Layer 2** | 28 文件 | ~15,717 行 | 依赖 core + 全部 Layer 1 + crypto

## 概述

主执行器、流程编排、调试模式、压力测试、业务命令。最大的 crate。

## 结构

```
flow-engine-executor/src/
├── lib.rs                  # 模块导出
├── executor.rs             # FlowExecutor 主执行循环 (~1,702 行)
├── default_registry.rs     # 默认节点注册表，注册 17 个插件 (107 行)
├── plugin_entry.rs         # Entry 节点 NodePlugin (60 行)
├── plugin_conditional.rs   # 条件分支 NodePlugin (84 行)
├── entry_executor.rs       # 入口节点执行器
├── conditional_executor.rs # 条件分支执行器 (~2,100 行)
├── debug_event_adapter.rs  # mpsc 适配器 (21 行)
├── validator.rs            # 流程验证器
├── storage.rs              # 文件存储 (~1,844 行)
├── command.rs              # 命令输入/输出类型
├── commands/               # 业务命令
│   ├── mod.rs
│   ├── execute_flow.rs     # 流程执行入口
│   ├── create_flow.rs      # 创建流程
│   ├── update_node.rs      # 更新节点
│   ├── delete_node.rs      # 删除节点
│   └── report_commands.rs  # 报告相关命令
├── debug/                  # 调试执行模式
│   ├── mod.rs
│   ├── executor.rs         # DebugFlowExecutor (~1,886 行)
│   └── types.rs            # DebugEvent, DebugCommand (~634 行)
└── stress_test/            # 压力测试
    ├── mod.rs
    ├── config.rs           # 压测配置
    ├── executor.rs         # 压测执行器 (~745 行)
    ├── metrics.rs          # 指标收集 (~917 行)
    ├── report.rs           # 压测报告
    ├── command.rs          # IPC 命令 (~684 行)
    └── types.rs            # 类型定义
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 修改执行循环 | `executor.rs` |
| 插件注册 | `default_registry.rs` |
| 条件分支逻辑 | `conditional_executor.rs` |
| 流程验证 | `validator.rs` |
| 调试执行 | `debug/executor.rs` |
| 压力测试 | `stress_test/executor.rs` |
| CLI 调用入口 | `commands/execute_flow.rs` |
| 业务命令 (IPC) | `commands/*.rs` |
| 存储逻辑 | `storage.rs` |
| Debug 事件适配 | `debug_event_adapter.rs` |

## 关键逻辑

### 插件架构
- **NodePlugin trait** (core): 每种节点类型实现一个 Plugin
- **default_registry.rs**: 注册全部 17 个插件 (entry, http, ws×3, db×6, script, assertion, grpc×3, conditional)
- **NodeRegistry**: 替代旧 match 分发模式，通过 `registry.get_executor(node_type)` 查找执行器

### FlowExecutor (executor.rs)
- `execute()`: 主执行循环，遍历节点图
- `replace_variables()`: 全局变量替换
- `execute_node()`: 通过 NodeRegistry 插件查找执行器 (替代旧 match 分发)
- 端口路由: HTTP/WS/DB/Conditional/Assertion/gRPC 节点使用 true/false 端口
- **gRPC 管理器初始化**: GrpcConnectionManager 注入 ExecutionContext
- **cancel 功能**: cancel_receiver + tokio::select! 支持流程取消
- **变量变更追踪**: compute_variable_changes() 追踪变量差异

### DebugFlowExecutor (debug/executor.rs)
- 断点暂停、单步执行、继续、跳过
- 通过 Channel 实现实时日志流
- 注意: 尚未集成 gRPC 节点

### 压力测试 (stress_test/)
- 并发用户数、渐进启动、持续时长
- 实时进度 (RPS、延迟、迭代数)
- 最终报告 (p50/p90/p95/p99)
- 注意: 尚未集成 gRPC 节点

## 编译影响

修改此 crate → `flow-engine` (门面) → `app` 重编译 (~2 crate)

```bash
cd src-tauri
cargo test -p flow-engine-executor      # 测试此 crate
cargo clippy -p flow-engine-executor    # Lint 此 crate
```
