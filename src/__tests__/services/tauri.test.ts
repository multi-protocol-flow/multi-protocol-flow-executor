/**
 * Tauri IPC 封装测试
 * 测试 Mock 服务的 createFlow 和 executeFlow 函数
 */

import { describe, it, expect } from 'vitest';
import { createFlow, executeFlow } from '@/services/tauri/mock';
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
      expect(flow.nodes[0].name).toBe('开始');
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

  describe('executeFlow', () => {
    // 创建测试用的流程
    function createTestFlow(withConnection: boolean): Flow {
      const flowId = 'test-flow-id';
      const startNode = {
        uuid: `start-${flowId}`,
        type: 'entry' as const,
        name: '开始',
        position: { x: 0, y: 0 },
        data: { type: 'entry' as const },
      };

      const httpNode = {
        uuid: `http-${flowId}`,
        type: 'http' as const,
        name: 'HTTP 请求',
        position: { x: 200, y: 0 },
        data: {
          type: 'http' as const,
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
      const result = await executeFlow(flow);

      expect(result.success).toBe(true);
      expect(result.error).toBeUndefined();
      expect(result.flow_report).toBeDefined();
      expect(result.flow_report?.flow_uuid).toBe(flow.uuid);
      expect(result.execution_time).toBeGreaterThanOrEqual(0);
    });

    it('无连接时应该返回错误', async () => {
      const flow = createTestFlow(false);
      const result = await executeFlow(flow);

      expect(result.success).toBe(false);
      expect(result.error).toBe('无连接');
      expect(result.flow_report).toBeUndefined();
    });

    it('缺少开始节点时应该返回错误', async () => {
      const flow: Flow = {
        uuid: 'test-flow-id',
        name: '测试流程',
        nodes: [
          {
            uuid: 'http-node',
            type: 'http',
            name: 'HTTP 请求',
            position: { x: 0, y: 0 },
            data: { type: 'http', url: 'https://example.com' },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      const result = await executeFlow(flow);

      expect(result.success).toBe(false);
      expect(result.error).toBe('流程缺少开始节点');
    });

    it('应该正确处理执行变量', async () => {
      const flow = createTestFlow(true);
      const variables = { api_key: 'test-key', timeout: 5000 };

      const result = await executeFlow(flow, variables);

      expect(result.success).toBe(true);
    });

    it('应该返回执行时间', async () => {
      const flow = createTestFlow(true);
      const result = await executeFlow(flow);

      expect(result.execution_time).toBeDefined();
      expect(typeof result.execution_time).toBe('number');
    });
  });
});
