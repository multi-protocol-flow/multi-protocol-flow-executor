/**
 * Tauri IPC 封装测试
 * 测试 Mock 服务的 createFlow 和 executeFlowWithChannel 函数
 */

import { describe, it, expect, vi } from 'vitest';
import { createFlow, executeFlowWithChannel } from '@/services/tauri/mock';
import type { Flow } from '@/types/flow';

describe('Tauri IPC Mock 服务', () => {
  describe('createFlow', () => {
    it('应该创建带有正确名称的流程', async () => {
      const flow = await createFlow('测试流程');
      expect(flow.name).toBe('测试流程');
    });

    it('应该生成唯一的流程 UUID', async () => {
      const flow1 = await createFlow('流程1');
      const flow2 = await createFlow('流程2');
      expect(flow1.uuid).not.toBe(flow2.uuid);
    });

    it('应该创建包含开始节点的流程', async () => {
      const flow = await createFlow('测试流程');
      expect(flow.nodes).toHaveLength(1);
      expect(flow.nodes[0].type).toBe('entry');
      expect(flow.nodes[0].name).toBe('Start');
    });

    it('开始节点 ID 应该包含流程 UUID', async () => {
      const flow = await createFlow('测试流程');
      expect(flow.nodes[0].uuid).toContain('start-');
      expect(flow.nodes[0].uuid).toContain(flow.uuid);
    });

    it('应该初始化空的连接列表', async () => {
      const flow = await createFlow('测试流程');
      expect(flow.connections).toHaveLength(0);
    });

    it('应该初始化空的初始变量', async () => {
      const flow = await createFlow('测试流程');
      expect(flow.initial_variables).toEqual({});
    });
  });

  describe('executeFlowWithChannel', () => {
    // 创建测试用的流程
    function createTestFlow(withConnection: boolean): Flow {
      const flowId = 'test-flow-id';
      const startNode = {
        uuid: `start-${flowId}`,
        type: 'entry' as const,
        name: 'Start',
        position: { x: 0, y: 0 },
        data: { type: 'entry' as const },
      };

      const httpNode = {
        uuid: `http-${flowId}`,
        type: 'http:request' as const,
        name: 'HTTP 请求',
        position: { x: 200, y: 0 },
        data: {
          type: 'http:request' as const,
          url: 'https://example.com',
          method: 'get' as const,
        },
      };

      const connection = {
        id: `conn-${flowId}`,
        source_node_uuid: startNode.uuid,
        target_node_uuid: httpNode.uuid,
        source_port_id: 'output',
        target_port_id: 'input',
      };

      return {
        uuid: flowId,
        name: '测试流程',
        nodes: [startNode, httpNode],
        connections: withConnection ? [connection] : [],
        initial_variables: {},
      };
    }

    it('有连接时应该执行成功', async () => {
      const flow = createTestFlow(true);
      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(true);
      expect(result.error).toBeUndefined();
      expect(result.flow_report).toBeDefined();
      expect(result.flow_report?.flow_uuid).toBe(flow.uuid);
      expect(result.execution_time).toBeGreaterThanOrEqual(0);
    });

    it('无连接时应该执行成功（Entry 节点无连接是正常终止）', async () => {
      const flow = createTestFlow(false);
      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(true);
      expect(result.flow_report).toBeDefined();
    });

    it('缺少开始节点时应该返回错误', async () => {
      const flow: Flow = {
        uuid: 'test-flow-id',
        name: '测试流程',
        nodes: [],
        connections: [],
        initial_variables: {},
      };

      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(false);
      expect(result.error).toBe('Flow is missing a start node');
    });

    it('无 Entry 节点但有普通节点时应该通过回退自适应执行', async () => {
      const flow: Flow = {
        uuid: 'test-flow-id',
        name: '测试流程',
        nodes: [
          {
            uuid: 'http-node',
            type: 'http:request',
            name: 'HTTP 请求',
            position: { x: 0, y: 0 },
            data: { type: 'http:request', url: 'https://example.com' },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.success).toBe(true);
    });

    it('应该调用日志回调', async () => {
      const flow = createTestFlow(true);
      const onLog = vi.fn();
      await executeFlowWithChannel(flow, onLog);

      expect(onLog).toHaveBeenCalled();
      // 检查是否有流程开始日志
      expect(onLog).toHaveBeenCalledWith(
        expect.objectContaining({
          level: 'INFO',
          message: expect.stringContaining('started execution'),
        })
      );
    });

    it('应该返回执行时间', async () => {
      const flow = createTestFlow(true);
      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      expect(result.execution_time).toBeDefined();
      expect(typeof result.execution_time).toBe('number');
    });
  });
});
