# 报告系统增强需求

**创建时间**: 2026-03-08
**状态**: 待实现
**讨论记录**: 本文档基于 2026-03-08 的讨论整理

## 一、功能概述

增强执行报告系统的数据详细程度和展示体验，包括：
1. 修复现有 bug（input_data 时序问题）
2. 增加节点级详细数据（配置快照、变量追踪、耗时分解）
3. 增加流程级追踪数据（执行路径、跳过原因）
4. 优化前端展示（结构化展示 + 原始 JSON 切换）

## 二、数据结构变更

### 2.1 NodeExecutionReport 新增字段

```rust
pub struct NodeExecutionReport {
    // === 现有字段 ===
    pub node_uuid: String,
    pub node_type: String,
    pub node_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
    pub status: ExecutionStatus,
    pub input_data: Option<Value>,      // 修复：执行前的变量快照
    pub output_data: Option<Value>,
    pub used_port_id: Option<String>,
    pub error: Option<ErrorDetail>,
    
    // === 新增字段 ===
    /// 节点配置（变量解析后的实际值）
    /// HTTP: { url, method, headers, body, body_type }
    /// WS Connect: { url, protocols, ... }
    /// Conditional: { field_path, operator, expected_value, ... }
    pub resolved_config: Option<Value>,
    
    /// 当前节点修改变量的记录
    pub variable_changes: Option<Vec<VariableChange>>,
    
    /// 跳过原因（如依赖节点失败、条件分支未命中）
    pub skip_reason: Option<String>,
    
    /// 等待依赖节点的耗时
    pub wait_duration_ms: Option<u64>,
}

/// 变量变更记录
pub struct VariableChange {
    pub key: String,
    pub old_value: Option<Value>,
    pub new_value: Value,
}
```

### 2.2 FlowExecutionReport 新增字段

```rust
pub struct FlowExecutionReport {
    // === 现有字段 ===
    pub uuid: String,
    pub flow_uuid: String,
    pub flow_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
    pub total_nodes: u32,
    pub executed_count: u32,
    pub status: ExecutionStatus,
    pub node_reports: Vec<NodeExecutionReport>,
    pub statistics: FlowStatistics,
    pub flow_error: Option<ErrorDetail>,
    pub execution_mode: ExecutionMode,
    
    // === 新增字段 ===
    /// 流程初始变量
    pub initial_variables: Option<Value>,
    
    /// 流程最终变量
    pub final_variables: Option<Value>,
    
    /// 执行路径（节点 UUID 列表，按执行顺序）
    pub execution_path: Vec<String>,
    
    /// 跳过的节点及原因
    pub skipped_nodes: Vec<SkippedNodeInfo>,
}

/// 跳过节点信息
pub struct SkippedNodeInfo {
    pub node_uuid: String,
    pub node_name: String,
    pub reason: String,
}
```

### 2.3 Bug 修复：input_data 时序

**问题**：当前 `input_data` 记录的是节点执行**后**的变量状态，已混入当前节点的输出。

**修复**：在变量合并前快照 `input_data`。

```rust
// executor.rs 修复逻辑
Ok((next_node_id_from_executor, output_data)) => {
    // 1. 先快照执行前的变量状态
    let input_data_snapshot = context.variables.get_merged_variables();
    
    // 2. 再合并输出数据到变量池
    if let Some(data) = &output_data {
        if let Some(obj) = data.as_object() {
            for (key, value) in obj {
                context.variables.set_variable(key.to_string(), value.clone(), node.uuid.clone());
            }
        }
    }
    
    // 3. 创建报告，使用快照
    let node_report = NodeExecutionReport {
        input_data: Some(json!(input_data_snapshot)),
        output_data: output_data.clone(),
        ...
    };
}
```

### 2.4 调试模式兼容

**ExecutionStackEntry 同步新增字段**：

```rust
pub struct ExecutionStackEntry {
    // 现有字段...
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub success: bool,
    pub error: Option<String>,
    pub used_port_id: Option<String>,
    
    // 新增字段（与 NodeExecutionReport 对齐）
    pub resolved_config: Option<Value>,
    pub variable_changes: Option<Vec<VariableChange>>,
    pub skip_reason: Option<String>,
}
```

**需要修改的位置**：
- `debug/types.rs` - ExecutionStackEntry 结构定义
- `debug/executor.rs:447-459` - push_stack_entry() 填充新字段
- `debug/executor.rs:537-543` - update_last_stack_entry() 更新新字段
- `debug/executor.rs:1004-1092` - generate_debug_report() 转换逻辑

## 三、节点执行器增强

### 3.1 HTTP 节点

**新增输出字段**：

