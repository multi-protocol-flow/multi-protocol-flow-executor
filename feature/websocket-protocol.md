# WebSocket 协议节点功能需求文档

**版本**: 1.0  
**创建日期**: 2026-03-05  
**状态**: 需求确认完成

---

## 一、概述

### 1.1 背景

当前系统支持 HTTP 协议节点，但缺少对 WebSocket 协议的支持。WebSocket 是一种全双工通信协议，适用于实时数据推送、聊天、行情订阅等场景。

### 1.2 目标

- 新增 WebSocket 协议支持
- 支持连接复用（多个节点共享同一连接）
- 支持发送消息并等待特定响应
- 支持灵活的消息匹配策略
- 与现有变量系统无缝集成

### 1.3 核心设计原则

| 原则 | 说明 |
|------|------|
| 连接隔离 | 每个流程实例独立管理自己的连接池，不跨流程共享 |
| 顺序执行 | 不支持并行执行，同一时间只有一个节点在操作连接 |
| 显式引用 | 节点通过 UUID 显式引用连接，UI 显示连接节点名称 |
| 错误透明 | 连接/发送/等待失败时明确走 false 端口或报错 |

---

## 二、节点设计

### 2.1 节点类型总览

| 节点类型 | 功能 | 输入端口 | 输出端口 |
|----------|------|----------|----------|
| WS Connect | 建立 WebSocket 连接 | 1 | 2 (true/false) |
| WS Send+Wait | 发送消息并等待响应 | 1 | 2 (true/false) |
| WS Close | 关闭 WebSocket 连接 | 1 | 2 (true/false) |

### 2.2 节点关系图

```
Entry → WS Connect ────→ WS Send+Wait ────→ WS Send+Wait ────→ WS Close
           │                    │                   │                │
           ├─ connection_id ────┴───────────────────┴────────────────┤
           │                    (UUID 引用)                          │
           │                                                        │
           └────────────────────────────────────────────────────────┘
                            同一个 WebSocket 连接
```

---

## 三、节点详细设计

### 3.1 WS Connect 节点

#### 3.1.1 功能描述

建立 WebSocket 连接，可选择在连接后立即等待特定消息。

#### 3.1.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  URL: wss://example.com/stream                              │
│  （支持 ws:// 和 wss:// 协议，支持变量替换）                   │
│                                                              │
│  Headers:                                                    │
│  ┌─────────────────────┬───────────────────────────────┐    │
│  │ Key                 │ Value                         │    │
│  ├─────────────────────┼───────────────────────────────┤    │
│  │ Authorization       │ Bearer {{initial_variables.token}} │
│  └─────────────────────┴───────────────────────────────┘    │
│                                                              │
│  Subprotocols: ["chat", "rpc"]  （可选）                      │
│                                                              │
│  连接超时: 10000 ms  （默认 10000ms，范围 1000-60000ms）       │
├─────────────────────────────────────────────────────────────┤
│  连接后等待消息（可选）                                        │
│  ☑ 启用                                                      │
│                                                              │
│  等待模式:                                                    │
│  ○ 等待第一条消息 - 收到任意消息即通过                         │
│  ○ 等待满足条件的消息 - 需配置匹配条件                         │
│  ○ 一定时间内接收消息 - 等待指定时间后通过                     │
│                                                              │
│  [如果选择"等待满足条件的消息"]                                │
│  匹配方式: JSONPath / 正则表达式 / 表达式                      │
│  条件: $.type == "connected"                                 │
│                                                              │
│  消息等待超时: 5000 ms  （默认 5000ms，范围 1000-30000ms）     │
├─────────────────────────────────────────────────────────────┤
│  变量映射（可选）                                             │
│  ┌─────────────────────────────┬───────────────────────┐    │
│  │ JSONPath                    │ 目标变量名             │    │
│  ├─────────────────────────────┼───────────────────────┤    │
│  │ $.parsed.token              │ ws_token              │    │
│  │ $.parsed.user_id            │ user_id               │    │
│  └─────────────────────────────┴───────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

