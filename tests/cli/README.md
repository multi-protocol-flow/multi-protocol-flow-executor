# CLI 测试数据

本目录包含 CLI 工具的测试数据文件，用于验证多协议流程执行器的功能。

## Schema 说明

### 节点类型

| 类型 | 说明 | data 配置 |
|------|------|----------|
| `entry` | 入口节点 | 空对象 `{}` |
| `http` | HTTP 请求节点 | url, method, headers, body |
| `conditional` | 条件分支节点 | condition_type, field_path, operator, expected_value / advanced_expression |

### 流程结构

流程文件使用 **FlowFile 包装格式**：

```json
{
  "_version": 1,
  "flow": {
    "uuid": "flow-identifier",
    "name": "流程名称",
    "initial_variables": { ... },
    "nodes": [...],
    "connections": [...]
  },
  "_meta": null
}
```

### 节点结构

```json
{
  "uuid": "node-001",
  "type": "entry|http|conditional",
  "name": "节点名称",
  "position": { "x": 0.0, "y": 0.0 },
  "data": { ... },
  "ports": null
}
```

### 连接结构

```json
{
  "id": "conn-001",
  "source_node_uuid": "node-001",
  "target_node_uuid": "node-002",
  "source_port_id": "output|true|false",
  "target_port_id": "input"
}
```

### 条件操作符

| 操作符 | 说明 | 适用类型 |
|--------|------|----------|
| `equal` | 等于 | 所有类型 |
| `not_equal` | 不等于 | 所有类型 |
| `greater` | 大于 | 数字 |
| `greater_or_equal` | 大于等于 | 数字 |
| `less` | 小于 | 数字 |
| `less_or_equal` | 小于等于 | 数字 |
| `contains` | 包含 | 字符串、数组 |
| `starts_with` | 以...开始 | 字符串 |
| `ends_with` | 以...结束 | 字符串 |
| `exists` | 存在 | 所有类型 |

### 变量引用

- `{{initial_variables.xxx}}` - 引用流程初始变量
- `{{http_response.data.xxx}}` - 引用HTTP响应数据
- `{{http_response.status}}` - 引用HTTP状态码

## 正常场景测试文件

| 文件 | 说明 | 覆盖功能 |
|------|------|----------|
| `01_simple_entry_http.json` | 简单流程 | Entry → HTTP GET 基础链路 |
| `02_http_all_methods.json` | HTTP方法 | GET/POST/PUT/DELETE/PATCH |
| `03_http_with_headers_body.json` | HTTP配置 | Headers、嵌套JSON body |
| `04_conditional_simple.json` | 简单条件 | Entry → HTTP → Conditional |
| `05_conditional_all_operators.json` | 所有操作符 | 10种条件操作符 |
| `06_conditional_advanced.json` | 高级条件 | advanced_expression |
| `07_variable_reference.json` | 变量引用 | initial_variables 在 URL/Headers 中 |
| `08_multi_http_chain.json` | HTTP链 | 3个HTTP请求链式调用 |
| `09_full_workflow.json` | 完整流程 | Entry → HTTP → Conditional → HTTP (分支) |

## 边界情况测试文件

| 文件 | 说明 | 预期行为 |
|------|------|----------|
| `01_empty_flow.json` | 空流程 | 无节点执行 |
| `02_no_entry_node.json` | 无Entry节点 | 验证器报错 |
| `03_missing_required_fields.json` | 缺少必需字段 | 反序列化失败 |
| `04_invalid_json_syntax.json` | 无效JSON语法 | JSON解析失败 |
| `05_invalid_node_type.json` | 无效节点类型 | 反序列化失败 |
| `06_orphan_node.json` | 孤立节点 | 节点不被执行 |
| `07_http_missing_url.json` | HTTP缺少URL | 执行失败 |
| `08_http_invalid_method.json` | 无效HTTP方法 | 反序列化失败 |
| `09_http_empty_body_for_post.json` | POST无body | 发送空body |
| `10_conditional_missing_operator.json` | 条件缺少operator | 使用默认值 |
| `11_conditional_empty_expression.json` | 空条件表达式 | 求值失败 |
| `12_connection_invalid_reference.json` | 无效连接引用 | 找不到节点 |

## 使用方法

### 编译 CLI 工具

```bash
cd src-tauri
cargo build --release
```

### 执行测试文件

```bash
./target/release/mpe run -f ../tests/cli/01_simple_entry_http.json
```

### 验证 JSON 语法

```bash
for f in tests/cli/*.json tests/cli/edge_cases/*.json; do
  jq . "$f" > /dev/null && echo "✓ $f" || echo "✗ $f"
done
```

## 注意事项

1. **Schema 版本**: 使用 `entry` 类型，而非旧的 `start`/`end`
2. **网络依赖**: 部分测试使用 httpbin.org
3. **边界情况**: edge_cases/ 中的文件设计为触发错误
