# WebSocket Send+Wait 节点重设计方案

**版本**: 2.0  
**创建日期**: 2026-03-05  
**状态**: 已确认

---

## 一、背景

现有 WebSocket Send+Wait 节点的 `true/false` 端口语义与 WS Connect 节点存在相同问题，需要保持一致：
1. 端口语义应只反映"发送成功/失败"，不应混入"等待条件是否满足"
2. 等待超时或条件不匹配应通过输出数据表达，而非改变端口走向
3. 与 WS Connect 节点保持设计一致性

---

## 二、设计原则

1. **与 WS Connect 保持一致**：端口语义、输出结构、变量映射规则保持统一
2. **节点职责单一**：WS Send+Wait 只负责发送消息，条件判断交给 Conditional 节点
3. **端口语义清晰**：`true` = 发送成功，`false` = 发送失败
4. **等待模式影响输出数据**：不改变端口走向，只决定输出变量的数据源

---

## 三、配置项修改

### 3.1 等待响应开关

| 原设计 | 新设计 |
|--------|--------|
| `wait_for_response` + `wait_mode: none` | 关闭开关即可 |
| `wait_mode` 包含 `none` 选项 | 移除 `none`，用开关控制 |
| `wait_for_response` 默认 `false` | 默认 `true`（大多数场景需要等待响应） |

**新设计 UI 结构**：

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: stock-stream（下拉选择已建立的连接）                   │
│                                                             │
│  消息内容: {"action": "subscribe", "channel": "price"}      │
│  （支持变量替换，如 {{ws_token}}）                            │
│                                                             │
│  消息类型: 文本 / 二进制（默认文本）                          │
│                                                             │
│  发送超时: 30000 ms                                          │
├─────────────────────────────────────────────────────────────┤
│  发送后等待响应                                              │
│  ☑ 启用（默认启用）                                          │
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
│  │ 条件: $.status == "ok"                               │    │
│  │                                                      │    │
│  │ 响应超时: 30000 ms                                   │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│  输出变量映射                                                │
│  [根据等待模式动态显示可用的 JSONPath 预设]                   │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 等待模式说明

| 模式 | 说明 | 阻塞行为 |
|------|------|----------|
| 关闭开关 | 发送成功后立即返回 | 不阻塞 |
| 等待第一条消息 | 等待收到任意响应消息（或超时） | 阻塞直到收到消息或超时 |
| 等待条件匹配 | 等待收到满足条件的响应（或超时） | 阻塞直到匹配或超时 |
| 等待指定时长 | 在指定时长内接收响应，返回最后一条 | 阻塞直到时长结束 |

---

## 四、端口语义修改

### 4.1 true/false 端口

| 端口 | 触发条件 | 说明 |
|------|----------|------|
| `true` | 发送成功 | 消息已成功发送到 WebSocket 连接 |
| `false` | 发送失败 | 连接已断开、网络错误、发送超时 |

**注意**：等待响应超时或条件不匹配**不影响端口走向**，仍走 `true` 端口。

### 4.2 与原设计对比

| 场景 | 原设计 | 新设计 |
|------|--------|--------|
| 发送成功 | true | true |
| 发送失败（连接断开） | false | false |
| 发送成功 + 等待响应超时 | false | **true** |
| 发送成功 + 条件不匹配 | false | **true** |

---

## 五、输出数据结构修改

### 5.1 根据等待模式返回不同数据

**关闭等待响应**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": false,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  }
}
```

**等待第一条消息（成功）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": false,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": {
    "data": "{\"status\":\"ok\",\"price\":100.5}",
    "type": "text",
    "timestamp": 1709876543300,
    "parsed": {
      "status": "ok",
      "price": 100.5
    }
  }
}
```

**等待第一条消息（超时）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": true,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": null
}
```

**等待条件匹配（成功）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": false,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": {
    "data": "{\"status\":\"ok\",\"price\":100.5}",
    "type": "text",
    "timestamp": 1709876543300,
    "parsed": {
      "status": "ok",
      "price": 100.5
    }
  }
}
```

