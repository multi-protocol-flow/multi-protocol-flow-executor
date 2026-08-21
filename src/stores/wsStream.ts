/**
 * WebSocket 流状态管理
 *
 * 从 executionStore 和 debugStore 中提取的共享 WS 流逻辑。
 * 处理所有 DebugEvent 中的 WS 变体。
 *
 * T20: 性能优化分析
 * 当前 handleWsEvent 中每个事件都创建新的 Map（new Map(state.wsStreams)），
 * 高频 WS 消息时 Map 克隆开销较大。
 *
 * 方案一：Immer 中间件
 *   使用 zustand immer 中间件可消除手动 Map 克隆：
 *   import { immer } from 'zustand/middleware/immer';
 *   const handleWsEvent = (event) => {
 *     set(produce((state) => {
 *       const stream = state.wsStreams.get(connection_id);
 *       if (!stream) return;
 *       stream.segments.at(-1)?.messages.push(message);
 *     }));
 *   };
 *   优点：代码更简洁，避免冗余克隆
 *   缺点：immer 的 Proxy 在 Map 上的性能不如普通对象；增加 bundle 体积
 *
 * 方案二：消息缓冲 + 节流
 *   引入 messageBuffer 队列，每 50ms 批量 flush：
 *   let msgBuffer: { connection_id: string; message: WsMessage }[] = [];
 *   let flushTimer: ReturnType<typeof setInterval> | null = null;
 *   const startBatchFlush = (set, interval = 50) => {
 *     flushTimer = setInterval(() => {
 *       if (msgBuffer.length === 0) return;
 *       const batch = msgBuffer.splice(0);
 *       set((state) => { /* 批量处理 batch *\/ });
 *     }, interval);
 *   };
 *   优点：减少 setState 频率，合并 DOM 更新
 *   缺点：消息展示有 50ms 延迟；WS 流面板需要兼容延迟更新
 *
 * 建议：当前实现可接受（HTTP 场景下 WS 消息频率通常不高），
 * 若未来需要处理高频 WS 流场景，推荐方案二（消息缓冲）以最小化侵入性改动。
 */
import { create } from 'zustand';
import type { DebugEvent } from '@/types/debug';
import type { WsMessage, WsConnectionStream, WsStreamSegment } from '@/types/ws_stream';

interface WsStreamState {
  /** WebSocket 连接流（connection_id → WsConnectionStream） */
  wsStreams: Map<string, WsConnectionStream>;
  /** 当前活动 WS 连接 ID */
  activeWsConnectionId: string | null;

  // ===== Actions =====
  /** 处理 WS 流事件（DebugEvent 中的 WS 变体） */
  handleWsEvent: (event: DebugEvent) => void;
  /** 设置活动 WS 连接 */
  setActiveWsConnection: (connectionId: string | null) => void;
  /** 重置 WS 流状态 */
  resetWsStreams: () => void;
  /** 重置全部状态 */
  reset: () => void;
}

const initialWsStreamState = {
  wsStreams: new Map<string, WsConnectionStream>(),
  activeWsConnectionId: null as string | null,
};

