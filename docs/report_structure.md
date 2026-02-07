# 后端执行器报告结构

## 设计原则

1. **保持通用**：所有节点类型使用统一的报告结构
2. **完整数据**：不截断 input_data 和 output_data，保留完整信息
3. **扁平结构**：node_reports 保持平铺列表，不引入复杂嵌套
4. **结构化错误**：提供详细的错误类型、代码、堆栈和上下文
5. **自动统计**：从节点报告自动计算流程级统计信息

---

## 核心数据结构

### 1. ExecutionStatus - 执行状态枚举

```rust
/// 执行状态枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 执行跳过
    Skipped,
}
```

---

### 2. ErrorDetail - 结构化错误详情

```rust
/// 错误详情结构（用于执行调试）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorDetail {
    /// 错误类型（如 "ConnectionError", "ValidationError", "TimeoutError"）
    pub error_type: String,

    /// 错误代码（可选，用于程序化处理）
    pub error_code: Option<String>,

    /// 错误消息（人类可读）
    pub message: String,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,

    /// 错误发生时的上下文快照（可选）
    pub context: Option<serde_json::Value>,
}
```

**常见错误类型**：
- `ConnectionError`：网络连接失败
- `TimeoutError`：操作超时
- `ValidationError`：参数验证失败
- `ParseError`：数据解析失败
- `ConditionError`：条件评估错误

---

### 3. NodeExecutionReport - 节点执行报告

```rust
/// 节点执行报告
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

    /// 输入数据（完整存储，不截断）
    pub input_data: Option<serde_json::Value>,

    /// 输出数据（完整存储，不截断）
    pub output_data: Option<serde_json::Value>,

    /// 结构化错误详情
    pub error: Option<ErrorDetail>,
}
```

---

### 4. FlowStatistics - 流程统计信息

```rust
/// 流程执行统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct FlowStatistics {
    // ===== 基础统计 =====
    /// 成功的节点数量
    pub success_count: u32,

    /// 失败的节点数量
    pub failed_count: u32,

    /// 跳过的节点数量
    pub skipped_count: u32,

    // ===== 时间统计 =====
    /// 平均节点执行时间（毫秒）
    pub avg_node_duration_ms: Option<u64>,

    /// 最长节点执行时间（毫秒）
    pub max_node_duration_ms: Option<u64>,

    /// 最短节点执行时间（毫秒）
    pub min_node_duration_ms: Option<u64>,

    // ===== 数据统计 =====
    /// 总输入数据大小（字节，估算）
    pub total_input_size_bytes: Option<u64>,

    /// 总输出数据大小（字节，估算）
    pub total_output_size_bytes: Option<u64>,

    // ===== 执行效率 =====
    /// 错误率（0.0 - 1.0）
    pub error_rate: Option<f64>,

    /// 完成率（executed_nodes / total_nodes）
    pub completion_rate: Option<f64>,
}

impl FlowStatistics {
    /// 从节点报告列表计算统计信息
    pub fn from_reports(reports: &[NodeExecutionReport], total_nodes: u32) -> Self {
        // 自动计算所有统计字段
    }
}
```

---

### 5. FlowExecutionReport - 流程执行报告

```rust
/// 流程执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 流程 ID
    pub flow_id: String,

    /// 流程名称
    pub flow_name: String,

    /// 开始时间（毫秒时间戳）
    pub start_time: u64,

    /// 结束时间（毫秒时间戳）
    pub end_time: u64,

    /// 总执行时长（毫秒）
    pub duration_ms: u64,

    /// 总节点数
    pub total_nodes: u32,

    /// 已执行节点数
    pub executed_nodes: u32,

    /// 整体执行状态
    pub status: ExecutionStatus,

    /// 节点执行报告列表（扁平结构）
    pub node_reports: Vec<NodeExecutionReport>,

    /// 流程统计信息
    pub statistics: FlowStatistics,

    /// 流程级错误详情（可选）
    pub flow_error: Option<ErrorDetail>,
}
```

---

### 6. ExecutionResult - 执行结果

```rust
/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行是否成功
    pub success: bool,

    /// 返回数据
    pub data: Option<serde_json::Value>,

    /// 错误信息（用于快速检查）
    pub error: Option<String>,

    /// 结构化错误详情
    pub error_detail: Option<ErrorDetail>,

    /// 已执行的节点列表
    pub executed_nodes: Vec<String>,

    /// 节点执行报告列表
    pub node_reports: Vec<NodeExecutionReport>,

    /// 流程执行报告
    pub flow_report: Option<FlowExecutionReport>,
}
```

---

## 完整示例

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
    },
    {
      "node_uuid": "node-uuid-002",
      "node_type": "http",
      "node_name": "获取用户信息",
      "start_time": 1704640800001,
      "end_time": 1704640800850,
      "duration_ms": 849,
      "status": "success",
      "input_data": {
        "user_id": 12345
      },
      "output_data": {
        "status": 200,
        "data": {
          "id": 12345,
          "name": "张三"
        }
      },
      "error": null
    },
    {
      "node_uuid": "node-uuid-003",
      "node_type": "end",
      "node_name": "结束",
      "start_time": 1704640800850,
      "end_time": 1704640801500,
      "duration_ms": 650,
      "status": "success",
      "input_data": {
        "user_id": 12345,
        "http_response": {
          "status": 200,
          "data": {
            "id": 12345,
            "name": "张三"
          }
        }
      },
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
    "total_input_size_bytes": 256,
    "total_output_size_bytes": 128,
    "error_rate": 0.0,
    "completion_rate": 1.0
  },
  "flow_error": null
}
```

---

## 扩展性

**节点类型特定字段**：建议在 `output_data` 中包含节点特定的信息，保持报告结构的通用性。

**数据限制**：对于大体积数据，可以通过配置设置 `input_data` 和 `output_data` 的大小限制。


