# Rust 执行引擎

**生成时间:** 2026-02-28

## OVERVIEW

流程执行核心：节点调度、变量替换、HTTP 请求、条件分支。基于 `NodeExecutor` trait 的插件式架构。

## STRUCTURE

```
src/
├── lib.rs              # 模块导出 + Tauri 入口
├── main.rs             # 应用入口
├── executor.rs         # FlowExecutor + NodeExecutor trait + replace_variables()
├── context.rs          # ExecutionContext + VariableStorage
├── node.rs             # Node/NodeType/Port/NodePosition 结构体
├── node_config.rs      # NodeData 枚举 (Entry/Http/Conditional 配置)
├── connection.rs       # Connection 连接定义
├── flow.rs             # Flow 流程定义
├── entry_executor.rs   # 入口节点执行器
├── http_executor.rs    # HTTP 请求执行器 (reqwest 封装)
├── conditional_executor.rs  # 条件分支执行器 (evalexpr)
├── validator.rs        # FlowValidator 流程验证
├── error.rs            # ErrorKind + ErrorDetail
├── report.rs           # NodeExecutionReport + FlowExecutionReport
├── statistics.rs       # FlowStatistics
├── storage.rs          # save_flow/load_flow 文件存储
├── command.rs          # IPC 输入/输出类型
└── commands.rs         # Tauri IPC 命令 (create_flow/execute_flow/update_node/delete_node)
```

## WHERE TO LOOK

| 任务 | 文件 | 位置 |
|------|------|------|
| 添加新节点类型 | `node.rs` + `node_config.rs` | NodeType 枚举 + NodeData 枚举 |
| 修改变量替换逻辑 | `executor.rs` | `replace_variables()` 函数 |
| 添加 HTTP 功能 | `http_executor.rs` | `HttpNodeExecutor::execute()` |
| 修改条件表达式 | `conditional_executor.rs` | evalexpr 调用 |
| 调整执行顺序/调度 | `executor.rs` | `FlowExecutor::execute_flow()` |
| 添加错误类型 | `error.rs` | `ErrorKind` 枚举 |
| 修改 IPC 接口 | `commands.rs` | Tauri 命令函数 |
| 流程验证规则 | `validator.rs` | `FlowValidator::validate()` |

## CONVENTIONS

- **async-trait**: `NodeExecutor` trait 使用 `#[async_trait::async_trait]`
- **错误处理**: 返回 `Result<T, String>`，无 `unwrap()` 调用
- **变量语法**: `{{path.to.value}}` - 嵌套路径访问
- **执行结果**: `(Vec<String>, Option<Value>)` - 下一步节点 ID 列表 + 输出数据
- **禁止**: `unsafe_code`, 空 `unwrap()`

## NODE EXECUTOR PATTERN

```rust
// 1. 定义执行器结构体
pub struct MyNodeExecutor;

// 2. 实现 NodeExecutor trait
#[async_trait::async_trait]
impl NodeExecutor for MyNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), String> {
        // 3. 从 node.data 解析配置
        // 4. 使用 replace_variables() 替换变量
        // 5. 执行业务逻辑
        // 6. 返回 (next_node_ids, output_data)
        Ok((vec!["next-node-id".to_string()], Some(json!({"result": "ok"}))))
    }
}

// 7. 在 executor.rs::execute_node() 中注册
NodeType::MyType => MyNodeExecutor.execute(node, context, connections, node_map).await,