#### 3.1.3 输出端口

| 端口 | 条件 | 输出数据 |
|------|------|----------|
| true | 连接成功（且满足等待条件，如果启用） | 连接信息 / 收到的消息 |
| false | 连接超时 / 连接失败 / 消息等待超时 / 条件不满足 | null 或错误信息 |

#### 3.1.4 输出数据结构

**不启用"连接后等待消息"时**：
```json
{
  "connection": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "wss://example.com/stream",
    "status": "connected"
  }
}
```

**启用"连接后等待消息"时**：
```json
{
  "connection": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "wss://example.com/stream",
    "status": "connected"
  },
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

#### 3.1.5 等待模式详解

| 模式 | 说明 | true 条件 | false 条件 |
|------|------|-----------|------------|
| 不启用 | 连接后不等待 | 连接成功 | 连接超时/失败 |
| 等待第一条消息 | 收到任意消息即通过 | 连接成功 + 收到消息 | 连接失败 / 消息等待超时 |
| 等待满足条件 | 需匹配指定条件 | 连接成功 + 收到满足条件的消息 | 连接失败 / 消息等待超时 / 条件不满足 |
| 一定时间内接收 | 等待指定时间 | 连接成功 + 等待时间结束 | 连接失败 |

---

### 3.2 WS Send+Wait 节点

#### 3.2.1 功能描述

向已建立的 WebSocket 连接发送消息，并根据配置等待响应。

#### 3.2.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: stock-stream  （下拉选择已建立的连接）                  │
│                                                              │
│  发送消息: {"action": "subscribe", "channel": "price"}       │
│  （支持变量替换，如 {{ws_token}}）                             │
│                                                              │
│  消息类型: 文本 / 二进制  （默认文本）                         │
│                                                              │
│  发送超时: 5000 ms  （默认 5000ms，范围 1000-30000ms）         │
├─────────────────────────────────────────────────────────────┤
│  等待响应                                                    │
│  ☑ 启用                                                      │
│                                                              │
│  等待模式:                                                    │
│  ○ 不等待响应 - 发送完成即通过                                 │
│  ○ 等待第一条消息 - 收到任意消息即通过                         │
│  ○ 等待满足条件的消息 - 需配置匹配条件                         │
│  ○ 一定时间内接收消息 - 等待指定时间后通过                     │
│                                                              │
│  [如果选择"等待满足条件的消息"]                                │
│  匹配方式: JSONPath / 正则表达式 / 表达式                      │
│  条件: $.status == "ok"                                      │
│                                                              │
│  [如果启用等待响应]                                           │
│  消息等待超时: 5000 ms  （默认 5000ms，范围 1000-30000ms）     │
├─────────────────────────────────────────────────────────────┤
│  变量映射（可选）                                             │
│  ┌─────────────────────────────┬───────────────────────┐    │
│  │ JSONPath                    │ 目标变量名             │    │
│  ├─────────────────────────────┼───────────────────────┤    │
│  │ $.parsed.price              │ current_price         │    │
│  │ $.parsed.timestamp          │ price_time            │    │
│  └─────────────────────────────┴───────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.3 输出端口

| 端口 | 条件 | 输出数据 |
|------|------|----------|
| true | 发送成功（且满足等待条件，如果启用） | 发送信息 / 收到的消息 |
| false | 发送超时 / 发送失败 / 消息等待超时 / 条件不满足 | null 或错误信息 |

#### 3.2.4 输出数据结构

**不启用"等待响应"时**：
```json
{
  "sent": {
    "data": "{\"action\":\"subscribe\"}",
    "type": "text",
    "timestamp": 1709876543210
  },
  "connection": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "wss://example.com/stream"
  }
}
```

**启用"等待响应"时**：
```json
{
  "sent": {
    "data": "{\"action\":\"subscribe\"}",
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
  },
  "connection": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "wss://example.com/stream"
  }
}
```

#### 3.2.5 等待模式详解

| 模式 | 说明 | true 条件 | false 条件 |
|------|------|-----------|------------|
| 不等待响应 | 发送后立即返回 | 发送成功 | 发送超时/失败 |
| 等待第一条消息 | 收到任意消息即通过 | 发送成功 + 收到消息 | 发送失败 / 消息等待超时 |
| 等待满足条件 | 需匹配指定条件 | 发送成功 + 收到满足条件的消息 | 发送失败 / 消息等待超时 / 条件不满足 |
| 一定时间内接收 | 等待指定时间 | 发送成功 + 等待时间结束 | 发送失败 |

---

### 3.3 WS Close 节点

#### 3.3.1 功能描述

关闭指定的 WebSocket 连接。

#### 3.3.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: stock-stream  （下拉选择要关闭的连接）                  │
│                                                              │
│  关闭码: 1000  （可选，默认 1000 - 正常关闭）                  │
│  （常用关闭码：1000 正常关闭、1001 端点离开、1002 协议错误）    │
│                                                              │
│  关闭原因: 流程执行完成  （可选）                              │
│                                                              │
│  等待对方确认超时: 5000 ms  （默认 5000ms，范围 1000-30000ms） │
└─────────────────────────────────────────────────────────────┘
```

