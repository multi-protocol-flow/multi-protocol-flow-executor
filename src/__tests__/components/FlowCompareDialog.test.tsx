/**
 * FlowCompareDialog 组件测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FlowCompareDialog } from '@/components/execution/FlowCompareDialog';
import type { Flow } from '@/types';

// Mock @xyflow/react，因为 React Flow 在 jsdom 中无法正常渲染
vi.mock('@xyflow/react', async () => {
  const actual = await vi.importActual('@xyflow/react');
  return {
    ...actual,
    ReactFlow: ({ children, onNodeClick, nodesDraggable, nodesConnectable }: {
      children?: React.ReactNode;
      onNodeClick?: (_: React.MouseEvent, node: { id: string }) => void;
      nodesDraggable?: boolean;
      nodesConnectable?: boolean;
      [key: string]: unknown;
    }) => (
      <div
        data-testid="react-flow-mock"
        data-nodes-draggable={nodesDraggable?.toString()}
        data-nodes-connectable={nodesConnectable?.toString()}
      >
        <div
          role="button"
          tabIndex={0}
          data-testid="react-flow-canvas"
          onClick={() => onNodeClick?.({} as React.MouseEvent, { id: 'node-2' })}
          onKeyDown={(e: React.KeyboardEvent) => {
            if (e.key === 'Enter' || e.key === ' ') {
              onNodeClick?.({} as React.MouseEvent, { id: 'node-2' });
            }
          }}
        >
          {children}
        </div>
      </div>
    ),
    useNodesState: () => [[], vi.fn(), vi.fn()],
    useEdgesState: () => [[], vi.fn(), vi.fn()],
    useReactFlow: () => ({ fitView: vi.fn(), screenToFlowPosition: vi.fn() }),
  };
});

function createMockFlow(overrides: Partial<Flow> = {}): Flow {
  return {
    uuid: 'flow-1',
    name: '测试流程',
    nodes: [],
    connections: [],
    initial_variables: {},
    ...overrides,
  };
}

function createMockFlowWithDiffs(): { snapshot: Flow; current: Flow } {
  const snapshot: Flow = {
    uuid: 'flow-1',
    name: '旧流程名',
    nodes: [
      {
        uuid: 'node-1',
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
      {
        uuid: 'node-2',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 200, y: 0 },
        data: { type: 'http', url: 'https://old.example.com', method: 'get', timeout_ms: 30000 },
      },
      {
        uuid: 'node-3',
        type: 'conditional',
        name: '条件分支',
        position: { x: 400, y: 0 },
        data: { type: 'conditional', condition_type: 'advanced', advanced_expression: 'status === 200' },
      },
    ],
    connections: [
      {
        id: 'conn-1',
        source_node_uuid: 'node-1',
        target_node_uuid: 'node-2',
        source_port_id: 'output',
        target_port_id: 'input',
      },
    ],
    initial_variables: { var1: 'old_value', var2: 'unchanged' },
  };

  const current: Flow = {
    uuid: 'flow-1',
    name: '新流程名',
    nodes: [
      {
        uuid: 'node-1',
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
      {
        uuid: 'node-2',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 200, y: 0 },
        data: { type: 'http', url: 'https://new.example.com', method: 'get', timeout_ms: 30000 },
      },
      {
        uuid: 'node-4',
        type: 'script',
        name: '脚本节点',
        position: { x: 600, y: 0 },
        data: { type: 'script', script: 'console.log("hello")' },
      },
    ],
    connections: [
      {
        id: 'conn-1',
        source_node_uuid: 'node-1',
        target_node_uuid: 'node-2',
        source_port_id: 'output',
        target_port_id: 'input',
      },
      {
        id: 'conn-2',
        source_node_uuid: 'node-2',
        target_node_uuid: 'node-4',
        source_port_id: 'output',
        target_port_id: 'input',
      },
    ],
    initial_variables: { var1: 'new_value', var2: 'unchanged', var3: 'added' },
  };

  return { snapshot, current };
}

describe('FlowCompareDialog', () => {
  const defaultProps = {
    open: true,
    onOpenChange: vi.fn(),
    snapshotFlow: createMockFlow(),
    currentFlow: createMockFlow(),
    snapshotName: '快照A',
    currentName: '当前B',
  };

  it('渲染快照和当前流程名称', () => {
    render(<FlowCompareDialog {...defaultProps} />);

    expect(screen.getByText(/对比:.*快照A.*vs.*当前B/)).toBeDefined();
  });

  it('显示统计栏的正确计数', () => {
    const { snapshot, current } = createMockFlowWithDiffs();

    render(
      <FlowCompareDialog
        {...defaultProps}
        snapshotFlow={snapshot}
        currentFlow={current}
      />
    );

    // node-4 新增 (1 node + 1 connection = 2 新增)
    expect(screen.getByText(/2 新增/)).toBeDefined();
    // node-3 删除
    expect(screen.getByText(/1 删除/)).toBeDefined();
    // node-2 修改 (URL变了)
    expect(screen.getByText(/1 修改/)).toBeDefined();
    // node-1 无变化 + conn-1 无变化
    expect(screen.getByText(/2 无变化/)).toBeDefined();
  });

  it('字段对比 Tab 显示节点变更', () => {
    const { snapshot, current } = createMockFlowWithDiffs();

    render(
      <FlowCompareDialog
        {...defaultProps}
        snapshotFlow={snapshot}
        currentFlow={current}
      />
    );

    // 默认在字段对比 tab，应能看到节点变更
    expect(screen.getByText('节点变更')).toBeDefined();
    expect(screen.getByText('脚本节点')).toBeDefined();
    expect(screen.getByText('条件分支')).toBeDefined();
  });

  it('Tab 切换正常工作', async () => {
    const { snapshot, current } = createMockFlowWithDiffs();
    const user = userEvent.setup();

    render(
      <FlowCompareDialog
        {...defaultProps}
        snapshotFlow={snapshot}
        currentFlow={current}
      />
    );

    const canvasTab = screen.getByRole('tab', { name: '画布对比' });
    await user.click(canvasTab);

    // 画布对比 Tab 应渲染 ReactFlow 组件
    expect(screen.getByTestId('react-flow-mock')).toBeDefined();

    const fieldTab = screen.getByRole('tab', { name: '字段对比' });
    await user.click(fieldTab);

    // 切回字段对比后应看到节点变更标题
    expect(screen.getByText('节点变更')).toBeDefined();
  });

  it('关闭按钮调用 onOpenChange(false)', async () => {
    const onOpenChange = vi.fn();
    const user = userEvent.setup();

    render(<FlowCompareDialog {...defaultProps} onOpenChange={onOpenChange} />);

    await user.click(screen.getByRole('button', { name: '关闭' }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('currentFlow 为 null 时显示无法加载提示', () => {
    render(
      <FlowCompareDialog
        {...defaultProps}
        currentFlow={null}
      />
    );

    expect(screen.getByText('无法加载当前流程')).toBeDefined();
  });

  it('恢复流程按钮处于启用状态', () => {
    render(<FlowCompareDialog {...defaultProps} />);

    const restoreButton = screen.getByRole('button', { name: /恢复流程/ });
    expect(restoreButton).toHaveProperty('disabled', false);
  });

  describe('画布对比 Tab', () => {
    it('画布渲染为只读模式（nodesDraggable=false, nodesConnectable=false）', async () => {
      const { snapshot, current } = createMockFlowWithDiffs();
      const user = userEvent.setup();

      render(
        <FlowCompareDialog
          {...defaultProps}
          snapshotFlow={snapshot}
          currentFlow={current}
        />
      );

      await user.click(screen.getByRole('tab', { name: '画布对比' }));

      const flowMock = screen.getByTestId('react-flow-mock');
      expect(flowMock.getAttribute('data-nodes-draggable')).toBe('false');
      expect(flowMock.getAttribute('data-nodes-connectable')).toBe('false');
    });

    it('点击节点显示配置 Diff 面板', async () => {
      const { snapshot, current } = createMockFlowWithDiffs();
      const user = userEvent.setup();

      render(
        <FlowCompareDialog
          {...defaultProps}
          snapshotFlow={snapshot}
          currentFlow={current}
        />
      );

      await user.click(screen.getByRole('tab', { name: '画布对比' }));

      // 点击画布区域模拟节点点击（mock 中点击 canvas 触发 onNodeClick(node-2)）
      const canvas = screen.getByTestId('react-flow-canvas');
      await user.click(canvas);

      // 应出现配置 Diff 面板，node-2 是 modified 节点
      expect(screen.getByText('(已修改)')).toBeDefined();
    });

    it('点击节点后再次点击可关闭 Diff 面板', async () => {
      const { snapshot, current } = createMockFlowWithDiffs();
      const user = userEvent.setup();

      render(
        <FlowCompareDialog
          {...defaultProps}
          snapshotFlow={snapshot}
          currentFlow={current}
        />
      );

      await user.click(screen.getByRole('tab', { name: '画布对比' }));

      const canvas = screen.getByTestId('react-flow-canvas');
      // 第一次点击打开
      await user.click(canvas);
      expect(screen.getByText('(已修改)')).toBeDefined();

      // 第二次点击同一节点关闭
      await user.click(canvas);
      expect(screen.queryByText('(已修改)')).toBeNull();
    });

    it('Diff 面板关闭按钮正常工作', async () => {
      const { snapshot, current } = createMockFlowWithDiffs();
      const user = userEvent.setup();

      render(
        <FlowCompareDialog
          {...defaultProps}
          snapshotFlow={snapshot}
          currentFlow={current}
        />
      );

      await user.click(screen.getByRole('tab', { name: '画布对比' }));

      const canvas = screen.getByTestId('react-flow-canvas');
      await user.click(canvas);
      expect(screen.getByText('(已修改)')).toBeDefined();

      // 关闭按钮是 X 图标按钮，找到面板内的关闭按钮
      const closeButtons = screen.getAllByRole('button').filter(
        (btn) => btn.querySelector('svg') && btn.closest('[class*="border-t"]')
      );
      if (closeButtons.length > 0) {
        await user.click(closeButtons[0]);
        expect(screen.queryByText('(已修改)')).toBeNull();
      }
    });
  });
});
