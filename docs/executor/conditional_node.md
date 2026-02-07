# 条件节点详细设计

**文档位置**：`docs/executor/conditional_node.md`
**语言**：中文
**版本**：v1.0
**参考文档**：`docs/executor_architecture.md`

---

## 1. 概述

条件节点是流程中的分支控制节点，根据条件表达式的评估结果选择执行路径。条件节点采用**双分支设计**，包含两个输出端口：`true` 分支和 `false` 分支。

**核心功能：**
- 评估条件表达式
- 根据 `true/false` 结果选择执行分支
- 支持变量引用和复杂逻辑运算

---

## 2. 双分支设计

### 2.1 输出端口

条件节点有两个固定输出端口：

| 端口名称 | 说明 | 触发条件 |
|---------|------|---------|
| `true` | 条件为真时的执行路径 | 条件表达式评估结果为 `true` |
| `false` | 条件为假时的执行路径 | 条件表达式评估结果为 `false` |

### 2.2 流程图

```
┌─────────────────┐
│  条件节点        │
│  (Conditional)  │
└────────┬────────┘
         │
    ┌────┴────┐
    ↓         ↓
┌──────┐  ┌──────┐
│true  │  │false │
│分支  │  │分支  │
└──────┘  └──────┘
```

---

## 3. 条件表达式

### 3.1 支持的表达式类型

使用 `evalexpr` 库评估表达式，支持以下操作：

**比较运算：**
- `==` 等于
- `!=` 不等于
- `>` 大于
- `<` 小于
- `>=` 大于等于
- `<=` 小于等于

**逻辑运算：**
- `&&` 与
- `||` 或
- `!` 非

**变量引用：**
- `${variable_name}` 访问变量

### 3.2 表达式示例

```
# 简单比较
${score} > 60

# 逻辑运算
${age} >= 18 && ${has_id_card}

# 复杂条件
(${status} == "active" || ${status} == "pending") && ${count} > 0
```

---

## 4. 节点配置

### 4.1 ConditionalConfig 结构

```rust
/// 条件节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalConfig {
    /// 条件表达式
    pub expression: String,
}
```

**配置说明：**
- `expression`: 条件表达式字符串，使用 `evalexpr` 语法

---

## 5. 执行逻辑

### 5.1 执行流程

```
开始
  ↓
1. 解析条件表达式
  ↓
2. 变量替换（将 ${var} 替换为实际值）
  ↓
3. 使用 evalexpr 评估表达式
  ↓
4. 获取评估结果（true/false）
  ↓
5. 根据结果选择分支
  ├─ true → 查找 true 端口连接的节点
  └─ false → 查找 false 端口连接的节点
  ↓
6. 返回下一个节点 ID
  ↓
结束
```

### 5.2 伪代码

```rust
async fn execute(
    &self,
    node: &Node,
    context: &ExecutionContext,
    connections: &[Connection],
    node_map: &HashMap<String, &Node>,
) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
    // 1. 获取条件节点配置
    let config = match &node.data {
        NodeData::Conditional(cfg) => cfg,
        _ => return Err("节点配置类型错误".to_string()),
    };

    // 2. 准备表达式上下文
    let expr_context = context.build_eval_context();

    // 3. 评估条件表达式
    let result = evalexpr::eval_boolean_with_context(
        &config.expression,
        &expr_context,
    )
    .map_err(|e| format!("表达式评估失败: {}", e))?;

    // 4. 确定分支端口
    let target_port = if result { "true" } else { "false" };

    // 5. 查找连接到目标端口的节点
    let next_node_ids = connections
        .iter()
        .filter(|c| c.source_node_id == node.uuid && c.source_port == target_port)
        .map(|c| c.target_node_id.clone())
        .collect::<Vec<_>>();

    // 6. 返回结果
    Ok((next_node_ids, None))
}
```

---

## 6. ConditionalNodeExecutor 实现

### 6.1 完整实现

