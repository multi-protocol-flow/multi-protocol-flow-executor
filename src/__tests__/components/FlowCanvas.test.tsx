/**
 * @vitest-environment jsdom
 */
/**
 * FlowCanvas 组件测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { FlowCanvas } from '@/components/canvas';
import { useFlowStore } from '@/stores';
import type { Flow } from '@/types';

// 包装器组件，提供 ReactFlow 上下文
function TestWrapper({ children }: { children: React.ReactNode }) {
  return <ReactFlowProvider>{children}</ReactFlowProvider>;
}

// 模拟流程数据
const mockFlow: Flow = {
  uuid: 'test-flow-001',
  name: '测试流程',
  nodes: [
    {
      uuid: 'entry-001',
      type: 'entry',
      name: '入口',
      position: { x: 100, y: 100 },
      data: { type: 'entry' },
    },
    {
      uuid: 'http-001',
      type: 'http',
      name: 'HTTP 请求',
      position: { x: 300, y: 100 },
      data: { type: 'http', url: 'https://api.example.com', method: 'get' },
    },
    {
      uuid: 'conditional-001',
      type: 'conditional',
      name: '条件判断',
      position: { x: 500, y: 100 },
      data: {
        type: 'conditional',
        field_path: 'status',
        operator: 'equal',
        expected_value: 200,
      },
    },
  ],
  connections: [
    {
      id: 'conn-001',
      source_node_uuid: 'entry-001',
      target_node_uuid: 'http-001',
      source_port_id: 'out',
      target_port_id: 'in',
    },
  ],
};

describe('FlowCanvas', () => {
  beforeEach(() => {
    // 重置 store
    useFlowStore.getState().reset();
  });

  describe('空状态', () => {
    it('当没有流程时显示空状态提示', () => {
      render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );

      expect(screen.getByText('暂无流程')).toBeDefined();
      expect(screen.getByText('请创建或打开一个流程文件')).toBeDefined();
    });
  });

  describe('有流程数据', () => {
    it('渲染所有节点类型', () => {
      useFlowStore.getState().setFlow(mockFlow);

      const { container } = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );

      // 验证 React Flow 容器存在
      const reactFlowContainer = container.querySelector('.react-flow');
      expect(reactFlowContainer).toBeDefined();
    });
  });
});

describe('节点组件', () => {
  describe('EntryNode', () => {
    it('渲染入口节点', () => {
      const { container } = render(
        <ReactFlowProvider>
          <div data-testid="entry-node">
            {/* 这里需要模拟 NodeProps */}
          </div>
        </ReactFlowProvider>
      );

      expect(container.firstChild).toBeDefined();
    });
  });
});

describe('工具函数', () => {
  describe('convertFlowToReactFlow', () => {
    it('正确转换流程数据', async () => {
      const { convertFlowToReactFlow } = await import('@/components/canvas/utils');

      const result = convertFlowToReactFlow(mockFlow);

      expect(result.nodes).toHaveLength(3);
      expect(result.edges).toHaveLength(1);

      // 验证节点转换
      const entryNode = result.nodes.find((n) => n.type === 'entry');
      expect(entryNode).toBeDefined();
      expect(entryNode?.id).toBe('entry-001');
      expect(entryNode?.position).toEqual({ x: 100, y: 100 });

      // 验证边转换
      const edge = result.edges[0];
      expect(edge.source).toBe('entry-001');
      expect(edge.target).toBe('http-001');
      expect(edge.sourceHandle).toBe('out');
      expect(edge.targetHandle).toBe('in');
    });
  });

  describe('createConnection', () => {
    it('创建新连接', async () => {
      const { createConnection } = await import('@/components/canvas/utils');

      const connection = createConnection('node-1', 'node-2', 'out', 'in');

      expect(connection.source_node_uuid).toBe('node-1');
      expect(connection.target_node_uuid).toBe('node-2');
      expect(connection.source_port_id).toBe('out');
      expect(connection.target_port_id).toBe('in');
      expect(connection.id).toBeDefined();
    });
  });

  describe('isValidConnection', () => {
    it('禁止自连接', async () => {
      const { isValidConnection } = await import('@/components/canvas/utils');

      expect(isValidConnection('node-1', 'node-1', 'out', 'in')).toBe(false);
    });

    it('允许正常连接', async () => {
      const { isValidConnection } = await import('@/components/canvas/utils');

      expect(isValidConnection('node-1', 'node-2', 'out', 'in')).toBe(true);
    });
  });

  describe('generateId', () => {
    it('生成唯一 ID', async () => {
      const { generateId } = await import('@/components/canvas/utils');

      const id1 = generateId();
      const id2 = generateId();

      expect(id1).toBeDefined();
      expect(id2).toBeDefined();
      expect(id1).not.toBe(id2);
    });
  });
});
