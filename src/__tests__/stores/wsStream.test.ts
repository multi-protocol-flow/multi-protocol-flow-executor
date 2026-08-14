/**
 * WebSocket 流 Store 单元测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useWsStreamStore, selectWsStreams, selectActiveWsConnectionId } from '@/stores/wsStream';
import type { DebugEvent } from '@/types/debug';
import type { WsStreamSegment } from '@/types/ws_stream';

/** 构造 WS 连接创建事件 */
function createCreatedEvent(connectionId = 'conn-1'): DebugEvent {
  return {
    WsStreamCreated: {
      connection_id: connectionId,
      url: 'ws://localhost:8080/ws',
      owner_node_uuid: 'node-a',
      owner_node_name: 'WS 连接',
    },
  };
}

describe('WsStreamStore', () => {
  beforeEach(() => {
    useWsStreamStore.getState().reset();
  });

  describe('初始状态', () => {
    it('初始为空 Map 且无活动连接', () => {
      const state = useWsStreamStore.getState();
      expect(state.wsStreams.size).toBe(0);
      expect(state.activeWsConnectionId).toBeNull();
      expect(selectWsStreams(state)).toBe(state.wsStreams);
      expect(selectActiveWsConnectionId(state)).toBeNull();
    });
  });

  describe('handleWsEvent - WsStreamCreated', () => {
    it('创建连接流并设为活动连接', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());

      const { wsStreams, activeWsConnectionId } = useWsStreamStore.getState();
      const stream = wsStreams.get('conn-1');
      expect(stream).toBeDefined();
      expect(stream?.url).toBe('ws://localhost:8080/ws');
      expect(stream?.segments).toHaveLength(1);
      expect(stream?.segments[0].node_type).toBe('ws_connect');
      expect(stream?.segments[0].ended_at).toBeNull();
      expect(stream?.is_closed).toBe(false);
      expect(activeWsConnectionId).toBe('conn-1');
    });
  });

  describe('handleWsEvent - WsMessageReceived / WsMessageSent', () => {
    it('接收消息追加到最近打开的段', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      const received: DebugEvent = {
        WsMessageReceived: {
          connection_id: 'conn-1',
          owner_node_uuid: 'node-a',
          timestamp: 1000,
          direction: 'received',
          content: { type: 'text', content: 'hello' },
          is_binary: false,
          size: 5,
        },
      };
      useWsStreamStore.getState().handleWsEvent(received);

      const stream = useWsStreamStore.getState().wsStreams.get('conn-1');
      expect(stream?.segments[0].messages).toHaveLength(1);
      expect(stream?.segments[0].messages[0].content).toEqual({ type: 'text', content: 'hello' });
      expect(stream?.segments[0].messages[0].direction).toBe('received');
    });

    it('未知连接的消息被忽略', () => {
      const event: DebugEvent = {
        WsMessageSent: {
          connection_id: 'unknown-conn',
          owner_node_uuid: 'node-a',
          timestamp: 1000,
          direction: 'sent',
          content: { type: 'text', content: 'hi' },
          is_binary: false,
          size: 2,
        },
      };
      useWsStreamStore.getState().handleWsEvent(event);
      expect(useWsStreamStore.getState().wsStreams.size).toBe(0);
    });
  });

  describe('handleWsEvent - 段事件', () => {
    it('WsSegmentStarted 追加新段并更新所有者', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      const newSegment: WsStreamSegment = {
        node_uuid: 'node-b',
        node_name: 'WS 收集',
        node_type: 'ws_send_collect',
        started_at: 2000,
        ended_at: null,
        messages: [],
        max_messages: 1000,
        dropped_message_count: 0,
      };
      useWsStreamStore.getState().handleWsEvent({
        WsSegmentStarted: { connection_id: 'conn-1', segment: newSegment },
      });

      const stream = useWsStreamStore.getState().wsStreams.get('conn-1');
      expect(stream?.segments).toHaveLength(2);
      expect(stream?.current_owner_node_uuid).toBe('node-b');
    });

    it('WsStreamOwnerChanged 结束上一段并创建新段', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      useWsStreamStore.getState().handleWsEvent({
        WsStreamOwnerChanged: {
          connection_id: 'conn-1',
          from_node_uuid: 'node-a',
          to_node_uuid: 'node-b',
        },
      });

      const stream = useWsStreamStore.getState().wsStreams.get('conn-1');
      expect(stream?.segments).toHaveLength(2);
      expect(stream?.segments[0].ended_at).not.toBeNull();
      expect(stream?.segments[1].node_type).toBe('ws_send_collect');
      expect(stream?.current_owner_node_uuid).toBe('node-b');
    });

    it('WsSegmentEnded 替换匹配段为已结束状态', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      const endedSegment: WsStreamSegment = {
        node_uuid: 'node-a',
        node_name: 'WS 连接',
        node_type: 'ws_connect',
        started_at: expect.any(Number) as unknown as number,
        ended_at: 3000,
        messages: [],
        max_messages: 1000,
        dropped_message_count: 0,
      };
      const streamBefore = useWsStreamStore.getState().wsStreams.get('conn-1');
      useWsStreamStore.getState().handleWsEvent({
        WsSegmentEnded: {
          connection_id: 'conn-1',
          segment: { ...endedSegment, started_at: streamBefore?.segments[0].started_at ?? 0 },
        },
      });

      const stream = useWsStreamStore.getState().wsStreams.get('conn-1');
      expect(stream?.segments[0].ended_at).toBe(3000);
    });
  });

  describe('handleWsEvent - WsStreamClosed', () => {
    it('关闭流并清除活动连接', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      useWsStreamStore.getState().handleWsEvent({
        WsStreamClosed: { connection_id: 'conn-1', last_owner_node_uuid: 'node-a' },
      });

      const { wsStreams, activeWsConnectionId } = useWsStreamStore.getState();
      const stream = wsStreams.get('conn-1');
      expect(stream?.is_closed).toBe(true);
      expect(stream?.segments[0].ended_at).not.toBeNull();
      expect(stream?.current_owner_node_uuid).toBeNull();
      expect(activeWsConnectionId).toBeNull();
    });
  });

  describe('非 WS 事件', () => {
    it('其他 DebugEvent 变体被忽略', () => {
      useWsStreamStore.getState().handleWsEvent({ SessionStarted: { flow_uuid: 'flow-1' } });
      expect(useWsStreamStore.getState().wsStreams.size).toBe(0);
      expect(useWsStreamStore.getState().activeWsConnectionId).toBeNull();
    });
  });

  describe('setActiveWsConnection / resetWsStreams / reset', () => {
    it('setActiveWsConnection 更新活动连接', () => {
      useWsStreamStore.getState().setActiveWsConnection('conn-9');
      expect(useWsStreamStore.getState().activeWsConnectionId).toBe('conn-9');
      useWsStreamStore.getState().setActiveWsConnection(null);
      expect(useWsStreamStore.getState().activeWsConnectionId).toBeNull();
    });

    it('resetWsStreams 清空流与活动连接', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      useWsStreamStore.getState().resetWsStreams();
      expect(useWsStreamStore.getState().wsStreams.size).toBe(0);
      expect(useWsStreamStore.getState().activeWsConnectionId).toBeNull();
    });

    it('reset 还原初始状态', () => {
      useWsStreamStore.getState().handleWsEvent(createCreatedEvent());
      useWsStreamStore.getState().setActiveWsConnection('conn-1');
      useWsStreamStore.getState().reset();
      const state = useWsStreamStore.getState();
      expect(state.wsStreams.size).toBe(0);
      expect(state.activeWsConnectionId).toBeNull();
    });
  });
});
