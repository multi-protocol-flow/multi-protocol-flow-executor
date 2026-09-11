/**
 * ExecutionStackView 组件测试
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ExecutionStackView } from '@/components/execution/ExecutionStackView';
import { useDebugStore } from '@/stores/debug';

describe('ExecutionStackView', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    vi.clearAllMocks();
  });

  it('空堆栈时显示空状态', () => {
    render(
      <ExecutionStackView
        executionStack={[]}
        currentNodeUuid={null}
      />
    );
    expect(screen.getByText('暂无执行记录')).toBeInTheDocument();
  });

  it('多层栈帧时显示调用栈帧列表并允许点击切换', () => {
    useDebugStore.setState({
      callFrames: [
        {
          frame_id: 0,
          flow_uuid: 'root-flow',
          flow_name: 'Root Flow',
          node_uuid: 'subflow-1',
          node_name: 'Call SubFlow',
        },
        {
          frame_id: 1,
          flow_uuid: 'child-flow',
          flow_name: 'User Auth SubFlow',
          node_uuid: 'auth-node',
          node_name: 'Verify Token',
          parent_subflow_node_uuid: 'subflow-1',
        },
      ],
      activeFrameIndex: 1,
    });

    const onNavigateMock = vi.fn();

    render(
      <ExecutionStackView
        executionStack={[
          {
            node_uuid: 'auth-node',
            node_name: 'Verify Token',
            node_type: 'http',
            started_at: 1000,
            ended_at: 1500,
            success: true,
          },
        ]}
        currentNodeUuid="auth-node"
        onNavigateToNode={onNavigateMock}
      />
    );

    expect(screen.getByText('调用栈帧')).toBeInTheDocument();
    expect(screen.getByText('Root Flow')).toBeInTheDocument();
    expect(screen.getByText('User Auth SubFlow')).toBeInTheDocument();

    // 点击 Frame 0 切换活跃栈帧
    const rootFrameBtn = screen.getByText('Root Flow');
    fireEvent.click(rootFrameBtn);
    expect(useDebugStore.getState().activeFrameIndex).toBe(0);
    expect(onNavigateMock).toHaveBeenCalledWith('subflow-1');
  });
});