```rust
use crate::node::{Node, NodeData};
use crate::executor::{ExecutionContext, NodeExecutor};
use crate::connection::Connection;
use std::collections::HashMap;

/// 条件节点执行器
pub struct ConditionalNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for ConditionalNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 1. 获取条件节点配置
        let config = match &node.data {
            NodeData::Conditional(cfg) => cfg,
            _ => return Err("节点配置类型错误".to_string()),
        };

        // 2. 准备表达式上下文
        let mut expr_context = evalexpr::HashMapContext::new();

        // 添加全局变量
        for (key, value) in &context.variables.global {
            expr_context.set_value(key.clone(), value.clone())
                .map_err(|e| format!("设置变量失败: {}", e))?;
        }

        // 3. 评估条件表达式
        let result = evalexpr::eval_boolean_with_context(
            &config.expression,
            &expr_context,
        )
        .map_err(|e| format!("表达式评估失败: {}", e))?;

        // 4. 确定分支端口
        let target_port = if result { "true" } else { "false" };

        // 5. 查找连接到目标端口的节点
        let next_node_ids = connections
            .iter()
            .filter(|c| c.source_node_id == node.uuid && c.source_port == target_port)
            .map(|c| c.target_node_id.clone())
            .collect::<Vec<_>>();

        // 6. 返回结果
        Ok((next_node_ids, None))
    }

    // 条件节点不支持容错
    fn is_fault_tolerant(&self, _node: &Node) -> bool {
        false
    }
}
```

### 6.2 特点

- **不支持容错**：条件表达式评估失败应立即终止流程
- **无输出数据**：条件节点不产生输出数据，返回 `None`
- **单分支**：每次评估只选择一个分支，不支持同时执行多个分支

---

## 7. 连接处理逻辑

### 7.1 连接过滤

条件节点的连接通过 `source_port` 过滤：

```rust
// 查找 true 端口的连接
let true_connections = connections
    .iter()
    .filter(|c| c.source_node_id == node.uuid && c.source_port == "true")
    .collect::<Vec<_>>();

// 查找 false 端口的连接
let false_connections = connections
    .iter()
    .filter(|c| c.source_node_id == node.uuid && c.source_port == "false")
    .collect::<Vec<_>>();
```

### 7.2 分支验证

在执行前验证连接：

```rust
// 验证至少有一个连接
if true_connections.is_empty() && false_connections.is_empty() {
    return Err("条件节点至少需要一个输出连接".to_string());
}

// 验证每个端口最多一个连接（可选，根据需求）
if true_connections.len() > 1 {
    return Err("true 端口只能有一个连接".to_string());
}
```

---

## 8. 代码示例

### 8.1 完整示例

```rust
use async_trait::async_trait;
use evalexpr::{build_operator_tree, Context};

pub struct ConditionalNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for ConditionalNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 获取配置
        let config = match &node.data {
            NodeData::Conditional(cfg) => cfg,
            _ => return Err("配置类型错误".to_string()),
        };

        // 构建表达式上下文
        let mut eval_context = evalexpr::HashMapContext::new();

        // 添加变量
        for (key, value) in &context.variables.global {
            eval_context.set_value(key.clone(), value.clone())
                .map_err(|e| format!("变量设置失败: {}", e))?;
        }

        // 评估表达式
        let tree = build_operator_tree(&config.expression)
            .map_err(|e| format!("表达式解析失败: {}", e))?;

        let result = tree
            .eval_boolean_with_context(&eval_context)
            .map_err(|e| format!("表达式评估失败: {}", e))?;

        // 选择分支
        let port = if result { "true" } else { "false" };

        // 查找下一个节点
        let next_nodes = connections
            .iter()
            .filter(|c| c.source_node_id == node.uuid && c.source_port == port)
            .map(|c| c.target_node_id.clone())
            .collect();

        Ok((next_nodes, None))
    }

    fn is_fault_tolerant(&self, _node: &Node) -> bool {
        false
    }
}
```

---

## 9. 总结

**关键设计要点：**

1. **双分支设计**：`true/false` 两个固定输出端口
2. **表达式评估**：使用 `evalexpr` 库，支持变量引用和复杂逻辑
3. **执行流程**：解析 → 替换 → 评估 → 选择分支
4. **无容错**：表达式评估失败立即终止
5. **无输出**：不产生输出数据，只控制流程走向

**扩展性：**

- 可以添加更多表达式类型支持
- 可以扩展表达式上下文，支持嵌套变量访问
- 可以添加表达式预编译优化

---

**文档结束**
