# WebSocket 消息流权设计方案

**文档版本**: v1.1  
**创建日期**: 2026-03-10  
**更新日期**: 2026-03-10  
**状态**: 设计阶段

---

## 一、问题背景

### 当前设计的局限

现有调试模式基于"节点执行"的时间线，每个节点执行完毕后才展示结果。但 WebSocket 是长连接，存在两个独立的时间线：

```
执行时间线:  Entry → Connect → Conditional → SendWait → Close
              ✓       ✓          ✓           执行中

消息时间线:  M1  M2  M3  M4  M5  M6  M7  M8  ...
             ↓   ↓   ↓   ↓   ↓   ↓   ↓   ↓
             (持续到达，不依赖节点执行)
```

**核心矛盾**：
- 断点暂停时，执行时间线停止，但消息时间线继续
- 用户看不到暂停期间收到的消息
- 消息展示被绑定在节点执行的生命周期上，无法体现完整消息流
- 普通执行模式下，报告中也缺少完整的消息流信息

---

## 二、设计目标

1. **完整可见性**：执行过程中，用户能看到每个 WS 连接收到的所有消息
2. **消息归属**：每条消息有明确的"归属节点"，便于理解消息与节点的关系
3. **断点兼容**：断点暂停时，消息流继续记录，不中断
4. **分段展示**：按节点分段展示消息流，清晰表达"哪个节点在哪个时间段收到了哪些消息"
5. **统一实现**：调试模式和普通执行模式使用同一套消息流逻辑
6. **性能可控**：消息流功能不影响执行性能，内存占用有上限

---

## 三、核心概念：消息流权

### 3.1 定义

**消息流权**：对于一个活跃的 WebSocket 连接，消息流权决定了新到达的消息归属于哪个节点。

### 3.2 规则

| 规则 | 描述 |
|------|------|
| 建立即持有 | WsConnect 节点建立连接后，持有该连接的消息流权 |
| 使用即转移 | 后续节点（WsSendWait）使用同一连接时，消息流权转移到该节点 |
| 关闭即释放 | WsClose 节点关闭连接时，消息流权释放，不再归属任何节点 |
| 转权不回溯 | 消息流权转移时，已归属旧节点的消息不转移，新消息归新节点 |

### 3.3 适用节点

| 节点类型 | 是否持有消息流权 | 说明 |
|----------|------------------|------|
| Entry | 否 | 不涉及 WS |
| Http | 否 | 不涉及 WS |
| Conditional | 否 | 不涉及 WS |
| WsConnect | 是 | 建立连接后持有，直到被转移 |
| WsSendWait | 是 | 使用连接时获取，直到被转移或连接关闭 |
| WsClose | 否 | 关闭连接，释放消息流权 |

---

## 四、消息归属流程图

### 4.1 基本流程

```
Timeline: ───────────────────────────────────────────────────────────►

WS 连接: wss://echo.example.com (connection_id: conn-001)

消息到达:     M1      M2      M3      M4      M5      M6      M7
               ↓       ↓       ↓       ↓       ↓       ↓       ↓

节点执行:   [Connect] ─── [Conditional] ─── [SendWait] ─── [Close]
             │    │                              │          │
             │    │                              │          │
消息流权:     └────┴──────────────────────────────┴──────────┘
             Connect                           SendWait    释放
             (持有)                            (持有)

消息归属:    M1, M2                            M3, M4, M5   M6, M7
             ↓                                 ↓            ↓
节点记录:    Connect.messages                  SendWait.    无归属
                                               messages     (连接已关)
```

### 4.2 断点暂停场景

```
Timeline: ───────────────────────────────────────────────────────────►

节点执行:   [Connect] ─── [断点暂停] ─── [Continue] ─── [SendWait]
             │               │                              │
             │               │                              │
             └───────────────┴──────────────────────────────┘
             Connect 持有消息流权，暂停期间仍归属 Connect

消息到达:              M1      M2      M3      M4
                        ↓       ↓       ↓       ↓
消息归属:              Connect                 SendWait
                       M1, M2, M3              M4
```

