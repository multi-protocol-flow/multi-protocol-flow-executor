/**
 * VariableTimelineDrawer 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { VariableTimelineDrawer } from '@/components/execution/VariableTimelineDrawer';
import { useDebugStore } from '@/stores/debug';
import type { ExecutionStackEntry } from '@/types/debug';

describe('VariableTimelineDrawer', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    vi.clearAllMocks();
  });

  it('varName 为 null 时不渲染任何内容', () => {
    const { container } = render(
      <VariableTimelineDrawer varName={null} onClose={vi.fn()} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('无变量变更历史时显示空状态', () => {
    render(<VariableTimelineDrawer varName="not_found_var" onClose={vi.fn()} />);
    expect(screen.getByText('暂无该变量的时空变更记录')).toBeInTheDocument();
  });

  it('从 executionStack 过滤并渲染变量时空历史', () => {
    const stack: ExecutionStackEntry[] = [
      {
        node_uuid: 'node-entry',
        node_name: '入口节点',
        node_type: 'entry',
        started_at: 1000,
        ended_at: 1050,
        success: true,
        variable_changes: [
          {
            key: 'token',
            old_value: null,
            new_value: 'init_token',
          },
        ],
      },
      {
        node_uuid: 'node-script',
        node_name: '脚本处理',
        node_type: 'script',
        started_at: 2000,
        ended_at: 2100,
        success: true,
        variable_changes: [
          {
            key: 'token',
            old_value: 'init_token',
            new_value: 'refreshed_token',
          },
        ],
      },
    ];

    useDebugStore.setState({ executionStack: stack });

    render(<VariableTimelineDrawer varName="token" onClose={vi.fn()} />);

    // 应该渲染节点名称
    expect(screen.getByText('入口节点')).toBeInTheDocument();
    expect(screen.getByText('脚本处理')).toBeInTheDocument();

    // 应该渲染创建与更新标识
    expect(screen.getByText('初始创建')).toBeInTheDocument();
    expect(screen.getByText('变量更新')).toBeInTheDocument();

    // 应该渲染变量值
    expect(screen.getAllByText('"init_token"')).toHaveLength(2);
    expect(screen.getByText('"refreshed_token"')).toBeInTheDocument();
  });
});
