// WebSocket 消息流追踪模块
//
// 定义 WebSocket 消息流的核心数据结构，支持消息追踪、所有权转移和 FIFO 丢弃。

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ============================================================================
// 常量定义
// ============================================================================

/// 单条消息大小上限（1MB）
const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// 每段消息上限（默认）
const DEFAULT_MAX_MESSAGES_PER_SEGMENT: usize = 500;

/// 总消息上限（默认）
const DEFAULT_MAX_TOTAL_MESSAGES: usize = 2000;

// ============================================================================
// 消息方向枚举
// ============================================================================

/// 消息方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    /// 发送的消息
    Sent,
    /// 接收的消息
    Received,
}

// ============================================================================
// 消息内容枚举
// ============================================================================

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    /// 文本消息
    Text {
        /// 文本消息内容
        content: String,
    },
    /// 二进制消息（Base64 编码存储）
    Binary {
        /// 二进制消息内容
        content: Vec<u8>,
    },
}

// ============================================================================
// WebSocket 消息结构
// ============================================================================

/// 单条 WebSocket 消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsMessage {
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 消息方向
    pub direction: MessageDirection,
    /// 消息内容
    pub content: MessageContent,
    /// 是否为二进制消息
    pub is_binary: bool,
    /// 消息大小（字节）
    pub size: usize,
}

impl WsMessage {
    /// 创建新的 WebSocket 消息
    ///
    /// # 参数
    ///
    /// * `direction` - 消息方向
    /// * `content` - 消息内容
    ///
    /// # 返回
    ///
    /// 新创建的 `WsMessage` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let msg = WsMessage::new(MessageDirection::Sent, MessageContent::Text { content: "hello".to_string() });
    /// ```
    pub fn new(direction: MessageDirection, content: MessageContent) -> Self {
        let is_binary = matches!(content, MessageContent::Binary { .. });
        let size = match &content {
            MessageContent::Text { content } => content.len(),
            MessageContent::Binary { content } => content.len(),
        };

        // 获取当前时间戳（毫秒）
        let timestamp = crate::utils::unix_timestamp_ms();

        Self {
            timestamp,
            direction,
            content,
            is_binary,
            size,
        }
    }

    /// 使用指定时间戳创建消息（用于测试）
    pub fn with_timestamp(
        direction: MessageDirection,
        content: MessageContent,
        timestamp: u64,
    ) -> Self {
        let is_binary = matches!(content, MessageContent::Binary { .. });
        let size = match &content {
            MessageContent::Text { content } => content.len(),
            MessageContent::Binary { content } => content.len(),
        };

        Self {
            timestamp,
            direction,
            content,
            is_binary,
            size,
        }
    }
}

// ============================================================================
// 消息流段结构
// ============================================================================

/// 消息流段（单个节点持有的消息集合）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsStreamSegment {
    /// 持有者节点 UUID
    pub node_uuid: String,
    /// 持有者节点名称
    pub node_name: String,
    /// 持有者节点类型 (`ws:connect`, `ws_send_wait`, `ws:close`)
    pub node_type: String,
    /// 段开始时间（毫秒）
    pub started_at: u64,
    /// 段结束时间（毫秒，None 表示尚未结束）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<u64>,
    /// 消息列表
    #[serde(default)]
    pub messages: VecDeque<WsMessage>,
    /// 段消息上限
    pub max_messages: usize,
    /// 已丢弃消息数量
    #[serde(default)]
    pub dropped_message_count: usize,
}

impl WsStreamSegment {
    /// 创建新的消息流段
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 持有者节点 UUID
    /// * `node_name` - 持有者节点名称
    /// * `node_type` - 持有者节点类型
    ///
    /// # 返回
    ///
    /// 新创建的 `WsStreamSegment` 实例
    pub fn new(node_uuid: String, node_name: String, node_type: String) -> Self {
        // 获取当前时间戳（毫秒）
        let started_at = crate::utils::unix_timestamp_ms();

        Self {
            node_uuid,
            node_name,
            node_type,
            started_at,
            ended_at: None,
            messages: VecDeque::new(),
            max_messages: DEFAULT_MAX_MESSAGES_PER_SEGMENT,
            dropped_message_count: 0,
        }
    }

