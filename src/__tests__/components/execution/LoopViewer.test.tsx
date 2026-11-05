/**
 * LoopViewer 测试
 *
 * 重点覆盖**运行中**的形态：报告由实时事件拼出（`iteration_summary` +
 * `sub_flow_report.node_reports` + `truncated_iterations`），与循环结束后
 * 的最终报告走同一个 Viewer。
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LoopViewer } from '@/components/execution/viewers/LoopViewer';
import { buildLiveLoopSubFlowReport } from '@/stores/execution';
import { getLL } from '@/i18n/i18n-lazy';
import type { LoopLiveProgress } from '@/types/execution_event';
import type { ExecutionStatus, NodeExecutionReport } from '@/types';

// Mock react-json-view-lite（NodeExecutionDetails 间接依赖）
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

/** 运行中的 loop 节点基础报告（无最终 output_data） */
function liveBaseReport(): NodeExecutionReport {
  return {
    node_uuid: 'loop-1',
    node_type: 'flow:loop',
    node_name: 'Loop',
    start_time: 1000,
    end_time: 0,
    duration_ms: 0,
    status: 'running' as unknown as ExecutionStatus,
    input_data: null,
    output_data: null,
  };
}

describe('LoopViewer（运行中实时数据）', () => {
  const progress: LoopLiveProgress = {
    iterations: [
      { index: 0, status: 'success', duration_ms: 10, wait_ms: 0 },
      { index: 1, status: 'failed', duration_ms: 20, wait_ms: 0 },
    ],
    reports: [
      {
        node_uuid: 'body-1',
        node_type: 'script',
        node_name: 'body',
        start_time: 1000,
        end_time: 1010,
        duration_ms: 10,
        status: 'success',
        input_data: null,
        output_data: { ok: true },
        iteration_index: 0,
        iteration_of: 'loop-1',
      },
    ],
    truncated: 1,
  };

  function liveReport(): NodeExecutionReport {
    const base = liveBaseReport();
    return {
      ...base,
      iteration_summary: progress.iterations,
      truncated_iterations: progress.truncated,
      sub_flow_report: buildLiveLoopSubFlowReport(base, progress),
    };
  }

  it('渲染每一轮迭代行（所有轮次实时可见）', () => {
    const ll = getLL();
    render(<LoopViewer report={liveReport()} />);

    for (const summary of progress.iterations) {
      expect(
        screen.getByText(ll.execution.report.loop_iteration_label({ index: summary.index }))
      ).toBeDefined();
    }
  });

  it('展示被裁剪轮次数', () => {
    const ll = getLL();
    render(<LoopViewer report={liveReport()} />);

    expect(screen.getByText(ll.execution.report.loop_truncated({ count: 1 }))).toBeDefined();
  });

  it('被裁剪轮次展开后提示详情已省略', () => {
    const ll = getLL();
    render(<LoopViewer report={liveReport()} />);

    // 失败轮默认展开，且该轮没有保留 body 报告
    expect(screen.getByText(ll.execution.report.loop_truncated_hint())).toBeDefined();
  });

  it('保留详情的轮次展开后渲染 body 节点报告', () => {
    const ll = getLL();
    render(<LoopViewer report={liveReport()} />);

    fireEvent.click(
      screen.getByText(ll.execution.report.loop_iteration_label({ index: 0 }))
    );

    // body 节点（script）走 GenericViewer，展开后应出现其输出数据视图
    expect(screen.getAllByTestId('json-view').length).toBeGreaterThan(0);
  });
});