**等待条件匹配（超时无匹配）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": true,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": null
}
```

**等待指定时长（收到消息，返回最后一条）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": false,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": {
    "data": "最后一条响应消息",
    "type": "text",
    "timestamp": 1709876549999,
    "parsed": { ... }
  }
}
```

**等待指定时长（超时无消息）**：
```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": true,
  "sent": {
    "data": "{\"action\":\"subscribe\",\"channel\":\"price\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "message": null
}
```

### 5.2 false 端口输出（发送失败）

```json
{
  "connection_id": "550e8400-e29b-41d4-a716-446655440000",
  "wait_timed_out": false,
  "sent": null,
  "message": null,
  "error": "发送失败: 连接已断开"
}
```

---

## 六、输出变量映射修改

### 6.1 可用的 JSONPath（根据模式动态变化）

| 模式 | 可用路径 | 说明 |
|------|----------|------|
| 关闭等待 | `$.connection_id`, `$.wait_timed_out`, `$.sent.*` | 发送信息 |
| 开启等待 | `$.connection_id`, `$.wait_timed_out`, `$.sent.*`, `$.message.*` | 发送信息 + 响应信息 |

### 6.2 UI 动态预设

**关闭等待响应时**：
```
$.connection_id       → 连接 ID
$.wait_timed_out      → 是否等待超时（关闭时始终 false）
$.sent.data           → 发送的消息内容
$.sent.type           → 发送消息类型（text/binary）
$.sent.timestamp      → 发送时间戳
```

**开启等待响应时**：
```
$.connection_id       → 连接 ID
$.wait_timed_out      → 是否等待超时
$.sent.data           → 发送的消息内容
$.sent.type           → 发送消息类型
$.sent.timestamp      → 发送时间戳
$.message             → 响应消息对象（超时为 null）
$.message.data        → 响应原始消息
$.message.parsed      → 解析后的 JSON
$.message.timestamp   → 响应时间戳
```

---

## 七、类型定义修改

### 7.1 TypeScript (src/types/node.ts)

```typescript
// 等待模式（移除 'none'）
export type WsWaitMode = 'first_message' | 'condition' | 'duration';

// WS Send+Wait 配置
export interface WsSendWaitConfig {
  type: 'ws_send_wait';
  connection_id: string;
  message: string;
  message_type?: WsMessageType;  // 默认 'text'
  send_timeout_ms?: number;      // 默认 30000
  
  // 等待响应（开关，默认 true）
  wait_for_response: boolean;
  
  // 以下字段仅当 wait_for_response = true 时有效
  wait_mode?: WsWaitMode;
  wait_condition?: WsWaitCondition;
  message_timeout_ms?: number;   // 默认 30000
  
  output_mappings?: OutputMapping[];
}

// 消息类型
export type WsMessageType = 'text' | 'binary';
```

### 7.2 Rust (src-tauri/crates/flow-engine/src/node_config.rs)

```rust
/// 等待模式（与 WS Connect 共用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsWaitMode {
    FirstMessage,  // 等待第一条消息
    Condition,     // 等待条件匹配
    Duration,      // 等待指定时长
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    #[default]
    Text,
    Binary,
}

/// WS Send+Wait 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSendWaitConfig {
    pub connection_id: String,
    pub message: String,
    #[serde(default)]
    pub message_type: WsMessageType,
    #[serde(default = "default_send_timeout")]
    pub send_timeout_ms: u64,
    
    // 等待响应开关（默认 true）
    #[serde(default = "default_wait_for_response")]
    pub wait_for_response: bool,
    
    // 以下字段仅当 wait_for_response = true 时有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_mode: Option<WsWaitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_condition: Option<WsWaitCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_timeout_ms: Option<u64>,
    
    #[serde(default)]
    pub output_mappings: Vec<OutputMapping>,
}

fn default_send_timeout() -> u64 { 30000 }
fn default_wait_for_response() -> bool { true }
```

