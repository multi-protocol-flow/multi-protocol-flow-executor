/**
 * 事件订阅服务测试
 * 测试 Mock 环境下的事件订阅功能
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { subscribeToFlowLogs } from '@/services/tauri/mock';
import { useConsoleStore } from '@/stores/console';
import type { LogEvent } from '@/types/events';

describe('事件订阅服务 (Mock)', () => {
  beforeEach(() => {
    // 重置 Console Store
    useConsoleStore.setState({
      logs: [],
      filter: {
        level: 'ALL',
        nodeUuid: 'ALL',
        searchKeyword: '',
      },
    });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('subscribeToFlowLogs', () => {
    it('应该返回 channel 对象和 cleanup 函数', () => {
      const onLog = vi.fn();
      const result = subscribeToFlowLogs(onLog);

      expect(result).toHaveProperty('channel');
      expect(result).toHaveProperty('cleanup');
      expect(result.channel).toEqual({ mock: true });
      expect(typeof result.cleanup).toBe('function');

      // 清理
      result.cleanup();
    });

    it('cleanup 函数应该可以安全调用', () => {
      const onLog = vi.fn();
      const { cleanup } = subscribeToFlowLogs(onLog);

      // 不应该抛出错误
      expect(() => cleanup()).not.toThrow();
    });

    it('应该定时发送模拟日志', () => {
      const onLog = vi.fn();
      const { cleanup } = subscribeToFlowLogs(onLog);

      // 初始时不应该有日志
      expect(onLog).not.toHaveBeenCalled();

      // 前进 1 秒
      vi.advanceTimersByTime(1000);
      expect(onLog).toHaveBeenCalledTimes(1);

      // 再前进 1 秒
      vi.advanceTimersByTime(1000);
      expect(onLog).toHaveBeenCalledTimes(2);

      cleanup();
    });

    it('日志事件应该包含正确的字段', () => {
      const onLog = vi.fn();
      const { cleanup } = subscribeToFlowLogs(onLog);

      vi.advanceTimersByTime(1000);

      expect(onLog).toHaveBeenCalledTimes(1);
      const logEvent = onLog.mock.calls[0][0] as LogEvent;

      // 验证必要字段
      expect(logEvent).toHaveProperty('level');
      expect(logEvent).toHaveProperty('message');
      expect(logEvent).toHaveProperty('timestamp');
      expect(logEvent).toHaveProperty('flowUuid');

      // 验证字段类型
      expect(['INFO', 'DEBUG', 'WARN', 'ERROR']).toContain(logEvent.level);
      expect(typeof logEvent.message).toBe('string');
      expect(typeof logEvent.timestamp).toBe('number');
      expect(typeof logEvent.flowUuid).toBe('string');

      cleanup();
    });

    it('日志消息应该包含 [Mock] 前缀', () => {
      const onLog = vi.fn();
      const { cleanup } = subscribeToFlowLogs(onLog);

      vi.advanceTimersByTime(1000);

      const logEvent = onLog.mock.calls[0][0] as LogEvent;
      expect(logEvent.message).toContain('[Mock]');

      cleanup();
    });
  });

  describe('取消订阅', () => {
    it('调用 cleanup 后应该停止发送日志', () => {
      const onLog = vi.fn();
      const { cleanup } = subscribeToFlowLogs(onLog);

      vi.advanceTimersByTime(1000);
      expect(onLog).toHaveBeenCalledTimes(1);

      // 调用 cleanup
      cleanup();

      // 再前进时间，不应该有新的日志
      vi.advanceTimersByTime(3000);
      expect(onLog).toHaveBeenCalledTimes(1);
    });
  });

  describe('与 Console Store 集成', () => {
    it('日志应该能添加到 Console Store', () => {
      const { cleanup } = subscribeToFlowLogs((log) => {
        useConsoleStore.getState().addLog(log);
      });

      vi.advanceTimersByTime(1000);

      const logs = useConsoleStore.getState().logs;
      expect(logs).toHaveLength(1);
      expect(logs[0]).toHaveProperty('id');
      expect(logs[0].message).toContain('[Mock]');

      cleanup();
    });

    it('多条日志应该正确累积', () => {
      const { cleanup } = subscribeToFlowLogs((log) => {
        useConsoleStore.getState().addLog(log);
      });

      // 模拟 3 秒
      vi.advanceTimersByTime(3000);

      const logs = useConsoleStore.getState().logs;
      expect(logs).toHaveLength(3);

      // 验证每条日志都有唯一 ID
      const ids = logs.map((l) => l.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(3);

      cleanup();
    });

    it('取消订阅后 Console Store 不应收到新日志', () => {
      const { cleanup } = subscribeToFlowLogs((log) => {
        useConsoleStore.getState().addLog(log);
      });

      vi.advanceTimersByTime(2000);
      expect(useConsoleStore.getState().logs).toHaveLength(2);

      cleanup();

      vi.advanceTimersByTime(5000);
      // 仍然是 2 条，没有新增
      expect(useConsoleStore.getState().logs).toHaveLength(2);
    });
  });
});