#### 3.3.3 输出端口

| 端口 | 条件 | 输出数据 |
|------|------|----------|
| true | 关闭成功（收到对方 Close 帧或对方已关闭） | 关闭信息 |
| false | 等待对方确认超时 / 发送 Close 帧失败 / 连接已断开 | 错误信息 |

#### 3.3.4 输出数据结构

```json
{
  "connection": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "wss://example.com/stream",
    "status": "closed"
  },
  "close": {
    "code": 1000,
    "reason": "流程执行完成",
    "remote_code": 1000,
    "remote_reason": "Goodbye"
  }
}
```

#### 3.3.5 关闭流程

```
正常关闭流程：
1. 本地发送 Close 帧（包含 close_code 和 close_reason）
2. 等待对方回复 Close 帧
3. 收到对方 Close 帧后，TCP 连接关闭
4. 返回 true

异常情况：
- 发送 Close 帧失败 → 返回 false
- 等待对方确认超时 → 返回 false
- 连接已断开 → 返回 false
```

---

## 四、消息匹配策略

### 4.1 匹配方式

| 方式 | 说明 | 示例 |
|------|------|------|
| JSONPath | 使用 JSONPath 表达式匹配 | `$.status == "ok"` |
| 正则表达式 | 使用正则表达式匹配原始消息 | `success.*` |
| 表达式 | 使用 evalexpr 表达式（与条件节点一致） | `body.status == "ok"` |

### 4.2 JSONPath 匹配示例

```
消息内容：
{
  "type": "response",
  "status": "ok",
  "data": {
    "price": 100.5,
    "items": [{"id": 1}, {"id": 2}]
  }
}

匹配条件：
- $.status == "ok"           → 匹配成功
- $.type == "response"       → 匹配成功
- $.data.price > 100         → 匹配成功
- $.status == "error"        → 匹配失败
```

### 4.3 正则表达式匹配示例

```
消息内容：
{"status":"ok","message":"success - user logged in"}

匹配条件：
- success.*         → 匹配成功
- "status":"ok"     → 匹配成功
- error.*           → 匹配失败
```

### 4.4 表达式匹配示例

```
消息内容：
{
  "status": "ok",
  "code": 200
}

匹配条件：
- body.status == "ok"        → 匹配成功
- body.code == 200           → 匹配成功
- body.status == "error"     → 匹配失败
```

---

## 五、变量系统

### 5.1 变量引用

WebSocket 节点的输出数据可通过 JSONPath 提取并映射到变量：