**关键点**：断点暂停不影响消息流权的归属，消息继续记录到当前持有者。

---

## 五、消息流权转移流程图

```
                    ┌─────────────────────────────┐
                    │     WS 连接已建立            │
                    │     当前持有者: Node A        │
                    └──────────────┬──────────────┘
                                   │
                                   ▼
              ┌────────────────────────────────────────┐
              │   新节点 (Node B) 开始执行 WS 操作      │
              │   (WsSendWait 使用同一 connection_id)  │
              └────────────────────┬───────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │  检查该连接的当前持有者       │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │                             │
                    ▼                             ▼
        ┌───────────────────┐         ┌───────────────────┐
        │ 持有者是自己       │         │ 持有者是其他节点   │
        │ (同一节点再次执行) │         │ 或无持有者         │
        └─────────┬─────────┘         └─────────┬─────────┘
                  │                             │
                  ▼                             ▼
        ┌───────────────────┐         ┌───────────────────┐
        │ 无需转移          │         │ 执行转移：         │
        │ 继续记录消息      │         │ 1. 关闭 Node A 的  │
        └───────────────────┘         │    消息流段        │
                                      │ 2. 发送转移事件    │
                                      │ 3. Node B 成为     │
                                      │    新持有者        │
                                      │ 4. 新消息归属 B    │
                                      └───────────────────┘
```

---

## 六、数据结构设计

### 6.1 消息结构

```rust
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// 消息方向
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsMessageDirection {
    Received,  // 接收到的消息
    Sent,      // 发送的消息
}

/// WebSocket 消息（完整内容记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 消息方向
    pub direction: WsMessageDirection,
    /// 消息内容（使用 Arc 避免深拷贝）
    #[serde(with = "arc_string")]
    pub content: Arc<String>,
    /// 是否二进制消息
    pub is_binary: bool,
    /// 消息大小（字节）
    pub size: usize,
}

/// Arc<String> 的序列化辅助
mod arc_string {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;
    
    pub fn serialize<S>(arc: &Arc<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        arc.as_str().serialize(serializer)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Arc::new(s))
    }
}

impl WsMessage {
    /// 创建新消息
    pub fn new(direction: WsMessageDirection, content: String, is_binary: bool) -> Self {
        let size = content.len();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            timestamp,
            direction,
            content: Arc::new(content),
            is_binary,
            size,
        }
    }
}
```

### 6.2 消息流段（使用 VecDeque 优化性能）

```rust
use std::collections::VecDeque;

/// 消息流段（节点级别的消息集合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsStreamSegment {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 节点类型
    pub node_type: String,
    /// 开始时间（毫秒）
    pub started_at: u64,
    /// 结束时间（毫秒），None 表示当前活跃段
    pub ended_at: Option<u64>,
    /// 消息列表（使用 VecDeque 实现 O(1) 头部删除）
    #[serde(with = "vecdeque_as_vec")]
    pub messages: VecDeque<WsMessage>,
    /// 消息数量上限
    #[serde(skip)]
    pub max_messages: usize,
    /// 总消息数（包含已丢弃的）
    pub total_message_count: usize,
    /// 已丢弃消息数
    pub dropped_message_count: usize,
}

/// VecDeque 序列化为 Vec 以保持 JSON 兼容
mod vecdeque_as_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::VecDeque;
    
    pub fn serialize<S, T>(deque: &VecDeque<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let vec: Vec<_> = deque.iter().collect();
        vec.serialize(serializer)
    }
    
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<VecDeque<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let vec: Vec<T> = Vec::deserialize(deserializer)?;
        Ok(vec.into())
    }
}

impl WsStreamSegment {
    /// 默认每段最大消息数
    pub const DEFAULT_MAX_MESSAGES: usize = 500;
    
    /// 创建新的消息流段
    pub fn new(node_uuid: String, node_name: String, node_type: String, max_messages: usize) -> Self {
        let started_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            node_uuid,
            node_name,
            node_type,
            started_at,
            ended_at: None,
            messages: VecDeque::with_capacity(max_messages),
            max_messages,
            total_message_count: 0,
            dropped_message_count: 0,
        }
    }
    
    /// 添加消息（O(1) 操作）
    pub fn push_message(&mut self, message: WsMessage) {
        self.total_message_count += 1;
        
        // 超过上限时，删除最旧的消息（FIFO）
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();  // O(1)
            self.dropped_message_count += 1;
        }
        
        self.messages.push_back(message);  // O(1)
    }
    
    /// 结束当前段
    pub fn end(&mut self) {
        self.ended_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64);
    }
    
    /// 获取消息数量
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
    
    /// 获取持续时间（毫秒）
    pub fn duration_ms(&self) -> u64 {
        self.ended_at
            .map(|end| end.saturating_sub(self.started_at))
            .unwrap_or(0)
    }
}
```

