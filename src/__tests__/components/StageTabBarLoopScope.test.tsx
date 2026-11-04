/**
 * StageTabBar 的面包屑 / 阶段标签必须能把画布从 loop body 带回阶段画布
 *
 * 回归背景：`setActiveStage` 曾经对「点击当前阶段」直接早退，导致进入 loop body 后
 * 点 Main 标签毫无反应，用户被卡在循环体内。
 */
import { describe, expect, it, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { StageTabBar } from '@/components/canvas/StageTabBar';
import { useFlowStore, selectStageNodes } from '@/stores/flow';
import { useDebugStore } from '@/stores/debug';
import type { Flow, Node } from '@/types';

const loopNode: Node = {
  uuid: 'loop-1',
  type: 'flow:loop',
  name: 'Poll',
  position: { x: 0, y: 0 },
  data: {
    type: 'flow:loop',
    source: 'count',
    count: 2,
    max_iterations: 100,
    concurrency: 1,
    break_on_error: true,
    report: { detail_iterations: 20, always_keep_failed: true },
    body: {
      nodes: [
        {
          uuid: 'body-1',
          type: 'flow:delay',
          name: 'body delay',
          position: { x: 0, y: 0 },
          data: { type: 'flow:delay', duration_ms: '10', jitter_ms: 0 },
        },
      ],
      connections: [],
    },
  },
};

const flow: Flow = { uuid: 'f1', name: 'loop scope', nodes: [loopNode], connections: [] };

describe('StageTabBar loop scope', () => {
  beforeEach(() => {
    cleanup();
    useFlowStore.getState().reset();
    useDebugStore.setState({ status: 'idle', currentNodeUuid: null });
    useFlowStore.getState().setFlow(flow);
    useFlowStore.getState().enterLoop('loop-1');
  });

  it('点击当前阶段标签退出 loop body', () => {
    render(<StageTabBar />);
    expect(screen.getByTestId('loop-breadcrumb')).toBeTruthy();

    fireEvent.click(screen.getByTestId('stage-tab-main'));

    const state = useFlowStore.getState();
    expect(state.loopPath).toEqual([]);
    expect(selectStageNodes(state).map((n) => n.uuid)).toEqual(['loop-1']);
  });

  it('点击面包屑根节点退出 loop body', () => {
    render(<StageTabBar />);
    const crumb = screen.getByTestId('loop-breadcrumb');
    const rootButton = crumb.querySelector('button');
    expect(rootButton).toBeTruthy();
    fireEvent.click(rootButton as HTMLButtonElement);

    expect(useFlowStore.getState().loopPath).toEqual([]);
  });

  it('调试跳回按钮恢复到暂停节点所在的 loop body', () => {
    // 暂停节点在 main 的 loop body 内，用户当前在 setup 阶段
    useFlowStore.getState().setActiveStage('setup');
    useDebugStore.setState({ status: 'paused', currentNodeUuid: 'body-1' });

    render(<StageTabBar />);
    fireEvent.click(screen.getByTestId('stage-jump-back'));

    const state = useFlowStore.getState();
    expect(state.activeStage).toBe('main');
    expect(state.loopPath).toEqual(['loop-1']);
  });
});