```
变量映射配置：
┌─────────────────────────────┬───────────────────────┐
│ JSONPath                    │ 目标变量名             │
├─────────────────────────────┼───────────────────────┤
│ $.message.parsed.token      │ ws_token              │
│ $.message.parsed.user_id    │ user_id               │
│ $.connection.id             │ conn_id               │
└─────────────────────────────┴───────────────────────┘

后续节点可使用：
{{ws_token}}
{{user_id}}
{{conn_id}}
```

### 5.2 消息内容引用

```
输出数据结构：
{
  "message": {
    "data": "{\"status\":\"ok\"}",
    "parsed": { "status": "ok" }
  }
}

引用方式：
- {{message.parsed.status}}  → "ok"
- {{message.data}}           → "{\"status\":\"ok\"}"
```

### 5.3 与 HTTP 节点对比

| 特性 | HTTP 节点 | WebSocket 节点 |
|------|-----------|----------------|
| 响应数据 | `body`, `headers`, `status` | `message.data`, `message.parsed` |
| 请求信息 | `request.url`, `request.method` | `sent.data` |
| 连接信息 | 无 | `connection.id`, `connection.url` |
| 变量映射 | 支持 | 支持 |
| JSONPath 提取 | 支持 | 支持 |

---

## 六、执行流程

### 6.1 单连接流程示例

```
┌─────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────┐
│  Entry  │───→│ WS Connect   │───→│ WS Send+Wait │───→│ WS Close │
└─────────┘    │ (conn-1)     │    │ (conn-1)     │    │ (conn-1) │
               │ true         │    │ true         │    │ true     │
               └──────────────┘    └──────────────┘    └──────────┘

执行顺序：
1. Entry 节点执行
2. WS Connect 节点执行，建立连接 conn-1
3. WS Send+Wait 节点执行，使用 conn-1 发送消息并等待响应
4. WS Close 节点执行，关闭 conn-1
```

### 6.2 多连接流程示例

```
                    ┌──────────────┐    ┌──────────────┐    ┌──────────┐
                    │ WS Connect   │───→│ WS Send+Wait │───→│ WS Close │
                    │ (conn-1)     │    │ (conn-1)     │    │ (conn-1) │
               ┌───→│ stock-stream │    │ stock-stream │    │          │
┌─────────┐    │    └──────────────┘    └──────────────┘    └──────────┘
│  Entry  │────┤
└─────────┘    │    ┌──────────────┐    ┌──────────────┐    ┌──────────┐
               └───→│ WS Connect   │───→│ WS Send+Wait │───→│ WS Close │
                    │ (conn-2)     │    │ (conn-2)     │    │ (conn-2) │
                    │ order-stream │    │ order-stream │    │          │
                    └──────────────┘    └──────────────┘    └──────────┘

注意：当前架构不支持并行，上述流程会顺序执行
```

### 6.3 条件分支流程示例

```
                    ┌──────────────────────┐
                    │ Conditional          │
               ┌───→│ $.price > 100        │───→ true ───→ 节点A
               │    └──────────────────────┘
┌─────────┐    │
│ WS      │────┤
│ Send+   │    │    ┌──────────────────────┐
│ Wait    │    └───→│                      │───→ false ───→ 节点B
└─────────┘         └──────────────────────┘

WS Send+Wait 的 true/false 端口可用于流程控制
```

### 6.4 流程异常处理

```
异常情况：
1. WS Connect 失败 → 走 false 端口，流程可继续或终止
2. WS Send+Wait 发送超时 → 走 false 端口
3. WS Send+Wait 消息等待超时 → 走 false 端口
4. WS Close 失败 → 走 false 端口
5. 连接意外断开 → 后续节点执行时报错，流程终止

自动清理：
- 流程执行完成时，自动关闭所有未关闭的连接
- 流程执行失败时，自动清理所有连接
```

---

## 七、验证规则

### 7.1 保存时验证

