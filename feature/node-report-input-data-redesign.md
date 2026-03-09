# 节点报告 input_data 语义重构方案

**创建时间**: 2026-03-09
**状态**: 待评审
**讨论记录**: 基于 2026-03-09 用户讨论整理

## 一、背景与问题

### 1.1 当前实现

在 `executor.rs` 的 `execute_flow` 方法中，`input_data` 被设置为**执行前的所有变量快照**：

```rust
// executor.rs 第 433-471 行
let variables_before = context.variables.get_merged_variables();
let input_data_snapshot = variables_before.clone();
// ...
input_data: Some(json!(input_data_snapshot)),
```

### 1.2 用户困惑

用户认为不同节点的 `input_data` 应该不同：

| 节点类型 | 用户期望的 input_data | 当前实现 |
|---------|---------------------|---------|
| **Entry** | `null`（无输入） | 变量快照 |
| **HTTP** | 请求配置 `{ url, method, headers, body }` | 变量快照 |
| **WS Connect** | 连接配置 `{ url, headers, ... }` | 变量快照 |
| **WS SendWait** | 发送配置 `{ connection_id, message, ... }` | 变量快照 |
| **WS Close** | 关闭配置 `{ connection_id, close_code, ... }` | 变量快照 |
| **Conditional** | 条件配置 `{ condition }` | 变量快照 |

### 1.3 语义不清晰

- **input_data** 应该代表"节点**消费**的输入数据"，而不是"执行时的变量状态"
- **output_data** 代表"节点**产出**的输出数据"（当前实现正确）
- **variable_changes** 代表"节点修改变量的记录"（当前实现正确）

## 二、新语义定义

### 2.1 字段语义

| 字段 | 语义 | 描述 |
|------|------|------|
| **input_data** | 节点消费的输入 | 节点执行所需的配置信息（请求配置、连接配置等） |
| **output_data** | 节点产出的输出 | 节点执行产生的结果（响应数据、执行结果等） |
| **variable_changes** | 变量变更记录 | 该节点执行期间修改的变量（新增或更新） |

### 2.2 各节点类型的 input_data 定义

| 节点类型 | input_data | 说明 |
|---------|------------|------|
| **Entry** | `null` | 入口节点无上游输入，只设置初始变量 |
| **HTTP** | `{ url, method, headers, body, body_type, timeout_ms }` | HTTP 请求配置（变量替换后的实际值） |
| **Conditional** | `{ condition }` | 条件表达式 |
| **WsConnect** | `{ url, headers, subprotocols, connect_timeout_ms, wait_for_message, wait_mode, message_timeout_ms }` | WebSocket 连接配置 |
| **WsSendWait** | `{ connection_id, message, message_type, send_timeout_ms, wait_for_response, wait_mode, message_timeout_ms }` | WebSocket 发送配置 |
| **WsClose** | `{ connection_id, close_code, close_reason, wait_close_timeout_ms }` | WebSocket 关闭配置 |

### 2.3 示例数据

**HTTP 节点报告**:
```json
{
  "node_uuid": "http-1",
  "node_type": "http",
  "node_name": "获取用户信息",
  "status": "success",
  "input_data": {
    "url": "https://api.example.com/users/123",
    "method": "GET",
    "headers": {
      "Authorization": "Bearer xxx",
      "Content-Type": "application/json"
    },
    "body": null,
    "body_type": "none",
    "timeout_ms": 30000
  },
  "output_data": {
    "success": true,
    "status": 200,
    "headers": { ... },
    "body": { "id": 123, "name": "张三" },
    "timing": { "dns_ms": 10, "connect_ms": 50, "ttfb_ms": 100, "download_ms": 20 },
    "http_version": "HTTP/1.1"
  },
  "variable_changes": [
    { "key": "user_name", "old_value": null, "new_value": "张三" }
  ]
}
```

