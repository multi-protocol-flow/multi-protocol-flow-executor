/**
 * FlowCanvas 拖放功能测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, act } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { FlowCanvas } from '@/components/canvas';
import { useFlowStore } from '@/stores';
import type { Flow } from '@/types';

const mockToast = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: mockToast,
}));
const mockFlowStorage = vi.hoisted(() => ({
  loadFlow: vi.fn(),
  saveFlow: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/services/storage', () => ({
  flowStorage: mockFlowStorage,
}));


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

  it('应该添加 HTTP 节点当拖放 http 类型时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
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

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('http');
    // 验证节点位置使用鼠标释放位置
    expect(state.flow?.nodes[1].position).toEqual({ x: 200, y: 150 });
  });

  it('应该添加 Conditional 节点当拖放 conditional 类型时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
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

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('conditional');
    // 验证节点位置使用鼠标释放位置
    expect(state.flow?.nodes[1].position).toEqual({ x: 300, y: 200 });
  });

  it('应该添加 WS Connect 节点当拖放 ws_connect 类型时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
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

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_connect');
    expect(state.flow?.nodes[1].position).toEqual({ x: 250, y: 180 });
  });

  it('应该添加 WS SendCollect 节点当拖放 ws_send_collect 类型时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/nodeType') return 'ws_send_collect';
          return '';
        },
      },
      clientX: 350,
      clientY: 220,
    });

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_send_collect');
    expect(state.flow?.nodes[1].position).toEqual({ x: 350, y: 220 });
  });

  it('应该添加 WS Close 节点当拖放 ws_close 类型时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
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

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    expect(state.flow?.nodes[1].type).toBe('ws_close');
    expect(state.flow?.nodes[1].position).toEqual({ x: 450, y: 280 });
  });
  it('当没有流程时不应该添加节点', async () => {
    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    // 不应该有 .react-flow 元素
    const reactFlowElement = container!.querySelector('.react-flow');
    expect(reactFlowElement).toBeNull();
  });
});

  it('节点应该放置在鼠标释放位置，而不是画布中心', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    // 在不同位置拖放多个节点，验证位置准确性
    const positions = [
      { x: 100, y: 100 },
      { x: 400, y: 300 },
      { x: 50, y: 500 },
    ];

    for (const [index, pos] of positions.entries()) {
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

      await act(async () => {
        reactFlowElement.dispatchEvent(dropEvent);
      });

      const state = useFlowStore.getState();
      const addedNode = state.flow?.nodes[index + 1]; // +1 因为第一个是 entry 节点
      expect(addedNode?.position).toEqual(pos);
    }
  });

  it('应该添加 SubFlow 节点当拖放子流程数据时', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/subflow') {
            return JSON.stringify({
              uuid: 'subflow-child-1',
              name: '认证子流程',
              filePath: '/path/to/subflow.mpf',
            });
          }
          return '';
        },
      },
      clientX: 260,
      clientY: 190,
    });

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    const subflowNode = state.flow?.nodes[1];
    expect(subflowNode?.type).toBe('sub_flow');
    expect(subflowNode?.name).toBe('认证子流程');
    expect(subflowNode?.position).toEqual({ x: 260, y: 190 });

    const nodeData = subflowNode?.data;
    expect(nodeData?.type).toBe('sub_flow');
    if (nodeData?.type === 'sub_flow') {
      expect(nodeData.target_flow_id).toBe('subflow-child-1');
      expect(nodeData.target_flow_path).toBe('');
      expect(nodeData.inherit_all_variables).toBe(true);
      expect(nodeData.export_all_variables).toBe(true);
    }
  });

  it('拖入当前流程自身时应该被拦截并显示错误提示', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/subflow') {
            return JSON.stringify({
              uuid: 'test-flow', // 同 mockFlow.uuid
              name: 'Test Flow',
            });
          }
          return '';
        },
      },
      clientX: 200,
      clientY: 200,
    });

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(1);
    expect(mockToast.error).toHaveBeenCalled();
  });

  it('当画布为空时拖入子流程应直接打开该流程', async () => {
    useFlowStore.getState().setFlow(null);
    const mockTargetFlow: Flow = {
      uuid: 'target-flow-id',
      name: '目标子流程',
      nodes: [],
      connections: [],
      initial_variables: {},
    };
    mockFlowStorage.loadFlow.mockResolvedValue(mockTargetFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const emptyCanvas = container!.querySelector('[data-guide="canvas-area"]');
    if (!emptyCanvas) {
      throw new Error('Empty canvas element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          if (key === 'application/reactflow/subflow') {
            return JSON.stringify({
              uuid: 'target-flow-id',
              name: '目标子流程',
            });
          }
          return '';
        },
      },
      clientX: 200,
      clientY: 200,
    });

    await act(async () => {
      emptyCanvas.dispatchEvent(dropEvent);
    });

    await act(async () => {
      await Promise.resolve();
    });

    const state = useFlowStore.getState();
    expect(state.flow?.uuid).toBe('target-flow-id');
  });

  it('通过 text/plain 降级传输时也能正常生成 SubFlow 节点', async () => {
    useFlowStore.getState().setFlow(mockFlow);

    let container: HTMLElement;
    await act(async () => {
      const result = render(
        <TestWrapper>
          <FlowCanvas />
        </TestWrapper>
      );
      container = result.container;
    });

    const reactFlowElement = container!.querySelector('.react-flow');
    if (!reactFlowElement) {
      throw new Error('ReactFlow element not found');
    }

    const dropEvent = new Event('drop', { bubbles: true });
    Object.assign(dropEvent, {
      preventDefault: vi.fn(),
      dataTransfer: {
        getData: (key: string) => {
          // 模拟某些 WebView 环境下自定义 MIME 被过滤，仅 text/plain 保留
          if (key === 'text/plain') {
            return JSON.stringify({
              uuid: 'fallback-flow-1',
              name: '降级子流程',
            });
          }
          return '';
        },
      },
      clientX: 120,
      clientY: 130,
    });

    await act(async () => {
      reactFlowElement.dispatchEvent(dropEvent);
    });

    const state = useFlowStore.getState();
    expect(state.flow?.nodes.length).toBe(2);
    const subflowNode = state.flow?.nodes[1];
    expect(subflowNode?.type).toBe('sub_flow');
    expect(subflowNode?.name).toBe('降级子流程');
  });
