/**
 * Events 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import type { LogLevel, LogEvent } from '@/types/events';

describe('Events types', () => {
  describe('LogLevel type', () => {
    it('should accept INFO level', () => {
      const level: LogLevel = 'INFO';
      expect(level).toBe('INFO');
    });

    it('should accept DEBUG level', () => {
      const level: LogLevel = 'DEBUG';
      expect(level).toBe('DEBUG');
    });

    it('should accept WARN level', () => {
      const level: LogLevel = 'WARN';
      expect(level).toBe('WARN');
    });

    it('should accept ERROR level', () => {
      const level: LogLevel = 'ERROR';
      expect(level).toBe('ERROR');
    });
  });

  describe('LogEvent interface', () => {
    it('should have correct structure with required fields', () => {
      const event: LogEvent = {
        level: 'INFO',
        message: 'Flow execution started',
        timestamp: 1709500000000,
        flowUuid: 'flow-1',
      };

      expect(event.level).toBe('INFO');
      expect(event.message).toBe('Flow execution started');
      expect(event.timestamp).toBe(1709500000000);
      expect(event.flowUuid).toBe('flow-1');
    });

    it('should accept optional node fields', () => {
      const event: LogEvent = {
        level: 'DEBUG',
        message: 'HTTP request completed',
        timestamp: 1709500001000,
        nodeUuid: 'node-1',
        nodeName: 'Get Users',
        flowUuid: 'flow-1',
      };

      expect(event.nodeUuid).toBe('node-1');
      expect(event.nodeName).toBe('Get Users');
    });

    it('should accept error level with node context', () => {
      const event: LogEvent = {
        level: 'ERROR',
        message: 'Connection timeout',
        timestamp: 1709500002000,
        nodeUuid: 'node-2',
        nodeName: 'External API',
        flowUuid: 'flow-1',
      };

      expect(event.level).toBe('ERROR');
      expect(event.message).toBe('Connection timeout');
      expect(event.nodeUuid).toBe('node-2');
    });

    it('should accept warn level without node context', () => {
      const event: LogEvent = {
        level: 'WARN',
        message: 'Flow execution completed with warnings',
        timestamp: 1709500003000,
        flowUuid: 'flow-2',
      };

      expect(event.level).toBe('WARN');
      expect(event.nodeUuid).toBeUndefined();
    });
  });
});