---

## 八、执行逻辑修改

### 8.1 核心逻辑伪代码

```rust
async fn execute(&self, node: &Node, context: &mut ExecutionContext) -> Result<(Vec<String>, Option<Value>), String> {
    let config = parse_config(&node.data)?;
    let sent_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    
    // 1. 检查连接是否存在且状态为 Connected
    let connection_status = ws_manager.get_status(&config.connection_id);
    if connection_status != Some(ConnectionStatus::Connected) {
        // 连接不存在或已断开，返回 false 端口
        let error_output = json!({
            "connection_id": config.connection_id,
            "wait_timed_out": false,
            "sent": null,
            "message": null,
            "error": "连接不存在或已断开"
        });
        let next_nodes = get_next_nodes_by_port(connections, "false");
        return Ok((next_nodes, Some(error_output)));
    }
    
    // 2. 发送消息
    let is_binary = config.message_type == WsMessageType::Binary;
    match ws_manager.send(&config.connection_id, &config.message, is_binary, config.send_timeout_ms).await {
        Ok(()) => {
            // 发送成功
            
            // 3. 构建发送数据
            let sent_data = json!({
                "data": config.message,
                "type": if is_binary { "binary" } else { "text" },
                "timestamp": sent_timestamp,
            });
            
            // 4. 如果启用了等待响应
            let (wait_timed_out, message_data) = if config.wait_for_response {
                if let Some(wait_mode) = &config.wait_mode {
                    let result = wait_for_message(&config.connection_id, wait_mode, &config, ws_manager).await?;
                    let timed_out = result.is_none();
                    (timed_out, result)
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            };
            
            // 5. 构建输出数据
            let output_data = build_output_data(&config.connection_id, wait_timed_out, sent_data, message_data);
            
            // 6. 应用变量映射
            apply_output_mappings(&output_data, &config.output_mappings, context);
            
            // 7. 返回 true 端口
            let next_nodes = get_next_nodes_by_port(connections, "true");
            Ok((next_nodes, Some(output_data)))
        }
        Err(e) => {
            // 发送失败，返回 false 端口
            let error_output = json!({
                "connection_id": config.connection_id,
                "wait_timed_out": false,
                "sent": null,
                "message": null,
                "error": format!("发送失败: {}", e)
            });
            
            let next_nodes = get_next_nodes_by_port(connections, "false");
            Ok((next_nodes, Some(error_output)))
        }
    }
}
```

### 8.2 构建输出数据

```rust
fn build_output_data(
    connection_id: &str,
    wait_timed_out: bool,
    sent_data: serde_json::Value,
    message_data: Option<MessageData>,
) -> serde_json::Value {
    let mut output = json!({
        "connection_id": connection_id,
        "wait_timed_out": wait_timed_out,
        "sent": sent_data,
    });
    
    if let Some(msg) = message_data {
        output["message"] = json!({
            "data": msg.data,
            "type": msg.msg_type,
            "timestamp": msg.timestamp,
            "parsed": msg.parsed,
        });
    } else {
        output["message"] = serde_json::Value::Null;
    }
    
    output
}
```

### 8.3 等待消息逻辑（与 WS Connect 共用）

