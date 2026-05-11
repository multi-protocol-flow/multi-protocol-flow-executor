# CLI 使用文档

## 概述

多协议流程执行器的命令行工具。支持 CLI 模式和 GUI 桌面模式：

- **无参数启动** → 打开 Tauri 桌面 GUI
- **有参数启动** → CLI 模式（执行流程、验证、调试、压力测试、流程管理、报告管理）

`mpe.exe` 是统一的单一二进制文件，同时支持两种运行模式。

## 安装

### 开发版本

```bash
cd src-tauri
cargo build
```

编译产物位置：
- Windows: `src-tauri\target\debug\mpe.exe`
- Linux/macOS: `src-tauri/target/debug/mpe`

### 生产版本

```bash
cd src-tauri
cargo build --release
```

编译产物位置：
- Windows: `src-tauri\target\release\mpe.exe`
- Linux/macOS: `src-tauri/target/release/mpe`

### GUI 桌面应用

```bash
npx tauri dev      # 开发模式（前端热重载）
npx tauri build    # 构建安装包
```

## 命令语法

### 基本用法

```bash
mpe run -f <流程文件.mpf>       # 执行流程
mpe validate -f <流程文件.mpf>  # 验证流程
mpe debug -f <流程文件.mpf>     # 调试流程
```

### 帮助信息

```bash
mpe --help
mpe run --help     # 查看子命令帮助
```

全部子命令：

| 命令 | 说明 |
|------|------|
| `run` | 执行流程 |
| `validate` | 验证流程 |
| `debug` | 调试流程 (NDJSON 双向协议) |
| `stress` | 压力测试 |
| `flow` | 流程管理（列出/查看/创建/复制/删除/导入/编辑） |
| `report` | 报告管理（列出/查看/删除） |

## 输出格式

### JSON 结构

