/**
 * DebugBreadcrumbs 组件测试
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DebugBreadcrumbs } from '@/components/canvas/DebugBreadcrumbs';
import { useDebugStore } from '@/stores/debug';
import { useFlowStore } from '@/stores/flow';

describe('DebugBreadcrumbs', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    useFlowStore.getState().reset();
    vi.clearAllMocks();
  });

  it('单层流程（callFrames <= 1）时不显示面包屑', () => {
    useDebugStore.setState({
      status: 'paused',
      callFrames: [
        {
          frame_id: 0,
          flow_uuid: 'root-flow',
          flow_name: 'Root Flow',
          node_uuid: 'node-1',
          node_name: 'Start',
        },
      ],
      activeFrameIndex: 0,
    });

    const { container } = render(<DebugBreadcrumbs />);
    expect(container.firstChild).toBeNull();
  });

  it('多层流程（callFrames > 1）时正确渲染面包屑层级并高亮活跃栈帧', () => {
    useDebugStore.setState({
      status: 'paused',
      callFrames: [
        {
          frame_id: 0,
          flow_uuid: 'root-flow',
          flow_name: 'Root Flow',
          node_uuid: 'subflow-1',
          node_name: 'Call Subflow A',
        },
        {
          frame_id: 1,
          flow_uuid: 'subflow-a',
          flow_name: 'SubFlow A',
          node_uuid: 'subflow-2',
          node_name: 'Call Subflow B',
          parent_subflow_node_uuid: 'subflow-1',
        },
        {
          frame_id: 2,
          flow_uuid: 'subflow-b',
          flow_name: 'SubFlow B',
          node_uuid: 'http-1',
          node_name: 'HTTP Fetch',
          parent_subflow_node_uuid: 'subflow-2',
        },
      ],
      activeFrameIndex: 2,
    });

    render(<DebugBreadcrumbs />);

    expect(screen.getByTestId('debug-breadcrumbs')).toBeInTheDocument();
    expect(screen.getByText('Root Flow')).toBeInTheDocument();
    expect(screen.getByText('SubFlow A')).toBeInTheDocument();
    expect(screen.getByText('SubFlow B')).toBeInTheDocument();
    expect(screen.getByText('HTTP Fetch')).toBeInTheDocument();

    // 点击某级面包屑切换栈帧
    const rootBtn = screen.getByText('Root Flow');
    fireEvent.click(rootBtn);
    expect(useDebugStore.getState().activeFrameIndex).toBe(0);
  });
});