```rust
async fn wait_for_message(
    connection_id: &str,
    wait_mode: &WsWaitMode,
    config: &WsSendWaitConfig,
    ws_manager: &WebSocketConnectionManager,
) -> Result<Option<MessageData>, String> {
    let timeout_ms = config.message_timeout_ms.unwrap_or(30000);
    
    match wait_mode {
        WsWaitMode::FirstMessage => {
            match ws_manager.receive(connection_id, timeout_ms).await {
                Ok(Some(msg)) => Ok(Some(parse_message(msg))),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        }
        
        WsWaitMode::Condition => {
            let condition = config.wait_condition.as_ref().ok_or("未指定等待条件")?;
            let start = Instant::now();
            
            loop {
                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed >= timeout_ms {
                    return Ok(None);
                }
                
                let remaining = timeout_ms - elapsed;
                match ws_manager.receive(connection_id, remaining.min(5000)).await {
                    Ok(Some(msg)) => {
                        if ws_manager.match_message(&msg, condition)? {
                            return Ok(Some(parse_message(msg)));
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        
        WsWaitMode::Duration => {
            let start = Instant::now();
            let mut last_message = None;
            
            while start.elapsed().as_millis() < timeout_ms as u128 {
                match ws_manager.receive(connection_id, 1000).await {
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

### 9.1 WsSendWaitConfig.tsx 修改点

1. **移除 `wait_mode` 的 `none` 选项**
2. **`wait_for_response` 默认值改为 `true`**
3. **条件显示**：`wait_mode` / `wait_condition` / `message_timeout_ms` 仅当 `wait_for_response = true` 时显示
4. **JSONPath 预设动态变化**：根据 `wait_for_response` 状态显示不同的预设选项

### 9.2 输出映射提示

**开启等待响应时显示**：
```
┌─────────────────────────────────────────────────────────────┐
│ ℹ️ JSONPath 语法：                                           │
│   $.connection_id       → 连接 ID                            │
│   $.wait_timed_out      → 是否等待超时                        │
│   $.sent.data           → 发送的消息内容                      │
│   $.sent.type           → 发送消息类型                        │
│   $.sent.timestamp      → 发送时间戳                          │
│   $.message             → 响应消息对象（超时为 null）          │
│   $.message.data        → 响应原始消息                        │
│   $.message.parsed      → 解析后的 JSON                       │
│                                                              │
│ 提示：使用 Conditional 节点判断 $.wait_timed_out 或           │
│       $.message == null 来处理超时情况                        │
└─────────────────────────────────────────────────────────────┘
```

**关闭等待响应时显示**：
```
┌─────────────────────────────────────────────────────────────┐
│ ℹ️ JSONPath 语法：                                           │
│   $.connection_id       → 连接 ID                            │
│   $.wait_timed_out      → 是否等待超时（始终为 false）         │
│   $.sent.data           → 发送的消息内容                      │
│   $.sent.type           → 发送消息类型                        │
│   $.sent.timestamp      → 发送时间戳                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 十、迁移计划

### 10.1 数据迁移

现有流程文件中的 `wait_mode: "none"` 需要迁移为 `wait_for_response: false`。

```typescript
// 迁移脚本
function migrateWsSendWaitConfig(config: WsSendWaitConfig): WsSendWaitConfig {
  if (config.wait_mode === 'none' || !config.wait_for_response) {
    return {
      ...config,
      wait_for_response: false,
      wait_mode: undefined,
      wait_condition: undefined,
      message_timeout_ms: undefined,
    };
  }
  return config;
}
```

### 10.2 向后兼容

在 Rust 端，`WsWaitMode::None` 保留但标记为废弃，解析时自动转换为 `wait_for_response = false`。

---

## 十一、测试用例

### 11.1 发送成功 + 不等待响应

```json
{
  "name": "WS Send+Wait - 不等待响应",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "{\"test\":\"hello\"}",
        "wait_for_response": false
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": false,
      "sent": {
        "data": "{\"test\":\"hello\"}",
        "type": "text",
        "timestamp": "..."
      }
    }
  }
}
```

### 11.2 发送成功 + 等待第一条消息 + 收到响应

```json
{
  "name": "WS Send+Wait - 等待第一条消息成功",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "Hello, WebSocket!",
        "wait_for_response": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 10000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": false,
      "sent": { "data": "Hello, WebSocket!", "type": "text", "timestamp": "..." },
      "message": { "data": "Hello, WebSocket!", "type": "text", "timestamp": "...", "parsed": null }
    }
  }
}
```

