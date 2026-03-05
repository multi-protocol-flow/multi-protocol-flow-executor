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
      fitView: vi.fn(),
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
    // 验证节点位置使用鼠标释放位置
    expect(state.flow?.nodes[1].position).toEqual({ x: 200, y: 150 });
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
    // 验证节点位置使用鼠标释放位置
    expect(state.flow?.nodes[1].position).toEqual({ x: 300, y: 200 });
  });

  it('应该添加 WS Connect 节点当拖放 ws_connect 类型时', () => {
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
          if (key === 'application/reactflow/nodeType') return 'ws_connect';
          return '';
        },
      },
      clientX: 250,
      clientY: 180,
    });

    reactFlowElement.dispatchEvent(dropEvent);

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_connect');
    expect(state.flow?.nodes[1].position).toEqual({ x: 250, y: 180 });
  });

  it('应该添加 WS SendWait 节点当拖放 ws_send_wait 类型时', () => {
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
          if (key === 'application/reactflow/nodeType') return 'ws_send_wait';
          return '';
        },
      },
      clientX: 350,
      clientY: 220,
    });

    reactFlowElement.dispatchEvent(dropEvent);

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_send_wait');
    expect(state.flow?.nodes[1].position).toEqual({ x: 350, y: 220 });
  });

  it('应该添加 WS Close 节点当拖放 ws_close 类型时', () => {
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
          if (key === 'application/reactflow/nodeType') return 'ws_close';
          return '';
        },
      },
      clientX: 450,
      clientY: 280,
    });

    reactFlowElement.dispatchEvent(dropEvent);

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_close');
    expect(state.flow?.nodes[1].position).toEqual({ x: 450, y: 280 });
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

  it('节点应该放置在鼠标释放位置，而不是画布中心', () => {
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

    // 在不同位置拖放多个节点，验证位置准确性
    const positions = [
      { x: 100, y: 100 },
      { x: 400, y: 300 },
      { x: 50, y: 500 },
    ];

    positions.forEach((pos, index) => {
      const dropEvent = new Event('drop', { bubbles: true });
      Object.assign(dropEvent, {
        preventDefault: vi.fn(),
        dataTransfer: {
          getData: (key: string) => {
            if (key === 'application/reactflow/nodeType') return 'http';
            return '';
          },
        },
        clientX: pos.x,
        clientY: pos.y,
      });

      reactFlowElement.dispatchEvent(dropEvent);

      const state = useFlowStore.getState();
      const addedNode = state.flow?.nodes[index + 1]; // +1 因为第一个是 entry 节点
      expect(addedNode?.position).toEqual(pos);
    });
  });