| 规则 | 错误级别 | 说明 |
|------|----------|------|
| WS Send+Wait 必须引用存在的连接 | 错误 | connection_id 必须对应一个 WS Connect 节点 |
| WS Close 必须引用存在的连接 | 错误 | connection_id 必须对应一个 WS Connect 节点 |
| 同一连接只能被关闭一次 | 警告 | 多个 WS Close 节点引用同一连接 |
| WS Send+Wait 必须在对应连接之后 | 错误 | 不能在 WS Connect 之前使用连接 |
| WS Close 必须在对应连接之后 | 错误 | 不能在 WS Connect 之前关闭连接 |
| URL 格式校验 | 错误 | 必须是 ws:// 或 wss:// 开头 |
| 超时时间范围校验 | 警告 | 超出推荐范围时提示 |

### 7.2 执行时验证

| 规则 | 处理方式 |
|------|----------|
| 引用的连接不存在 | 报错，流程终止 |
| 连接已关闭 | 报错，流程终止 |
| 连接正在被其他节点使用 | 报错，流程终止（理论上不会发生，因为顺序执行） |

---

## 八、数据结构定义

### 8.1 Rust 后端结构

```rust
// WebSocket 节点类型
pub enum NodeType {
    // ... 现有类型
    WsConnect,
    WsSendWait,
    WsClose,
}

// WS Connect 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConnectConfig {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub subprotocols: Option<Vec<String>>,
    pub connect_timeout_ms: Option<u64>,  // 默认 10000
    
    // 连接后等待消息
    pub wait_for_message: bool,
    pub wait_mode: WsWaitMode,
    pub wait_condition: Option<WsWaitCondition>,
    pub message_timeout_ms: Option<u64>,  // 默认 5000
    
    // 变量映射
    pub output_mappings: Vec<OutputMapping>,
}

// WS Send+Wait 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSendWaitConfig {
    pub connection_id: String,  // 引用 WS Connect 节点的 UUID
    pub message: String,
    pub message_type: WsMessageType,  // Text | Binary
    pub send_timeout_ms: Option<u64>,  // 默认 5000
    
    // 等待响应
    pub wait_for_response: bool,
    pub wait_mode: WsWaitMode,
    pub wait_condition: Option<WsWaitCondition>,
    pub message_timeout_ms: Option<u64>,  // 默认 5000
    
    // 变量映射
    pub output_mappings: Vec<OutputMapping>,
}

// WS Close 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCloseConfig {
    pub connection_id: String,
    pub close_code: Option<u16>,  // 默认 1000
    pub close_reason: Option<String>,
    pub wait_close_timeout_ms: Option<u64>,  // 默认 5000
}

// 等待模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsWaitMode {
    None,              // 不等待
    FirstMessage,      // 等待第一条消息
    Condition,         // 等待满足条件
    Duration,          // 一定时间内接收
}

// 等待条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsWaitCondition {
    pub match_type: WsMatchType,
    pub expression: String,
}

// 匹配类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsMatchType {
    JsonPath,    // JSONPath 表达式
    Regex,       // 正则表达式
    Expression,  // evalexpr 表达式
}

// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsMessageType {
    Text,
    Binary,
}

// NodeData 枚举扩展
pub enum NodeData {
    // ... 现有类型
    WsConnect(WsConnectConfig),
    WsSendWait(WsSendWaitConfig),
    WsClose(WsCloseConfig),
}
```

### 8.2 执行上下文扩展

```rust
// ExecutionContext 扩展
pub struct ExecutionContext {
    // ... 现有字段
    
    // WebSocket 连接管理器
    pub websocket_manager: Option<Arc<RwLock<WebSocketConnectionManager>>>,
}

// WebSocket 连接管理器
pub struct WebSocketConnectionManager {
    connections: HashMap<String, ActiveConnection>,
}

// 活跃连接
pub struct ActiveConnection {
    pub uuid: String,
    pub url: String,
    pub sender: mpsc::Sender<WsMessage>,
    pub receiver: mpsc::Receiver<WsMessage>,
    pub status: ConnectionStatus,
}

pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closing,
    Closed,
}
```

### 8.3 TypeScript 前端类型

