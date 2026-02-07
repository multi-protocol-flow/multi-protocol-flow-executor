# 错误处理详细设计

**文档位置**：`docs/executor/error_handling.md`
**语言**：中文
**版本**：v1.0

---

## 1. 概述

错误处理机制处理节点执行失败情况，支持容错和严格两种模式。

**核心目标：**
- 提供灵活的失败策略配置
- 支持关键节点和非关键节点的差异化处理
- 完整记录错误信息，便于调试和监控

---

## 2. 错误处理模式

### 2.1 容错模式

**特点：**
- 失败后继续执行后续节点
- 记录错误信息到执行历史
- 标记节点为 `Failed` 状态
- 不影响流程整体执行

**适用场景：**
- 非关键节点的失败
- 可恢复的错误
- 日志记录、通知等辅助操作

**行为：**
```rust
// 执行失败，但容错
context.execution_history.push(ExecutionRecord {
    node_uuid: node.uuid.clone(),
    timestamp: current_timestamp(),
    result: ExecutionResultType::Failed(error.clone()),
});

context.metadata.failed_nodes += 1;

// 继续执行下一个节点
continue;
```

### 2.2 严格模式

**特点：**
- 失败后立即终止流程
- 返回错误信息
- 停止整个流程执行

**适用场景：**
- 关键节点的失败
- 无法恢复的错误
- 数据验证失败、认证失败等

**行为：**
```rust
// 执行失败，不容错
return ExecutionResult {
    success: false,
    error: Some(error),
    // ...
};
```

---

## 3. 节点失败策略

每个节点通过 `NodeExecutor` trait 配置失败策略：

```rust
#[async_trait::async_trait]
pub trait NodeExecutor {
    /// 执行节点
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String>;

    /// 检查节点是否支持容错
    ///
    /// 返回 true 表示失败后继续执行（容错模式）
    /// 返回 false 表示失败后立即终止（严格模式）
    fn is_fault_tolerant(&self, _node: &Node) -> bool {
        false  // 默认严格模式
    }
}
```

**示例配置：**

```rust
impl NodeExecutor for HttpNodeExecutor {
    fn is_fault_tolerant(&self, node: &Node) -> bool {
        // 读取节点配置
        if let NodeData::Http(config) = &node.data {
            config.fault_tolerant
        } else {
            false
        }
    }
}

impl NodeExecutor for DataTransformNodeExecutor {
    fn is_fault_tolerant(&self, _node: &Node) -> bool {
        // 数据转换节点默认容错
        true
    }
}
```

---

## 4. 错误类型定义

### 4.1 执行结果类型

```rust
#[derive(Debug, Clone)]
pub enum ExecutionResultType {
    /// 执行成功
    Success,
    /// 执行失败（包含错误信息）
    Failed(String),
    /// 执行跳过（包含跳过原因）
    Skipped(String),
}
```

### 4.2 执行记录

```rust
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// 节点 UUID
    pub node_uuid: String,
    /// 执行时间
    pub timestamp: u64,
    /// 执行结果
    pub result: ExecutionResultType,
}
```

### 4.3 执行元数据

```rust
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

---

## 5. 错误处理流程

```
节点执行失败
    ↓
检查容错配置 (is_fault_tolerant)
    ↓
    ├─ 容错模式 → 记录错误信息 → 更新元数据 → 继续执行下一个节点
    │
    └─ 严格模式 → 立即返回错误 → 终止流程执行
```

**详细流程：**

```rust
match self.execute_node(node, &context, &flow.connections, &node_map).await {
    Ok((next_node_ids, output_data)) => {
        // 执行成功
        // 更新上下文，继续执行
    }
    Err(error) => {
        // 执行失败
        if node.executor().is_fault_tolerant(node) {
            // 容错模式
            context.execution_history.push(ExecutionRecord {
                node_uuid: node.uuid.clone(),
                timestamp: current_timestamp(),
                result: ExecutionResultType::Failed(error.clone()),
            });
            context.metadata.failed_nodes += 1;
            continue;
        } else {
            // 严格模式
            return ExecutionResult {
                success: false,
                error: Some(error),
                // ...
            };
        }
    }
}
```

---

## 6. 错误报告机制

### 6.1 节点执行报告

```rust
#[derive(Debug, Clone)]
pub struct NodeExecutionReport {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 执行开始时间
    pub start_time: u64,
    /// 执行结束时间
    pub end_time: u64,
    /// 执行耗时（毫秒）
    pub duration: u64,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 输出数据
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Success,
    Failed(String),
    Skipped(String),
}
```

### 6.2 流程执行报告

```rust
#[derive(Debug, Clone)]
pub struct FlowExecutionReport {
    /// 流程 ID
    pub flow_id: String,
    /// 流程名称
    pub flow_name: String,
    /// 执行状态
    pub status: FlowStatus,
    /// 流程开始时间
    pub start_time: u64,
    /// 流程结束时间
    pub end_time: u64,
    /// 总耗时（毫秒）
    pub duration: u64,
    /// 节点执行报告列表
    pub node_reports: Vec<NodeExecutionReport>,
    /// 执行统计
    pub stats: ExecutionStats,
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// 总节点数
    pub total_nodes: u32,
    /// 成功节点数
    pub success_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 跳过节点数
    pub skipped_nodes: u32,
}

