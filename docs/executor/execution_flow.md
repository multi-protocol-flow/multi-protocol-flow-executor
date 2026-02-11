# 执行流程详细设计

**文档位置**：`docs/executor/execution_flow.md`
**语言**：中文
**版本**：v2.0
**参考文档**：`docs/executor_architecture.md`

---

## TL;DR

> **快速摘要**：本文档描述 Multi-Protocol Flow Executor 的执行流程设计。
>
> **核心流程**：
> 1. **初始化阶段**：创建执行上下文、验证流程（查找 Entry 节点）、构建映射表
> 2. **执行循环**：顺序执行节点、处理执行结果、查找下一个节点
> 3. **报告生成**：生成节点和流程级别的执行报告
>
> **设计特点**：
> - 顺序执行（单线程）
> - 节点隔离（独立输出空间）
> - 容错模式（执行/验证/解析错误时记录失败状态，继续执行）
> - 严格模式（配置错误时立即终止流程）
> - 条件节点双分支设计

---

## 1. 概述

执行流程是指从输入 Flow Definition 到生成 ExecutionResult 的完整过程。

```
Flow Definition
    ↓
┌─────────────────────────────────────┐
│ 1. 初始化阶段                        │
│    ├─ 创建 ExecutionContext          │
│    ├─ 验证流程（查找 Entry 节点）     │
│    └─ 构建映射表                     │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 2. 执行循环                          │
│    ├─ 执行当前节点                   │
│    ├─ 处理结果/错误                  │
│    └─ 查找下一个节点                 │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 3. 报告生成                          │
└─────────────────────────────────────┘
    ↓
ExecutionResult
```

---

## 2. 初始化阶段

### 2.1 创建 ExecutionContext

```rust
/// 执行上下文（详细定义见 executor_architecture.md）
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

### 2.2 查找 Entry 节点

每个有效的流程必须包含且仅包含一个 Entry 节点。

```rust
// 查找 Entry 节点
let entry_nodes = flow.nodes.iter()
    .filter(|n| matches!(n.node_type, NodeType::Entry))
    .collect::<Vec<_>>();

if entry_nodes.is_empty() {
    return Err("流程必须包含一个 Entry 节点".to_string());
}
if entry_nodes.len() > 1 {
    return Err("流程只能包含一个 Entry 节点".to_string());
}
```

### 2.3 构建映射表

**Node Map**：UUID → Node 引用

```rust
let node_map: HashMap<String, &Node> = flow.nodes
    .iter()
    .map(|n| (n.uuid.clone(), n))
    .collect();
```

**Connection Map**：source_node_uuid → Vec<Connection>

```rust
let connection_map: HashMap<String, Vec<&Connection>> = flow.connections
    .iter()
    .fold(HashMap::new(), |mut map, conn| {
        map.entry(conn.source_node_uuid.clone())
            .or_insert_with(Vec::new)
            .push(conn);
        map
    });
```

---

## 3. 执行循环

### 3.1 循环结构

**执行流程：**
1. 从 Entry 节点开始，顺序执行节点
2. 节点执行成功：更新上下文（记录报告、保存变量），继续下一个节点
3. 节点执行失败：根据错误类型决定处理策略
   - **配置错误**（ConfigurationError）：记录失败状态，立即终止流程
   - **执行/验证/解析错误**：记录失败状态，当前分支终止，流程可能继续执行其他分支
4. 无下一个节点时退出循环，生成报告

### 3.2 条件节点特殊处理

条件节点有两个输出端口（true/false），根据条件评估结果选择分支：

```rust
// ConditionalNodeExecutor 返回
let target_port = if condition_result { "true" } else { "false" };

let next_node_ids = connections
    .iter()
    .filter(|c| c.source_port_id == target_port)
    .map(|c| c.target_node_uuid.clone())
    .collect();

// 返回空列表表示该分支无连接，流程终止
```

**关键设计**：
- 条件为 true 时使用 true 端口的连接
- 条件为 false 时使用 false 端口的连接
- 匹配到的分支无下游连接时流程终止（与普通节点一致）

---

## 4. 报告生成

执行循环结束后，从 `ExecutionContext` 收集数据生成报告。

### 4.1 生成流程执行报告

```rust
let flow_report = FlowExecutionReport {
    flow_uuid: context.flow_uuid.clone(),
    flow_name: flow.name.clone(),
    start_time: /* 从 node_reports 获取第一个节点开始时间 */,
    end_time: /* 当前时间戳 */,
    duration_ms: /* end_time - start_time */,
    total_nodes: flow.nodes.len() as u32,
    executed_count: context.node_reports.len() as u32,
    status: /* 根据节点报告计算 */,
    node_reports: context.node_reports,
    statistics: FlowStatistics::from_reports(&context.node_reports, flow.nodes.len() as u32),
    flow_error: None,
};
```

### 4.2 生成执行结果

```rust
let execution_result = ExecutionResult {
    success: /* 流程是否成功 */,
    data: None,
    error: /* 错误信息（如果有） */,
    error_detail: /* 结构化错误详情（如果有） */,
    executed_nodes: /* 已执行节点 UUID 列表 */,
    node_reports: context.node_reports,
    flow_report: Some(flow_report),
};
```

---

## 5. 关键设计决策

| 设计领域 | 决策 | 说明 |
|---------|------|------|
| **条件分支** | 双分支 | true/false 两个输出端口，根据条件选择分支 |
| **失败策略** | 容错+严格混合模式 | 配置错误立即终止流程，执行/验证/解析错误记录失败状态并继续执行 |
| **变量管理** | 节点隔离 | 每个节点独立输出空间 |
| **执行模式** | 顺序执行 | 单线程顺序遍历 DAG |
| **流程终止** | 无下游节点 | 没有下一个节点时生成报告并终止 |

---

## 6. 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **节点结构**：`docs/node_structure.md`
- **报告结构**：`docs/report_structure.md`
- **变量管理**：`docs/executor/variable_management.md`
- **错误处理**：`docs/executor/error_handling.md`
- **上下文管理**：`docs/executor/context_management.md`
- **条件节点**：`docs/executor/conditional_node.md`

---

**文档结束**
