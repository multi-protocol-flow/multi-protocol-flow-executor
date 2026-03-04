# Flow Engine 核心库

**生成时间:** 2026-03-04
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
│   ├── execute_flow.rs    # 流程执行入口
│   ├── create_flow.rs     # 创建流程
│   ├── update_node.rs     # 更新节点
│   └── delete_node.rs     # 删除节点
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
└── *_executor.rs          # 节点执行器实现
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

## 约定

1. **独立运行**: 所有核心逻辑通过 `traits/` 抽象，Tauri 依赖通过 trait 注入
2. **日志发送**: `LogSender` trait 支持多实现，CLI 用 `NoopLogSender`
3. **变量语法**: `{{path.to.value}}`，不支持索引和函数调用
4. **错误分类**: `ErrorKind` 区分致命/可恢复错误，决定流程是否终止
