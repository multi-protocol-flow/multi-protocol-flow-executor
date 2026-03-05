# WebSocket Connect 节点重设计方案

**版本**: 2.0  
**创建日期**: 2026-03-05  
**状态**: 设计确认中

---

## 一、背景

现有 WebSocket Connect 节点的 `true/false` 端口语义混淆了"连接成功"和"条件满足"两个概念，导致：
1. 用户难以理解节点行为
2. 条件不满足 ≠ 连接失败，但被归类为 false 端口
3. 节点职责不单一

---

## 二、设计原则

1. **节点职责单一**：WS Connect 只负责建立连接，条件判断交给 Conditional 节点
2. **端口语义清晰**：`true` = 连接成功，`false` = 连接失败
3. **等待模式影响输出数据**：不改变端口走向，只决定输出变量的数据源

---

## 三、配置项修改

### 3.1 等待消息开关

| 原设计 | 新设计 |
|--------|--------|
| `wait_for_message` + `wait_mode: none` | 关闭开关即可 |
| `wait_mode` 包含 `none` 选项 | 移除 `none`，用开关控制 |

**新设计 UI 结构**：

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  URL: wss://example.com/stream                              │
│  Headers: ...                                               │
│  Subprotocols: ...                                          │
│  连接超时: 30000 ms                                          │
├─────────────────────────────────────────────────────────────┤
│  连接后等待消息                                              │
│  ☐ 启用                                                     │
│                                                             │
│  [仅当启用时显示以下内容]                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 等待模式:                                            │    │
│  │ ○ 等待第一条消息                                     │    │
│  │ ○ 等待条件匹配                                       │    │
│  │ ○ 等待指定时长                                       │    │
│  │                                                      │    │
│  │ [如果选择"等待条件匹配"]                              │    │
│  │ 匹配类型: JSONPath / 正则表达式 / 表达式              │    │
│  │ 条件: $.type == "connected"                          │    │
│  │                                                      │    │
│  │ 消息超时: 30000 ms                                   │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│  输出变量映射                                                │
│  [根据等待模式动态显示可用的 JSONPath 预设]                   │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 等待模式说明

| 模式 | 说明 | 阻塞行为 |
|------|------|----------|
| 关闭开关 | 连接成功后立即返回 | 不阻塞 |
| 等待第一条消息 | 等待收到任意消息（或超时） | 阻塞直到收到消息或超时 |
| 等待条件匹配 | 等待收到满足条件的消息（或超时） | 阻塞直到匹配或超时 |
| 等待指定时长 | 在指定时长内接收消息 | 阻塞直到时长结束 |

---

## 四、端口语义修改

### 4.1 true/false 端口

| 端口 | 触发条件 | 说明 |
|------|----------|------|
| `true` | 连接成功 | TCP/TLS 握手完成，WebSocket 连接已建立 |
| `false` | 连接失败 | 网络错误、DNS 解析失败、TLS 错误、连接超时 |

**注意**：等待消息超时或条件不匹配**不影响端口走向**，仍走 `true` 端口。

### 4.2 与原设计对比

| 场景 | 原设计 | 新设计 |
|------|--------|--------|
| 连接成功 | true | true |
| 连接失败 | false | false |
| 连接成功 + 等待消息超时 | false | **true** |
| 连接成功 + 条件不匹配 | false | **true** |

---

## 五、输出数据结构修改

### 5.1 根据等待模式返回不同数据

**关闭等待消息**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**等待第一条消息（成功）**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": {
    "data": "{\"type\":\"connected\",\"token\":\"xxx\"}",
    "type": "text",
    "timestamp": 1709876543210,
    "parsed": {
      "type": "connected",
      "token": "xxx"
    }
  }
}
```

**等待第一条消息（超时）**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": null
}
```

**等待条件匹配（成功）**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": {
    "data": "{\"type\":\"connected\",\"token\":\"xxx\"}",
    "type": "text",
    "timestamp": 1709876543210,
    "parsed": {
      "type": "connected",
      "token": "xxx"
    }
  }
}
```

**等待条件匹配（超时无匹配）**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": null
}
```

**等待指定时长（可能收到多条，返回最后一条）**：
```json
{
  "connected": true,
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": {
    "data": "最后一条消息内容",
    "type": "text",
    "timestamp": 1709876549999,
    "parsed": { ... }
  }
}
```

### 5.2 false 端口输出

```json
{
  "connected": false,
  "connection_id": null,
  "error": "连接超时: wss://example.com/stream"
}
```

---

## 六、输出变量映射修改

