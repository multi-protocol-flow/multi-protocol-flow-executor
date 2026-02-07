# 报告生成详细设计

**文档位置**：`docs/executor/reporting.md`
**语言**：中文
**版本**：v1.0

---

## 1. 概述

报告生成机制收集执行过程中的所有信息，生成结构化的执行报告。

**核心目标**：
- 从 `ExecutionContext` 收集执行数据
- 为每个节点生成 `NodeExecutionReport`
- 汇总生成 `FlowExecutionReport`
- 自动计算统计信息

---

## 2. 报告结构

### 2.1 节点执行报告（NodeExecutionReport）

记录单个节点的执行详情：

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionReport {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点类型
    pub node_type: String,
    /// 节点名称
    pub node_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 输入数据（完整存储）
    pub input_data: Option<serde_json::Value>,
    /// 输出数据（完整存储）
    pub output_data: Option<serde_json::Value>,
    /// 结构化错误详情
    pub error: Option<ErrorDetail>,
}
```

### 2.2 流程执行报告（FlowExecutionReport）

汇总整个流程的执行情况：

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 流程 ID
    pub flow_id: String,
    /// 流程名称
    pub flow_name: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: u64,
    /// 总执行时长（毫秒）
    pub duration_ms: u64,
    /// 总节点数
    pub total_nodes: u32,
    /// 已执行节点数
    pub executed_nodes: u32,
    /// 整体执行状态
    pub status: ExecutionStatus,
    /// 节点执行报告列表
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程统计信息
    pub statistics: FlowStatistics,
    /// 流程级错误详情
    pub flow_error: Option<ErrorDetail>,
}
```

---

## 3. 报告生成流程

```
执行完成
    ↓
从 ExecutionContext 收集数据
    ├─ execution_history: Vec<ExecutionRecord>
    ├─ metadata: ExecutionMetadata
    └─ variables: VariableStorage
    ↓
为每个节点生成 NodeExecutionReport
    ↓
生成 FlowExecutionReport
    ├─ 计算流程级统计信息
    ├─ 汇总节点报告
    └─ 确定流程状态
    ↓
返回 ExecutionResult
```

### 3.1 生成入口

在 `FlowExecutor::execute_flow()` 的最后阶段：

```rust
// 执行循环结束后
let flow_report = self.generate_flow_report(&context, &flow);

// 返回结果
ExecutionResult {
    success: true,
    data: None,
    error: None,
    executed_nodes: /* ... */,
    flow_report: Some(flow_report),
}
```

---

## 4. 数据收集机制

### 4.1 从 ExecutionContext 收集

**执行历史（execution_history）**：
```rust
pub struct ExecutionRecord {
    /// 节点 UUID
    pub node_uuid: String,
    /// 执行时间
    pub timestamp: u64,
    /// 执行结果
    pub result: ExecutionResultType,  // Success, Failed(String), Skipped(String)
}
```

**执行元数据（metadata）**：
```rust
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

**变量存储（variables）**：
```rust
pub struct VariableStorage {
    /// 全局变量
    pub global: HashMap<String, serde_json::Value>,
    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}
```

### 4.2 数据来源映射

| 字段 | 来源 |
|-----|------|
| flow_id | `ExecutionContext.flow_id` |
| flow_name | `Flow.name` |
| start_time | `ExecutionMetadata.flow_start_time` |
| end_time | 当前时间戳 |
| total_nodes | `Flow.nodes.len()` |
| executed_nodes | `ExecutionMetadata.executed_nodes` |
| node_reports | 从 `execution_history` 生成 |

---

## 5. 统计信息计算

### 5.1 基础统计

从 `ExecutionMetadata` 直接获取：

```rust
executed_nodes: u32,      // 已执行节点数
failed_nodes: u32,        // 失败节点数
skipped_nodes: u32,       // 跳过节点数
```

### 5.2 计算逻辑

```rust
pub struct FlowStatistics {
    /// 成功的节点数量
    pub success_count: u32,
    /// 失败的节点数量
    pub failed_count: u32,
    /// 跳过的节点数量
    pub skipped_count: u32,
    /// 平均节点执行时间
    pub avg_node_duration_ms: Option<u64>,
    /// 最长节点执行时间
    pub max_node_duration_ms: Option<u64>,
    /// 最短节点执行时间
    pub min_node_duration_ms: Option<u64>,
    /// 错误率（0.0 - 1.0）
    pub error_rate: Option<f64>,
    /// 完成率
    pub completion_rate: Option<f64>,
}

