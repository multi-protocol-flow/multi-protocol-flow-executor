/**
 * WebSocket 消息流类型定义
 * 与后端 src-tauri/crates/flow-engine/src/debug/ws_stream.rs 中的结构体对齐
 */

// ============================================================================
// 消息方向和内容类型
// ============================================================================

/**
 * 消息方向
 */
export type MessageDirection = 'sent' | 'received';

/**
 * 消息内容（判别联合类型）
 */
export type MessageContent =
  | { type: 'text'; content: string }
  | { type: 'binary'; content: number[] }; // binary 序列化为字节数组

// ============================================================================
// WebSocket 消息
// ============================================================================

/**
 * WebSocket 消息
 */
export interface WsMessage {
  /** 时间戳（Unix 时间戳，毫秒） */
  timestamp: number;
  /** 消息方向 */
  direction: MessageDirection;
  /** 消息内容 */
  content: MessageContent;
  /** 是否为二进制消息 */
  is_binary: boolean;
  /** 消息大小（字节） */
  size: number;
}

// ============================================================================
// WebSocket 流段
// ============================================================================

/**
 * WebSocket 流段
 */
export interface WsStreamSegment {
  /** 节点 UUID */
  node_uuid: string;
  /** 节点名称 */
  node_name: string;
  /** 节点类型 */
  node_type: string;
  /** 开始时间（Unix 时间戳，毫秒） */
  started_at: number;
  /** 结束时间（Unix 时间戳，毫秒） */
  ended_at: number | null;
  /** 消息列表 */
  messages: WsMessage[];
  /** 每段最大消息数 */
  max_messages: number;
  /** 丢弃消息数 */
  dropped_message_count: number;
}

/**
 * 流段摘要
 */
export interface WsSegmentSummary {
  /** 节点 UUID */
  node_uuid: string;
  /** 节点名称 */
  node_name: string;
  /** 节点类型 */
  node_type: string;
  /** 消息数量 */
  message_count: number;
}

// ============================================================================
// WebSocket 连接流
// ============================================================================

/**
 * WebSocket 连接流
 */
export interface WsConnectionStream {
  /** 连接 ID */
  connection_id: string;
  /** WebSocket URL */
  url: string;
  /** 当前所有者节点 UUID */
  current_owner_node_uuid: string | null;
  /** 流段列表 */
  segments: WsStreamSegment[];
  /** 每段最大消息数 */
  max_messages_per_segment: number;
  /** 总最大消息数 */
  max_total_messages: number;
  /** 是否已关闭 */
  is_closed: boolean;
}

// ============================================================================
// WebSocket 流摘要
// ============================================================================

/**
 * WebSocket 流摘要
 */
export interface WsStreamSummary {
  /** 连接 ID */
  connection_id: string;
  /** WebSocket URL */
  url: string;
  /** 总接收消息数 */
  total_received: number;
  /** 总发送消息数 */
  total_sent: number;
  /** 丢弃消息数 */
  dropped_messages: number;
  /** 流段摘要列表 */
  segments: WsSegmentSummary[];
}