### 6.1 可用的 JSONPath（根据模式动态变化）

| 模式 | 可用路径 | 说明 |
|------|----------|------|
| 关闭等待 | `$.connected`, `$.connection_id` | 仅连接元信息 |
| 开启等待 | `$.connected`, `$.connection_id`, `$.message.data`, `$.message.parsed.*`, `$.message.timestamp` | 包含消息内容 |

### 6.2 UI 动态预设

**关闭等待时**：
```
$.connection_id  → 连接 ID
$.connected      → 连接状态
```

**开启等待时**：
```
$.connection_id      → 连接 ID
$.connected          → 连接状态
$.message            → 消息对象
$.message.data       → 原始消息
$.message.parsed     → 解析后的 JSON
$.message.timestamp  → 消息时间戳
```

---

## 七、类型定义修改

### 7.1 TypeScript (src/types/node.ts)

```typescript
// 等待模式（移除 'none'）
export type WsWaitMode = 'first_message' | 'condition' | 'duration';

// WS Connect 配置
export interface WsConnectConfig {
  type: 'ws_connect';
  url: string;
  headers?: FormField[];
  subprotocols?: string[];
  connect_timeout_ms?: number;  // 默认 30000
  
  // 等待消息（开关）
  wait_for_message: boolean;
  
  // 以下字段仅当 wait_for_message = true 时有效
  wait_mode?: WsWaitMode;
  wait_condition?: WsWaitCondition;
  message_timeout_ms?: number;  // 默认 30000
  
  output_mappings?: OutputMapping[];
}
```

### 7.2 Rust (src-tauri/crates/flow-engine/src/node_config.rs)

```rust
/// 等待模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsWaitMode {
    FirstMessage,  // 等待第一条消息
    Condition,     // 等待条件匹配
    Duration,      // 等待指定时长
}

/// WS Connect 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConnectConfig {
    pub url: String,
    pub headers: Vec<FormField>,
    #[serde(default)]
    pub subprotocols: Vec<String>,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    
    // 等待消息开关
    #[serde(default)]
    pub wait_for_message: bool,
    
    // 以下字段仅当 wait_for_message = true 时有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_mode: Option<WsWaitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_condition: Option<WsWaitCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_timeout_ms: Option<u64>,
    
    #[serde(default)]
    pub output_mappings: Vec<OutputMapping>,
}
```

---

## 八、执行逻辑修改

### 8.1 核心逻辑伪代码

```rust
async fn execute(&self, node: &Node, context: &mut ExecutionContext) -> Result<(Vec<String>, Option<Value>), String> {
    let config = parse_config(&node.data)?;
    
    // 1. 建立 WebSocket 连接
    match ws_manager.connect(&node.uuid, &config.url, config.connect_timeout_ms).await {
        Ok(()) => {
            // 连接成功
            
            // 2. 如果启用了等待消息
            let message_data = if config.wait_for_message {
                if let Some(wait_mode) = &config.wait_mode {
                    wait_for_message(&node.uuid, wait_mode, &config, ws_manager).await?
                } else {
                    None
                }
            } else {
                None
            };
            
            // 3. 构建输出数据
            let output_data = build_output_data(&node.uuid, message_data);
            
            // 4. 应用变量映射
            apply_output_mappings(&output_data, &config.output_mappings, context);
            
            // 5. 返回 true 端口
            let next_nodes = get_next_nodes_by_port(connections, "true");
            Ok((next_nodes, Some(output_data)))
        }
        Err(e) => {
            // 连接失败，返回 false 端口
            let error_output = json!({
                "connected": false,
                "connection_id": null,
                "error": e
            });
            
            let next_nodes = get_next_nodes_by_port(connections, "false");
            Ok((next_nodes, Some(error_output)))
        }
    }
}
```

### 8.2 等待消息逻辑