### 6.3 连接级消息流

```rust
use std::collections::HashMap;

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsConnectionStatus {
    Connected,
    Closed,
}

/// 连接级消息流
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConnectionStream {
    /// 连接唯一标识
    pub connection_id: String,
    /// 连接地址
    pub url: String,
    /// 当前持有者节点 UUID
    pub current_owner_node_uuid: Option<String>,
    /// 连接状态
    pub status: WsConnectionStatus,
    /// 消息流段列表
    pub segments: Vec<WsStreamSegment>,
    /// 每段最大消息数
    #[serde(skip)]
    pub max_messages_per_segment: usize,
    /// 整个连接的最大消息数
    #[serde(skip)]
    pub max_total_messages: usize,
}

impl WsConnectionStream {
    /// 默认每段最大消息数
    pub const DEFAULT_MAX_MESSAGES_PER_SEGMENT: usize = 500;
    /// 默认总最大消息数
    pub const DEFAULT_MAX_TOTAL_MESSAGES: usize = 2000;
    
    /// 创建新的连接消息流
    pub fn new(connection_id: String, url: String) -> Self {
        Self {
            connection_id,
            url,
            current_owner_node_uuid: None,
            status: WsConnectionStatus::Connected,
            segments: Vec::new(),
            max_messages_per_segment: Self::DEFAULT_MAX_MESSAGES_PER_SEGMENT,
            max_total_messages: Self::DEFAULT_MAX_TOTAL_MESSAGES,
        }
    }
    
    /// 创建第一个段（WsConnect 时调用）
    pub fn create_initial_segment(&mut self, node_uuid: String, node_name: String, node_type: String) {
        let segment = WsStreamSegment::new(
            node_uuid.clone(),
            node_name,
            node_type,
            self.max_messages_per_segment,
        );
        self.segments.push(segment);
        self.current_owner_node_uuid = Some(node_uuid);
    }
    
    /// 转移消息流权（WsSendWait 时调用）
    pub fn transfer_ownership(&mut self, new_node_uuid: String, new_node_name: String, new_node_type: String) {
        // 结束当前活跃段
        if let Some(segment) = self.segments.last_mut() {
            segment.end();
        }
        
        // 创建新段
        let new_segment = WsStreamSegment::new(
            new_node_uuid.clone(),
            new_node_name,
            new_node_type,
            self.max_messages_per_segment,
        );
        self.segments.push(new_segment);
        self.current_owner_node_uuid = Some(new_node_uuid);
    }
    
    /// 添加消息到当前活跃段
    pub fn push_message(&mut self, message: WsMessage) {
        // 检查总消息数上限
        let current_total: usize = self.segments.iter().map(|s| s.messages.len()).sum();
        if current_total >= self.max_total_messages {
            // 删除最旧段的最旧消息
            if let Some(first_segment) = self.segments.first_mut() {
                if first_segment.messages.pop_front().is_some() {
                    first_segment.dropped_message_count += 1;
                }
            }
        }
        
        // 添加到当前活跃段
        if let Some(segment) = self.segments.last_mut() {
            segment.push_message(message);
        }
    }
    
    /// 关闭连接
    pub fn close(&mut self) {
        // 结束当前活跃段
        if let Some(segment) = self.segments.last_mut() {
            segment.end();
        }
        self.status = WsConnectionStatus::Closed;
        self.current_owner_node_uuid = None;
    }
    
    /// 获取当前活跃段
    pub fn current_segment(&self) -> Option<&WsStreamSegment> {
        self.segments.last()
    }
    
    /// 获取总消息数
    pub fn total_message_count(&self) -> usize {
        self.segments.iter().map(|s| s.total_message_count).sum()
    }
    
    /// 获取总丢弃消息数
    pub fn total_dropped_count(&self) -> usize {
        self.segments.iter().map(|s| s.dropped_message_count).sum()
    }
}
```