#[derive(Debug, Clone)]
pub enum FlowStatus {
    Success,
    Failed(String),
    PartialSuccess,  // 部分成功（容错模式下）
}
```

---

## 7. 简单代码示例

### 7.1 配置容错节点

```rust
// 在节点配置中启用容错
let http_node = Node {
    uuid: "node-001".to_string(),
    name: "HTTP请求".to_string(),
    node_type: NodeType::Http,
    data: NodeData::Http(HttpConfig {
        url: "https://api.example.com".to_string(),
        method: "GET".to_string(),
        fault_tolerant: true,  // 启用容错
    }),
    position: (0, 0),
};
```

### 7.2 处理错误

```rust
// 执行节点并处理错误
match executor.execute(node, &context, &connections, &node_map).await {
    Ok((next_node_ids, output)) => {
        // 执行成功
        println!("节点 {} 执行成功", node.name);
    }
    Err(error) => {
        // 执行失败
        if executor.is_fault_tolerant(node) {
            // 容错模式
            println!("节点 {} 执行失败（容错模式）：{}", node.name, error);
            // 继续执行
        } else {
            // 严格模式
            eprintln!("节点 {} 执行失败（严格模式）：{}", node.name, error);
            // 终止流程
        }
    }
}
```

### 7.3 生成错误报告

```rust
// 生成节点执行报告
let node_report = NodeExecutionReport {
    node_uuid: node.uuid.clone(),
    node_name: node.name.clone(),
    status: ExecutionStatus::Failed("网络超时".to_string()),
    start_time,
    end_time,
    duration: end_time - start_time,
    error: Some("请求超时：30秒内无响应".to_string()),
    output: None,
};

// 生成流程执行报告
let flow_report = FlowExecutionReport {
    flow_id: flow.uuid.clone(),
    flow_name: flow.name.clone(),
    status: FlowStatus::PartialSuccess,
    start_time,
    end_time,
    duration: end_time - start_time,
    node_reports: vec![node_report],
    stats: ExecutionStats {
        total_nodes: 5,
        success_nodes: 4,
        failed_nodes: 1,
        skipped_nodes: 0,
    },
};
```

---

## 8. 扩展性

### 8.1 重试机制（未来）

```rust
#[async_trait::async_trait]
pub trait NodeExecutor {
    /// 获取最大重试次数
    fn max_retries(&self, _node: &Node) -> u32 {
        0  // 默认不重试
    }
}

// 执行时重试
for attempt in 0..=max_retries {
    match executor.execute(node, context, connections, node_map).await {
        Ok(result) => break,
        Err(error) if attempt < max_retries => {
            // 等待后重试
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Err(error) => return Err(error),
    }
}
```

### 8.2 自定义错误处理（未来）

```rust
pub trait ErrorHandler {
    /// 处理错误
    fn handle_error(
        &self,
        error: &str,
        context: &ExecutionContext,
    ) -> ErrorHandlingStrategy;
}

pub enum ErrorHandlingStrategy {
    Continue,   // 继续执行
    Retry,      // 重试
    Abort,      // 终止流程
    Custom(String),  // 自定义处理
}
```

---

## 9. 最佳实践

1. **关键节点使用严格模式**
   - 数据验证、认证等关键操作失败时立即终止
   - 避免执行不安全的操作

2. **非关键节点使用容错模式**
   - 日志记录、通知等辅助操作失败不影响主流程
   - 提高流程健壮性

3. **记录详细错误信息**
   - 包含错误类型、上下文、堆栈信息
   - 便于问题定位和修复

4. **设置合理的超时时间**
   - 防止节点长时间阻塞
   - 提高执行效率

---

**文档结束**