### 11.3 发送成功 + 等待第一条消息 + 超时

```json
{
  "name": "WS Send+Wait - 等待响应超时",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "test",
        "wait_for_response": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 2000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": true,
      "sent": { "data": "test", "type": "text", "timestamp": "..." },
      "message": null
    }
  }
}
```

### 11.4 发送成功 + 等待条件匹配 + 匹配成功

```json
{
  "name": "WS Send+Wait - 条件匹配成功",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "{\"action\":\"ping\"}",
        "wait_for_response": true,
        "wait_mode": "condition",
        "wait_condition": {
          "match_type": "json_path",
          "expression": "$.action == 'ping'"
        },
        "message_timeout_ms": 10000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": false,
      "sent": { "data": "{\"action\":\"ping\"}", "type": "text", "timestamp": "..." },
      "message": { "data": "{\"action\":\"ping\"}", "parsed": { "action": "ping" }, ... }
    }
  }
}
```

### 11.5 发送成功 + 等待条件匹配 + 超时无匹配

```json
{
  "name": "WS Send+Wait - 条件匹配超时",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "test",
        "wait_for_response": true,
        "wait_mode": "condition",
        "wait_condition": {
          "match_type": "json_path",
          "expression": "$.status == 'success'"
        },
        "message_timeout_ms": 2000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": true,
      "sent": { "data": "test", "type": "text", "timestamp": "..." },
      "message": null
    }
  }
}
```

### 11.6 发送失败 - 连接已断开

```json
{
  "name": "WS Send+Wait - 发送失败（连接已断开）",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "nonexistent-conn",
        "message": "test",
        "wait_for_response": false
      }
    }
  ],
  "expected": {
    "port": "false",
    "output": {
      "connection_id": "nonexistent-conn",
      "wait_timed_out": false,
      "sent": null,
      "message": null,
      "error": "连接不存在或已断开"
    }
  }
}
```

### 11.7 发送二进制消息

```json
{
  "name": "WS Send+Wait - 发送二进制消息",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    {
      "type": "ws_connect",
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "wait_for_message": false
      }
    },
    {
      "type": "ws_send_wait",
      "uuid": "send-1",
      "data": {
        "connection_id": "conn-1",
        "message": "0102030405",
        "message_type": "binary",
        "wait_for_response": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 10000
      }
    }
  ],
  "expected": {
    "port": "true",
    "output": {
      "connection_id": "conn-1",
      "wait_timed_out": false,
      "sent": { "data": "0102030405", "type": "binary", "timestamp": "..." },
      "message": { "data": "0102030405", "type": "binary", ... }
    }
  }
}
```

---

## 十二、已确认事项

1. **等待响应超时不影响端口走向**：仍走 true 端口，通过 `wait_timed_out` 字段表达。✅

2. **发送失败的定义**：连接已断开、网络错误、发送超时 → 走 false 端口。✅

3. **`wait_for_response` 默认值**：`true`（大多数场景需要等待响应）。✅

4. **不需要 `sent_success` 字段**：端口已表达成功/失败，此字段冗余。✅

5. **发送失败时 `sent` 字段**：`sent: null`。✅

6. **Duration 模式返回最后一条消息**：在指定时长内可能收到多条响应，返回最后一条。✅

7. **变量映射包含发送数据**：用户可以引用 `$.sent.data` 等字段。✅

---

## 十三、与 WS Connect 的一致性

| 特性 | WS Connect | WS Send+Wait |
|------|------------|--------------|
| true 端口含义 | 连接成功 | 发送成功 |
| false 端口含义 | 连接失败 | 发送失败 |
| 等待超时影响端口 | 不影响 | 不影响 |
| `wait_timed_out` 字段 | 有 | 有 |
| `wait_mode` 选项 | `first_message`, `condition`, `duration` | 相同 |
| `message` 输出字段 | 有（等待时） | 有（等待时） |
| 特有输出字段 | `connected` | `sent` |

---

**文档结束**