impl FlowStatistics {
    pub fn from_reports(reports: &[NodeExecutionReport], total_nodes: u32) -> Self {
        // 统计各状态数量
        let success_count = reports.iter().filter(|r| r.status == ExecutionStatus::Success).count() as u32;
        let failed_count = reports.iter().filter(|r| r.status == ExecutionStatus::Failed).count() as u32;
        let skipped_count = reports.iter().filter(|r| r.status == ExecutionStatus::Skipped).count() as u32;

        // 计算时间统计
        let durations: Vec<u64> = reports.iter().map(|r| r.duration_ms).collect();
        let avg_node_duration_ms = if !durations.is_empty() {
            Some(durations.iter().sum::<u64>() / durations.len() as u64)
        } else {
            None
        };
        let max_node_duration_ms = durations.iter().max().copied();
        let min_node_duration_ms = durations.iter().min().copied();

        // 计算效率指标
        let executed_count = reports.len() as u32;
        let error_rate = if executed_count > 0 {
            Some(failed_count as f64 / executed_count as f64)
        } else {
            None
        };
        let completion_rate = if total_nodes > 0 {
            Some(executed_count as f64 / total_nodes as f64)
        } else {
            None
        };

        FlowStatistics {
            success_count,
            failed_count,
            skipped_count,
            avg_node_duration_ms,
            max_node_duration_ms,
            min_node_duration_ms,
            error_rate,
            completion_rate,
            total_input_size_bytes: None,
            total_output_size_bytes: None,
        }
    }
}
```

---

## 6. 代码示例

### 6.1 生成节点报告

```rust
fn generate_node_report(
    record: &ExecutionRecord,
    node: &Node,
    context: &ExecutionContext,
) -> NodeExecutionReport {
    // 从 ExecutionRecord 获取状态
    let status = match &record.result {
        ExecutionResultType::Success => ExecutionStatus::Success,
        ExecutionResultType::Failed(msg) => {
            ExecutionStatus::Failed
        }
        ExecutionResultType::Skipped(reason) => ExecutionStatus::Skipped,
    };

    // 从 VariableStorage 获取节点输出
    let output_data = context.variables.nodes.get(&record.node_uuid)
        .map(|vars| json!(vars));

    NodeExecutionReport {
        node_uuid: record.node_uuid.clone(),
        node_type: node.node_type.to_string(),
        node_name: node.name.clone(),
        start_time: record.timestamp,
        end_time: record.timestamp + duration_estimate(&node.node_type),
        duration_ms: duration_estimate(&node.node_type),
        status,
        input_data: None,  // 可从节点配置获取
        output_data,
        error: None,  // 可从 ExecutionResultType 提取
    }
}
```

### 6.2 生成流程报告

```rust
fn generate_flow_report(
    context: &ExecutionContext,
    flow: &Flow,
    node_map: &HashMap<String, &Node>,
) -> FlowExecutionReport {
    // 为每个执行记录生成节点报告
    let node_reports: Vec<NodeExecutionReport> = context
        .execution_history
        .iter()
        .filter_map(|record| {
            node_map.get(&record.node_uuid).map(|node| {
                generate_node_report(record, node, context)
            })
        })
        .collect();

    // 计算统计信息
    let statistics = FlowStatistics::from_reports(&node_reports, flow.nodes.len() as u32);

    // 确定流程状态
    let status = if statistics.failed_count > 0 {
        ExecutionStatus::Failed
    } else {
        ExecutionStatus::Success
    };

    FlowExecutionReport {
        flow_id: context.flow_id.clone(),
        flow_name: flow.name.clone(),
        start_time: context.metadata.flow_start_time,
        end_time: current_timestamp(),
        duration_ms: current_timestamp() - context.metadata.flow_start_time,
        total_nodes: flow.nodes.len() as u32,
        executed_nodes: context.metadata.executed_nodes,
        status,
        node_reports,
        statistics,
        flow_error: None,
    }
}
```

---

## 7. 输出格式

### 7.1 JSON 格式示例

```json
{
  "flow_id": "flow-uuid-456",
  "flow_name": "用户注册流程",
  "start_time": 1704640800000,
  "end_time": 1704640801500,
  "duration_ms": 1500,
  "total_nodes": 3,
  "executed_nodes": 3,
  "status": "success",
  "node_reports": [
    {
      "node_uuid": "node-uuid-001",
      "node_type": "start",
      "node_name": "开始",
      "start_time": 1704640800000,
      "end_time": 1704640800001,
      "duration_ms": 1,
      "status": "success",
      "input_data": null,
      "output_data": null,
      "error": null
    }
  ],
  "statistics": {
    "success_count": 3,
    "failed_count": 0,
    "skipped_count": 0,
    "avg_node_duration_ms": 500,
    "max_node_duration_ms": 849,
    "min_node_duration_ms": 1,
    "error_rate": 0.0,
    "completion_rate": 1.0
  },
  "flow_error": null
}
```

---

## 8. 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **报告结构**：`docs/report_structure.md`
- **节点结构**：`docs/node_structure.md`

---

**文档结束**
