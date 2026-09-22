/**
 * ExecutionPanel 组件测试
 * 重点防回归：useSyncExternalStore 依赖引用稳定，执行期间避免无限重渲染（Maximum update depth exceeded）
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { ExecutionPanel } from '@/components/execution/ExecutionPanel';
import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import { useUIStore } from '@/stores/ui';
import type { Flow, Node } from '@/types';

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
});