```json
{
  "success": true,
  "data": null,
  "error": null,
  "execution_time": 156,
  "node_reports": [
    {
      "node_uuid": "entry-001",
      "node_type": "entry",
      "node_name": "入口",
      "start_time": 1707700000000,
      "end_time": 1707700000001,
      "duration_ms": 1,
      "status": "success",
      "input_data": null,
      "output_data": null,
      "used_port_id": "output",
      "error": null
    }
  ],
  "flow_report": {
    "flow_uuid": "flow-001",
    "flow_name": "示例流程",
    "start_time": 1707700000000,
    "end_time": 1707700000156,
    "duration_ms": 156,
    "total_nodes": 3,
    "executed_count": 3,
    "status": "success",
    "node_reports": [...],
    "statistics": { ... },
    "flow_error": null
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `success` | boolean | 流程是否执行成功 |
| `data` | object/null | 输出数据（当前为 null） |
| `error` | string/null | 错误信息（失败时存在） |
| `execution_time` | number | 总执行时间（毫秒） |
| `node_reports` | array | 各节点执行报告 |
| `flow_report` | object | 流程级执行报告 |

### NodeExecutionReport 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `node_uuid` | string | 节点唯一标识 |
| `node_type` | string | 节点类型（entry/http/conditional） |
| `node_name` | string | 节点名称 |
| `start_time` | number | 开始时间（毫秒时间戳） |
| `end_time` | number | 结束时间（毫秒时间戳） |
| `duration_ms` | number | 执行时长（毫秒） |
| `status` | string | 执行状态（success/failed/skipped） |
| `input_data` | object/null | 输入数据 |
| `output_data` | object/null | 输出数据 |
| `used_port_id` | string/null | 实际使用的输出端口 |
| `error` | object/null | 错误详情 |

### used_port_id 说明

| 节点类型 | 可能值 | 说明 |
|----------|--------|------|
| entry | `output` | 固定输出端口 |
| http | `output` | 固定输出端口 |
| conditional | `true` / `false` | 根据条件评估结果 |

## 流程文件格式

### 基本结构

```json
{
  "uuid": "flow-identifier",
  "name": "流程名称",
  "initial_variables": {
    "key": "value"
  },
  "nodes": [...],
  "connections": [...]
}
```

### 节点类型

| 类型 | 说明 | 必需配置 |
|------|------|----------|
| `entry` | 入口节点，流程起点 | 无 |
| `http` | HTTP 请求节点 | url, method |
| `conditional` | 条件分支节点 | condition_type + 条件配置 |

### Entry 节点

```json
{
  "uuid": "entry-001",
  "type": "entry",
  "name": "入口",
  "position": { "x": 0.0, "y": 0.0 },
  "data": { "type": "entry" },
  "ports": null
}
```

### HTTP 节点

```json
{
  "uuid": "http-001",
  "type": "http",
  "name": "获取数据",
  "position": { "x": 100.0, "y": 0.0 },
  "data": {
    "type": "http",
    "url": "https://api.example.com/users",
    "method": "get",
    "headers": {
      "Authorization": "Bearer token123"
    },
    "timeout_ms": 30000
  },
  "ports": null
}
```

**HTTP 方法支持**: `get`, `post`, `put`, `delete`, `patch`

**HTTP 配置字段**:

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `url` | string | 请求 URL（支持变量引用） | - |
| `method` | string | HTTP 方法 | `get` |
| `headers` | object | 请求头 | `{}` |
| `body_type` | string | 请求体类型 | `none` |
| `content_type` | string | Content-Type | `json` |
| `body` | string | 请求体内容（body_type=body 时） | - |
| `form_fields` | array | 表单字段（body_type=form-data/x-www-form-urlencoded 时） | - |
| `file_path` | string | 文件路径（body_type=binary 时） | - |
| `timeout_ms` | number | 超时时间（毫秒），范围 1000-300000 | 30000 |

**body_type 类型**:

| 值 | 说明 | Content-Type |
|------|------|------|
| `none` | 无请求体 | - |
| `body` | 原始请求体 | 根据 content_type 字段设置 |
| `form-data` | multipart/form-data | multipart/form-data |
| `x-www-form-urlencoded` | application/x-www-form-urlencoded | application/x-www-form-urlencoded |
| `binary` | 二进制文件 | application/octet-stream |

**form_fields 字段结构**:

```json
{
  "key": "field_name",
  "value": "field_value",
  "type": "text",
  "file_path": null
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `key` | string | 字段名 |
| `value` | string | 字段值 |
| `type` | string | 字段类型（`text` / `file`） |
| `file_path` | string | 文件路径（type=file 时必需） |

### Conditional 节点（简单条件）

```json
{
  "uuid": "conditional-001",
  "type": "conditional",
  "name": "检查状态",
  "position": { "x": 200.0, "y": 0.0 },
  "data": {
    "type": "conditional",
    "condition_type": "simple",
    "field_path": "http_response.status",
    "operator": "equal",
    "expected_value": 200
  },
  "ports": null
}
```

### Conditional 节点（高级表达式）

```json
{
  "uuid": "conditional-001",
  "type": "conditional",
  "name": "复杂条件",
  "position": { "x": 200.0, "y": 0.0 },
  "data": {
    "type": "conditional",
    "condition_type": "advanced",
    "advanced_expression": "status >= 200 && status < 300"
  },
  "ports": null
}
```

### 条件操作符

| 操作符 | 说明 | 示例 |
|--------|------|------|
| `equal` | 等于 | `status == 200` |
| `not_equal` | 不等于 | `status != 404` |
| `greater` | 大于 | `count > 0` |
| `greater_or_equal` | 大于等于 | `score >= 60` |
| `less` | 小于 | `price < 100` |
| `less_or_equal` | 小于等于 | `age <= 18` |
| `contains` | 包含 | `message contains "error"` |
| `starts_with` | 以...开始 | `name starts_with "prefix"` |
| `ends_with` | 以...结束 | `email ends_with "@gmail.com"` |
| `exists` | 存在 | `field exists` |

### 连接结构

```json
{
  "id": "conn-001",
  "source_node_uuid": "entry-001",
  "target_node_uuid": "http-001",
  "source_port_id": "output",
  "target_port_id": "input"
}
```

**source_port_id 值**:
- Entry/HTTP 节点: `output`
- Conditional 节点: `true` 或 `false`

### 变量引用

使用双花括号语法引用变量：

| 语法 | 说明 |
|------|------|
| `{{initial_variables.xxx}}` | 引用流程初始变量 |
| `{{status}}` | 引用上一个 HTTP 响应的状态码 |
| `{{body.xxx}}` | 引用上一个 HTTP 响应的 JSON 数据 |
| `{{headers.xxx}}` | 引用上一个 HTTP 响应的响应头 |

**示例**：
```json
{
  "url": "{{initial_variables.api_base}}/users",
  "headers": {
    "Authorization": "Bearer {{initial_variables.token}}"
  }
}
```

## 测试用例

### 正常场景测试

| 文件 | 说明 | 路径 |
|------|------|------|
| 01_simple_entry_http.json | Entry → HTTP 基础链路 | tests/cli/ |
| 02_http_all_methods.json | GET/POST/PUT/DELETE/PATCH | tests/cli/ |
| 03_http_with_headers_body.json | Headers 和 JSON Body | tests/cli/ |
| 04_conditional_simple.json | 简单条件判断 | tests/cli/ |
| 05_conditional_all_operators.json | 10 种条件操作符 | tests/cli/ |
| 06_conditional_advanced.json | 高级表达式条件 | tests/cli/ |
| 07_variable_reference.json | 变量引用示例 | tests/cli/ |
| 08_multi_http_chain.json | 多 HTTP 请求链 | tests/cli/ |
| 09_full_workflow.json | 完整分支流程 | tests/cli/ |

### 边界情况测试

| 文件 | 说明 | 预期行为 |
|------|------|----------|
| edge_cases/01_empty_flow.json | 空流程 | 无节点执行 |
| edge_cases/02_no_entry_node.json | 无 Entry 节点 | 验证失败 |
| edge_cases/03_missing_required_fields.json | 缺少必需字段 | 解析失败 |
| edge_cases/04_invalid_json_syntax.json | 无效 JSON | 解析失败 |
| edge_cases/05_invalid_node_type.json | 无效节点类型 | 解析失败 |
| edge_cases/06_orphan_node.json | 孤立节点 | 节点跳过 |
| edge_cases/07_http_missing_url.json | HTTP 缺少 URL | 执行失败 |
| edge_cases/08_http_invalid_method.json | 无效 HTTP 方法 | 解析失败 |
| edge_cases/09_http_empty_body_for_post.json | POST 无 body | 发送空 body |
| edge_cases/10_conditional_missing_operator.json | 缺少操作符 | 使用默认值 |
| edge_cases/11_conditional_empty_expression.json | 空表达式 | 求值失败 |
| edge_cases/12_connection_invalid_reference.json | 无效连接引用 | 找不到节点 |

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 执行成功 |
| 1 | 执行失败（错误信息输出到 stderr） |

## 使用示例

### 执行简单流程

```bash
mpe run -f tests/cli/01_simple_entry_http.json
```

输出：
```json
{
  "success": true,
  "data": null,
  "error": null,
  "execution_time": 234,
  "node_reports": [
    {
      "node_uuid": "entry-001",
      "node_type": "entry",
      "node_name": "入口",
      "start_time": 1707700000000,
      "end_time": 1707700000000,
      "duration_ms": 0,
      "status": "success",
      "input_data": null,
      "output_data": null,
      "used_port_id": "output",
      "error": null
    },
    {
      "node_uuid": "http-001",
      "node_type": "http",
      "node_name": "HTTP GET 请求",
      "start_time": 1707700000001,
      "end_time": 1707700000234,
      "duration_ms": 233,
      "status": "success",
      "input_data": null,
      "output_data": {
        "status": 200,
        "headers": {...},
        "data": {...}
      },
      "used_port_id": "output",
      "error": null
    }
  ],
  "flow_report": {...}
}
```

### 执行带条件分支的流程

```bash
mpe run -f tests/cli/09_full_workflow.json
```

### 执行失败示例

```bash
mpe run -f tests/cli/edge_cases/02_no_entry_node.json
```

输出（stdout）：
```json
{
  "success": false,
  "data": null,
  "error": "流程缺少 Entry 节点",
  "execution_time": 0,
  "node_reports": [],
  "flow_report": null
}
```

输出（stderr）：
```
错误: 流程缺少 Entry 节点
```

## 注意事项

### 路径处理
- 支持相对路径和绝对路径
- 相对路径基于当前工作目录

### 错误处理
- 成功：结果输出到 stdout，退出码 0
- 失败：错误信息同时输出到 stdout（JSON）和 stderr（纯文本），退出码 1

### JSON 格式要求
- 节点类型使用小写：`entry`、`http`、`conditional`
- UUID 格式不限制，建议使用 `{type}-{uuid}` 模式
- 连接的 `source_port_id` 必须与节点输出端口匹配

### 变量模板限制
- 仅支持点号路径访问：`{{obj.field.subfield}}`
- **不支持**数组索引：`{{items[0]}}` ❌
- **不支持**函数调用：`{{func()}}` ❌
- **不支持**复杂对象直接替换

### 网络依赖
- HTTP 节点需要网络连接
- 测试用例使用 httpbin.org 作为测试服务器
