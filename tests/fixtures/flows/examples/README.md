# 节点配置示例

本目录包含所有节点类型的 JSON 配置示例，用于 CLI 模式和测试。

## 文件列表

### 基础节点示例

| 文件 | 说明 | 节点类型 |
|------|------|----------|
| `06_start_node_example.json` | Start 节点示例，展示初始变量定义 | Start |
| `07_http_node_example.json` | HTTP 节点示例，展示 GET 请求配置 | Start, Http, End |
| `08_conditional_node_example.json` | 条件节点示例，展示简单条件分支 | Start, Http, Conditional, End |
| `09_conditional_advanced_example.json` | 高级条件表达式示例 | Start, Conditional, End |
| `10_complete_example.json` | 完整流程示例，包含所有节点类型 | Start, Http, Conditional, End |

## 节点类型说明

### 1. Start 节点

流程的起始点，可定义初始变量供后续节点使用。

**配置字段：**
- `initial_variables`: 初始变量字典（可选）

**JSON 示例：**
```json
{
  "type": "entry",
  "data": {
    "type": "entry",
    "initial_variables": {
      "api_key": "sk-xxx",
      "environment": "production"
    }
  }
}
```

### 2. Http 节点

发送 HTTP 请求的节点。

**配置字段：**
- `url`: 请求 URL（必需）
- `method`: HTTP 方法（get/post/put/delete/patch），默认 `get`
- `headers`: 请求头字典（可选）
- `body`: 请求体（可选，仅限 POST/PUT/PATCH）

**JSON 示例：**
```json
{
  "type": "http:request",
  "data": {
    "type": "http:request",
    "url": "https://api.example.com/users",
    "method": "get",
    "headers": {
      "Authorization": "Bearer {{api_key}}"
    }
  }
}
```

### 3. Conditional 节点

条件分支节点，根据条件判断执行不同分支。

**配置字段：**

**简单条件模式：**
- `condition_type`: 必须为 `"simple"`
- `field_path`: 字段路径（如 `http_response.status`）
- `operator`: 操作符（equal/not_equal/greater/less/contains/exists）
- `expected_value`: 期望值

**高级条件模式：**
- `condition_type`: 必须为 `"advanced"`
- `advanced_expression`: 条件表达式（支持逻辑运算符）

**JSON 示例（简单条件）：**
```json
{
  "type": "conditional",
  "data": {
    "type": "conditional",
    "condition_type": "simple",
    "field_path": "http_response.status",
    "operator": "equal",
    "expected_value": 200
  }
}
```

**JSON 示例（高级条件）：**
```json
{
  "type": "conditional",
  "data": {
    "type": "conditional",
    "condition_type": "advanced",
    "advanced_expression": "http_response.status >= 200 && http_response.status < 300"
  }
}
```

### 4. End 节点

流程的结束点，目前无额外配置。

**JSON 示例：**
```json
{
  "type": "end",
  "data": {
    "type": "end"
  }
}
```

## 使用方式

### CLI 模式执行

```bash
# 执行流程文件
cli --execute --flow <path-to-flow.json>
```

### 测试中使用

```rust
let flow: Flow = load_flow("tests/fixtures/flows/examples/10_complete_example.json").unwrap();
let executor = FlowExecutor::new();
let result = executor.execute_flow(&flow, None);
```

## 变量引用规则

节点间数据传递使用变量引用语法：

- `{{xxx}}` - 引用初始变量（变量已拍平为顶级 key，无需前缀）
- `{{http_response.data.xxx}}` - 引用 HTTP 响应数据
- `{{node_id.output.xxx}}` - 引用指定节点的输出

## 条件操作符

| 操作符 | 说明 | 适用类型 |
|--------|------|----------|
| equal | 等于 | 所有类型 |
| not_equal | 不等于 | 所有类型 |
| greater | 大于 | 数字 |
| greater_or_equal | 大于等于 | 数字 |
| less | 小于 | 数字 |
| less_or_equal | 小于等于 | 数字 |
| contains | 包含 | 字符串、数组 |
| starts_with | 以...开始 | 字符串 |
| ends_with | 以...结束 | 字符串 |
| exists | 存在（非空/非 null） | 所有类型 |
