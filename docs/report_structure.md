# 后端执行器报告结构

**版本**：v2.0

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
    /// 跳过执行（如条件分支未命中）
    Skipped,
}
```

---

### 2. ErrorDetail - 结构化错误详情

```rust
/// 错误详情结构（用于执行调试）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorDetail {
    /// 错误类型（决定错误处理策略，同时标识具体错误类型）
    pub error_kind: ErrorKind,

    /// 错误代码（可选，用于程序化处理）
    pub error_code: Option<String>,

    /// 错误消息（人类可读）
    pub message: String,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,

    /// 错误发生时的上下文快照（可选）
    pub context: Option<serde_json::Value>,
}

/// 错误类型枚举（决定错误处理策略，同时标识具体错误类型）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    // ===== 配置错误（立即终止流程） =====
    /// 通用配置错误
    Configuration,

    /// 条件表达式语法错误（如表达式语法错误、字段配置缺失）
    ConditionSyntax,

    // ===== 执行错误（容错模式，继续执行） =====
    /// 通用执行错误
    Execution,

    /// 网络连接失败
    Connection,

    /// 操作超时
    Timeout,

    /// 条件评估运行时错误（如字段不存在、类型不匹配）
    ConditionEvaluation,

    // ===== 验证错误（容错模式，继续执行） =====
    /// 参数验证失败
    Validation,

    // ===== 解析错误（容错模式，继续执行） =====
    /// 数据解析失败（如 JSON 解析失败、数据格式转换失败）
    Parse,
}

impl ErrorKind {
    /// 判断是否为致命错误（需要立即终止流程）
    pub fn is_fatal(&self) -> bool {
        matches!(self,
            ErrorKind::Configuration |
            ErrorKind::ConditionSyntax
        )
    }
}
```

**常见错误类型**：
| ErrorKind | 处理策略 | 说明 |
|-----------|---------|------|
| `Configuration` | 立即终止 | 通用配置错误 |
| `ConditionSyntax` | 立即终止 | 条件表达式语法错误、字段配置缺失 |
| `Connection` | 容错 | 网络连接失败 |
| `Timeout` | 容错 | 操作超时 |
| `Validation` | 容错 | 参数验证失败 |
| `Parse` | 容错 | 数据解析失败、格式转换失败 |
| `ConditionEvaluation` | 容错 | 条件评估运行时错误（字段不存在、类型不匹配等） |

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

    /// 实际使用的输出端口 ID（仅已执行节点有值）
    /// - Entry 节点：固定为 "out"
    /// - Http 节点：固定为 "out"
    /// - Conditional 节点：根据条件评估结果为 "true" 或 "false"
    /// 用于追踪分支执行路径，便于调试和分析
    pub used_port_id: Option<String>,

    /// 错误详情
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

    /// 完成率（executed_count / total_nodes）
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
    /// 流程 UUID
    pub flow_uuid: String,

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
    pub executed_count: u32,

    /// 整体执行状态
    pub status: ExecutionStatus,

    /// 节点执行报告列表（扁平结构）
    pub node_reports: Vec<NodeExecutionReport>,

    /// 流程统计信息
    pub statistics: FlowStatistics,

    /// 流程级错误详情（可选）
    /// 命名说明：使用 `flow_error` 而非 `error` 以区分层级：
    /// - 节点级错误：`NodeExecutionReport.error`（字段名简洁，因为在节点报告上下文中）
    /// - 流程级错误：`FlowExecutionReport.flow_error`（前缀明确，与节点错误区分）
    /// - ExecutionResult 同时包含两者：`error_detail`（结构化）、`error`（快速检查字符串）
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

## 示例

### 简单流程报告

```json
{
  "flow_uuid": "flow-uuid-456",
  "flow_name": "用户注册流程",
  "status": "success",
  "duration_ms": 850,
  "total_nodes": 2,
  "executed_count": 2,
  "node_reports": [
    {
      "node_uuid": "node-001",
      "node_type": "entry",
      "node_name": "入口",
      "start_time": 1707565200000,
      "end_time": 1707565200001,
      "status": "success",
      "duration_ms": 1,
      "input_data": null,
      "output_data": null,
      "used_port_id": "out"
    },
    {
      "node_uuid": "node-002",
      "node_type": "http",
      "status": "success",
      "duration_ms": 849,
      "input_data": {"user_id": 12345},
      "output_data": {"status": 200, "data": {"id": 12345, "name": "张三"}},
      "used_port_id": "out"
    }
  ],
  "statistics": {
    "success_count": 2,
    "failed_count": 0,
    "avg_node_duration_ms": 425,
    "error_rate": 0.0,
    "completion_rate": 1.0
  }
}
```

### 条件分支报告（含 used_port_id）

```json
{
  "node_reports": [
    {
      "node_uuid": "n2",
      "node_type": "conditional",
      "status": "success",
      "input_data": {"user_age": 25},
      "output_data": true,               // 布尔值：条件评估结果
      "used_port_id": "true"             // 字符串：实际使用的端口 ID
    },
    {
      "node_uuid": "n3",
      "node_type": "http",
      "status": "success",
      "used_port_id": "out"
    }
  ]
}
```

---

## 节点端口定义

详见 `docs/node_structure.md` 端口类型定义章节。

## output_data 语义

| 节点类型 | output_data | 说明 |
|---------|-------------|------|
| Entry | `null` | 不产生数据 |
| Http | 响应数据 | status, headers, body |
| Conditional | 布尔值 | 条件评估结果 (true/false) |

## 扩展性

节点类型特定字段可在 `output_data` 中包含，保持结构通用性。大体积数据可通过配置限制大小。

---

## 流程状态计算规则

在容错错误处理模式下：
- 节点执行失败时记录失败状态，继续执行后续节点
- 流程完成后，根据已执行节点状态确定流程状态：
  - 有任意节点失败：流程状态为 `Failed`，`flow_error` 字段包含第一个失败节点的错误详情
  - 所有节点成功：流程状态为 `Success`
- `error_rate = failed_count / executed_count`：失败节点数与已执行节点数的比率

## 相关文档

- **执行器架构设计**：`docs/executor_architecture.md` - 执行引擎、ExecutionContext、NodeExecutor trait、执行流程
- **节点结构设计**：`docs/node_structure.md` - Node、NodeType、Port、Connection 等数据结构定义