**Entry 节点报告**:
```json
{
  "node_uuid": "entry-1",
  "node_type": "entry",
  "node_name": "入口",
  "status": "success",
  "input_data": null,
  "output_data": null,
  "variable_changes": [
    { "key": "api_base", "old_value": null, "new_value": "https://api.example.com" },
    { "key": "token", "old_value": null, "new_value": "xxx" }
  ]
}
```

## 三、实现方案

### 3.1 方案选择

#### 方案 A：修改 NodeExecutor trait（复杂，风险高）

**修改内容**:
1. 修改 `NodeExecutor` trait 的返回类型
2. 修改所有 6 个执行器实现
3. 修改所有相关测试

**优点**: 语义清晰，每个执行器负责构建自己的 input_data
**缺点**: 修改范围大，风险高，测试工作量大

#### 方案 B：在 executor.rs 中统一构建（推荐）

**修改内容**:
1. 只修改 `executor.rs` 中的 `execute_flow` 方法
2. 添加辅助函数 `build_input_data_from_node_config`
3. 根据节点类型从 `node.data` 中提取配置信息

**优点**: 修改范围小，风险低，不影响现有代码
**缺点**: input_data 构建逻辑集中在 executor.rs 中

### 3.2 推荐方案：方案 B

#### 3.2.1 新增辅助函数

在 `executor.rs` 中添加：

```rust
/// 根据节点类型构建 input_data
///
/// input_data 代表节点消费的输入数据（配置信息）
/// - Entry: None（无输入）
/// - HTTP: { url, method, headers, body, body_type, timeout_ms }
/// - Conditional: { condition }
/// - WS Connect: { url, headers, subprotocols, ... }
/// - WS SendWait: { connection_id, message, message_type, ... }
/// - WS Close: { connection_id, close_code, close_reason, ... }
fn build_input_data_from_node_config(node: &Node) -> Option<serde_json::Value> {
    match &node.data {
        NodeData::Entry(_) => None,
        
        NodeData::Http(config) => {
            Some(serde_json::json!({
                "url": config.url,
                "method": config.method,
                "headers": config.headers,
                "body": config.body,
                "body_type": config.body_type,
                "timeout_ms": config.timeout_ms,
            }))
        },
        
        NodeData::Conditional(config) => {
            Some(serde_json::json!({
                "condition": config.condition,
            }))
        },
        
        NodeData::WsConnect(config) => {
            Some(serde_json::json!({
                "url": config.url,
                "headers": config.headers,
                "subprotocols": config.subprotocols,
                "connect_timeout_ms": config.connect_timeout_ms,
                "wait_for_message": config.wait_for_message,
                "wait_mode": config.wait_mode,
                "message_timeout_ms": config.message_timeout_ms,
            }))
        },
        
        NodeData::WsSendWait(config) => {
            Some(serde_json::json!({
                "connection_id": config.connection_id,
                "message": config.message,
                "message_type": config.message_type,
                "send_timeout_ms": config.send_timeout_ms,
                "wait_for_response": config.wait_for_response,
                "wait_mode": config.wait_mode,
                "message_timeout_ms": config.message_timeout_ms,
            }))
        },
        
        NodeData::WsClose(config) => {
            Some(serde_json::json!({
                "connection_id": config.connection_id,
                "close_code": config.close_code,
                "close_reason": config.close_reason,
                "wait_close_timeout_ms": config.wait_close_timeout_ms,
            }))
        },
        
        // 未知节点类型返回 null
        _ => None,
    }
}
```

#### 3.2.2 修改 execute_flow 方法

在 `executor.rs` 的 `execute_flow` 方法中，创建 `NodeExecutionReport` 时：

```rust
// 修改前（第 433-471 行）
let variables_before = context.variables.get_merged_variables();
let input_data_snapshot = variables_before.clone();
// ...
let report = NodeExecutionReport {
    // ...
    input_data: Some(json!(input_data_snapshot)),
    // ...
};

// 修改后
let input_data = build_input_data_from_node_config(node);
// ...
let report = NodeExecutionReport {
    // ...
    input_data,
    // ...
};
```

### 3.3 前端修改

