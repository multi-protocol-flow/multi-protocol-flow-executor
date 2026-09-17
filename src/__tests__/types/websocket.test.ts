import { describe, it, expect } from 'vitest';
import type {
  WsCollectMode,
  WsMatchType,
  WsMessageType,
  WsCollectCondition,
  WsConnectConfig,
  WsSendCollectConfig,
  WsCloseConfig,
  NodeData,
} from '@/types/node';
import {
  isWsConnectConfig,
  isWsSendCollectConfig,
  isWsCloseConfig,
  isHttpConfig,
  isConditionalConfig,
  isEntryConfig,
} from '@/types/node';

describe('WebSocket Types', () => {
  describe('WsCollectMode', () => {
    it('should have valid collect modes', () => {
      const modes: WsCollectMode[] = ['first_match', 'condition_match', 'duration_collect'];
      expect(modes).toHaveLength(3);
    });
  });

  describe('WsMatchType', () => {
    it('should have valid match types', () => {
      const types: WsMatchType[] = ['json_path', 'regex', 'expression'];
      expect(types).toHaveLength(3);
    });
  });

  describe('WsMessageType', () => {
    it('should have valid message types', () => {
      const types: WsMessageType[] = ['text', 'binary'];
      expect(types).toHaveLength(2);
    });
  });

  describe('WsCollectCondition', () => {
    it('should create a valid collect condition', () => {
      const condition: WsCollectCondition = {
        match_type: 'json_path',
        expression: '$.data.status',
      };
      expect(condition.match_type).toBe('json_path');
      expect(condition.expression).toBe('$.data.status');
    });

    it('should support regex match type', () => {
      const condition: WsCollectCondition = {
        match_type: 'regex',
        expression: '^[0-9]+$',
      };
      expect(condition.match_type).toBe('regex');
      expect(condition.expression).toBe('^[0-9]+$');
    });

    it('should support expression match type', () => {
      const condition: WsCollectCondition = {
        match_type: 'expression',
        expression: 'value > 100',
      };
      expect(condition.match_type).toBe('expression');
      expect(condition.expression).toBe('value > 100');
    });
  });

  describe('WsConnectConfig', () => {
    it('should create a minimal WsConnectConfig with required fields', () => {
      const config: WsConnectConfig = {
        type: 'ws:connect',
        url: 'wss://example.com/ws',
      };
      expect(config.type).toBe('ws:connect');
      expect(config.url).toBe('wss://example.com/ws');
    });

    it('should create a full WsConnectConfig with all fields', () => {
      const config: WsConnectConfig = {
        type: 'ws:connect',
        url: 'wss://api.example.com/stream',
        headers: [
          { key: 'Authorization', value: 'Bearer token' },
        ],
        subprotocols: ['json', 'binary'],
        connect_timeout_ms: 30000,
        collect_message: true,
        collect_mode: 'condition_match',
        collect_condition: {
          match_type: 'json_path',
          expression: '$.ready',
        },
        collect_timeout_ms: 10000,
      };
      expect(config.url).toBe('wss://api.example.com/stream');
      expect(config.headers).toHaveLength(1);
      expect(config.subprotocols).toEqual(['json', 'binary']);
      expect(config.connect_timeout_ms).toBe(30000);
      expect(config.collect_message).toBe(true);
      expect(config.collect_mode).toBe('condition_match');
      expect(config.collect_condition?.match_type).toBe('json_path');
      expect(config.collect_timeout_ms).toBe(10000);
    });
  });

  describe('WsSendCollectConfig', () => {
    it('should create a minimal WsSendCollectConfig with required fields', () => {
      const config: WsSendCollectConfig = {
        type: 'ws:send_collect',
        connection_id: 'conn-123',
        message: '{"action": "ping"}',
        collect_response: false,
      };
      expect(config.type).toBe('ws:send_collect');
      expect(config.connection_id).toBe('conn-123');
      expect(config.message).toBe('{"action": "ping"}');
    });

    it('should create a full WsSendCollectConfig with all fields', () => {
      const config: WsSendCollectConfig = {
        type: 'ws:send_collect',
        connection_id: 'conn-456',
        message: '{"request": "data"}',
        message_type: 'binary',
        send_timeout_ms: 5000,
        collect_response: true,
        collect_mode: 'first_match',
        collect_condition: {
          match_type: 'regex',
          expression: 'success',
        },
        collect_timeout_ms: 8000,
      };
      expect(config.connection_id).toBe('conn-456');
      expect(config.message_type).toBe('binary');
      expect(config.send_timeout_ms).toBe(5000);
      expect(config.collect_response).toBe(true);
      expect(config.collect_mode).toBe('first_match');
      expect(config.collect_condition?.expression).toBe('success');
      expect(config.collect_timeout_ms).toBe(8000);
    });
  });

  describe('WsCloseConfig', () => {
    it('should create a minimal WsCloseConfig with required fields', () => {
      const config: WsCloseConfig = {
        type: 'ws:close',
        connection_id: 'conn-789',
      };
      expect(config.type).toBe('ws:close');
      expect(config.connection_id).toBe('conn-789');
    });

    it('should create a full WsCloseConfig with all fields', () => {
      const config: WsCloseConfig = {
        type: 'ws:close',
        connection_id: 'conn-abc',
        close_code: 1000,
        close_reason: 'Normal closure',
        close_timeout_ms: 5000,
      };
      expect(config.connection_id).toBe('conn-abc');
      expect(config.close_code).toBe(1000);
      expect(config.close_reason).toBe('Normal closure');
      expect(config.close_timeout_ms).toBe(5000);
    });
  });

  describe('Type Guards', () => {
    it('should correctly identify WsConnectConfig', () => {
      const config: NodeData = {
        type: 'ws:connect',
        url: 'wss://example.com',
      };
      expect(isWsConnectConfig(config)).toBe(true);
      expect(isWsSendCollectConfig(config)).toBe(false);
      expect(isWsCloseConfig(config)).toBe(false);
      expect(isHttpConfig(config)).toBe(false);
    });

    it('should correctly identify WsSendCollectConfig', () => {
      const config: NodeData = {
        type: 'ws:send_collect',
        connection_id: 'conn-1',
        message: 'test',
        collect_response: false,
      };
      expect(isWsSendCollectConfig(config)).toBe(true);
      expect(isWsConnectConfig(config)).toBe(false);
      expect(isWsCloseConfig(config)).toBe(false);
      expect(isConditionalConfig(config)).toBe(false);
    });

    it('should correctly identify WsCloseConfig', () => {
      const config: NodeData = {
        type: 'ws:close',
        connection_id: 'conn-2',
      };
      expect(isWsCloseConfig(config)).toBe(true);
      expect(isWsConnectConfig(config)).toBe(false);
      expect(isWsSendCollectConfig(config)).toBe(false);
      expect(isEntryConfig(config)).toBe(false);
    });

    it('should not confuse WebSocket types with other types', () => {
      const httpConfig: NodeData = {
        type: 'http:request',
        url: 'https://example.com',
      };
      expect(isWsConnectConfig(httpConfig)).toBe(false);

      const entryConfig: NodeData = {
        type: 'entry',
      };
      expect(isWsSendCollectConfig(entryConfig)).toBe(false);

      const conditionalConfig: NodeData = {
        type: 'conditional',
      };
      expect(isWsCloseConfig(conditionalConfig)).toBe(false);
    });
  });
});
