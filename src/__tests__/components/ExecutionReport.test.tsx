/**
 * ExecutionReport 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ExecutionReport } from '@/components/execution/ExecutionReport';
import { useExecutionStore } from '@/stores';
import type { FlowExecutionReport } from '@/types';

// Mock Zustand store
vi.mock('@/stores', () => ({
  useExecutionStore: vi.fn(),
}));

// Mock scroll-area to avoid Radix issues in tests
vi.mock('@/components/ui/scroll-area', () => ({
  ScrollArea: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="scroll-area">{children}</div>
  ),
}));

describe('ExecutionReport', () => {
  const mockCloseReportDialog = vi.fn();
  const mockReport: FlowExecutionReport = {
    uuid: 'report-001',
    flow_uuid: 'flow-001',
    flow_name: '测试流程',
    start_time: Date.now() - 1000,
    end_time: Date.now(),
    duration_ms: 1000,
    total_nodes: 3,
    executed_count: 3,
    status: 'success',
    node_reports: [
      {
        node_uuid: 'node-001',
        node_type: 'entry',
        node_name: '入口节点',
        start_time: Date.now() - 1000,
        end_time: Date.now() - 900,
        duration_ms: 100,
        status: 'success',
        input_data: null,
        output_data: { message: '开始执行' },
      },
      {
        node_uuid: 'node-002',
        node_type: 'http',
        node_name: 'HTTP请求节点',
        start_time: Date.now() - 900,
        end_time: Date.now() - 400,
        duration_ms: 500,
        status: 'success',
        input_data: { url: 'https://api.example.com' },
        output_data: { status: 200, data: { id: 1 } },
      },
      {
        node_uuid: 'node-003',
        node_type: 'end',
        node_name: '结束节点',
        start_time: Date.now() - 400,
        end_time: Date.now(),
        duration_ms: 400,
        status: 'success',
        input_data: { result: 'success' },
        output_data: null,
      },
    ],
    statistics: {
      success_count: 3,
      failed_count: 0,
      avg_node_duration_ms: 333.33,
      max_node_duration_ms: 500,
      min_node_duration_ms: 100,
      completion_rate: 1,
      error_rate: 0,
    },
  };

  const failedReport: FlowExecutionReport = {
    ...mockReport,
    flow_name: '失败的流程',
    status: 'failed',
    statistics: {
      success_count: 2,
      failed_count: 1,
      avg_node_duration_ms: 200,
      completion_rate: 1,
      error_rate: 0.33,
    },
    node_reports: [
      mockReport.node_reports[0],
      {
        ...mockReport.node_reports[1],
        status: 'failed',
        error: {
          error_kind: 'timeout',
          message: '请求超时',
        },
      },
      mockReport.node_reports[2],
    ],
    flow_error: {
      error_kind: 'execution',
      message: '流程执行失败：节点执行出错',
    },
  };

  /** 创建 store mock state 的辅助函数 */
  const createStoreState = (overrides: Partial<{
    report: FlowExecutionReport | null;
    isReportDialogOpen: boolean;
    selectedHistoryReport: FlowExecutionReport | null;
    closeReportDialog: ReturnType<typeof vi.fn>;
  }> = {}) => ({
    report: null as FlowExecutionReport | null,
    isReportDialogOpen: false,
    selectedHistoryReport: null as FlowExecutionReport | null,
    closeReportDialog: mockCloseReportDialog,
    ...overrides,
  });

  /** 创建 selector mock */
  const mockStoreState = (state: Record<string, unknown>) => {
    (useExecutionStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) => {
        return selector ? selector(state) : state;
      }
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('当 isReportDialogOpen 为 false 时不渲染任何内容', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: false }));

    const { container } = render(<ExecutionReport />);
    expect(container.firstChild).toBeNull();
  });

  it('setReport 单独不打开弹窗（isReportDialogOpen 为 false）', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: false }));

    const { container } = render(<ExecutionReport />);
    expect(container.firstChild).toBeNull();
  });

  it('openReportDialog 打开弹窗后正确渲染成功的执行报告', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: true }));

    render(<ExecutionReport />);

    // 检查标题
    expect(screen.getByText('执行报告')).toBeDefined();
    // 成功状态会出现在多个地方（标题和统计卡片），使用 getAllByText
    expect(screen.getAllByText('成功').length).toBeGreaterThan(0);

    // 检查流程名称
    expect(screen.getByText('测试流程')).toBeDefined();

    // 检查统计信息
    expect(screen.getByText('1000ms')).toBeDefined();
    expect(screen.getByText('3/3')).toBeDefined();

    // 检查节点详情 - 分栏布局下节点名称在左右两侧都会出现
    expect(screen.getAllByText('入口节点').length).toBeGreaterThan(0);
    expect(screen.getAllByText('HTTP请求节点').length).toBeGreaterThan(0);
    expect(screen.getAllByText('结束节点').length).toBeGreaterThan(0);
  });

  it('正确渲染失败的执行报告', () => {
    mockStoreState(createStoreState({ report: failedReport, isReportDialogOpen: true }));

    render(<ExecutionReport />);

    // 检查失败状态
    expect(screen.getAllByText('失败').length).toBeGreaterThan(0);
    expect(screen.getByText('失败的流程')).toBeDefined();

    // 检查错误信息显示
    expect(screen.getByText('流程错误')).toBeDefined();
    expect(screen.getByText('流程执行失败：节点执行出错')).toBeDefined();
  });

  it('点击关闭按钮时调用 closeReportDialog', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: true }));

    render(<ExecutionReport />);

    // Dialog 的关闭按钮由 Radix UI 处理
    // 模拟关闭操作
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeDefined();

    // 按 Escape 键关闭
    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(mockCloseReportDialog).toHaveBeenCalled();
  });

  it('正确显示节点类型标签', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: true }));

    render(<ExecutionReport />);

    // 检查节点类型标签 - 分栏布局下类型标签在左右两侧都会出现
    expect(screen.getAllByText('入口').length).toBeGreaterThan(0);
    // HTTP 类型标签
    expect(screen.getAllByText('HTTP').length).toBeGreaterThan(0);
  });

  it('显示正确的统计卡片数值', () => {
    mockStoreState(createStoreState({ report: mockReport, isReportDialogOpen: true }));

    render(<ExecutionReport />);

    // 统计标签
    expect(screen.getByText('总耗时')).toBeDefined();
    expect(screen.getByText('执行节点')).toBeDefined();
    // 成功/失败会出现在多个地方
    expect(screen.getAllByText('成功').length).toBeGreaterThan(0);
    expect(screen.getByText('失败')).toBeDefined();
  });

  it('当 avg_node_duration_ms 为 null 时不显示平均耗时（修复白屏问题）', () => {
    const reportWithNullStats: FlowExecutionReport = {
      ...mockReport,
      statistics: {
        success_count: 3,
        failed_count: 0,
        // 模拟调试会话立即结束的情况，统计信息为 null
        avg_node_duration_ms: null as unknown as undefined,
        max_node_duration_ms: null as unknown as undefined,
        min_node_duration_ms: null as unknown as undefined,
        completion_rate: 1,
        error_rate: 0,
      },
    };

    mockStoreState(createStoreState({ report: reportWithNullStats, isReportDialogOpen: true }));

    // 不应该崩溃（Dialog 使用 Portal 渲染到 body，所以用 screen 检查）
    render(<ExecutionReport />);
    expect(screen.getByRole('dialog')).toBeDefined();

    // 不应该显示平均耗时
    expect(screen.queryByText('平均耗时:')).toBeNull();
  });

  it('当 statistics 字段全部为 undefined 时正常渲染', () => {
    const reportWithEmptyStats: FlowExecutionReport = {
      ...mockReport,
      statistics: {
        success_count: 0,
        failed_count: 0,
        // 所有可选字段都为 undefined（模拟调试会话立即结束）
        avg_node_duration_ms: undefined,
        max_node_duration_ms: undefined,
        min_node_duration_ms: undefined,
        total_input_size_bytes: undefined,
        total_output_size_bytes: undefined,
        error_rate: undefined,
        completion_rate: undefined,
      },
    };

    mockStoreState(createStoreState({ report: reportWithEmptyStats, isReportDialogOpen: true }));

    // 不应该崩溃或白屏（Dialog 使用 Portal 渲染到 body，所以用 screen 检查）
    render(<ExecutionReport />);
    expect(screen.getByRole('dialog')).toBeDefined();
    expect(screen.getByText('执行报告')).toBeDefined();
  });

  it('selectedHistoryReport 优先于 report 显示', () => {
    const historyReport: FlowExecutionReport = {
      ...mockReport,
      flow_name: '历史报告流程',
      uuid: 'history-report-001',
    };

    mockStoreState(createStoreState({
      report: mockReport,
      selectedHistoryReport: historyReport,
      isReportDialogOpen: true,
    }));

    render(<ExecutionReport />);

    // 应显示 selectedHistoryReport 的流程名称
    expect(screen.getByText('历史报告流程')).toBeDefined();
    // 不应显示 report 的流程名称
    expect(screen.queryByText('测试流程')).toBeNull();
  });

  it('仅 isReportDialogOpen 为 true 但 displayReport 为 null 时不渲染', () => {
    mockStoreState(createStoreState({
      report: null,
      selectedHistoryReport: null,
      isReportDialogOpen: true,
    }));

    const { container } = render(<ExecutionReport />);
    expect(container.firstChild).toBeNull();
  });
});