### 6.4 执行上下文扩展

```rust
// 在 ExecutionContext 中添加
pub struct ExecutionContext {
    // ... 现有字段
    
    /// WS 连接消息流映射（统一实现，调试和普通模式共用）
    pub ws_streams: HashMap<String, WsConnectionStream>,
}
```

### 6.5 执行报告扩展

```rust
/// WS 消息流摘要（用于报告）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsStreamSummary {
    /// 连接 ID
    pub connection_id: String,
    /// 连接地址
    pub url: String,
    /// 总接收消息数
    pub total_received: usize,
    /// 总发送消息数
    pub total_sent: usize,
    /// 总消息数
    pub total_messages: usize,
    /// 丢弃消息数
    pub dropped_messages: usize,
    /// 段摘要列表
    pub segments: Vec<WsSegmentSummary>,
    /// 所有消息（已受 max_total_messages 限制）
    pub messages: Vec<WsMessage>,
}

/// 消息流段摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSegmentSummary {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 消息数量
    pub message_count: usize,
    /// 持续时间（毫秒）
    pub duration_ms: u64,
    /// 接收消息数
    pub received_count: usize,
    /// 发送消息数
    pub sent_count: usize,
}

impl WsConnectionStream {
    /// 转换为报告摘要
    pub fn to_summary(&self) -> WsStreamSummary {
        let all_messages: Vec<WsMessage> = self.segments.iter()
            .flat_map(|s| s.messages.iter().cloned())
            .collect();
        
        let segments: Vec<WsSegmentSummary> = self.segments.iter()
            .map(|s| WsSegmentSummary {
                node_uuid: s.node_uuid.clone(),
                node_name: s.node_name.clone(),
                message_count: s.messages.len(),
                duration_ms: s.duration_ms(),
                received_count: s.messages.iter().filter(|m| m.direction == WsMessageDirection::Received).count(),
                sent_count: s.messages.iter().filter(|m| m.direction == WsMessageDirection::Sent).count(),
            })
            .collect();
        
        let total_received = all_messages.iter().filter(|m| m.direction == WsMessageDirection::Received).count();
        let total_sent = all_messages.iter().filter(|m| m.direction == WsMessageDirection::Sent).count();
        
        WsStreamSummary {
            connection_id: self.connection_id.clone(),
            url: self.url.clone(),
            total_received,
            total_sent,
            total_messages: self.total_message_count(),
            dropped_messages: self.total_dropped_count(),
            segments,
            messages: all_messages,
        }
    }
}

// 扩展 FlowExecutionReport
pub struct FlowExecutionReport {
    // ... 现有字段
    
    /// WS 消息流摘要列表
    pub ws_streams: Vec<WsStreamSummary>,
}
```

---

## 七、性能优化设计

### 7.1 核心优化点

| 优化项 | 方案 | 复杂度 |
|--------|------|--------|
| 消息存储 | `VecDeque` 替代 `Vec` | push O(1), pop_front O(1) |
| 消息内容 | `Arc<String>` 引用计数 | 克隆 O(1)，避免深拷贝 |
| 上限检查 | 容量预分配 + 溢出删除 | 均摊 O(1) |
| 事件推送 | 异步 `try_send`，满则丢弃 | 不阻塞执行 |

### 7.2 内存占用估算

```
单条消息:
- timestamp: 8 bytes
- direction: 1 byte
- content: Arc<String> ≈ 8 bytes (指针) + 实际内容
- is_binary: 1 byte
- size: 8 bytes
≈ 26 bytes + 内容

默认配置:
- 每段 500 条 × 每条 500 字节 ≈ 250 KB
- 每连接 2000 条 × 500 字节 ≈ 1 MB
- 10 个连接 ≈ 10 MB

结论: 内存占用可控
```

