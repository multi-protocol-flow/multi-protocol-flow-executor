/**
 * StagesSection 组件测试
 *
 * 覆盖：三阶段面板渲染、状态/耗时/节点流水展示、未配置阶段、Main 折叠摘要。
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { StagesSection } from '@/components/common/StagesSection';
import type { FlowStagesReport, NodeExecutionReport } from '@/types';

const nodeReport = (
  uuid: string,
  name: string,
  status: NodeExecutionReport['status'],
  durationMs: number,
): NodeExecutionReport => ({
  node_uuid: uuid,
  node_type: 'script',
  node_name: name,
  start_time: 0,
  end_time: durationMs,
  duration_ms: durationMs,
  status,
});

const stages: FlowStagesReport = {
  setup: {
    status: 'success',
    duration_ms: 42,
    node_reports: [nodeReport('setup-1', 'Setup Login', 'success', 40)],
  },
  main: {
    status: 'success',
    duration_ms: 7,
    node_reports: [
      nodeReport('main-1', 'Main Request', 'success', 5),
      nodeReport('main-2', 'Main Assert', 'success', 2),
    ],
  },
  teardown: {
    status: 'failed',
    duration_ms: 120,
    node_reports: [nodeReport('teardown-1', 'Notify Webhook', 'failed', 118)],
    error: { error_kind: 'connection', message: 'teardown notify timeout' },
  },
};

describe('StagesSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('渲染三个生命周期阶段面板，展示状态耗时与节点流水', () => {
    render(<StagesSection stages={stages} />);

    expect(screen.getByTestId('stages-section')).toBeTruthy();
    expect(screen.getByTestId('stage-panel-setup')).toBeTruthy();
    expect(screen.getByTestId('stage-panel-main')).toBeTruthy();
    expect(screen.getByTestId('stage-panel-teardown')).toBeTruthy();

    expect(screen.getByTestId('stage-panel-setup').textContent).toContain('42ms');
    expect(screen.getByTestId('stage-panel-main').textContent).toContain('7ms');
    expect(screen.getByTestId('stage-panel-teardown').textContent).toContain('120ms');

    // 默认展开：节点流水可见
    expect(screen.getByText('Setup Login')).toBeTruthy();
    expect(screen.getByText('Main Request')).toBeTruthy();
    expect(screen.getByText('Notify Webhook')).toBeTruthy();
  });

  it('Teardown 失败时展示错误摘要', () => {
    render(<StagesSection stages={stages} />);
    expect(screen.getAllByText('teardown notify timeout').length).toBeGreaterThan(0);
  });

  it('点击面板标题折叠节点流水', () => {
    render(<StagesSection stages={stages} />);
    expect(screen.getByText('Setup Login')).toBeTruthy();

    fireEvent.click(screen.getByTestId('stage-panel-setup'));
    expect(screen.queryByText('Setup Login')).toBeNull();
  });

  it('未配置的阶段展示"未配置"占位', () => {
    render(
      <StagesSection
        stages={{ main: { status: 'success', duration_ms: 3, node_reports: [] } }}
      />,
    );

    expect(screen.getByTestId('stage-panel-setup').getAttribute('data-configured')).toBe('false');
    expect(screen.getByTestId('stage-panel-setup').textContent).toContain('未配置');
    expect(screen.getByTestId('stage-panel-teardown').textContent).toContain('未配置');
    expect(screen.getByTestId('stage-panel-main').getAttribute('data-configured')).toBeNull();
    expect(screen.getByTestId('stage-panel-main').textContent).toContain('3ms');
  });

  it('nodeRowLimit 限制每个阶段展示的节点流水行数', () => {
    render(<StagesSection stages={stages} nodeRowLimit={1} />);
    expect(screen.getByText('Main Request')).toBeTruthy();
    expect(screen.queryByText('Main Assert')).toBeNull();
  });
});