```json
{
  "success": true,
  "status": 200,
  "headers": { ... },
  "body": { ... },
  "request": {
    "url": "https://api.example.com/users",
    "method": "GET",
    "headers": { ... },
    "body": null
  },
  "timing": {
    "dns_ms": 15,
    "connect_ms": 32,
    "tls_ms": 28,
    "ttfb_ms": 120,
    "download_ms": 45
  }
}
```

**实现方案**：手动计时

```rust
// 使用 tokio::time::Instant 在各阶段手动计时
// 1. DNS 解析：tokio::net::lookup_host()
// 2. TCP 连接：tokio::net::TcpStream::connect()
// 3. TLS 握手：tokio_rustls::TlsConnector::connect()
// 4. TTFB：发送请求到收到响应头
// 5. Download：下载响应体
```

**注意**：需要重构 HTTP 执行器，不能直接使用 `reqwest::Client::execute()`

### 3.2 WebSocket Connect 节点

**当前输出**：
```json
{
  "connected": true,
  "connection_id": "node-uuid-xxx",
  "wait_timed_out": false,
  "duration_ms": 1234,
  "message": { "data": "...", "type": "text" }
}
```

**新增字段**：
```json
{
  "connected": true,
  "connection_id": "node-uuid-xxx",
  "url": "wss://example.com/ws",           // 新增
  "connect_duration_ms": 150,              // 新增
  "wait_message_duration_ms": 200,         // 新增
  "wait_timed_out": false,
  "duration_ms": 1234,
  "message": { ... }
}
```

### 3.3 WebSocket Send+Wait 节点

**新增字段**：
```json
{
  "connection_id": "node-uuid-xxx",
  "url": "wss://example.com/ws",           // 新增（反查）
  "send_duration_ms": 5,                   // 新增
  "wait_response_duration_ms": 150,        // 新增
  "wait_timed_out": false,
  "sent": { "data": "...", "type": "text" },
  "message": { "data": "...", "type": "text", "parsed": {...} }
}
```

**实现**：通过 `ws_manager.get_connection(&connection_id)` 反查 URL

### 3.4 WebSocket Close 节点

**新增字段**：
```json
{
  "connection_id": "node-uuid-xxx",
  "url": "wss://example.com/ws",           // 新增（反查）
  "close_code": 1000,
  "close_reason": "正常关闭",
  "duration_ms": 50,
  "success": true
}
```

### 3.5 Conditional 节点

**当前输出**：`true` 或 `false`（纯布尔值）

**新增字段**：
```json
{
  "result": true,
  "expression": "{{status}} == 200",
  "evaluated_value": "200 == 200",
  "field_value": 200
}
```

## 四、变量追踪

### 4.1 追踪粒度

**方案**：只记录当前节点修改变量的 old→new

```rust
pub struct VariableChange {
    pub key: String,           // 变量名
    pub old_value: Option<Value>,  // 旧值（新增变量时为 None）
    pub new_value: Value,      // 新值
}
```

### 4.2 实现逻辑

```rust
// executor.rs
Ok((next_node_id_from_executor, output_data)) => {
    let variables_before = context.variables.get_merged_variables();
    
    // 合并输出数据
    let mut variable_changes = Vec::new();
    if let Some(data) = &output_data {
        if let Some(obj) = data.as_object() {
            for (key, new_value) in obj {
                let old_value = variables_before.get(key).cloned();
                variable_changes.push(VariableChange {
                    key: key.clone(),
                    old_value,
                    new_value: new_value.clone(),
                });
                context.variables.set_variable(key.clone(), new_value.clone(), node.uuid.clone());
            }
        }
    }
    
    // 创建报告
    let node_report = NodeExecutionReport {
        variable_changes: if variable_changes.is_empty() { None } else { Some(variable_changes) },
        ...
    };
}
```

## 五、前端展示优化

### 5.1 Tab 切换

每个节点详情提供两个视图：
- **结构化**：按节点类型差异化展示
- **原始 JSON**：完整的 input_data + output_data + resolved_config

### 5.2 组件结构

```
NodeExecutionDetails.tsx
├── TabSwitcher: [结构化] [原始 JSON]
├── StructuredViewer（默认）
│   ├── HttpNodeViewer
│   │   ├── 请求配置（URL、Method、Headers、Body）
│   │   ├── 响应状态（状态码着色：2xx绿、4xx黄、5xx红）
│   │   ├── 响应头
│   │   ├── 响应体（JSON 语法高亮）
│   │   └── 耗时分解（DNS/Connect/TLS/TTFB/Download）
│   ├── WsConnectViewer
│   │   ├── 连接 URL
│   │   ├── 连接状态
│   │   ├── 耗时分解
│   │   └── 首条消息
│   ├── WsSendWaitViewer
│   │   ├── 连接 URL（关联）
│   │   ├── 发送内容
│   │   ├── 响应内容
│   │   └── 耗时分解
│   ├── WsCloseViewer
│   │   ├── 连接 URL
│   │   └── 关闭状态
│   ├── ConditionalViewer
│   │   ├── 条件表达式
│   │   ├── 评估值
│   │   └── 结果
│   └── GenericViewer（兜底，Entry 等）
└── RawJsonViewer
    └── <pre>{JSON.stringify(all_data, null, 2)}</pre>
```

