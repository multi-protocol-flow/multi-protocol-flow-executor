/**
 * FlowCanvas 拖放功能测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { FlowCanvas } from '@/components/canvas';
import { useFlowStore } from '@/stores';
import type { Flow } from '@/types';

// Mock useReactFlow
vi.mock('@xyflow/react', async () => {
  const actual = await vi.importActual('@xyflow/react');
  return {
    ...actual,
    useReactFlow: () => ({
      screenToFlowPosition: ({ x, y }: { x: number; y: number }) => ({ x, y }),
      getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
      setCenter: vi.fn(),
    }),
  };
});

// Mock crypto.randomUUID
vi.stubGlobal('crypto', {
  randomUUID: () => 'test-uuid-' + Math.random().toString(36).slice(2),
});

function TestWrapper({ children }: { children: React.ReactNode }) {
  return <ReactFlowProvider>{children}</ReactFlowProvider>;
}

const mockFlow: Flow = {
  uuid: 'test-flow',
  name: 'Test Flow',
  nodes: [
    {
      uuid: 'entry-1',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' },
    },
  ],
  connections: [],
  initial_variables: {},
};

describe('FlowCanvas - Drag and Drop', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  it('应该添加 HTTP 节点当拖放 http 类型时', () => {
    useFlowStore.getState().setFlow(mockFlow);

    const { container } = render(
      <TestWrapper>
        <FlowCanvas />
      </TestWrapper>
    );

    const reactFlowElement = container.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    // 创建 drop 事件
    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/nodeType') return 'http';
          return '';
        },
      },
      clientX: 200,
      clientY: 150,
    });

    reactFlowElement.dispatchEvent(dropEvent);

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('http');
  });

  it('应该添加 Conditional 节点当拖放 conditional 类型时', () => {
    useFlowStore.getState().setFlow(mockFlow);

    const { container } = render(
      <TestWrapper>
        <FlowCanvas />
      </TestWrapper>
    );

    const reactFlowElement = container.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/nodeType') return 'conditional';
          return '';
        },
      },
      clientX: 300,
      clientY: 200,
    });

    reactFlowElement.dispatchEvent(dropEvent);

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('conditional');
  });

  it('当没有流程时不应该添加节点', () => {
    const { container } = render(
      <TestWrapper>
        <FlowCanvas />
      </TestWrapper>
    );

    // 不应该有 .react-flow 元素
    const reactFlowElement = container.querySelector('.react-flow');
    expect(reactFlowElement).toBeNull();
  });
});
