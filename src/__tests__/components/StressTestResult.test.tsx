/**
 * StressTestResult 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { StressTestResult } from '@/components/stress-test/StressTestResult';
import { useStressTestStore } from '@/stores/stressTest';
import type { StressTestReport } from '@/types/stressTest';

// Mock dialog and services
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock('@/services/tauri', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/tauri')>();
  return {
    ...actual,
    getStressReportMarkdown: vi.fn().mockResolvedValue('## Mock Stress Markdown'),
    exportStressReportHtml: vi.fn().mockResolvedValue(undefined),
  };
});

describe('StressTestResult', () => {
  const mockReport: StressTestReport = {
    uuid: 'stress-report-001',
    flowUuid: 'flow-001',
    flowName: '压测流程',
    startTime: 1000,
    endTime: 61000,
    durationMs: 60000,
    config: {
      concurrentUsers: 20,
      rampUpSeconds: 5,
      durationSeconds: 60,
      flowUuid: 'flow-001',
    },
    statistics: {
      totalIterations: 1000,
      successful: 990,
      failed: 10,
      successRate: 0.99,
      rps: 16.5,
    },
    responseTimeDistribution: {
      avgMs: 55.4,
      minMs: 12,
      maxMs: 340,
      p50Ms: 45,
      p90Ms: 90,
      p95Ms: 120,
      p99Ms: 200,
    },
    nodeStatistics: [],
    errorStatistics: [],
  };

  beforeEach(() => {
    useStressTestStore.getState().reset();
  });

  it('当没有打开弹窗时不渲染内容', () => {
    const { container } = render(<StressTestResult />);
    expect(container.firstChild).toBeNull();
  });

  it('打开弹窗后正确渲染报告标题与操作按钮', () => {
    useStressTestStore.setState({
      report: mockReport,
      isReportDialogOpen: true,
    });

    render(<StressTestResult />);

    expect(screen.getByText('压测流程')).toBeDefined();
    expect(screen.getByRole('button', { name: /复制 Markdown 摘要|Copy Markdown Summary/i })).toBeDefined();
    expect(screen.getByRole('button', { name: /导出 HTML 报告|Export HTML Report/i })).toBeDefined();
  });

  it('点击复制 Markdown 摘要按钮时调用 getStressReportMarkdown 并写入剪贴板', async () => {
    useStressTestStore.setState({
      report: mockReport,
      isReportDialogOpen: true,
    });

    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText },
    });

    render(<StressTestResult />);

    const copyBtn = screen.getByRole('button', { name: /复制 Markdown 摘要|Copy Markdown Summary/i });
    fireEvent.click(copyBtn);

    const { getStressReportMarkdown } = await import('@/services/tauri');
    expect(getStressReportMarkdown).toHaveBeenCalledWith(mockReport);
  });

  it('点击导出 HTML 报告按钮时调用 save 对话框与 exportStressReportHtml', async () => {
    useStressTestStore.setState({
      report: mockReport,
      isReportDialogOpen: true,
    });

    const platform = await import('@/lib/platform');
    const isTauriSpy = vi.spyOn(platform, 'isTauriEnv').mockReturnValue(true);
    const { save } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(save).mockResolvedValue('/tmp/stress-report.html');
    const { exportStressReportHtml } = await import('@/services/tauri');

    render(<StressTestResult />);

    const exportBtn = screen.getByRole('button', { name: /导出 HTML 报告|Export HTML Report/i });
    await fireEvent.click(exportBtn);

    expect(save).toHaveBeenCalled();
    expect(exportStressReportHtml).toHaveBeenCalledWith(mockReport, '/tmp/stress-report.html');

    isTauriSpy.mockRestore();
  });
});
