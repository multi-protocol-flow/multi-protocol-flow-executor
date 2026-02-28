/**
 * 事件订阅服务测试
 * 测试 Mock 环境下的 executeFlowWithChannel 功能
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { executeFlowWithChannel } from '@/services/tauri/mock';
import { useConsoleStore } from '@/stores/console';
import type { Flow } from '@/types/flow';

// 创建测试用的流程对象
const createTestFlow = (hasConnection = false): Flow => ({
  uuid: 'test-flow-uuid',
  name: '测试流程',
  nodes: [
    { uuid: 'start-node', type: 'entry', name: '开始', position: { x: 0, y: 0 }, data: { type: 'entry' } },
  ],
  connections: hasConnection
    ? [{ id: 'conn-1', source_node_uuid: 'start-node', target_node_uuid: 'other-node', source_port_id: 'out', target_port_id: 'in' }]
    : [],
  initial_variables: {},
});

describe('executeFlowWithChannel (Mock)', () => {
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
  });

  describe('执行流程', () => {
    it('应该返回执行结果', async () => {
      const flow = createTestFlow();
      const onLog = vi.fn();

      const result = await executeFlowWithChannel(flow, onLog);

      expect(result).toHaveProperty('success');
      expect(result).toHaveProperty('execution_time');
    });

    it('没有开始节点时应该返回失败', async () => {
      const flow: Flow = {
        uuid: 'test-flow',
        name: '测试流程',
        nodes: [{ uuid: 'http-1', type: 'http', name: 'HTTP', position: { x: 0, y: 0 }, data: { type: 'http', url: 'https://example.com', method: 'get' } }],
        connections: [],
        initial_variables: {},
      };
      const onLog = vi.fn();

      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(false);
      expect(result.error).toContain('缺少开始节点');
    });

    it('单个 Entry 节点无连接应该成功', async () => {
      const flow = createTestFlow(false);
      const onLog = vi.fn();

      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(true);
    });
  });

  describe('日志回调', () => {
    it('应该发送流程开始日志', async () => {
      const flow = createTestFlow();
      const onLog = vi.fn();

      await executeFlowWithChannel(flow, onLog);

      const startLog = onLog.mock.calls.find(
        (call) => call[0].message.includes('开始执行')
      );
      expect(startLog).toBeDefined();
    });

    it('应该发送流程结束日志', async () => {
      const flow = createTestFlow();
      const onLog = vi.fn();

      await executeFlowWithChannel(flow, onLog);

      const endLog = onLog.mock.calls.find(
        (call) => call[0].message.includes('执行成功')
      );
      expect(endLog).toBeDefined();
    });

    it('日志事件应该包含正确的 flowUuid', async () => {
      const flow = createTestFlow();
      const onLog = vi.fn();

      await executeFlowWithChannel(flow, onLog);

      onLog.mock.calls.forEach((call) => {
        expect(call[0].flowUuid).toBe(flow.uuid);
      });
    });

    it('日志事件应该包含正确的字段', async () => {
      const flow = createTestFlow();
      const onLog = vi.fn();

      await executeFlowWithChannel(flow, onLog);

      const logEvent = onLog.mock.calls[0][0];

      expect(logEvent).toHaveProperty('level');
      expect(logEvent).toHaveProperty('message');
      expect(logEvent).toHaveProperty('timestamp');
      expect(logEvent).toHaveProperty('flowUuid');
      expect(['INFO', 'DEBUG', 'WARN', 'ERROR']).toContain(logEvent.level);
      expect(typeof logEvent.message).toBe('string');
      expect(typeof logEvent.timestamp).toBe('number');
    });
  });

  describe('与 Console Store 集成', () => {
    it('日志应该能添加到 Console Store', async () => {
      const flow = createTestFlow();

      await executeFlowWithChannel(flow, (log) => {
        useConsoleStore.getState().addLog(log);
      });

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBeGreaterThan(0);
      expect(logs[0]).toHaveProperty('id');
    });

    it('多次执行应该替换日志（每次执行前清空）', async () => {
      const flow = createTestFlow();

      // 第一次执行
      await executeFlowWithChannel(flow, (log) => {
        useConsoleStore.getState().addLog(log);
      });
      const firstLogCount = useConsoleStore.getState().logs.length;

      // 清空后第二次执行
      useConsoleStore.getState().clearLogs();
      await executeFlowWithChannel(flow, (log) => {
        useConsoleStore.getState().addLog(log);
      });
      const secondLogCount = useConsoleStore.getState().logs.length;

      expect(firstLogCount).toBeGreaterThan(0);
      expect(secondLogCount).toBeGreaterThan(0);
      // 日志数量应该相同（同样的流程）
      expect(firstLogCount).toBe(secondLogCount);
    });
  });
});