### 5.3 JSON 语法高亮

**方案**：使用轻量级库或自定义实现

**候选**：
- `react-json-view-lite`
- 自定义实现（正则匹配关键字、字符串、数字等）

### 5.4 通用展示元素

- 状态码着色：2xx 绿色、4xx 黄色、5xx 红色
- 折叠/展开长 JSON
- 数据大小指示（如 2.3KB）
- 变量变更对比（old → new 高亮）

## 六、实现优先级

| 优先级 | 模块 | 内容 |
|--------|------|------|
| P0 | Bug 修复 | input_data 时序问题 |
| P0 | 调试兼容 | ExecutionStackEntry 同步修改 |
| P1 | 数据结构 | NodeExecutionReport 新增字段 |
| P1 | 数据结构 | FlowExecutionReport 新增字段 |
| P1 | HTTP | 耗时分解（手动计时方案） |
| P1 | WS Connect | 输出 url、耗时分解 |
| P1 | WS Send+Wait | 输出 url（反查）、耗时分解 |
| P1 | WS Close | 输出 url（反查） |
| P1 | Conditional | 输出 expression、evaluated_value |
| P2 | 前端 | Tab 切换 [结构化/原始JSON] |
| P2 | 前端 | HTTP 节点结构化展示 |
| P2 | 前端 | WS 节点结构化展示 |
| P2 | 前端 | Conditional 节点结构化展示 |
| P2 | 前端 | JSON 语法高亮 |

## 七、文件变更清单

### 7.1 后端（Rust）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src-tauri/crates/flow-engine/src/report.rs` | 修改 | 新增字段定义 |
| `src-tauri/crates/flow-engine/src/executor.rs` | 修改 | 修复 input_data 时序、新增字段填充 |
| `src-tauri/crates/flow-engine/src/http_executor.rs` | 重构 | 耗时分解（手动计时） |
| `src-tauri/crates/flow-engine/src/ws_connect_executor.rs` | 修改 | 输出 url、耗时分解 |
| `src-tauri/crates/flow-engine/src/ws_send_wait_executor.rs` | 修改 | 输出 url（反查）、耗时分解 |
| `src-tauri/crates/flow-engine/src/ws_close_executor.rs` | 修改 | 输出 url（反查） |
| `src-tauri/crates/flow-engine/src/conditional_executor.rs` | 修改 | 输出 expression、evaluated_value |
| `src-tauri/crates/flow-engine/src/debug/types.rs` | 修改 | ExecutionStackEntry 新增字段 |
| `src-tauri/crates/flow-engine/src/debug/executor.rs` | 修改 | push_stack_entry、update_last_stack_entry、generate_debug_report |

### 7.2 前端（TypeScript）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/types/report.ts` | 修改 | 新增字段类型定义 |
| `src/types/debug.ts` | 修改 | ExecutionStackEntry 新增字段 |
| `src/components/execution/NodeExecutionDetails.tsx` | 重构 | Tab 切换、节点类型分发 |
| `src/components/execution/viewers/HttpNodeViewer.tsx` | 新增 | HTTP 节点结构化展示 |
| `src/components/execution/viewers/WsConnectViewer.tsx` | 新增 | WS Connect 节点展示 |
| `src/components/execution/viewers/WsSendWaitViewer.tsx` | 新增 | WS Send+Wait 节点展示 |
| `src/components/execution/viewers/WsCloseViewer.tsx` | 新增 | WS Close 节点展示 |
| `src/components/execution/viewers/ConditionalViewer.tsx` | 新增 | Conditional 节点展示 |
| `src/components/execution/viewers/GenericViewer.tsx` | 新增 | 通用节点展示（兜底） |
| `src/components/execution/viewers/RawJsonViewer.tsx` | 新增 | 原始 JSON 展示 |

## 八、回归测试

**任何功能变更必须执行回归测试：**

```bash
# 前端
npm run typecheck && npm run test

# 后端
cd src-tauri && cargo test && cargo clippy

# 完整回归
npm run typecheck && npm run test && cd src-tauri && cargo test && cargo clippy
```

## 九、风险点

1. **HTTP 耗时分解重构**：需要重构 HTTP 执行器，影响范围较大，需充分测试
2. **调试模式兼容**：ExecutionStackEntry 变更需同步前后端，需确保调试功能正常
3. **数据量增加**：新增字段可能增加报告文件大小，需注意存储性能