```rust
async fn wait_for_message(
    uuid: &str,
    wait_mode: &WsWaitMode,
    config: &WsConnectConfig,
    ws_manager: &WebSocketConnectionManager,
) -> Result<Option<MessageData>, String> {
    let timeout_ms = config.message_timeout_ms.unwrap_or(30000);
    
    match wait_mode {
        WsWaitMode::FirstMessage => {
            // 等待第一条消息（或超时）
            match ws_manager.receive(uuid, timeout_ms).await {
                Ok(Some(msg)) => Ok(Some(parse_message(msg))),
                Ok(None) => Ok(None),  // 超时，返回 None
                Err(e) => Err(e),
            }
        }
        
        WsWaitMode::Condition => {
            // 等待满足条件的消息（或超时）
            let condition = config.wait_condition.as_ref().ok_or("未指定等待条件")?;
            let start = Instant::now();
            
            loop {
                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed >= timeout_ms {
                    return Ok(None);  // 超时，返回 None
                }
                
                let remaining = timeout_ms - elapsed;
                match ws_manager.receive(uuid, remaining.min(5000)).await {
                    Ok(Some(msg)) => {
                        if ws_manager.match_message(&msg, condition)? {
                            return Ok(Some(parse_message(msg)));  // 匹配成功
                        }
                        // 不匹配，继续等待
                    }
                    Ok(None) => continue,  // 接收超时，继续
                    Err(e) => return Err(e),
                }
            }
        }
        
        WsWaitMode::Duration => {
            // 在指定时长内接收消息，返回最后一条
            let start = Instant::now();
            let mut last_message = None;
            
            while start.elapsed().as_millis() < timeout_ms as u128 {
                match ws_manager.receive(uuid, 1000).await {
                    Ok(Some(msg)) => {
                        last_message = Some(parse_message(msg));
                    }
                    Ok(None) | Err(_) => {}
                }
            }
            
            Ok(last_message)
        }
    }
}
```

---

## 九、UI 组件修改

### 9.1 WsConnectConfig.tsx 修改点

1. **移除 `wait_mode` 的 `none` 选项**
2. **条件显示**：`wait_mode` / `wait_condition` / `message_timeout_ms` 仅当 `wait_for_message = true` 时显示
3. **JSONPath 预设动态变化**：根据 `wait_for_message` 状态显示不同的预设选项

### 9.2 输出映射提示

```
开启等待时显示：
┌─────────────────────────────────────────────────────────────┐
│ ℹ️ JSONPath 语法：                                           │
│   $.connection_id   → 连接 ID                                │
│   $.message         → 消息对象                               │
│   $.message.data    → 原始消息                               │
│   $.message.parsed  → 解析后的 JSON                          │
│                                                              │
│ 注意：如果等待超时未收到消息，$.message 将为 null             │
└─────────────────────────────────────────────────────────────┘

关闭等待时显示：
┌─────────────────────────────────────────────────────────────┐
│ ℹ️ JSONPath 语法：                                           │
│   $.connection_id   → 连接 ID                                │
│   $.connected       → 连接状态                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 十、迁移计划

### 10.1 数据迁移

现有流程文件中的 `wait_mode: "none"` 需要迁移为 `wait_for_message: false`。

```typescript
// 迁移脚本
function migrateWsConnectConfig(config: WsConnectConfig): WsConnectConfig {
  if (config.wait_mode === 'none' || !config.wait_for_message) {
    return {
      ...config,
      wait_for_message: false,
      wait_mode: undefined,
      wait_condition: undefined,
      message_timeout_ms: undefined,
    };
  }
  return config;
}
```

### 10.2 向后兼容

在 Rust 端，`WsWaitMode::None` 保留但标记为废弃，解析时自动转换为 `wait_for_message = false`。

---

## 十一、测试用例

### 11.1 连接成功 + 不等待

```json
{
  "name": "WS Connect - 不等待消息",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": { "connected": true, "connection_id": "conn-1" }
  }
}
```

### 11.2 连接成功 + 等待第一条消息 + 收到消息

```json
{
  "name": "WS Connect - 等待第一条消息成功",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 10000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": { "connected": true, "connection_id": "conn-1", "message": { ... } }
  }
}
```

### 11.3 连接成功 + 等待第一条消息 + 超时

```json
{
  "name": "WS Connect - 等待消息超时",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 2000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": { "connected": true, "connection_id": "conn-1", "message": null }
  }
}
```

### 11.4 连接失败

```json
{
  "name": "WS Connect - 连接失败",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://invalid.example.com",
        "wait_for_message": false
      }
    }
  ],
  "expected": {
    "port": "false",
    "output": { "connected": false, "connection_id": null, "error": "..." }
  }
}
```

---

## 十二、待确认事项

1. **等待超时后的 message 为 null**，用户需要用 Conditional 节点判断 `$.message == null`，这个交互是否可接受？

2. **Duration 模式返回最后一条消息**，还是返回所有消息的数组？当前设计是返回最后一条。

3. **是否需要在输出中增加 `wait_timed_out` 字段**，方便用户判断？
   ```json
   {
     "connected": true,
     "connection_id": "xxx",
     "wait_timed_out": true,
     "message": null
   }
   ```

---

**文档结束**
