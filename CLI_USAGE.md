# CLI 使用文档

## 安装

### 开发版本

```bash
cd src-tauri
cargo build --bin cli
```

编译完成后，CLI 二进制文件位于：
- Windows: `src-tauri/target/debug/cli.exe`
- Linux/macOS: `src-tauri/target/debug/cli`

### 生产版本

```bash
cd src-tauri
cargo build --release --bin cli
```

编译完成后，CLI 二进制文件位于：
- Windows: `src-tauri/target/release/cli.exe`
- Linux/macOS: `src-tauri/target/release/cli`

## 命令语法

### 基本用法

```bash
cli --execute --flow <路径>
```

或使用短格式：

```bash
cli -e -f <路径>
```

### 命令参数

| 参数 | 长格式 | 短格式 | 必需 | 说明 |
|------|---------|---------|------|------|
| 执行 | `--execute` | `-e` | 是 | 执行指定的流程 |
| 流程文件 | `--flow <路径>` | `-f <路径>` | 是 | 流程 JSON 文件路径 |
| 帮助 | `--help` | `-h` | 否 | 显示帮助信息 |

### 帮助信息

```bash
cli --help
```

或：

```bash
cli -h
```

输出：

```
多协议流程执行器 - CLI 工具

用法:
  cli --execute --flow <path>  执行指定路径的流程
  cli -e -f <path>             执行指定路径的流程（短格式）
  cli --help                   显示帮助信息
  cli -h                       显示帮助信息（短格式）

参数:
  -e, --execute                执行流程
  -f, --flow <path>            流程文件路径
  -h, --help                   显示帮助信息

示例:
  cli --execute --flow my-flow.json
  cli -e -f /path/to/flow.json
```

## 输出格式

### JSON 结构

CLI 输出 JSON 格式的执行结果，结构如下：

```json
{
  "success": true/false,
  "data": {
    "flow_id": "流程 ID",
    "executed_nodes": ["已执行的节点 ID 列表"],
    "execution_time": 执行时间（毫秒）
  },
  "error": "错误信息（如有）",
  "execution_time": 总执行时间（毫秒）
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| success | boolean | 执行是否成功 |
| data | object/null | 执行结果数据（成功时存在） |
| data.flow_id | string | 流程 ID |
| data.executed_nodes | array | 已执行的节点 ID 列表 |
| data.execution_time | number | 执行时间（毫秒） |
| error | string/null | 错误信息（失败时存在） |
| execution_time | number | 总执行时间（毫秒） |

## 测试文件

项目包含 5 个测试文件，用于验证 CLI 的各种场景：

### 01_valid_start_with_connection.json

**描述**: 有效的流程 - 开始节点连接到结束节点

**预期结果**:
- 退出码: 0
- success: true
- executed_nodes: 1（仅开始节点）
- execution_time: < 10ms

**用途**: 验证基本流程执行功能

**示例**:
```bash
cli --execute --flow tests/fixtures/flows/01_valid_start_with_connection.json
```

### 02_no_start_node.json

**描述**: 无效流程 - 缺少开始节点

**预期结果**:
- 退出码: 1
- success: false
- error: "流程缺少开始节点"

**用途**: 验证错误处理 - 缺少必需节点

**示例**:
```bash
cli --execute --flow tests/fixtures/flows/02_no_start_node.json
```

### 03_no_connection.json

**描述**: 无效流程 - 开始节点无连接

**预期结果**:
- 退出码: 1
- success: false
- error: "开始节点没有输出连接"

**用途**: 验证错误处理 - 流程未连接

**示例**:
```bash
cli --execute --flow tests/fixtures/flows/03_no_connection.json
```

### 04_loop_detection.json

**描述**: 无效流程 - 循环检测（自连接）

**预期结果**:
- 退出码: 1
- success: false
- error: "检测到循环：节点 ... 已被访问"

**用途**: 验证循环检测功能

**示例**:
```bash
cli --execute --flow tests/fixtures/flows/04_loop_detection.json
```

### 05_invalid_json.json

**描述**: 无效的 JSON 格式

**预期结果**:
- 退出码: 1
- 错误: JSON 解析错误

**用途**: 验证 JSON 格式验证

**示例**:
```bash
cli --execute --flow tests/fixtures/flows/05_invalid_json.json
```

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 执行成功 |
| 1 | 执行失败（错误信息输出到 stderr） |

## 使用示例

### 执行有效流程

```bash
cli --execute --flow tests/fixtures/flows/01_valid_start_with_connection.json
```

输出：
```json
{
  "success": true,
  "data": {
    "flow_id": "550e8400-e29b-41d4-a716-446655440000",
    "executed_nodes": ["start-550e8400-e29b-41d4-a716-446655440000"],
    "execution_time": 0
  },
  "error": null,
  "execution_time": 1
}
```

### 执行无效流程

```bash
cli --execute --flow tests/fixtures/flows/02_no_start_node.json
```

输出（stdout）：
```json
{
  "success": false,
  "data": null,
  "error": "流程缺少开始节点",
  "execution_time": 0
}
```

错误输出（stderr）：
```
错误: 流程缺少开始节点
```

### 使用绝对路径

```bash
cli --execute --flow "D:\projects\multi-protocol-flow-executor\tests\fixtures\flows\01_valid_start_with_connection.json"
```

### 使用短格式参数

```bash
cli -e -f tests/fixtures/flows/01_valid_start_with_connection.json
```

## 注意事项

1. **路径处理**:
   - 使用绝对路径以避免路径解析问题
   - 相对路径会被解析到用户数据目录的 flows 子目录

2. **错误处理**:
   - 错误信息输出到 stderr
   - JSON 结果输出到 stdout
   - 失败时退出码为非零值

3. **JSON 格式**:
   - 节点类型使用小写序列化格式（"start", "end" 等）
   - Flow ID 必须是 UUID 格式

4. **执行限制**:
   - 当前仅支持开始节点的执行
   - 其他节点类型（HTTP、条件分支等）尚未实现