```typescript
// node.ts 新增类型
export type NodeType = 'entry' | 'http' | 'conditional' | 'ws_connect' | 'ws_send_wait' | 'ws_close';

// WS Connect 配置
export interface WsConnectConfig {
  type: 'ws_connect';
  url: string;
  headers?: Record<string, string>;
  subprotocols?: string[];
  connect_timeout_ms?: number;
  
  wait_for_message: boolean;
  wait_mode: WsWaitMode;
  wait_condition?: WsWaitCondition;
  message_timeout_ms?: number;
  
  output_mappings: OutputMapping[];
}

// WS Send+Wait 配置
export interface WsSendWaitConfig {
  type: 'ws_send_wait';
  connection_id: string;
  message: string;
  message_type: WsMessageType;
  send_timeout_ms?: number;
  
  wait_for_response: boolean;
  wait_mode: WsWaitMode;
  wait_condition?: WsWaitCondition;
  message_timeout_ms?: number;
  
  output_mappings: OutputMapping[];
}

// WS Close 配置
export interface WsCloseConfig {
  type: 'ws_close';
  connection_id: string;
  close_code?: number;
  close_reason?: string;
  wait_close_timeout_ms?: number;
}

// 等待模式
export type WsWaitMode = 'none' | 'first_message' | 'condition' | 'duration';

// 匹配类型
export type WsMatchType = 'json_path' | 'regex' | 'expression';

// 消息类型
export type WsMessageType = 'text' | 'binary';

// 等待条件
export interface WsWaitCondition {
  match_type: WsMatchType;
  expression: string;
}

// NodeData 联合类型扩展
export type NodeData = 
  | { type: 'entry' }
  | { type: 'http' } & HttpConfig
  | { type: 'conditional' } & ConditionalConfig
  | { type: 'ws_connect' } & WsConnectConfig
  | { type: 'ws_send_wait' } & WsSendWaitConfig
  | { type: 'ws_close' } & WsCloseConfig;
```

---

## 九、UI 设计建议

### 9.1 节点外观

```
┌─────────────────────────────────┐
│  🔌 WS Connect                  │  ← 图标 + 节点名称
│  stock-stream                   │  ← 用户自定义的连接名称
│  wss://example.com/stream       │  ← URL 预览
├─────────────────────────────────┤
│  ○ ────────────────────── ●  ●  │  ← 输入端口 + true/false 输出
└─────────────────────────────────┘

┌─────────────────────────────────┐
│  📤 WS Send+Wait                │
│  stock-stream                   │  ← 引用的连接名称
│  {"action":"subscribe"}         │  ← 发送消息预览
├─────────────────────────────────┤
│  ○ ────────────────────── ●  ●  │
└─────────────────────────────────┘

┌─────────────────────────────────┐
│  ❌ WS Close                    │
│  stock-stream                   │  ← 引用的连接名称
├─────────────────────────────────┤
│  ○ ────────────────────── ●  ●  │
└─────────────────────────────────┘
```

### 9.2 连接选择器

```
┌─────────────────────────────────────────────────────────────┐
│  连接:                                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ stock-stream (wss://example.com/stream)        ▼   │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  可用连接:                                                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 🟢 stock-stream                                     │    │
│  │    wss://example.com/stream                         │    │
│  │                                                      │    │
│  │ 🟢 order-stream                                     │    │
│  │    wss://orders.example.com/ws                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ℹ️ 选择要使用的 WebSocket 连接                               │
└─────────────────────────────────────────────────────────────┘
```

### 9.3 端口标签提示

```
当鼠标悬停在端口上时显示：

true 端口: "成功时执行（连接成功/发送成功/等待条件满足）"
false 端口: "失败时执行（超时/条件不满足/连接断开）"
```

---

## 十、实现注意事项

### 10.1 依赖库

**Rust 后端**：
```toml
[dependencies]
tokio-tungstenite = "0.21"  # WebSocket 客户端
futures-util = "0.3"         # 异步工具
```

