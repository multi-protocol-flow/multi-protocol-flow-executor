# 上下文管理详细设计

**文档位置**: `docs/executor/context_management.md`
**语言**: 中文
**版本**: v1.0

---

## 1. 概述

`ExecutionContext` 是流程执行期间的核心状态容器，负责维护整个执行过程中的所有关键信息。它在流程执行时创建，在流程结束时销毁，贯穿整个执行生命周期。

---

## 2. ExecutionContext 结构

```rust
/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 流程 ID
    pub flow_id: String,
    /// 当前节点 ID
    pub current_node_id: Option<String>,
    /// 执行变量（用于变量替换）
    pub variables: VariableStorage,
    /// 执行历史
    pub execution_history: Vec<ExecutionRecord>,
    /// 执行元数据
    pub metadata: ExecutionMetadata,
}
```

**核心职责：**
- **状态维护**：存储流程执行的所有状态信息
- **变量管理**：通过 `VariableStorage` 管理全局和局部变量
- **历史记录**：通过 `execution_history` 记录节点执行历史
- **元数据跟踪**：通过 `metadata` 跟踪执行统计信息

---

## 3. 变量存储 (VariableStorage)

```rust
/// 变量存储（节点隔离）
pub struct VariableStorage {
    /// 全局变量（初始变量 + 显式提升的变量）
    pub global: HashMap<String, serde_json::Value>,
    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}
```

**设计原则：**
- **节点隔离**：每个节点有独立的输出空间
- **访问方式**：
  - 全局变量：直接使用变量名
  - 节点变量：通过 `nodes.{node_uuid}.{key}` 访问
- **隔离机制**：避免节点之间的变量冲突

---

## 4. 执行历史管理 (ExecutionRecord)

```rust
/// 执行历史记录
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// 节点 UUID
    pub node_uuid: String,
    /// 执行时间
    pub timestamp: u64,
    /// 执行结果
    pub result: ExecutionResultType,
}

/// 执行结果类型
#[derive(Debug, Clone)]
pub enum ExecutionResultType {
    Success,
    Failed(String),
    Skipped(String),
}
```

**管理机制：**
- **不可变历史**：执行历史只追加，不修改
- **顺序记录**：按照节点执行顺序记录
- **用途**：
  - 调试：追踪执行路径
  - 审计：完整执行轨迹
  - 报告：生成执行报告

---

## 5. 元数据跟踪 (ExecutionMetadata)

```rust
/// 执行元数据
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    /// 流程开始时间
    pub flow_start_time: u64,
    /// 已执行节点数
    pub executed_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 跳过节点数
    pub skipped_nodes: u32,
}
```

**跟踪内容：**
- **时间信息**：记录流程开始时间
- **执行统计**：成功、失败、跳过的节点数
- **监控指标**：用于性能分析和故障排查

---

## 6. 上下文生命周期

### 6.1 初始化阶段

```rust
impl ExecutionContext {
    pub fn new(flow_id: String, variables: Option<Value>) -> Self {
        ExecutionContext {
            flow_id,
            current_node_id: None,
            variables: VariableStorage::from_initial_variables(variables),
            execution_history: Vec::new(),
            metadata: ExecutionMetadata::new(),
        }
    }
}
```

**初始化步骤：**
1. 创建空执行上下文
2. 加载初始变量到全局变量空间
3. 初始化执行计数器
4. 记录流程开始时间

### 6.2 更新阶段

```rust
impl ExecutionContext {
    /// 记录节点执行
    pub fn record_execution(&mut self, node_id: String, result: ExecutionResultType) {
        // 添加执行历史
        self.execution_history.push(ExecutionRecord {
            node_uuid: node_id.clone(),
            timestamp: get_current_timestamp(),
            result,
        });

        // 更新元数据
        match result {
            ExecutionResultType::Success => self.metadata.executed_nodes += 1,
            ExecutionResultType::Failed(_) => self.metadata.failed_nodes += 1,
            ExecutionResultType::Skipped(_) => self.metadata.skipped_nodes += 1,
        }
    }

    /// 更新当前节点
    pub fn set_current_node(&mut self, node_id: Option<String>) {
        self.current_node_id = node_id;
    }
}
```

**更新时机：**
- **节点执行后**：记录执行结果
- **变量更新后**：同步变量存储
- **节点切换时**：更新当前节点 ID

### 6.3 销毁阶段

```rust
impl ExecutionContext {
    /// 生成执行报告
    pub fn generate_report(&self) -> FlowExecutionReport {
        FlowExecutionReport {
            flow_id: self.flow_id.clone(),
            start_time: self.metadata.flow_start_time,
            end_time: get_current_timestamp(),
            total_nodes: self.metadata.executed_nodes,
            failed_nodes: self.metadata.failed_nodes,
            skipped_nodes: self.metadata.skipped_nodes,
            execution_history: self.execution_history.clone(),
        }
    }
}
```

**销毁操作：**
1. 生成执行报告
2. 提取关键统计信息
3. 资源释放（由 Rust 所有权机制自动处理）

---

## 7. 上下文更新机制

### 7.1 节点执行流程

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

    // 2. 记录执行历史
    context.record_execution(node.uuid.clone(), ExecutionResultType::Success);

    // 3. 更新变量
    context.variables.insert_node_output(&node.uuid, output_data);

    // 4. 更新元数据
    context.metadata.executed_nodes += 1;

    // 5. 设置下一个节点
    if !next_node_ids.is_empty() {
        context.set_current_node(Some(next_node_ids[0].clone()));
    }

    Ok(())
}
```

### 7.2 容错处理流程

```rust
// 容错模式下的上下文更新
if node.executor().is_fault_tolerant(node) {
    // 记录失败但继续执行
    context.record_execution(
        node.uuid.clone(),
        ExecutionResultType::Failed(error.clone()),
    );
    context.metadata.failed_nodes += 1;
    // 不终止执行，继续处理下一个节点
}
```

---

## 8. 使用示例

### 8.1 创建上下文

```rust
// 初始化执行上下文
let mut context = ExecutionContext::new(
    flow.uuid.clone(),
    Some(json!({
        "api_key": "secret123",
        "base_url": "https://api.example.com"
    }))
);
```

### 8.2 记录执行

```rust
// 记录节点执行成功
context.record_execution(
    "node-123".to_string(),
    ExecutionResultType::Success,
);

// 记录节点执行失败
context.record_execution(
    "node-456".to_string(),
    ExecutionResultType::Failed("Connection timeout".to_string()),
);
```

### 8.3 访问变量

```rust
// 访问全局变量
let api_key = context.variables.global.get("api_key");

// 访问节点输出变量
let response = context.variables.nodes
    .get("node-123")
    .and_then(|vars| vars.get("response"));
```

### 8.4 生成报告

```rust
// 流程结束时生成报告
let report = context.generate_report();
println!("Executed: {}, Failed: {}, Skipped: {}",
    report.total_nodes,
    report.failed_nodes,
    report.skipped_nodes,
);
```

---

## 9. 设计要点总结

| 要点 | 说明 |
|-----|------|
| **节点隔离** | 每个节点有独立的变量空间 |
| **不可变历史** | 执行历史只追加，便于调试 |
| **元数据跟踪** | 实时更新执行统计 |
| **容错支持** | 失败节点不影响整体流程 |
| **简化设计** | 仅关注后端执行逻辑 |

---

## 10. 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **变量管理详细设计**：`docs/executor/variable_management.md`（待创建）
- **错误处理详细设计**：`docs/executor/error_handling.md`（待创建）
- **报告生成详细设计**：`docs/executor/reporting.md`（待创建）

---

**文档结束**
