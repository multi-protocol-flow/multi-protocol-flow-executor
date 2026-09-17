/**
 * VariableTreeView 单步 Diff 订阅回归测试
 *
 * 覆盖线上崩溃：调试暂停面板订阅变量 Diff 时，
 * 选择器若每次返回新对象引用，会导致 React "Maximum update depth exceeded"。
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { VariableTreeView } from '@/components/execution/VariableTreeView';
import { useDebugStore } from '@/stores/debug';
import type { DebugEvent } from '@/types/debug';

function pausedAt(nodeUuid: string, variables: Record<string, unknown>): DebugEvent {
  return {
    PausedAtNode: {
      node_uuid: nodeUuid,
      reason: { type: 'breakpoint', node_uuid: nodeUuid },
      variables,
      execution_stack: [],
    },
  };
}

describe('VariableTreeView 变量 Diff 订阅', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
  });

  it('暂停状态下渲染变量树不触发无限重渲染', () => {
    const store = useDebugStore.getState();
    store.handleDebugEvent(pausedAt('n1', { order_id: 'OLD_1', kept: 1 }));
    store.handleDebugEvent(pausedAt('n2', { order_id: 'NEW_123', kept: 1, extra: true }));

    const { container } = render(
      <VariableTreeView variables={useDebugStore.getState().variables} />
    );

    expect(screen.getByText('order_id')).toBeInTheDocument();
    expect(screen.getByText('extra')).toBeInTheDocument();

    // 重复订阅取值引用必须稳定
    const first = useDebugStore.getState().variableDiff;
    useDebugStore.getState().handleDebugEvent({
      Heartbeat: { status: 'paused', current_node_uuid: 'n2', timestamp: Date.now() },
    });
    expect(useDebugStore.getState().variableDiff).toBe(first);

    expect(container.textContent).toContain('order_id');
  });

  it('变量按差异标记新增与修改', () => {
    const store = useDebugStore.getState();
    store.handleDebugEvent(pausedAt('n1', { order_id: 'OLD_1' }));
    store.handleDebugEvent(pausedAt('n2', { order_id: 'NEW_123', extra: true }));

    render(<VariableTreeView variables={useDebugStore.getState().variables} />);

    expect(screen.getByText('[+]')).toBeInTheDocument();
    expect(screen.getByText('[~]')).toBeInTheDocument();
  });
});
