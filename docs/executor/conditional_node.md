# 条件节点详细设计

**文档位置**：`docs/executor/conditional_node.md`
**语言**：中文
**版本**：v2.0
**参考文档**：`docs/executor_architecture.md`

---

## 概述

条件节点是流程中的分支控制节点，根据条件表达式的评估结果选择执行路径。条件节点采用**双分支设计**，包含两个输出端口：`true` 分支和 `false` 分支。

**核心功能：**
- 评估条件表达式
- 根据 `true/false` 结果选择执行分支
- 支持简单条件和高级表达式

---

## 双分支设计

### 输出端口

条件节点有两个固定输出端口：

| 端口名称 | 说明 | 触发条件 |
|---------|------|---------|
| `true` | 条件为真时的执行路径 | 条件表达式评估结果为 `true` |
| `false` | 条件为假时的执行路径 | 条件表达式评估结果为 `false` |

### 流程图

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

## 条件表达式

### 支持的表达式类型

**简单条件**：使用 `field_path` + `operator` + `expected_value`

```
input.{field_path} {operator} {expected_value}
```

**高级表达式**：使用自定义表达式引擎评估，支持复杂逻辑

```
user.age >= 18 AND user.status == "active"
```

### 条件运算符

| 运算符 | 说明 |
|-------|------|
| `Equal` | 等于 |
| `NotEqual` | 不等于 |
| `Greater` | 大于 |
| `GreaterOrEqual` | 大于等于 |
| `Less` | 小于 |
| `LessOrEqual` | 小于等于 |
| `Contains` | 包含 |
| `StartsWith` | 以...开头 |
| `EndsWith` | 以...结尾 |
| `Exists` | 存在 |

---

## 节点配置

### ConditionalConfig 结构

详细定义见 `docs/node_structure.md`：

```rust
pub struct ConditionalConfig {
    /// 字段路径（简单条件）
    #[serde(default)]
    pub field_path: Option<String>,
    /// 运算符（简单条件）
    #[serde(default)]
    pub operator: Option<ConditionOperator>,
    /// 期望值（简单条件）
    #[serde(default)]
    pub expected_value: Option<serde_json::Value>,
    /// 高级表达式（可选）
    #[serde(default)]
    pub advanced_expression: Option<String>,
}
```

**字段组合规则：**
- **简单条件**：`field_path` + `operator` + `expected_value`
- **高级表达式**：仅 `advanced_expression`
- **无效配置**：任意字段缺失或组合错误，返回 `ConditionSyntaxError`（配置错误），流程终止

**优先级：**
- 如果 `advanced_expression` 有值，忽略简单条件字段
- 如果 `advanced_expression` 为空，必须提供完整的简单条件

---

## 执行逻辑

### 执行流程

```
开始
  ↓
1. 检查配置完整性
  ↓
2. 根据配置选择评估方式
  ├─ 高级表达式 → 使用表达式引擎评估
  └─ 简单条件 → 字段比较评估
  ↓
3. 获取评估结果（true/false）
  ↓
4. 根据结果选择分支端口
  ├─ true → 查找 true 端口连接的节点
  └─ false → 查找 false 端口连接的节点
  ↓
5. 返回下一个节点 ID 列表（可能为空）
  ↓
结束
```

### 伪代码

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

    // 2. 评估条件
    let condition_result = if let Some(expr) = &config.advanced_expression {
        // 高级表达式评估
        evaluate_advanced_expression(expr, context).map_err(|e| {
            ErrorDetail {
                error_kind: ErrorKind::ConditionSyntaxError,
                message: format!("高级表达式语法错误: {}", e),
                error_code: None,
                stack_trace: None,
                context: None,
            }
        })?
    } else {
        // 简单条件评估
        evaluate_simple_condition(
            &config.field_path,
            &config.operator,
            &config.expected_value,
            context,
        ).map_err(|e| {
            ErrorDetail {
                error_kind: ErrorKind::ConditionSyntaxError,
                message: format!("简单条件配置错误: {}", e),
                error_code: None,
                stack_trace: None,
                context: None,
            }
        })?
    };

    // 3. 确定分支端口
    let target_port = if condition_result { "true" } else { "false" };

    // 4. 查找连接到目标端口的节点
    let next_node_ids = connections
        .iter()
        .filter(|c| c.source_port_id == target_port)
        .map(|c| c.target_node_uuid.clone())
        .collect();

    // 5. 返回结果（布尔值作为输出数据）
    Ok((next_node_ids, Some(json!(condition_result))))
}
```

---

## 关键设计决策

| 设计领域 | 决策 | 说明 |
|---------|------|------|
| **分支设计** | 双分支 | true/false 两个固定输出端口 |
| **分支选择** | 单分支 | 每次评估只选择一个分支执行 |
| **无连接处理** | 流程终止 | 匹配到的分支无下游连接时流程终止 |
| **输出数据** | 布尔值 | 返回条件评估结果的布尔值 |
| **端口记录** | used_port_id | 报告中记录实际使用的端口 ID |

---

## 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **节点结构**：`docs/node_structure.md`
- **报告结构**：`docs/report_structure.md`

---

**文档结束**