    /// 使用指定时间戳创建段（用于测试）
    #[cfg(test)]
    pub fn with_timestamp(
        node_uuid: String,
        node_name: String,
        node_type: String,
        started_at: u64,
    ) -> Self {
        Self {
            node_uuid,
            node_name,
            node_type,
            started_at,
            ended_at: None,
            messages: VecDeque::new(),
            max_messages: DEFAULT_MAX_MESSAGES_PER_SEGMENT,
            dropped_message_count: 0,
        }
    }

    /// 推送消息到段中
    ///
    /// # 参数
    ///
    /// * `message` - 要推送的消息
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 推送Success
    /// * `Err(msg)` - 消息大小超过 1MB
    ///
    /// # 说明
    ///
    /// 当消息数量超过 `max_messages` 时，会丢弃最早的消息（FIFO）
    pub fn push_message(&mut self, message: WsMessage) -> Result<(), String> {
        // 检查消息大小
        if message.size > MAX_MESSAGE_SIZE {
            return Err(format!(
                "Message size ({}) exceeds limit ({} bytes)",
                message.size, MAX_MESSAGE_SIZE
            ));
        }

        // 如果已达到上限，丢弃最早的消息
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            self.dropped_message_count += 1;
        }

        self.messages.push_back(message);
        Ok(())
    }

    /// 结束段
    ///
    /// 设置段的结束时间
    pub fn end(&mut self) {
        self.ended_at = Some(crate::utils::unix_timestamp_ms());
    }

    /// 获取当前消息数量
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 获取总消息数量（包括已丢弃的）
    pub fn total_message_count(&self) -> usize {
        self.messages.len() + self.dropped_message_count
    }
}

// ============================================================================
// WebSocket 连接消息流结构
// ============================================================================

/// WebSocket 连接消息流
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsConnectionStream {
    /// 连接 ID
    pub connection_id: String,
    /// WebSocket URL
    pub url: String,
    /// 当前持有者节点 UUID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_owner_node_uuid: Option<String>,
    /// 消息流段列表
    #[serde(default)]
    pub segments: Vec<WsStreamSegment>,
    /// 每段消息上限
    pub max_messages_per_segment: usize,
    /// 总消息上限
    pub max_total_messages: usize,
    /// 是否已关闭
    #[serde(default)]
    pub is_closed: bool,
}

impl WsConnectionStream {
    /// 创建新的连接消息流
    ///
    /// # 参数
    ///
    /// * `connection_id` - 连接 ID
    /// * `url` - WebSocket URL
    ///
    /// # 返回
    ///
    /// 新创建的 `WsConnectionStream` 实例
    pub fn new(connection_id: String, url: String) -> Self {
        Self {
            connection_id,
            url,
            current_owner_node_uuid: None,
            segments: Vec::new(),
            max_messages_per_segment: DEFAULT_MAX_MESSAGES_PER_SEGMENT,
            max_total_messages: DEFAULT_MAX_TOTAL_MESSAGES,
            is_closed: false,
        }
    }

    /// 创建初始段（第一个所有者）
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    /// * `node_name` - 节点名称
    /// * `node_type` - 节点类型
    ///
    /// # 说明
    ///
    /// 此方法用于 `ws:connect` 节点首次创建连接时调用
    pub fn create_initial_segment(
        &mut self,
        node_uuid: String,
        node_name: String,
        node_type: String,
    ) {
        let segment = WsStreamSegment::new(node_uuid.clone(), node_name, node_type);
        self.segments.push(segment);
        self.current_owner_node_uuid = Some(node_uuid);
    }

    /// 转移所有权到新节点
    ///
    /// # 参数
    ///
    /// * `new_node_uuid` - 新节点 UUID
    /// * `new_node_name` - 新节点名称
    /// * `new_node_type` - 新节点类型
    ///
    /// # 说明
    ///
    /// 此方法在以下场景调用：
    /// - `ws_send_wait` 节点start execution时
    /// - `ws:close` 节点start execution时
    ///
    /// 转移时会结束当前段并创建新段
    pub fn transfer_ownership(
        &mut self,
        new_node_uuid: String,
        new_node_name: String,
        new_node_type: String,
    ) {
        // 结束当前段
        if let Some(segment) = self.segments.last_mut() {
            segment.end();
        }

        // 创建新段
        let segment = WsStreamSegment::new(new_node_uuid.clone(), new_node_name, new_node_type);
        self.segments.push(segment);
        self.current_owner_node_uuid = Some(new_node_uuid);
    }

