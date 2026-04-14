/**
 * StressTest Store 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useStressTestStore } from '@/stores/stressTest';
import type {
  StressTestReport,
  StressTestProgress,
  StressTestConfig,
} from '@/types/stressTest';

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { toast } from 'sonner';

// ============================================================================
// Mock 数据
// ============================================================================

function createMockConfig(overrides: Partial<StressTestConfig> = {}): StressTestConfig {
  return {
    concurrentUsers: 10,
    rampUpSeconds: 5,
    durationSeconds: 30,
    flowUuid: 'flow-1',
    ...overrides,
  };
}

function createMockReport(overrides: Partial<StressTestReport> = {}): StressTestReport {
  return {
    uuid: 'report-1',
    flowUuid: 'flow-1',
    flowName: '测试流程',
    startTime: 1000,
    endTime: 31000,
    durationMs: 30000,
    config: createMockConfig(),
    statistics: {
      totalIterations: 100,
      successful: 95,
      failed: 5,
      successRate: 0.95,
      rps: 50,
    },
    responseTimeDistribution: {
      avgMs: 100,
      minMs: 10,
      maxMs: 500,
      p50Ms: 80,
      p90Ms: 200,
      p95Ms: 300,
      p99Ms: 450,
    },
    errorStatistics: [],
    nodeStatistics: [],
    nodeTypeStatistics: [],
    ...overrides,
  };
}

function createMockProgress(overrides: Partial<StressTestProgress> = {}): StressTestProgress {
  return {
    status: 'running',
    elapsedSeconds: 10,
    totalIterations: 50,
    successfulIterations: 48,
    failedIterations: 2,
    currentRps: 45,
    avgLatencyMs: 120,
    nodeMetrics: [],
    ...overrides,
  };
}

// ============================================================================
// 测试
// ============================================================================

describe('StressTestStore', () => {
  beforeEach(() => {
    useStressTestStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('初始状态正确', () => {
      const state = useStressTestStore.getState();

      expect(state.status).toBe('idle');
      expect(state.config).toBeNull();
      expect(state.progress).toBeNull();
      expect(state.progressHistory).toEqual([]);
      expect(state.report).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.isReportDialogOpen).toBe(false);
    });
  });

  describe('handleStressTestEvent', () => {
    it('started 事件设置状态为 rampUp 并存储配置', () => {
      const config = createMockConfig();
      useStressTestStore.getState().handleStressTestEvent({
        type: 'started',
        flowUuid: 'flow-1',
        config,
      });

      const state = useStressTestStore.getState();
      expect(state.status).toBe('rampUp');
      expect(state.config).toEqual(config);
      expect(state.error).toBeNull();
    });

    it('progress 事件更新状态和进度', () => {
      const progress = createMockProgress();
      useStressTestStore.getState().handleStressTestEvent({
        type: 'progress',
        progress,
      });

      const state = useStressTestStore.getState();
      expect(state.status).toBe('running');
      expect(state.progress).toEqual(progress);
      expect(state.progressHistory).toEqual([progress]);
    });

    it('多次 progress 事件累积 progressHistory', () => {
      const progress1 = createMockProgress({ elapsedSeconds: 5 });
      const progress2 = createMockProgress({ elapsedSeconds: 10 });
      const progress3 = createMockProgress({ elapsedSeconds: 15 });

      useStressTestStore.getState().handleStressTestEvent({ type: 'progress', progress: progress1 });
      useStressTestStore.getState().handleStressTestEvent({ type: 'progress', progress: progress2 });
      useStressTestStore.getState().handleStressTestEvent({ type: 'progress', progress: progress3 });

      const state = useStressTestStore.getState();
      expect(state.progressHistory).toEqual([progress1, progress2, progress3]);
      expect(state.progress).toEqual(progress3);
    });

    it('ended 事件设置报告和状态为 stopped', () => {
      const report = createMockReport();
      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report,
      });

      const state = useStressTestStore.getState();
      expect(state.status).toBe('stopped');
      expect(state.report).toEqual(report);
      expect(state.progress).toBeNull();
    });
  });

  describe('openReportDialog', () => {
    it('设置 isReportDialogOpen 为 true', () => {
      expect(useStressTestStore.getState().isReportDialogOpen).toBe(false);

      useStressTestStore.getState().openReportDialog();

      expect(useStressTestStore.getState().isReportDialogOpen).toBe(true);
    });
  });

  describe('closeReportDialog', () => {
    it('设置 isReportDialogOpen 为 false', () => {
      useStressTestStore.getState().openReportDialog();
      expect(useStressTestStore.getState().isReportDialogOpen).toBe(true);

      useStressTestStore.getState().closeReportDialog();

      expect(useStressTestStore.getState().isReportDialogOpen).toBe(false);
    });

    it('不清除报告数据', () => {
      // 先通过 ended 事件设置报告
      const report = createMockReport();
      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report,
      });
      expect(useStressTestStore.getState().report).not.toBeNull();

      // 打开 dialog 再关闭
      useStressTestStore.getState().openReportDialog();
      useStressTestStore.getState().closeReportDialog();

      // 报告仍在
      expect(useStressTestStore.getState().report).toEqual(report);
      expect(useStressTestStore.getState().isReportDialogOpen).toBe(false);
    });
  });

  describe('reset', () => {
    it('重置后恢复初始状态', () => {
      // 设置各种状态
      const config = createMockConfig();
      useStressTestStore.getState().handleStressTestEvent({
        type: 'started',
        flowUuid: 'flow-1',
        config,
      });
      useStressTestStore.getState().handleStressTestEvent({
        type: 'progress',
        progress: createMockProgress(),
      });
      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report: createMockReport(),
      });
      useStressTestStore.getState().openReportDialog();

      // 重置
      useStressTestStore.getState().reset();

      const state = useStressTestStore.getState();
      expect(state.status).toBe('idle');
      expect(state.config).toBeNull();
      expect(state.progress).toBeNull();
      expect(state.progressHistory).toEqual([]);
      expect(state.report).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.isReportDialogOpen).toBe(false);
    });
  });

  describe('toast 通知', () => {
    it('ended 事件 successRate >= 0.99 时调用 toast.success', () => {
      const report = createMockReport({
        statistics: {
          ...createMockReport().statistics,
          totalIterations: 200,
          successRate: 0.995,
        },
      });

      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report,
      });

      expect(toast.success).toHaveBeenCalledWith('压力测试完成', {
        description: '200 次迭代 · 成功率 99.5%',
        action: expect.objectContaining({ label: '查看报告' }),
        duration: 10000,
      });
      expect(toast.error).not.toHaveBeenCalled();
    });

    it('ended 事件 successRate < 0.99 时调用 toast.error', () => {
      const report = createMockReport({
        statistics: {
          ...createMockReport().statistics,
          totalIterations: 100,
          successRate: 0.5,
        },
      });

      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report,
      });

      expect(toast.error).toHaveBeenCalledWith('压力测试完成', {
        description: '100 次迭代 · 成功率 50.0%',
        action: expect.objectContaining({ label: '查看报告' }),
        duration: 10000,
      });
      expect(toast.success).not.toHaveBeenCalled();
    });

    it('toast 查看报告 action onClick 调用 openReportDialog', () => {
      const report = createMockReport({
        statistics: {
          ...createMockReport().statistics,
          successRate: 0.995,
        },
      });

      useStressTestStore.getState().handleStressTestEvent({
        type: 'ended',
        report,
      });

      expect(useStressTestStore.getState().isReportDialogOpen).toBe(false);

      // 提取 onClick 回调并调用
      const callArgs = (toast.success as ReturnType<typeof vi.fn>).mock.calls[0][1];
      callArgs.action.onClick();

      expect(useStressTestStore.getState().isReportDialogOpen).toBe(true);
    });
  });
});