### 7.3 执行时间影响分析

| 操作 | 时间 | 是否阻塞执行 |
|------|------|-------------|
| 流权转移 | ~100ns | 否 |
| 消息记录 | ~1μs | 否（在 WS 等待期间完成） |
| 事件推送 | ~10μs | 否（异步发送） |
| 上限检查 | ~100ns | 否 |

**结论**: 消息流功能不会明显拉长执行时间

---

## 八、统一实现策略

### 8.1 调试模式 vs 普通模式

| 功能 | 调试模式 | 普通模式 |
|------|----------|----------|
| 消息流追踪 | ✅ | ✅ |
| 消息流权管理 | ✅ | ✅ |
| 实时事件推送 | ✅ | ❌ |
| 报告包含摘要 | ✅ | ✅ |

### 8.2 差异点

**调试模式额外功能**：

```rust
// 调试模式下，通过 channel 实时推送消息事件
if let Some(event_sender) = &self.debug_event_sender {
    // 使用 try_send 避免阻塞
    let _ = event_sender.send(DebugEvent::WsMessageReceived {
        connection_id: stream.connection_id.clone(),
        owner_node_uuid: stream.current_owner_node_uuid.clone(),
        message: message.clone(),
    });
}
```

**普通模式**：

```rust
// 普通模式只记录，不推送
stream.push_message(message);
// 无实时事件
```

---

## 九、UI 设计

### 9.1 调试面板布局

