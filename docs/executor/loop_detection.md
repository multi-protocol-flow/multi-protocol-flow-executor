# 循环检测详细设计

**文档位置**：`docs/executor/loop_detection.md`
**语言**：中文
**版本**：v1.0
**目标读者**：后端开发人员

---

## 1. 概述

循环检测机制防止节点无限重入，允许有控制的循环执行。该机制基于执行计数器，限制每个节点的最大执行次数，防止流程陷入无限循环。

**核心设计**：
- 允许节点重入（如 Loop 节点）
- 限制最大执行次数
- 超限后终止执行并返回错误

---

## 2. 允许重入设计

### 2.1 设计原则

- **支持重入**：节点可以被多次访问（如循环节点、条件回退）
- **独立计数**：每个节点维护独立的执行计数器
- **超限保护**：超过最大执行次数时终止流程

### 2.2 实现结构

```rust
pub struct FlowExecutor {
    /// 最大执行次数（防止无限重入）
    max_execution_count: HashMap<String, u32>,
    /// 每个节点的当前执行次数
    execution_count: HashMap<String, u32>,
}
```

**说明**：
- `max_execution_count`：存储每个节点的最大执行次数限制
- `execution_count`：跟踪每个节点的当前执行次数

---

## 3. 最大执行次数

### 3.1 默认配置

- **默认值**：100
- **配置方式**：可按节点类型设置不同限制
- **作用**：防止无限循环和资源耗尽

### 3.2 配置示例

```rust
// 不同节点类型可配置不同限制
max_execution_count = {
    "http_node": 100,
    "loop_node": 1000,
    "default": 100
}
```

---

## 4. 执行计数器机制

### 4.1 初始化

```rust
// 执行流程前初始化计数器
execution_count = HashMap::new();
for node in flow.nodes {
    execution_count.insert(node.uuid.clone(), 0);
}
```

### 4.2 增加计数

```rust
// 节点执行前检查并增加计数
fn increment_execution_count(&mut self, node_id: &str) {
    let count = self.execution_count.entry(node_id.to_string()).or_insert(0);
    *count += 1;
}
```

---

## 5. 检测逻辑

### 5.1 检查函数

```rust
/// 检查执行限制
fn check_execution_limit(&self, node_id: &str) -> bool {
    let count = self.execution_count.get(node_id).unwrap_or(&0);
    let max = self.max_execution_count.get(node_id).unwrap_or(&100);
    count <= max
}
```

**逻辑说明**：
- 获取当前节点的执行次数
- 获取当前节点的最大执行次数限制
- 返回是否未超限

### 5.2 集成到执行循环

```rust
while !current_node_ids.is_empty() {
    let current_node_id = current_node_ids[0].clone();

    // 检测重入限制
    if !self.check_execution_limit(&current_node_id) {
        return ExecutionResult {
            success: false,
            error: Some(format!("节点 {} 超过最大执行次数", current_node_id)),
            // ...
        };
    }

    // 增加执行计数
    self.increment_execution_count(&current_node_id);

    // 执行节点...
}
```

---

## 6. 处理流程

### 6.1 完整流程

1. **检查执行限制**：调用 `check_execution_limit`
2. **判断是否超限**：
   - 超限：返回错误，终止执行
   - 未超限：继续执行
3. **增加计数器**：更新 `execution_count`
4. **执行节点**：调用节点执行器
5. **处理结果**：成功或失败处理
6. **查找下一个节点**：继续执行循环

### 6.2 流程图

```
开始执行
    ↓
检查执行限制
    ↓
├─ 超限 → 返回错误，终止执行
└─ 未超限 → 继续执行
        ↓
    增加计数器
        ↓
    执行节点
        ↓
    查找下一个节点
        ↓
    返回检查执行限制（循环）
```

---

## 7. 使用场景

### 7.1 Loop 节点实现循环

Loop 节点通过回连到前序节点实现循环，执行计数器限制循环次数。

### 7.2 递归流程

流程设计中的递归调用（如处理嵌套数据结构）。

### 7.3 条件回退

条件节点根据条件回退到前序节点，重试或重新处理。

---

## 8. 错误处理

### 8.1 超限错误

```rust
ExecutionResult {
    success: false,
    error: Some("节点 xxx 超过最大执行次数".to_string()),
    // ...
}
```

### 8.2 调试信息

执行报告中包含节点的执行次数，便于调试：
```json
{
  "node_uuid": "xxx",
  "execution_count": 100,
  "max_execution_count": 100,
  "status": "exceeded"
}
```

---

## 9. 相关代码位置

- **主执行器**：`src-tauri/src/executor.rs`
- **FlowExecutor 结构**：`executor.rs` 第 223-228 行
- **检查逻辑**：`executor.rs` 第 262-268 行

---

**文档结束**