### 10.2 错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| 连接超时 | 返回 false，记录日志 |
| 连接被拒 | 返回 false，记录日志 |
| 发送超时 | 返回 false，记录日志 |
| 消息等待超时 | 返回 false，记录日志 |
| 连接意外断开 | 报错，流程终止 |
| JSON 解析失败 | 跳过解析，parsed 为 null |

### 10.3 日志输出

```
[流程 xxx] [WS Connect] 开始连接: wss://example.com/stream
[流程 xxx] [WS Connect] 连接成功: conn-1
[流程 xxx] [WS Send+Wait] 发送消息到 conn-1: {"action":"subscribe"}
[流程 xxx] [WS Send+Wait] 收到消息: {"status":"ok"}
[流程 xxx] [WS Send+Wait] 条件匹配成功
[流程 xxx] [WS Close] 关闭连接: conn-1
[流程 xxx] [WS Close] 连接已关闭
```

### 10.4 性能考虑

- 每个连接使用独立的 tokio task 接收消息
- 消息接收与节点执行解耦，避免阻塞
- 连接在流程结束时自动清理，防止资源泄漏

### 10.5 安全考虑

- 支持 wss:// (TLS) 加密连接
- 支持自定义 Headers（如 Authorization）
- 不自动重连，避免潜在的安全风险

---

## 十一、测试用例

### 11.1 基本连接测试

```json
{
  "name": "WS 基本连接测试",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    { 
      "type": "ws_connect", 
      "uuid": "conn-1",
      "data": {
        "url": "wss://echo.websocket.org",
        "connect_timeout_ms": 10000,
        "wait_for_message": false
      }
    },
    { "type": "ws_close", "uuid": "close-1", "data": { "connection_id": "conn-1" } }
  ],
  "connections": [
    { "source_node_uuid": "entry-1", "target_node_uuid": "conn-1" },
    { "source_node_uuid": "conn-1", "target_node_uuid": "close-1", "source_port_id": "true" }
  ]
}
```

### 11.2 发送并等待响应测试

```json
{
  "name": "WS 发送等待测试",
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
        "wait_for_response": true,
        "wait_mode": "first_message",
        "message_timeout_ms": 5000
      }
    },
    { "type": "ws_close", "uuid": "close-1", "data": { "connection_id": "conn-1" } }
  ],
  "connections": [...]
}
```

### 11.3 条件匹配测试

```json
{
  "name": "WS 条件匹配测试",
  "nodes": [
    { "type": "entry", "uuid": "entry-1" },
    { 
      "type": "ws_connect", 
      "uuid": "conn-1",
      "data": {
        "url": "wss://example.com/stream",
        "wait_for_message": true,
        "wait_mode": "condition",
        "wait_condition": {
          "match_type": "json_path",
          "expression": "$.type == 'connected'"
        }
      }
    },
    ...
  ]
}
```

---

## 十二、版本规划

### 12.1 第一阶段（MVP）

- [ ] WS Connect 节点（不含等待消息功能）
- [ ] WS Send+Wait 节点（仅支持"等待第一条消息"）
- [ ] WS Close 节点
- [ ] 基本变量映射

### 12.2 第二阶段

- [ ] WS Connect 节点的"连接后等待消息"功能
- [ ] WS Send+Wait 节点的"等待满足条件"功能
- [ ] JSONPath / 正则 / 表达式匹配
- [ ] "一定时间内接收"模式

### 12.3 第三阶段

- [ ] 二进制消息支持
- [ ] Subprotocol 协商
- [ ] 更详细的执行日志
- [ ] 连接状态监控面板

---

## 十三、参考资料

- [RFC 6455 - WebSocket Protocol](https://datatracker.ietf.org/doc/html/rfc6455)
- [tokio-tungstenite 文档](https://docs.rs/tokio-tungstenite/)
- [JSONPath 规范](https://goessner.net/articles/JsonPath/)
- [evalexpr 文档](https://docs.rs/evalexpr/)

---

**文档结束**