```
┌─────────────────────────────────────────────────────────────┐
│ 调试面板                                                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ 当前节点: SendWait                                         │
│   类型: WebSocket 发送等待                                   │
│   状态: 执行中                                               │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ WS 消息流                                    [仅在WS节点显示]│
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 连接: conn-001                                       │   │
│   │ 地址: wss://echo.example.com                         │   │
│   │ 状态: 已连接                                          │   │
│   │ 当前持有者: SendWait                                  │   │
│   │ 消息统计: 收 23 条 / 发 5 条 / 丢弃 2 条              │   │
│   ├─────────────────────────────────────────────────────┤   │
│   │                                                       │   │
│   │ ─── [Connect 阶段] ──────────────────────────────    │   │
│   │ 10:23:45.123  ←  {"type":"connected","id":"abc"}    │   │
│   │ 10:23:45.456  ←  {"type":"welcome"}                 │   │
│   │                                                       │   │
│   │ ─── [SendWait 阶段] 当前 ────────────────────────    │   │
│   │ 10:24:01.001  →  {"action":"subscribe"}             │   │
│   │ 10:24:01.234  ←  {"type":"subscribed"}              │   │
│   │ 10:24:05.678  ←  {"data":"update-1"}     ◀ 实时     │   │
│   │                                                       │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ 变量                                                       │
│   connection_id: "conn-001"                                  │
│   last_message: {"data":"update-1"}                          │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ 执行堆栈                                                   │
│   1. Entry              ✓ 已完成                             │
│   2. Connect            ✓ 已完成        [点击查看消息流]      │
│   3. Conditional        ✓ 已完成                             │
│   4. SendWait           ◀ 当前                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 9.2 交互设计

| 操作 | 效果 |
|------|------|
| 当前节点是 WS 节点 | 自动展示 WS 消息流区域 |
| 当前节点非 WS 节点 | 隐藏 WS 消息流区域，或显示"无活跃连接" |
| 点击执行堆栈中的 WS 节点 | 展示该节点持有的消息流段（只读，历史） |
| 消息流实时更新 | 当前持有者的消息段持续滚动更新 |
| 消息点击 | 展开查看完整内容（针对长消息） |
| 丢弃消息提示 | 显示"已丢弃 N 条旧消息" |

### 9.3 执行报告展示

```
┌─────────────────────────────────────────────────────────────┐
│ 执行报告                                                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ WS 消息流                                                  │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 连接: conn-001 (wss://echo.example.com)             │   │
│   │ 总计: 收 150 条 / 发 10 条 / 丢弃 5 条               │   │
│   │                                                       │   │
│   │ 段统计:                                               │   │
│   │   Connect:   收 50 条, 持续 120ms                    │   │
│   │   SendWait:  收 80 条, 发 8 条, 持续 500ms           │   │
│   │   SendWait2: 收 20 条, 发 2 条, 持续 300ms           │   │
│   │                                                       │   │
│   │ 消息记录 (155 条):                                    │   │
│   │   10:23:45.123  ←  {"type":"connected","id":"abc"}  │   │
│   │   10:23:45.456  ←  {"type":"welcome"}               │   │
│   │   ...                                                 │   │
│   │   10:25:30.001  ←  {"type":"ping"}                  │   │
│   │   10:25:30.002  →  {"type":"pong"}                  │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 十、前后端交互流程

### 10.1 调试模式启动

```
前端                                后端
  │                                  │
  │──── StartDebug ─────────────────►│
  │                                  │
  │                                  ├── 初始化调试会话
  │                                  ├── 初始化 ws_streams (空)
  │                                  │
  │◄─── SessionStarted ──────────────│
  │                                  │
```

### 10.2 WS 节点执行与消息流

```
前端                                后端
  │                                  │
  │                    [Connect 节点开始执行]
  │                                  │
  │◄─── WsStreamCreated ─────────────│  (连接建立)
  │     {connection_id, url,         │
  │      owner_node_uuid}            │
  │                                  │
  │◄─── WsMessageReceived ───────────│  (消息 M1 到达)
  │     {connection_id,              │
  │      owner_node_uuid,            │
  │      message}                    │
  │                                  │
  │◄─── WsMessageReceived ───────────│  (消息 M2 到达)
  │     ...                          │
  │                                  │
  │                    [Connect 执行完成]
  │                    [SendWait 开始执行]
  │                                  │
  │◄─── WsStreamOwnerChanged ────────│  (消息流权转移)
  │     {connection_id,              │
  │      from_node_uuid,             │
  │      to_node_uuid}               │
  │                                  │
  │◄─── WsMessageReceived ───────────│  (消息 M3 到达，归属 SendWait)
  │     {connection_id,              │
  │      owner_node_uuid: SendWait,  │
  │      message}                    │
  │                                  │
  │                    [SendWait 执行完成]
  │                    [Close 开始执行]
  │                                  │
  │◄─── WsStreamClosed ──────────────│  (连接关闭)
  │     {connection_id,              │
  │      last_owner_node_uuid}       │
  │                                  │
```

### 10.3 断点暂停期间

```
前端                                后端
  │                                  │
  │◄─── PausedAtNode ────────────────│  (断点触发)
  │     {node_uuid: Conditional}     │
  │                                  │
  │                    [暂停等待命令]
  │                                  │
  │◄─── WsMessageReceived ───────────│  (消息继续到达)
  │     {connection_id,              │  (归属上一个 WS 节点)
  │      owner_node_uuid: Connect,   │
  │      message}                    │
  │                                  │
  │     [用户在调试面板看到消息流更新]   │
  │                                  │
  │──── Continue ───────────────────►│
  │                                  │
```

---

## 十一、事件类型定义

| 事件名 | 触发时机 | 数据 | 调试模式 | 普通模式 |
|--------|----------|------|----------|----------|
| WsStreamCreated | 连接建立时 | connection_id, url, owner_node_uuid | ✅ | ❌ |
| WsMessageReceived | 收到消息时 | connection_id, owner_node_uuid, message | ✅ | ❌ |
| WsMessageSent | 发送消息时 | connection_id, owner_node_uuid, message | ✅ | ❌ |
| WsStreamOwnerChanged | 消息流权转移时 | connection_id, from_node_uuid, to_node_uuid | ✅ | ❌ |
| WsStreamClosed | 连接关闭时 | connection_id, last_owner_node_uuid | ✅ | ❌ |

---

## 十二、多连接场景

当流程中存在多个 WS 连接时，每个连接独立管理消息流：

```
┌─────────────────────────────────────────────────────────────┐
│ WS 消息流                                                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ 连接: conn-001 (wss://api.example.com)                    │
│   当前持有者: SendWait                                        │
│   消息统计: 收 23 条 / 发 5 条                                │
│   [Connect 阶段] ...                                         │
│   [SendWait 阶段] ... ◀ 当前                                 │
│                                                              │
│ ▼ 连接: conn-002 (wss://stream.example.com)                 │
│   当前持有者: Connect2                                        │
│   消息统计: 收 100 条 / 发 2 条 / 丢弃 10 条                  │
│   [Connect2 阶段] ... ◀ 当前                                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 十三、设计要点总结

1. **消息流权**：每个活跃 WS 连接有一个"持有者节点"，新消息归属该节点
2. **按节点分段**：消息流按持有者节点分段存储，转移时分段切换
3. **断点兼容**：断点暂停不影响消息流记录，后台持续接收
4. **统一实现**：调试模式和普通模式共用一套消息流逻辑，仅在事件推送上有差异
5. **性能优化**：VecDeque + Arc<String> 确保高效，O(1) 操作
6. **内存可控**：设置消息上限，FIFO 丢弃旧消息
7. **完整记录**：消息内容完整保存，便于事后分析
8. **报告集成**：执行报告包含消息流摘要，便于问题排查

---

## 十四、实现计划

### 阶段一：后端基础设施
- [ ] 定义 WsMessage、WsStreamSegment、WsConnectionStream 数据结构
- [ ] 实现 VecDeque + Arc<String> 的高效消息存储
- [ ] 实现消息上限和 FIFO 丢弃逻辑
- [ ] 扩展 ExecutionContext，添加 ws_streams 字段
- [ ] 实现 WebSocketConnectionManager 中的消息流权管理逻辑

### 阶段二：后端执行器改造
- [ ] 修改 WsConnectExecutor，连接建立时创建消息流
- [ ] 修改 WsSendWaitExecutor，使用连接时转移消息流权
- [ ] 修改 WsCloseExecutor，关闭连接时关闭消息流
- [ ] 确保普通执行模式也启用消息流追踪

### 阶段三：调试事件系统（仅调试模式）
- [ ] 添加新的 DebugEvent 类型（WsStreamCreated、WsMessageReceived 等）
- [ ] 实现异步事件推送，使用 try_send 避免阻塞
- [ ] 实现后台消息监听任务（调试模式下持续推送）

### 阶段四：报告集成
- [ ] 扩展 FlowExecutionReport，添加 ws_streams 摘要
- [ ] 实现 WsConnectionStream::to_summary() 方法
- [ ] 报告生成时包含消息流摘要

### 阶段五：前端展示
- [ ] 扩展 debug store，添加 ws_streams 状态管理
- [ ] 实现 WsMessageStreamPanel 组件
- [ ] 集成到 DebugPanel，根据当前节点类型动态展示
- [ ] 实现执行堆栈中 WS 节点的消息流查看功能
- [ ] 实现执行报告中的消息流展示

### 阶段六：测试与优化
- [ ] 单元测试：消息流权转移逻辑
- [ ] 单元测试：消息上限和丢弃逻辑
- [ ] 集成测试：完整调试流程
- [ ] 集成测试：普通执行流程（验证报告包含消息流）
- [ ] 性能测试：大量消息时的内存和执行时间
- [ ] 用户体验优化：消息格式化、过滤等

---

## 十五、风险与注意事项

1. **内存管理**：虽然设置了上限，但多连接场景仍需监控内存使用
2. **事件推送**：调试模式下高频消息可能导致事件堆积，需要考虑限流
3. **边界情况**：连接断开重连、多个节点使用同一连接等场景需要仔细处理
4. **UI 复杂度**：多连接场景下的 UI 展示需要合理设计，避免信息过载
5. **序列化性能**：大量消息的报告序列化可能较慢，考虑异步生成

---

## 十六、配置项

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| max_messages_per_segment | 500 | 每个消息流段的最大消息数 |
| max_total_messages | 2000 | 每个连接的总最大消息数 |
| last_messages_in_report | 10 | 报告中包含的最后 N 条消息 |
