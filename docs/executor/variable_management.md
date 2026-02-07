# 变量管理详细设计

**文档位置**：`docs/executor/variable_management.md`
**语言**：中文
**版本**：v1.0

---

## 1. 概述

变量管理系统在节点执行期间传递数据，采用节点隔离设计。每个节点拥有独立的输出空间，确保变量不会互相污染，同时提供全局变量支持跨节点数据共享。

---

## 2. 节点隔离设计

### 2.1 设计目标
- **隔离性**：节点局部变量互不影响
- **全局性**：支持全局变量在所有节点间共享
- **安全性**：通过访问路径规范避免命名冲突
- **可扩展性**：支持显式提升节点变量到全局作用域

### 2.2 两层结构
1. **全局变量层**：存储初始变量和显式提升的变量
2. **节点局部变量层**：存储各节点的输出数据

---

## 3. VariableStorage 结构

```rust
/// 变量存储（节点隔离）
pub struct VariableStorage {
    /// 全局变量（初始变量 + 显式提升的变量）
    pub global: HashMap<String, serde_json::Value>,
    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}
```

**字段说明：**
- `global`：全局变量字典，键为变量名，值为 JSON 数据
- `nodes`：节点变量字典，键为节点 UUID，值为该节点的变量字典

---

## 4. 变量类型

### 4.1 全局变量

**来源：**
- 初始变量（流程执行前传入）
- 显式提升的变量（节点输出标记为全局）

**特点：**
- 所有节点可直接访问
- 变量名全局唯一
- 生命周期贯穿整个流程执行

### 4.2 节点局部变量

**来源：**
- 节点执行输出
- 节点配置产生的中间数据

**特点：**
- 仅存储在对应节点的输出空间
- 需要通过完整访问路径引用
- 生命周期限定在节点执行期间及后续引用

---

## 5. 变量访问路径

### 5.1 语法规则

| 变量类型 | 访问路径语法 | 示例 |
|---------|-------------|------|
| 全局变量 | `${variable_name}` | `${userId}` |
| 节点变量 | `${nodes.{node_uuid}.{key}}` | `${nodes.http1.data.user.name}` |

### 5.2 路径解析规则

1. **全局变量**：直接在 `global` 字典中查找
2. **节点变量**：
   - 首先在 `nodes` 字典中查找 `node_uuid`
   - 然后在节点变量字典中查找 `key`
   - 支持嵌套路径（如 `data.user.name`）

### 5.3 特殊语法

**节点 UUID 占位符**：`${nodes.{http1}.data}` 中的 `{http1}` 表示使用节点的自定义名称（非 UUID），系统会在运行时自动解析为实际 UUID。

---

## 6. 变量替换机制

### 6.1 替换流程

```rust
/// 变量替换流程
pub fn replace_variables(
    expression: &str,
    storage: &VariableStorage,
) -> Result<String, String> {
    // 1. 扫描表达式中的变量引用（${...}）
    // 2. 从 VariableStorage 查找变量值
    // 3. 替换为实际值（JSON 字符串）
    // 4. 返回处理后的表达式
}
```

### 6.2 查找顺序

1. 优先查找全局变量
2. 其次查找节点局部变量
3. 变量不存在时返回错误

### 6.3 类型处理

- 简单类型（字符串、数字）：直接替换
- 对象/数组类型：转换为 JSON 字符串
- null 值：替换为 `"null"` 字符串

---

## 7. 变量提升机制

### 7.1 提升语法

在节点配置中标记需要提升的输出字段：

```json
{
  "node_id": "http1",
  "outputs": {
    "userId": {
      "type": "global",
      "global_name": "current_user_id"
    }
  }
}
```

### 7.2 提升规则

- 节点执行完成后，检查 `outputs` 配置
- 如果字段标记为 `global` 类型，则复制到 `global` 字典
- 可指定新的全局变量名（`global_name`）

---

## 8. 使用示例

### 8.1 初始变量

```rust
// 初始化 VariableStorage
let variables = serde_json::json!({
    "userId": 12345,
    "apiKey": "sk-xxx"
});

let storage = VariableStorage::from_initial_variables(Some(variables));
```

### 8.2 节点输出存储

```rust
// HTTP 节点执行输出
let output = serde_json::json!({
    "data": {
        "user": {
            "name": "Alice",
            "email": "alice@example.com"
        }
    }
});

// 存储到节点输出空间
storage.insert_node_output("http1", Some(output));
```

### 8.3 变量替换示例

**输入表达式：**
```
用户ID: ${userId}, 用户名: ${nodes.{http1}.data.user.name}
```

**替换结果：**
```
用户ID: 12345, 用户名: Alice
```

**实现逻辑：**
1. `${userId}` → 从 `global["userId"]` 获取 `12345`
2. `${nodes.{http1}.data.user.name}` → 从 `nodes["http1"]["data"]["user"]["name"]` 获取 `"Alice"`

### 8.4 完整代码示例

```rust
use std::collections::HashMap;
use serde_json::{json, Value};

// 创建存储
let mut storage = VariableStorage::new();

// 设置全局变量
storage.global.insert("userId".to_string(), json!(12345));

// 设置节点输出
let mut node_output = HashMap::new();
node_output.insert("data".to_string(), json!({
    "user": {
        "name": "Alice",
        "email": "alice@example.com"
    }
}));
storage.nodes.insert("http1".to_string(), node_output);

// 替换变量
let expression = "用户名: ${nodes.http1.data.user.name}";
let result = replace_variables(expression, &storage)?;
assert_eq!(result, "用户名: Alice");
```

---

## 9. 边界情况处理

### 9.1 变量不存在

- 返回明确错误信息
- 标识缺失的变量名和访问路径
- 建议检查节点执行顺序

### 9.2 类型不匹配

- 优先类型自动转换（如数字转字符串）
- 复杂类型无法转换时返回错误

### 9.3 循环引用

- 检测并阻止循环依赖
- 避免无限递归替换

---

## 10. 相关文档

- **主架构文档**：`docs/executor_architecture.md`
- **上下文管理**：`docs/executor/context_management.md`（待创建）
- **节点结构**：`docs/node_structure.md`

---

**文档结束**