#### 3.3.1 添加变量变更卡片组件

创建新组件 `VariableChangesCard.tsx`：

```tsx
interface VariableChangesCardProps {
  changes: VariableChange[] | null;
}

export function VariableChangesCard({ changes }: VariableChangesCardProps) {
  if (!changes || changes.length === 0) {
    return null;
  }

  return (
    <div className="variable-changes-card">
      <h4>变量变更</h4>
      <div className="changes-list">
        {changes.map((change, index) => (
          <div key={index} className="change-item">
            <span className="key">{change.key}</span>
            <span className="arrow">→</span>
            <span className="new-value">{JSON.stringify(change.new_value)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
```

#### 3.3.2 调整节点报告展示布局

修改节点报告展示组件，增加"变量变更"卡片：

```
┌─────────────────────────────────────┐
│ 节点报告: HTTP 请求                   │
├─────────────────────────────────────┤
│ 状态: ✅ 成功  耗时: 180ms            │
├─────────────────────────────────────┤
│ 📥 输入数据                          │
│ ┌─────────────────────────────────┐ │
│ │ URL: https://api.example.com/... │ │
│ │ Method: GET                     │ │
│ │ Headers: {...}                  │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 📤 输出数据                          │
│ ┌─────────────────────────────────┐ │
│ │ Status: 200                     │ │
│ │ Body: {...}                     │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 🔄 变量变更                          │  ← 新增卡片
│ ┌─────────────────────────────────┐ │
│ │ user_id → 12345                 │ │
│ │ user_name → "张三"               │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 📄 原始 JSON                         │
│ [折叠/展开]                          │
└─────────────────────────────────────┘
```

## 四、实施步骤

### 4.1 后端修改

| 步骤 | 任务 | 文件 | 行数估计 |
|------|------|------|----------|
| 1 | 添加 `build_input_data_from_node_config` 函数 | `executor.rs` | +50 行 |
| 2 | 修改 `execute_flow` 中的 `input_data` 构建 | `executor.rs` | ~5 行 |
| 3 | 同步 TypeScript 类型定义 | `report.ts` | 0（无变化） |

### 4.2 前端修改

| 步骤 | 任务 | 文件 | 行数估计 |
|------|------|------|----------|
| 1 | 创建 `VariableChangesCard` 组件 | 新文件 | +80 行 |
| 2 | 修改节点报告展示布局 | 报告组件 | +30 行 |

### 4.3 测试验证

| 步骤 | 任务 | 命令 |
|------|------|------|
| 1 | Rust 单元测试 | `cd src-tauri && cargo test` |
| 2 | Rust Lint 检查 | `cd src-tauri && cargo clippy` |
| 3 | 前端类型检查 | `npm run typecheck` |
| 4 | 前端单元测试 | `npm run test` |
| 5 | 集成测试 | 手动运行流程验证报告 |

## 五、风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| input_data 语义变化导致前端展示异常 | 中 | 保留原始 JSON 展示，新旧数据都能展示 |
| 遗漏某些节点类型的 input_data 构建 | 低 | 默认返回 null，不影响功能 |
| variable_changes 为空时的 UI 展示 | 低 | 组件判断为空时不渲染 |

## 六、后续优化

1. **input_data 变量替换**: 当前方案展示的是配置中的原始值（包含 `{{variable}}`），后续可以考虑展示替换后的实际值
2. **敏感信息脱敏**: headers 中的 Authorization 等敏感信息可以考虑脱敏展示
3. **大型数据截断**: body 等大型数据可以考虑截断展示，提供展开查看完整内容的功能

## 七、决策记录

| 日期 | 决策 | 原因 |
|------|------|------|
| 2026-03-09 | 采用方案 B（在 executor.rs 中统一构建） | 修改范围小，风险低 |
| 2026-03-09 | input_data 展示配置原始值 | 避免增加变量替换逻辑的复杂度 |
| 2026-03-09 | variable_changes 单独卡片展示 | 提高可读性，突出变量变更 |
