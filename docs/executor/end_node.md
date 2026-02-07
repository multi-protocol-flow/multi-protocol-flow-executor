# End 节点详细设计

**文档位置**：`docs/executor/end_node.md`
**语言**：中文
**版本**：v1.0
**适用范围**：后端执行逻辑（不涉及前端）

---

## 1. 概述

End 节点是流程的终止点，负责收集最终数据并完成流程执行。

**核心作用：**
- 标记流程正常结束
- 收集并返回最终输出数据
- 可选执行自定义最终处理逻辑

---

## 2. 执行逻辑

### 2.1 基本流程

```
1. 接收执行上下文
   ├─ 读取流程变量
   └─ 读取执行历史

2. 标记流程结束
   └─ 返回空 next_node_ids

3. 收集输出数据
   ├─ 从上下文提取变量
   ├─ 构建最终输出
   └─ 应用 EndConfig 配置

4. 返回执行结果
   ├─ success: true
   ├─ data: 最终输出数据
   └─ next_node_ids: []（空列表，终止循环）
```

### 2.2 FlowExecutor 集成

```rust
// FlowExecutor 中的处理逻辑
match next_node_ids {
    next if next.is_empty() => {
        // End 节点返回空列表，终止执行循环
        break;
    }
    _ => {
        // 继续执行下一个节点
        // ...
    }
}
```

---

## 3. EndConfig 结构

### 3.1 基本定义

```rust
/// End 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndConfig {
    /// 最终消息（可选）
    #[serde(default)]
    pub final_message: Option<String>,

    /// 是否返回完整变量数据
    #[serde(default)]
    pub return_full_variables: bool,

    /// 指定返回的变量路径（JSONPath 格式）
    #[serde(default)]
    pub output_paths: Option<Vec<String>>,
}
```

### 3.2 配置说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `final_message` | `Option<String>` | `None` | 可选的最终消息文本 |
| `return_full_variables` | `bool` | `false` | 是否返回所有流程变量 |
| `output_paths` | `Option<Vec<String>>` | `None` | 指定提取的变量路径列表 |

---

## 4. EndNodeExecutor 实现

### 4.1 执行器结构

```rust
use crate::node::Node;
use crate::executor::{ExecutionContext, NodeExecutor};
use crate::connection::Connection;
use std::collections::HashMap;

/// End 节点执行器
pub struct EndNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for EndNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 1. 获取 EndConfig
        let config = match &node.data {
            crate::node::NodeData::End(config) => config,
            _ => return Err("节点类型不匹配".to_string()),
        };

        // 2. 构建最终输出数据
        let output_data = self.build_output_data(context, config);

        // 3. 返回空 next_node_ids（终止流程）
        Ok((Vec::new(), Some(output_data)))
    }
}
```

### 4.2 输出数据构建

```rust
impl EndNodeExecutor {
    /// 构建最终输出数据
    fn build_output_data(
        &self,
        context: &ExecutionContext,
        config: &crate::node::EndConfig,
    ) -> serde_json::Value {
        let mut output = serde_json::Map::new();

        // 添加最终消息（如果存在）
        if let Some(message) = &config.final_message {
            output.insert("message".to_string(), serde_json::json!(message));
        }

        // 处理变量输出
        if config.return_full_variables {
            // 返回所有变量
            output.insert(
                "variables".to_string(),
                context.variables.to_json()
            );
        } else if let Some(paths) = &config.output_paths {
            // 按路径提取特定变量
            let selected_vars = self.extract_variables_by_paths(
                &context.variables,
                paths
            );
            output.insert("variables".to_string(), selected_vars);
        }

        // 添加执行元数据
        output.insert(
            "executed_nodes".to_string(),
            serde_json::json!(context.metadata.executed_nodes)
        );
        output.insert(
            "failed_nodes".to_string(),
            serde_json::json!(context.metadata.failed_nodes)
        );

        serde_json::Value::Object(output)
    }

    /// 按路径提取变量
    fn extract_variables_by_paths(
        &self,
        variables: &crate::executor::VariableStorage,
        paths: &[String],
    ) -> serde_json::Value {
        // 实现路径提取逻辑
        // 示例：支持 "nodes.{uuid}.output" 格式
        let mut result = serde_json::Map::new();
        for path in paths {
            // 简化示例，实际需要完整的 JSONPath 实现
            if let Some(value) = variables.get_by_path(path) {
                result.insert(path.clone(), value);
            }
        }
        serde_json::Value::Object(result)
    }
}
```

