import { describe, it, expect } from 'vitest';
import type {
  WsWaitMode,
  WsMatchType,
  WsMessageType,
  WsWaitCondition,
  WsConnectConfig,
  WsSendWaitConfig,
  WsCloseConfig,
  NodeData,
} from '@/types/node';
import {
  isWsConnectConfig,
  isWsSendWaitConfig,
  isWsCloseConfig,
  isHttpConfig,
  isConditionalConfig,
  isEntryConfig,
} from '@/types/node';

describe('WebSocket Types', () => {
  describe('WsWaitMode', () => {
    it('should have valid wait modes', () => {
      const modes: WsWaitMode[] = ['none', 'first_message', 'condition', 'duration'];
      expect(modes).toHaveLength(4);
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

  describe('WsWaitCondition', () => {
    it('should create a valid wait condition', () => {
      const condition: WsWaitCondition = {
        match_type: 'json_path',
        expression: '$.data.status',
      };
      expect(condition.match_type).toBe('json_path');
      expect(condition.expression).toBe('$.data.status');
    });

    it('should support regex match type', () => {
      const condition: WsWaitCondition = {
        match_type: 'regex',
        expression: '^[0-9]+$',
      };
      expect(condition.match_type).toBe('regex');
      expect(condition.expression).toBe('^[0-9]+$');
    });

    it('should support expression match type', () => {
      const condition: WsWaitCondition = {
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
        type: 'ws_connect',
        url: 'wss://example.com/ws',
      };
      expect(config.type).toBe('ws_connect');
      expect(config.url).toBe('wss://example.com/ws');
    });

    it('should create a full WsConnectConfig with all fields', () => {
      const config: WsConnectConfig = {
        type: 'ws_connect',
        url: 'wss://api.example.com/stream',
        headers: [
          { key: 'Authorization', value: 'Bearer token' },
        ],
        subprotocols: ['json', 'binary'],
        connect_timeout_ms: 30000,
        wait_for_message: true,
        wait_mode: 'condition',
        wait_condition: {
          match_type: 'json_path',
          expression: '$.ready',
        },
        message_timeout_ms: 10000,
        output_mappings: [
          { source: 'message', target: 'ws_response' },
        ],
      };
      expect(config.url).toBe('wss://api.example.com/stream');
      expect(config.headers).toHaveLength(1);
      expect(config.subprotocols).toEqual(['json', 'binary']);
      expect(config.connect_timeout_ms).toBe(30000);
      expect(config.wait_for_message).toBe(true);
      expect(config.wait_mode).toBe('condition');
      expect(config.wait_condition?.match_type).toBe('json_path');
      expect(config.message_timeout_ms).toBe(10000);
      expect(config.output_mappings).toHaveLength(1);
    });
  });

  describe('WsSendWaitConfig', () => {
    it('should create a minimal WsSendWaitConfig with required fields', () => {
      const config: WsSendWaitConfig = {
        type: 'ws_send_wait',
        connection_id: 'conn-123',
        message: '{"action": "ping"}',
      };
      expect(config.type).toBe('ws_send_wait');
      expect(config.connection_id).toBe('conn-123');
      expect(config.message).toBe('{"action": "ping"}');
    });

    it('should create a full WsSendWaitConfig with all fields', () => {
      const config: WsSendWaitConfig = {
        type: 'ws_send_wait',
        connection_id: 'conn-456',
        message: '{"request": "data"}',
        message_type: 'binary',
        send_timeout_ms: 5000,
        wait_for_response: true,
        wait_mode: 'first_message',
        wait_condition: {
          match_type: 'regex',
          expression: 'success',
        },
        message_timeout_ms: 8000,
        output_mappings: [
          { source: 'response', target: 'ws_result' },
        ],
      };
      expect(config.connection_id).toBe('conn-456');
      expect(config.message_type).toBe('binary');
      expect(config.send_timeout_ms).toBe(5000);
      expect(config.wait_for_response).toBe(true);
      expect(config.wait_mode).toBe('first_message');
      expect(config.wait_condition?.expression).toBe('success');
      expect(config.message_timeout_ms).toBe(8000);
      expect(config.output_mappings).toHaveLength(1);
    });
  });

  describe('WsCloseConfig', () => {
    it('should create a minimal WsCloseConfig with required fields', () => {
      const config: WsCloseConfig = {
        type: 'ws_close',
        connection_id: 'conn-789',
      };
      expect(config.type).toBe('ws_close');
      expect(config.connection_id).toBe('conn-789');
    });

    it('should create a full WsCloseConfig with all fields', () => {
      const config: WsCloseConfig = {
        type: 'ws_close',
        connection_id: 'conn-abc',
        close_code: 1000,
        close_reason: 'Normal closure',
        wait_close_timeout_ms: 5000,
      };
      expect(config.connection_id).toBe('conn-abc');
      expect(config.close_code).toBe(1000);
      expect(config.close_reason).toBe('Normal closure');
      expect(config.wait_close_timeout_ms).toBe(5000);
    });
  });

  describe('Type Guards', () => {
    it('should correctly identify WsConnectConfig', () => {
      const config: NodeData = {
        type: 'ws_connect',
        url: 'wss://example.com',
      };
      expect(isWsConnectConfig(config)).toBe(true);
      expect(isWsSendWaitConfig(config)).toBe(false);
      expect(isWsCloseConfig(config)).toBe(false);
      expect(isHttpConfig(config)).toBe(false);
    });

    it('should correctly identify WsSendWaitConfig', () => {
      const config: NodeData = {
        type: 'ws_send_wait',
        connection_id: 'conn-1',
        message: 'test',
      };
      expect(isWsSendWaitConfig(config)).toBe(true);
      expect(isWsConnectConfig(config)).toBe(false);
      expect(isWsCloseConfig(config)).toBe(false);
      expect(isConditionalConfig(config)).toBe(false);
    });

    it('should correctly identify WsCloseConfig', () => {
      const config: NodeData = {
        type: 'ws_close',
        connection_id: 'conn-2',
      };
      expect(isWsCloseConfig(config)).toBe(true);
      expect(isWsConnectConfig(config)).toBe(false);
      expect(isWsSendWaitConfig(config)).toBe(false);
      expect(isEntryConfig(config)).toBe(false);
    });

    it('should not confuse WebSocket types with other types', () => {
      const httpConfig: NodeData = {
        type: 'http',
        url: 'https://example.com',
      };
      expect(isWsConnectConfig(httpConfig)).toBe(false);

      const entryConfig: NodeData = {
        type: 'entry',
      };
      expect(isWsSendWaitConfig(entryConfig)).toBe(false);

      const conditionalConfig: NodeData = {
        type: 'conditional',
      };
      expect(isWsCloseConfig(conditionalConfig)).toBe(false);
    });
  });
});
