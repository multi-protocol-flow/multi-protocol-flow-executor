# 上下文管理详细设计

**文档位置**：`docs/executor/context_management.md`
**语言**：中文
**版本**：v2.0

---

## 概述

`ExecutionContext` 是流程执行期间的核心状态容器，负责维护整个执行过程中的所有关键信息。

---

## ExecutionContext 结构

详细定义见 `docs/executor_architecture.md`：

```rust
/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 流程 UUID（前端生成）
    pub flow_uuid: String,
    /// 当前执行节点的 UUID
    pub current_node_uuid: Option<String>,
    /// 执行变量（用于变量替换）
    pub variables: VariableStorage,
    /// 执行历史记录（每个节点的详细执行报告）
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程触发时间（用于记录触发和开始执行的时间差）
    pub flow_trigger_time: u64,
}
```

**核心职责：**
- **状态维护**：存储流程执行的所有状态信息
- **变量管理**：通过 `VariableStorage` 管理全局和局部变量
- **历史记录**：通过 `node_reports` 记录节点执行历史
- **时间跟踪**：记录流程触发时间

---

## 上下文生命周期

### 初始化阶段

```rust
impl ExecutionContext {
    pub fn new(flow_uuid: String, variables: Option<Value>) -> Self {
        ExecutionContext {
            flow_uuid,
            current_node_uuid: None,
            variables: VariableStorage::from_initial_variables(variables),
            node_reports: Vec::new(),
            flow_trigger_time: get_current_timestamp(),
        }
    }
}
```

**初始化步骤：**
1. 创建空执行上下文
2. 加载初始变量到全局变量空间
3. 初始化节点报告为空列表
4. 记录流程触发时间

### 更新阶段

```rust
impl ExecutionContext {
    /// 更新当前节点
    pub fn set_current_node(&mut self, node_id: Option<String>) {
        self.current_node_uuid = node_id;
    }
}
```

**更新时机：**
- **节点执行后**：记录节点执行报告
- **变量更新后**：同步变量存储
- **节点切换时**：更新当前节点 ID

### 销毁阶段

执行上下文在流程结束时用于生成报告，然后由 Rust 所有权机制自动销毁。

---

## 上下文更新机制

### 节点执行流程

```rust
// 在 FlowExecutor::execute_flow() 中
async fn execute_node(
    &self,
    node: &Node,
    context: &mut ExecutionContext,
    ...
) -> Result<(), String> {
    // 1. 执行节点
    let (next_node_ids, output_data) = node.executor()
        .execute(node, context, connections, node_map)
        .await?;

    // 2. 记录节点执行报告
    context.node_reports.push(NodeExecutionReport {
        // ...
    });

    // 3. 更新变量
    context.variables.insert_node_output(&node.uuid, output_data);

    // 4. 设置下一个节点
    if !next_node_ids.is_empty() {
        context.set_current_node(Some(next_node_ids[0].clone()));
    }

    Ok(())
}
```

---

## 设计要点总结

| 要点 | 说明 |
|-----|------|
| **节点隔离** | 每个节点有独立的变量空间 |
| **完整报告** | 使用统一的 NodeExecutionReport 记录所有节点执行信息 |
| **元数据跟踪** | 实时更新执行统计 |
| **时间跟踪** | 记录流程触发时间 |

---

## 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **变量管理**：`docs/executor/variable_management.md`
- **报告结构**：`docs/report_structure.md`

---

**文档结束**