    /// 推送消息到当前段
    ///
    /// # 参数
    ///
    /// * `message` - 要推送的消息
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 推送Success
    /// * `Err(msg)` - 推送Failed（消息过大或无当前段）
    pub fn push_message(&mut self, message: WsMessage) -> Result<(), String> {
        // 检查是否已关闭
        if self.is_closed {
            return Err("Connection closed, cannot push message".to_string());
        }

        // 检查是否有段
        if self.segments.is_empty() {
            return Err("No current message segment".to_string());
        }

        // 检查消息大小
        if message.size > MAX_MESSAGE_SIZE {
            return Err(format!(
                "Message size ({}) exceeds limit ({} bytes)",
                message.size, MAX_MESSAGE_SIZE
            ));
        }

        // 检查总消息上限（在获取可变引用之前计算）
        let total = self.total_message_count();
        if total >= self.max_total_messages {
            // 需要丢弃最早的消息
            self.drop_oldest_message()?;
        }

        // 获取当前段并推送消息
        let current_segment = self.segments.last_mut().ok_or("No segments available")?;
        current_segment.push_message(message)
    }

    /// 关闭连接流
    ///
    /// 结束当前段并标记为已关闭
    pub fn close(&mut self) {
        // 结束当前段
        if let Some(segment) = self.segments.last_mut() {
            segment.end();
        }
        self.is_closed = true;
    }

    /// 获取总消息数量
    pub fn total_message_count(&self) -> usize {
        self.segments
            .iter()
            .map(|s| s.messages.len() + s.dropped_message_count)
            .sum()
    }

    /// 获取当前保留的消息数量
    pub fn retained_message_count(&self) -> usize {
        self.segments.iter().map(|s| s.messages.len()).sum()
    }

    /// 获取当前段
    pub fn current_segment(&self) -> Option<&WsStreamSegment> {
        self.segments.last()
    }

    /// 获取当前段（可变）
    pub fn current_segment_mut(&mut self) -> Option<&mut WsStreamSegment> {
        self.segments.last_mut()
    }

