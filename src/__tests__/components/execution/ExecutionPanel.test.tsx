/**
 * ExecutionPanel 组件测试
 * 重点防回归：useSyncExternalStore 依赖引用稳定，执行期间避免无限重渲染（Maximum update depth exceeded）
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, fireEvent, waitFor } from '@testing-library/react';
import { ExecutionPanel } from '@/components/execution/ExecutionPanel';
import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import type { Flow, Node } from '@/types';

// Mock react-json-view-lite（LoopViewer → NodeExecutionDetails 间接依赖）
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

function createMockFlow(): Flow {
  const mainNode: Node = {
    uuid: 'node-main-1',
    name: 'HTTP 节点',
    type: 'http:request',
    position: { x: 0, y: 0 },
    data: { type: 'http:request', method: 'get', url: 'https://example.com' },
  };
  const setupNode: Node = {
    uuid: 'node-setup-1',
    name: 'Setup 节点',
    type: 'http:request',
    position: { x: 0, y: 0 },
    data: { type: 'http:request', method: 'post', url: 'https://example.com/login' },
  };
  const teardownNode: Node = {
    uuid: 'node-teardown-1',
    name: 'Teardown 节点',
    type: 'http:request',
    position: { x: 0, y: 0 },
    data: { type: 'http:request', method: 'delete', url: 'https://example.com/cleanup' },
  };

  return {
    uuid: 'flow-test-1',
    name: '多阶段流程',
    nodes: [mainNode],
    connections: [],
    setup: { nodes: [setupNode], connections: [] },
    teardown: { nodes: [teardownNode], connections: [] },
  };
}

describe('ExecutionPanel', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    useExecutionStore.getState().reset();
    useUIStore.getState().resetPanels();
  });

  it('未执行且无历史时展示点击运行引导，且不会发生 Maximum update depth exceeded', () => {
    const flow = createMockFlow();
    useFlowStore.getState().setFlow(flow);

    const { container } = render(<ExecutionPanel />);
    expect(container).toBeDefined();
    expect(screen.getByText(/点击「执行」/)).toBeDefined();
  });

  it('执行期间连续派发 nodeStarted / nodeCompleted 事件时，面板正常渲染无无限循环异常', () => {
    const flow = createMockFlow();
    useFlowStore.getState().setFlow(flow);

    // 模拟执行状态开始
    act(() => {
      useExecutionStore.setState({
        isExecuting: true,
        currentNodeUuid: 'node-setup-1',
        nodeReportsMap: {},
      });
    });

    const { rerender } = render(<ExecutionPanel />);
    expect(screen.getAllByText('Setup 节点').length).toBeGreaterThan(0);

    // 触发连续节点事件更新，伴随阶段切换
    act(() => {
      useExecutionStore.setState({
        currentNodeUuid: 'node-main-1',
        nodeReportsMap: {
          'node-setup-1': {
            node_uuid: 'node-setup-1',
            node_type: 'http:request',
            node_name: 'Setup 节点',
            start_time: Date.now() - 100,
            end_time: Date.now(),
            duration_ms: 100,
            status: 'success',
          },
        },
      });
      useFlowStore.getState().setActiveStage('main');
    });

    rerender(<ExecutionPanel />);
    expect(screen.getAllByText('HTTP 节点').length).toBeGreaterThan(0);
  });
  it('当节点触发 HITL 人机交互时，面板正常展示交互卡片并可交互', () => {
    const flow = createMockFlow();
    useFlowStore.getState().setFlow(flow);
    act(() => {
      useExecutionStore.setState({
        isExecuting: true,
        currentNodeUuid: 'node-main-1',
        activeInteractions: {
          'node-main-1': {
            interaction_id: 'ask_123',
            execution_id: 'exec_123',
            interaction_type: 'ask_human',
            prompt: '您希望测试什么消息？',
            created_at_ms: Date.now(),
          },
        },
      });
    });

    render(<ExecutionPanel />);

    // 应该展示智能体提问内容
    expect(screen.getByText('您希望测试什么消息？')).toBeDefined();
    expect(screen.getByText('HITL')).toBeDefined();
  });

  it('当执行切片流程时，展示测试运行不入历史徽章', () => {
    const flow = createMockFlow();
    useFlowStore.getState().setFlow(flow);

    act(() => {
      useExecutionStore.setState({
        isExecuting: true,
        activeFlowUuid: 'slice-123456789',
        currentNodeUuid: 'node-main-1',
        totalNodes: 1,
        completedNodeCount: 0,
      });
    });

    render(<ExecutionPanel />);
    expect(screen.getByText('测试运行 / 不入历史')).toBeInTheDocument();
  });

  it('执行中的 loop 节点实时展示每一轮迭代结果', async () => {
    const loopNode: Node = {
      uuid: 'loop-1',
      name: '循环',
      type: 'flow:loop',
      position: { x: 0, y: 0 },
      data: {
        type: 'flow:loop',
        source: 'count',
        count: 2,
        max_iterations: 100,
        concurrency: 1,
        body: { nodes: [], connections: [] },
        break_on_error: true,
        report: { detail_iterations: 20, always_keep_failed: true },
      },
    };
    useFlowStore.getState().setFlow({
      ...createMockFlow(),
      nodes: [loopNode],
      connections: [],
    });

    act(() => {
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'flow-test-1', flowName: '多阶段流程', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeStarted: { nodeUuid: 'loop-1', nodeName: '循环', nodeType: 'flow:loop' },
      });
      useExecutionStore.getState().handleExecutionEvent({
        loopIteration: {
          nodeUuid: 'loop-1',
          iteration: { index: 0, status: 'success', duration_ms: 12, wait_ms: 0 },
          reports: [
            {
              node_uuid: 'body-1',
              node_type: 'script',
              node_name: 'body',
              start_time: 1000,
              end_time: 1012,
              duration_ms: 12,
              status: 'success',
              input_data: null,
              output_data: { round: 0 },
              iteration_index: 0,
              iteration_of: 'loop-1',
            },
          ],
        },
      });
    });

    render(<ExecutionPanel />);

    // 运行中即出现「第 0 轮」，而不是等整个循环结束
    const label = getLL().execution.report.loop_iteration_label({ index: 0 });
    await waitFor(() => expect(screen.getByText(label)).toBeDefined());

    // 展开该轮即可看到 body 节点（script）的输出结果
    fireEvent.click(screen.getByText(label));
    await waitFor(() =>
      expect(screen.getAllByTestId('json-view').length).toBeGreaterThan(0),
    );

    // 循环节点完成：热通道报告剥离了 sub_flow_report，实时进度补回各轮正文，
    // 已展开的轮次保持展开，流程剩余时间内仍可查看结果
    act(() => {
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'loop-1',
            node_type: 'flow:loop',
            node_name: '循环',
            start_time: 1000,
            end_time: 1020,
            duration_ms: 20,
            status: 'success',
            input_data: null,
            output_data: null,
            is_detail_stripped: true,
            iteration_summary: [
              { index: 0, status: 'success', duration_ms: 12, wait_ms: 0 },
            ],
          },
        },
      });
    });

    expect(screen.getByText(label)).toBeDefined();
    expect(screen.getAllByTestId('json-view').length).toBeGreaterThan(0);
  });
});