export const useWsStreamStore = create<WsStreamState>((set, _get) => ({
  ...initialWsStreamState,

  handleWsEvent: (event: DebugEvent) => {
    if ('WsStreamCreated' in event) {
      // WebSocket 连接创建事件
      const { connection_id, url, owner_node_uuid, owner_node_name } = event.WsStreamCreated;
      const stream: WsConnectionStream = {
        connection_id,
        url,
        current_owner_node_uuid: owner_node_uuid,
        segments: [{
          node_uuid: owner_node_uuid,
          node_name: owner_node_name,
          node_type: 'ws_connect',
          started_at: Date.now(),
          ended_at: null,
          messages: [],
          max_messages: 1000,
          dropped_message_count: 0,
        }],
        max_messages_per_segment: 1000,
        max_total_messages: 10000,
        is_closed: false,
      };
      set((state) => {
        const newStreams = new Map(state.wsStreams);
        newStreams.set(stream.connection_id, stream);
        return {
          wsStreams: newStreams,
          activeWsConnectionId: state.activeWsConnectionId || stream.connection_id,
        };
      });
    } else if ('WsMessageReceived' in event) {
      // WebSocket 消息接收事件（后端 #[serde(flatten)] 展平 message 字段）
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageReceived;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      void _owner;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = stream.segments.map((s, idx) => {
          if (idx === stream.segments.length - 1 && s.ended_at === null) {
            const nextMessages = [...s.messages, message];
            const max = s.max_messages;
            if (nextMessages.length > max) {
              const dropped = s.dropped_message_count + (nextMessages.length - max);
              return {
                ...s,
                messages: nextMessages.slice(-max),
                dropped_message_count: dropped,
              };
            }
            return {
              ...s,
              messages: nextMessages,
            };
          }
          return s;
        });

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsMessageSent' in event) {
      // WebSocket 消息发送事件（后端 #[serde(flatten)] 展平 message 字段）
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageSent;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      void _owner;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = stream.segments.map((s, idx) => {
          if (idx === stream.segments.length - 1 && s.ended_at === null) {
            const nextMessages = [...s.messages, message];
            const max = s.max_messages;
            if (nextMessages.length > max) {
              const dropped = s.dropped_message_count + (nextMessages.length - max);
              return {
                ...s,
                messages: nextMessages.slice(-max),
                dropped_message_count: dropped,
              };
            }
            return {
              ...s,
              messages: nextMessages,
            };
          }
          return s;
        });

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsSegmentStarted' in event) {
      // WebSocket 流段开始事件
      const { connection_id, segment } = event.WsSegmentStarted;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = [...stream.segments, segment];
        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments,
          current_owner_node_uuid: segment.node_uuid,
        });
        return { wsStreams: newStreams };
      });
    } else if ('WsSegmentEnded' in event) {
      // WebSocket 流段结束事件
      const { connection_id, segment } = event.WsSegmentEnded;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = stream.segments.map(s =>
          s.node_uuid === segment.node_uuid && s.started_at === segment.started_at
            ? segment
            : s
        );
        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsStreamOwnerChanged' in event) {
      // WebSocket 流所有权变更事件
      const { connection_id, to_node_uuid } = event.WsStreamOwnerChanged;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const now = Date.now();

        // 结束上一个 segment
        const updatedSegments = stream.segments.map((s, idx) => {
          if (idx === stream.segments.length - 1 && s.ended_at === null) {
            return { ...s, ended_at: now };
          }
          return s;
        });

        // 添加新 segment
        const newSegment: WsStreamSegment = {
          node_uuid: to_node_uuid,
          node_name: '',
          node_type: 'ws_send_collect',
          started_at: now,
          ended_at: null,
          messages: [],
          max_messages: 1000,
          dropped_message_count: 0,
        };

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments: [...updatedSegments, newSegment],
          current_owner_node_uuid: to_node_uuid,
        });
        return { wsStreams: newStreams };
      });
    } else if ('WsStreamClosed' in event) {
      // WebSocket 流关闭事件
      const { connection_id } = event.WsStreamClosed;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        // 结束当前活动的 segment
        const now = Date.now();
        const segments = stream.segments.map((s, idx) => {
          if (idx === stream.segments.length - 1 && s.ended_at === null) {
            return { ...s, ended_at: now };
          }
          return s;
        });

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments,
          is_closed: true,
          current_owner_node_uuid: null,
        });

        const newActiveWsConnectionId = state.activeWsConnectionId === connection_id
          ? null
          : state.activeWsConnectionId;

        return {
          wsStreams: newStreams,
          activeWsConnectionId: newActiveWsConnectionId,
        };
      });
    }
    // 其他 DebugEvent 变体（PausedAtNode、Heartbeat 等）忽略
  },

  setActiveWsConnection: (connectionId) => set({ activeWsConnectionId: connectionId }),

  resetWsStreams: () => set({
    wsStreams: new Map(),
    activeWsConnectionId: null,
  }),

  reset: () => set(initialWsStreamState),
}));

// ===== Selectors =====
/** 选择 WS 连接流映射 */
export const selectWsStreams = (state: WsStreamState) => state.wsStreams;
/** 选择活动 WS 连接 ID */
export const selectActiveWsConnectionId = (state: WsStreamState) => state.activeWsConnectionId;