    /// 丢弃最早的消息
    ///
    /// 从最早的段中丢弃一条消息
    fn drop_oldest_message(&mut self) -> Result<(), String> {
        // 找到第一个有消息的段
        for segment in &mut self.segments {
            if !segment.messages.is_empty() {
                segment.messages.pop_front();
                segment.dropped_message_count += 1;
                return Ok(());
            }
        }
        Err("No messages to discard".to_string())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // MessageDirection 测试
    // ============================================================================

    #[test]
    fn test_message_direction_serialization() {
        let sent = MessageDirection::Sent;
        let json = serde_json::to_string(&sent).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert_eq!(json, r#""sent""#);

        let received = MessageDirection::Received;
        let json = serde_json::to_string(&received).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert_eq!(json, r#""received""#);
    }

    #[test]
    fn test_message_direction_deserialization() {
        let sent: MessageDirection =
            serde_json::from_str(r#""sent""#).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(sent, MessageDirection::Sent);

        let received: MessageDirection =
            serde_json::from_str(r#""received""#).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(received, MessageDirection::Received);
    }

    // ============================================================================
    // MessageContent 测试
    // ============================================================================

    #[test]
    fn test_message_content_text_serialization() {
        let text = MessageContent::Text {
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&text).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert_eq!(json, r#"{"type":"text","content":"hello"}"#);
    }

    #[test]
    fn test_message_content_binary_serialization() {
        let binary = MessageContent::Binary {
            content: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&binary).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        // Base64 编码: [1, 2, 3] -> "AQID"
        assert!(json.contains(r#""type":"binary""#));
        assert!(json.contains(r#""content""#));
    }

    #[test]
    fn test_message_content_deserialization() {
        let text: MessageContent = serde_json::from_str(r#"{"type":"text","content":"hello"}"#)
            .unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert!(matches!(text, MessageContent::Text { content } if content == "hello"));

        let binary: MessageContent = serde_json::from_str(r#"{"type":"binary","content":[1,2,3]}"#)
            .unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert!(matches!(binary, MessageContent::Binary { content } if content == vec![1, 2, 3]));
    }

    // ============================================================================
    // WsMessage 测试
    // ============================================================================

    #[test]
    fn test_ws_message_creation() {
        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        assert_eq!(msg.direction, MessageDirection::Sent);
        assert!(!msg.is_binary);
        assert_eq!(msg.size, 5);
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn test_ws_message_binary() {
        let msg = WsMessage::new(
            MessageDirection::Received,
            MessageContent::Binary {
                content: vec![1, 2, 3],
            },
        );

        assert_eq!(msg.direction, MessageDirection::Received);
        assert!(msg.is_binary);
        assert_eq!(msg.size, 3);
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::with_timestamp(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "test".to_string(),
            },
            1234567890,
        );

        let json = serde_json::to_string(&msg).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains(r#""timestamp":1234567890"#));
        assert!(json.contains(r#""direction":"sent""#));
        assert!(json.contains(r#""is_binary":false"#));
        assert!(json.contains(r#""size":4"#));
    }

    // ============================================================================
    // WsStreamSegment 测试
    // ============================================================================

    #[test]
    fn test_segment_creation() {
        let segment = WsStreamSegment::new(
            "node-1".to_string(),
            "Test Node".to_string(),
            "ws:connect".to_string(),
        );

        assert_eq!(segment.node_uuid, "node-1");
        assert_eq!(segment.node_name, "Test Node");
        assert_eq!(segment.node_type, "ws:connect");
        assert!(segment.ended_at.is_none());
        assert_eq!(segment.message_count(), 0);
        assert_eq!(segment.dropped_message_count, 0);
    }

    #[test]
    fn test_segment_push_message() {
        let mut segment = WsStreamSegment::new(
            "node-1".to_string(),
            "Test".to_string(),
            "ws:connect".to_string(),
        );

        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = segment.push_message(msg);
        assert!(result.is_ok());
        assert_eq!(segment.message_count(), 1);
    }

    #[test]
    fn test_segment_message_size_limit() {
        let mut segment = WsStreamSegment::new(
            "node-1".to_string(),
            "Test".to_string(),
            "ws:connect".to_string(),
        );

        // 创建超过 1MB 的消息
        let large_content = "x".repeat(MAX_MESSAGE_SIZE + 1);
        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: large_content,
            },
        );

        let result = segment.push_message(msg);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("exceeds limit"));
    }

    #[test]
    fn test_segment_fifo_discard() {
        let mut segment = WsStreamSegment::with_timestamp(
            "node-1".to_string(),
            "Test".to_string(),
            "ws:connect".to_string(),
            1000,
        );
        segment.max_messages = 3; // 设置较小的上限便于测试

        // 添加 4 条消息
        for i in 0..4 {
            let msg = WsMessage::with_timestamp(
                MessageDirection::Sent,
                MessageContent::Text {
                    content: format!("msg-{}", i),
                },
                1000 + i,
            );
            segment.push_message(msg).unwrap_or_else(|e| panic!("{e}"));
        }

        // 应该只保留最后 3 条
        assert_eq!(segment.message_count(), 3);
        assert_eq!(segment.dropped_message_count, 1);

        // 第一条消息应该被丢弃
        let first_msg = segment
            .messages
            .front()
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(
            matches!(&first_msg.content, MessageContent::Text { content } if content == "msg-1")
        );
    }

    #[test]
    fn test_segment_end() {
        let mut segment = WsStreamSegment::new(
            "node-1".to_string(),
            "Test".to_string(),
            "ws:connect".to_string(),
        );

        assert!(segment.ended_at.is_none());

        segment.end();

        assert!(segment.ended_at.is_some());
    }

    // ============================================================================
    // WsConnectionStream 测试
    // ============================================================================

    #[test]
    fn test_connection_stream_creation() {
        let stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        assert_eq!(stream.connection_id, "conn-1");
        assert_eq!(stream.url, "wss://example.com/ws");
        assert!(stream.current_owner_node_uuid.is_none());
        assert!(stream.segments.is_empty());
        assert!(!stream.is_closed);
    }

    #[test]
    fn test_connection_stream_create_initial_segment() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        assert_eq!(stream.current_owner_node_uuid, Some("node-1".to_string()));
        assert_eq!(stream.segments.len(), 1);

        let segment = &stream.segments[0];
        assert_eq!(segment.node_uuid, "node-1");
        assert_eq!(segment.node_name, "Connect Node");
        assert_eq!(segment.node_type, "ws:connect");
    }

    #[test]
    fn test_connection_stream_transfer_ownership() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        // 创建初始段
        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        // 转移所有权
        stream.transfer_ownership(
            "node-2".to_string(),
            "Send Node".to_string(),
            "ws_send_wait".to_string(),
        );

        assert_eq!(stream.current_owner_node_uuid, Some("node-2".to_string()));
        assert_eq!(stream.segments.len(), 2);

        // 第一个段应该已结束
        assert!(stream.segments[0].ended_at.is_some());

        // 第二个段应该是当前段
        assert!(stream.segments[1].ended_at.is_none());
        assert_eq!(stream.segments[1].node_uuid, "node-2");
    }

    #[test]
    fn test_connection_stream_push_message() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = stream.push_message(msg);
        assert!(result.is_ok());

        assert_eq!(stream.total_message_count(), 1);
        assert_eq!(stream.retained_message_count(), 1);
    }

    #[test]
    fn test_connection_stream_push_message_without_segment() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = stream.push_message(msg);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("No current message segment"));
    }

    #[test]
    fn test_connection_stream_push_message_when_closed() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        stream.close();

        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = stream.push_message(msg);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("closed"));
    }

    #[test]
    fn test_connection_stream_close() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        assert!(!stream.is_closed);

        stream.close();

        assert!(stream.is_closed);
        // 当前段应该已结束
        assert!(stream.segments[0].ended_at.is_some());
    }

    #[test]
    fn test_connection_stream_total_message_count() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        // 添加 3 条消息
        for i in 0..3 {
            let msg = WsMessage::new(
                MessageDirection::Sent,
                MessageContent::Text {
                    content: format!("msg-{}", i),
                },
            );
            stream.push_message(msg).unwrap_or_else(|e| panic!("{e}"));
        }

        assert_eq!(stream.total_message_count(), 3);
        assert_eq!(stream.retained_message_count(), 3);
    }

    #[test]
    fn test_connection_stream_current_segment() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        // 没有段时
        assert!(stream.current_segment().is_none());

        // 创建初始段
        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        assert!(stream.current_segment().is_some());
        assert_eq!(
            stream
                .current_segment()
                .unwrap_or_else(|| panic!("expected Some"))
                .node_uuid,
            "node-1"
        );
    }

    #[test]
    fn test_connection_stream_serialization() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws:connect".to_string(),
        );

        let msg = WsMessage::with_timestamp(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
            1234567890,
        );
        stream.push_message(msg).unwrap_or_else(|e| panic!("{e}"));

        let json = serde_json::to_string(&stream).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains(r#""connection_id":"conn-1""#));
        assert!(json.contains(r#""url":"wss://example.com/ws""#));
        assert!(json.contains(r#""current_owner_node_uuid":"node-1""#));
        assert!(json.contains(r#""is_closed":false"#));

        // 反序列化
        let deserialized: WsConnectionStream =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized.connection_id, "conn-1");
        assert_eq!(deserialized.url, "wss://example.com/ws");
        assert_eq!(deserialized.segments.len(), 1);
    }

    #[test]
    fn test_connection_stream_default_values() {
        let stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        assert_eq!(
            stream.max_messages_per_segment,
            DEFAULT_MAX_MESSAGES_PER_SEGMENT
        );
        assert_eq!(stream.max_total_messages, DEFAULT_MAX_TOTAL_MESSAGES);
    }
}
