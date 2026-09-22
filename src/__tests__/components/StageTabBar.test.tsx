/**
 * StageTabBar 组件测试
 *
 * 覆盖：阶段切换、节点计数 Badge、阶段提示条、调试状态指示与快速跳回。
 */
import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { StageTabBar } from '@/components/canvas/StageTabBar';
import { useFlowStore } from '@/stores/flow';
import { useDebugStore } from '@/stores/debug';
import type { Flow, Node } from '@/types';

const makeNode = (uuid: string, name: string): Node => ({
  uuid,
  type: 'script',
  name,
  position: { x: 0, y: 0 },
  data: { type: 'script', script: '' },
});

const makeFlow = (): Flow => ({
  uuid: 'flow-stage-bar',
  name: '阶段切换流程',
  setup: { nodes: [makeNode('setup-1', 'Setup 节点')], connections: [] },
  nodes: [makeNode('main-1', 'Main 节点'), makeNode('main-2', 'Main 节点 2')],
  connections: [],
  teardown: { nodes: [makeNode('teardown-1', 'Teardown 节点')], connections: [] },
});

describe('StageTabBar', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    useDebugStore.getState().reset();
    vi.clearAllMocks();
  });

  it('渲染三个阶段标签与各自的节点计数', () => {
    useFlowStore.getState().setFlow(makeFlow());
    render(<StageTabBar />);

    expect(screen.getByTestId('stage-tab-setup')).toBeTruthy();
    expect(screen.getByTestId('stage-tab-main')).toBeTruthy();
    expect(screen.getByTestId('stage-tab-teardown')).toBeTruthy();

    expect(screen.getByTestId('stage-tab-count-setup').textContent).toBe('1');
    expect(screen.getByTestId('stage-tab-count-main').textContent).toBe('2');
    expect(screen.getByTestId('stage-tab-count-teardown').textContent).toBe('1');

    // 验证按钮具有可访问性标签与 SVG 图标
    expect(screen.getByTestId('stage-tab-setup').getAttribute('aria-label')).toBe('准备阶段');
    expect(screen.getByTestId('stage-tab-main').getAttribute('aria-label')).toBe('核心流程');
    expect(screen.getByTestId('stage-tab-teardown').getAttribute('aria-label')).toBe('收尾阶段');
    expect(screen.getByTestId('stage-tab-setup').querySelector('svg')).toBeTruthy();
    expect(screen.getByTestId('stage-tab-main').querySelector('svg')).toBeTruthy();
    expect(screen.getByTestId('stage-tab-teardown').querySelector('svg')).toBeTruthy();
  });

  it('默认激活 Main 阶段并高亮', () => {
    useFlowStore.getState().setFlow(makeFlow());
    render(<StageTabBar />);

    expect(screen.getByTestId('stage-tab-main').getAttribute('data-active')).toBe('true');
    expect(screen.getByTestId('stage-tab-setup').getAttribute('data-active')).toBe('false');
  });

  it('点击标签切换 activeStage', () => {
    useFlowStore.getState().setFlow(makeFlow());
    render(<StageTabBar />);

    fireEvent.click(screen.getByTestId('stage-tab-setup'));
    expect(useFlowStore.getState().activeStage).toBe('setup');
    expect(screen.getByTestId('stage-tab-setup').getAttribute('data-active')).toBe('true');
    expect(screen.getByTestId('stage-tab-main').getAttribute('data-active')).toBe('false');

    fireEvent.click(screen.getByTestId('stage-tab-teardown'));
    expect(useFlowStore.getState().activeStage).toBe('teardown');
  });

  it('切换阶段时展示对应说明提示条', () => {
    useFlowStore.getState().setFlow(makeFlow());
    render(<StageTabBar />);

    const hint = screen.getByTestId('stage-hint');

    fireEvent.click(screen.getByTestId('stage-tab-setup'));
    expect(hint.textContent).toContain('测试前执行 1 次');

    fireEvent.click(screen.getByTestId('stage-tab-teardown'));
    expect(hint.textContent).toContain('$report');

    fireEvent.click(screen.getByTestId('stage-tab-main'));
    expect(hint.textContent).toContain('核心业务流程');
  });

  it('调试暂停在 Setup 节点时自动切到 Setup 画布（状态指示）', () => {
    const flow = makeFlow();
    useFlowStore.getState().setFlow(flow);
    useDebugStore.setState({ status: 'paused', currentNodeUuid: 'setup-1' });

    render(<StageTabBar />);
    // 暂停节点位于 Setup：main 阶段显示"等待中"，setup 阶段显示"暂停在断点"
    expect(screen.getByTestId('stage-tab-setup').getAttribute('title')).toBe('暂停在断点');
    expect(screen.getByTestId('stage-tab-main').getAttribute('title')).toBe('等待中');
  });

  it('暂停节点不在当前画布时浮现快速跳回条并定位', () => {
    useFlowStore.getState().setFlow(makeFlow());
    useFlowStore.getState().setActiveStage('main');
    useDebugStore.setState({ status: 'paused', currentNodeUuid: 'teardown-1' });

    render(<StageTabBar />);

    const jumpBack = screen.getByTestId('stage-jump-back');
    expect(jumpBack.textContent).toContain('Teardown 节点');

    fireEvent.click(jumpBack);
    expect(useFlowStore.getState().activeStage).toBe('teardown');
  });

  it('未调试时不渲染调试状态指示与跳回条', () => {
    useFlowStore.getState().setFlow(makeFlow());
    render(<StageTabBar />);

    expect(screen.queryByTestId('stage-jump-back')).toBeNull();
    expect(screen.getByTestId('stage-tab-setup').getAttribute('title')).toBeNull();
  });

  it('Setup 存在多个未连线节点时展示连线顺序警告，连线后警告消失', () => {
    const flow = makeFlow();
    flow.setup = {
      nodes: [makeNode('setup-1', 'Setup 节点 1'), makeNode('setup-2', 'Setup 节点 2')],
      connections: [],
    };
    useFlowStore.getState().setFlow(flow);
    useFlowStore.getState().setActiveStage('setup');

    const { rerender } = render(<StageTabBar />);
    expect(screen.getByTestId('stage-unlinked-warning')).toBeTruthy();

    // 连上线：setup-1 -> setup-2，入度为 0 的根节点减为 1 个，警告消失
    flow.setup.connections = [
      {
        id: 'c-setup',
        source_node_uuid: 'setup-1',
        source_port_id: 'out',
        target_node_uuid: 'setup-2',
        target_port_id: 'in',
      },
    ];
    act(() => {
      useFlowStore.getState().setFlow({ ...flow });
    });
    rerender(<StageTabBar />);
    expect(screen.queryByTestId('stage-unlinked-warning')).toBeNull();
  });
});
