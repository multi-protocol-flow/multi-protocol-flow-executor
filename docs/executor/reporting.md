# 报告生成详细设计

**文档位置**：`docs/executor/reporting.md`
**语言**：中文
**版本**：v2.0

---

## 概述

报告生成机制收集执行过程中的所有信息，生成结构化的执行报告。

**核心目标：**
- 从 `ExecutionContext` 收集执行数据
- 为每个节点生成 `NodeExecutionReport`
- 汇总生成 `FlowExecutionReport`
- 自动计算统计信息

---

## 报告结构

### NodeExecutionReport

详细定义见 `docs/report_structure.md`：

```rust
pub struct NodeExecutionReport {
    pub node_uuid: String,
    pub node_type: String,
    pub node_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
    pub status: ExecutionStatus,
    pub input_data: Option<serde_json::Value>,
    pub output_data: Option<serde_json::Value>,
    pub used_port_id: Option<String>,
    pub error: Option<ErrorDetail>,
}
```

### FlowExecutionReport

详细定义见 `docs/report_structure.md`：

```rust
pub struct FlowExecutionReport {
    pub flow_uuid: String,
    pub flow_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
    pub total_nodes: u32,
    pub executed_count: u32,
    pub status: ExecutionStatus,
    pub node_reports: Vec<NodeExecutionReport>,
    pub statistics: FlowStatistics,
    pub flow_error: Option<ErrorDetail>,
}
```

---

## 报告生成流程

```
执行完成
    ↓
从 ExecutionContext 收集数据
    ├─ node_reports: Vec<NodeExecutionReport>
    ├─ variables: VariableStorage
    └─ flow_trigger_time: u64
    ↓
生成 FlowExecutionReport
    ├─ 计算流程级统计信息
    ├─ 汇总节点报告
    └─ 确定流程状态
    ↓
返回 ExecutionResult
```

---

## 统计信息计算

### FlowStatistics

详细定义见 `docs/report_structure.md`：

```rust
pub struct FlowStatistics {
    pub success_count: u32,
    pub failed_count: u32,
    pub avg_node_duration_ms: Option<u64>,
    pub max_node_duration_ms: Option<u64>,
    pub min_node_duration_ms: Option<u64>,
    pub total_input_size_bytes: Option<u64>,
    pub total_output_size_bytes: Option<u64>,
    pub error_rate: Option<f64>,
    pub completion_rate: Option<f64>,
}
```

**计算逻辑：**
- `success_count`：从节点报告中统计状态为 Success 的节点数
- `failed_count`：从节点报告中统计状态为 Failed 的节点数
- `avg_node_duration_ms`：所有节点执行时间的平均值
- `error_rate`：失败节点数 / 已执行节点数
- `completion_rate`：已执行节点数 / 总节点数

---

## 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **报告结构**：`docs/report_structure.md`
- **节点结构**：`docs/node_structure.md`

## 流程状态计算规则

在容错错误处理模式下：
- 节点执行失败时记录失败状态，继续执行后续节点
- 流程完成后，根据已执行节点状态确定流程状态：
  - 有任意节点失败：流程状态为 `Failed`，`flow_error` 字段包含第一个失败节点的错误详情
  - 所有节点成功：流程状态为 `Success`
- `error_rate = failed_count / executed_count`：失败节点数与已执行节点数的比率

---

**文档结束**