---

## 5. 输出数据处理

### 5.1 数据来源

End 节点的输出数据来自：

| 数据源 | 说明 | 访问方式 |
|--------|------|----------|
| **全局变量** | 流程初始变量 + 显式提升的变量 | `variables.global.{key}` |
| **节点输出** | 各节点的输出数据 | `variables.nodes.{node_uuid}.{key}` |
| **执行元数据** | 执行统计信息 | `executed_nodes`, `failed_nodes` |

### 5.2 输出格式

```json
{
  "message": "流程执行成功",
  "variables": {
    "global": {
      "api_key": "sk-xxx",
      "user_id": "12345"
    },
    "nodes": {
      "550e8400-...": {
        "output": "开始执行"
      },
      "6ba7b810-...": {
        "response": {
          "status": 200,
          "data": { "name": "张三" }
        }
      }
    }
  },
  "executed_nodes": 3,
  "failed_nodes": 0
}
```

### 5.3 路径提取示例

```json
// EndConfig 配置
{
  "output_paths": [
    "nodes.6ba7b810-....response.data.name",
    "global.user_id"
  ]
}

// 输出结果
{
  "variables": {
    "nodes.6ba7b810-....response.data.name": "张三",
    "global.user_id": "12345"
  }
}
```

---

## 6. 终止流程机制

### 6.1 终止信号

End 节点通过返回**空 next_node_ids 列表**来终止流程：

```rust
// EndNodeExecutor::execute 返回
Ok((Vec::new(), Some(output_data)))
//     ^^^^^^^^^^^
//     空列表 = 终止信号
```

### 6.2 FlowExecutor 检测

```rust
// FlowExecutor 执行循环中
let (next_node_ids, output_data) = executor.execute(node, &context, ...).await?;

// 检测终止信号
if next_node_ids.is_empty() {
    // End 节点，终止执行循环
    break;
}

// 继续执行下一个节点
context.current_node_id = Some(next_node_ids[0].clone());
```

### 6.3 执行结果返回

```rust
// FlowExecutor 生成最终结果
ExecutionResult {
    success: true,
    data: Some(output_data),  // End 节点的输出数据
    error: None,
    executed_nodes: /* 已执行节点列表 */,
    flow_report: Some(/* 执行报告 */),
}
```

---

## 7. 完整代码示例

### 7.1 使用示例

```rust
// 创建 End 节点
let end_node = Node {
    uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    node_type: NodeType::End,
    name: "结束".to_string(),
    position: NodePosition { x: 700, y: 100 },
    data: NodeData::End(EndConfig {
        final_message: Some("流程执行成功".to_string()),
        return_full_variables: true,
        output_paths: None,
    }),
    ports: None,
};

// 执行 End 节点
let executor = EndNodeExecutor;
let (next_node_ids, output_data) = executor
    .execute(&end_node, &context, &connections, &node_map)
    .await?;

// next_node_ids 为空，流程终止
assert!(next_node_ids.is_empty());
```

### 7.2 简化输出配置

```rust
// 仅返回特定变量
let config = EndConfig {
    final_message: None,
    return_full_variables: false,
    output_paths: Some(vec![
        "nodes.http-node-123.response.data".to_string(),
        "global.user_id".to_string(),
    ]),
};

// 执行结果
{
  "variables": {
    "nodes.http-node-123.response.data": { "name": "张三" },
    "global.user_id": "12345"
  }
}
```

---

## 8. 注意事项

### 8.1 设计约束

- End 节点**必须返回**空 next_node_ids 列表
- 不应该有从 End 节点出发的连接（前端验证）
- End 节点的输出数据应该是流程的最终结果

### 8.2 扩展建议

**未来可能的扩展：**

1. **自定义后处理函数**
   ```rust
   pub struct EndConfig {
       pub post_process_script: Option<String>,  // 自定义处理脚本
   }
   ```

2. **异步回调**
   ```rust
   pub struct EndConfig {
       pub webhook_url: Option<String>,  // 执行完成后回调
   }
   ```

3. **数据持久化**
   ```rust
   pub struct EndConfig {
       pub save_to_database: Option<DatabaseConfig>,  // 保存到数据库
   }
   ```

---

**文档结束**
