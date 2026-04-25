/**
 * DatasetExecution Store 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useDatasetExecutionStore, selectProgressPercent } from '@/stores/datasetExecution';
import type {
  DatasetExecutionReport,
  DatasetEvent,
} from '@/types/dataset';

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

import { toast } from 'sonner';

// Mock IPC 服务
vi.mock('@/services/tauri', () => ({
  executeFlowWithDataset: vi.fn(),
  cancelDatasetExecution: vi.fn(),
  listDatasetReports: vi.fn(),
}));

// ============================================================================
// Mock 数据
// ============================================================================

function createMockReport(overrides: Partial<DatasetExecutionReport> = {}): DatasetExecutionReport {
  return {
    uuid: 'report-1',
    flow_uuid: 'flow-1',
    flow_name: '测试流程',
    dataset_path: '/data/test.csv',
    dataset_name: 'test.csv',
    encoding: 'utf-8',
    executed_at: 1000000,
    total_duration_ms: 5000,
    total_rows: 10,
    success_count: 8,
    fail_count: 1,
    error_count: 1,
    avg_duration_ms: 500,
    p50_duration_ms: 450,
    p95_duration_ms: 800,
    p99_duration_ms: 900,
    row_summaries: [],
    ...overrides,
  };
}

// ============================================================================
// 测试
// ============================================================================

describe('DatasetExecutionStore', () => {
  beforeEach(() => {
    useDatasetExecutionStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('初始状态正确', () => {
      const state = useDatasetExecutionStore.getState();

      expect(state.isExecuting).toBe(false);
      expect(state.totalRows).toBe(0);
      expect(state.completedRows).toBe(0);
      expect(state.successCount).toBe(0);
      expect(state.failCount).toBe(0);
      expect(state.errorCount).toBe(0);
      expect(state.elapsedTimeMs).toBe(0);
      expect(state.estimatedRemainingMs).toBeNull();
      expect(state.currentReport).toBeNull();
      expect(state.error).toBeNull();
      expect(state.isReportDialogOpen).toBe(false);
      expect(state.history).toEqual([]);
    });
  });

  describe('handleEvent', () => {
    it('started 事件设置 totalRows', () => {
      const event: DatasetEvent = {
        type: 'started',
        total_rows: 100,
        dataset_name: 'data.csv',
      };
      useDatasetExecutionStore.getState().handleEvent(event);

      expect(useDatasetExecutionStore.getState().totalRows).toBe(100);
    });

    it('row_completed 事件递增 completedRows 和 successCount', () => {
      useDatasetExecutionStore.getState().handleEvent({
        type: 'started',
        total_rows: 10,
        dataset_name: 'test.csv',
      });
      useDatasetExecutionStore.getState().handleEvent({
        type: 'row_completed',
        row_index: 0,
        status: 'success',
        duration_ms: 100,
      });

      const state = useDatasetExecutionStore.getState();
      expect(state.completedRows).toBe(1);
      expect(state.successCount).toBe(1);
    });

    it('row_failed 事件递增 completedRows 和 failCount', () => {
      useDatasetExecutionStore.getState().handleEvent({
        type: 'row_failed',
        row_index: 1,
        error: '断言失败',
        duration_ms: 200,
      });

      const state = useDatasetExecutionStore.getState();
      expect(state.completedRows).toBe(1);
      expect(state.failCount).toBe(1);
    });

    it('progress 事件更新所有进度字段', () => {
      useDatasetExecutionStore.getState().handleEvent({
        type: 'progress',
        completed: 5,
        total: 10,
        success_count: 4,
        fail_count: 1,
        error_count: 0,
        elapsed_ms: 500,
        estimated_remaining_ms: 500,
      });

      const state = useDatasetExecutionStore.getState();
      expect(state.completedRows).toBe(5);
      expect(state.successCount).toBe(4);
      expect(state.failCount).toBe(1);
      expect(state.errorCount).toBe(0);
      expect(state.elapsedTimeMs).toBe(500);
      expect(state.estimatedRemainingMs).toBe(500);
    });

    it('progress 事件无 estimated_remaining_ms 时设为 null', () => {
      useDatasetExecutionStore.getState().handleEvent({
        type: 'progress',
        completed: 1,
        total: 10,
        success_count: 1,
        fail_count: 0,
        error_count: 0,
        elapsed_ms: 100,
      });

      expect(useDatasetExecutionStore.getState().estimatedRemainingMs).toBeNull();
    });

    it('ended 事件设置 isExecuting=false 并存储报告', () => {
      const report = createMockReport();
      useDatasetExecutionStore.getState().handleEvent({
        type: 'ended',
        report,
      });

      const state = useDatasetExecutionStore.getState();
      expect(state.isExecuting).toBe(false);
      expect(state.currentReport).toEqual(report);
    });

    it('ended 事件显示 toast 成功通知', () => {
      const report = createMockReport({
        total_rows: 10,
        success_count: 8,
        fail_count: 1,
        error_count: 1,
      });
      useDatasetExecutionStore.getState().handleEvent({
        type: 'ended',
        report,
      });

      expect(toast.success).toHaveBeenCalledWith(
        '数据集执行完成',
        expect.objectContaining({
          description: '10 行 · 成功 8 · 失败 2',
        })
      );
    });

    it('cancelled 事件设置 isExecuting=false 并存储部分报告', () => {
      const report = createMockReport({ total_rows: 5, success_count: 3 });
      useDatasetExecutionStore.getState().handleEvent({
        type: 'cancelled',
        report,
      });

      const state = useDatasetExecutionStore.getState();
      expect(state.isExecuting).toBe(false);
      expect(state.currentReport).toEqual(report);
    });

    it('cancelled 事件显示 toast 取消通知', () => {
      useDatasetExecutionStore.getState().handleEvent({
        type: 'cancelled',
        report: createMockReport(),
      });

      expect(toast.info).toHaveBeenCalledWith('数据集执行已取消');
    });
  });

  describe('完整事件序列', () => {
    it('started → row_completed × 3 → progress → ended 序列正确', () => {
      // 注意：每次调用 getState() 获取最新状态
      useDatasetExecutionStore.getState().handleEvent({ type: 'started', total_rows: 3, dataset_name: 'test.csv' });
      expect(useDatasetExecutionStore.getState().totalRows).toBe(3);

      useDatasetExecutionStore.getState().handleEvent({ type: 'row_completed', row_index: 0, status: 'success', duration_ms: 100 });
      useDatasetExecutionStore.getState().handleEvent({ type: 'row_completed', row_index: 1, status: 'success', duration_ms: 120 });
      useDatasetExecutionStore.getState().handleEvent({ type: 'row_completed', row_index: 2, status: 'success', duration_ms: 90 });

      const state = useDatasetExecutionStore.getState();
      expect(state.completedRows).toBe(3);
      expect(state.successCount).toBe(3);

      const report = createMockReport({ total_rows: 3, success_count: 3 });
      useDatasetExecutionStore.getState().handleEvent({ type: 'ended', report });

      expect(useDatasetExecutionStore.getState().isExecuting).toBe(false);
      expect(useDatasetExecutionStore.getState().currentReport).toEqual(report);
    });
  });

  describe('openReportDialog / closeReportDialog', () => {
    it('打开和关闭报告 Dialog', () => {
      useDatasetExecutionStore.getState().openReportDialog();
      expect(useDatasetExecutionStore.getState().isReportDialogOpen).toBe(true);

      useDatasetExecutionStore.getState().closeReportDialog();
      expect(useDatasetExecutionStore.getState().isReportDialogOpen).toBe(false);
    });

    it('关闭 Dialog 不清除报告数据', () => {
      const report = createMockReport();
      useDatasetExecutionStore.getState().handleEvent({ type: 'ended', report });
      useDatasetExecutionStore.getState().openReportDialog();
      useDatasetExecutionStore.getState().closeReportDialog();

      expect(useDatasetExecutionStore.getState().currentReport).toEqual(report);
      expect(useDatasetExecutionStore.getState().isReportDialogOpen).toBe(false);
    });
  });

  describe('reset', () => {
    it('重置后恢复初始状态', () => {
      const store = useDatasetExecutionStore.getState();
      store.handleEvent({ type: 'started', total_rows: 10, dataset_name: 'test.csv' });
      store.handleEvent({ type: 'progress', completed: 5, total: 10, success_count: 4, fail_count: 1, error_count: 0, elapsed_ms: 500, estimated_remaining_ms: 500 });
      store.handleEvent({ type: 'ended', report: createMockReport() });
      store.openReportDialog();

      store.reset();

      const state = useDatasetExecutionStore.getState();
      expect(state.isExecuting).toBe(false);
      expect(state.totalRows).toBe(0);
      expect(state.completedRows).toBe(0);
      expect(state.successCount).toBe(0);
      expect(state.failCount).toBe(0);
      expect(state.errorCount).toBe(0);
      expect(state.elapsedTimeMs).toBe(0);
      expect(state.estimatedRemainingMs).toBeNull();
      expect(state.currentReport).toBeNull();
      expect(state.error).toBeNull();
      expect(state.isReportDialogOpen).toBe(false);
    });
  });

  describe('Selectors', () => {
    it('selectProgressPercent 计算正确', () => {
      expect(selectProgressPercent(useDatasetExecutionStore.getState())).toBe(0);

      useDatasetExecutionStore.getState().handleEvent({
        type: 'progress',
        completed: 5,
        total: 10,
        success_count: 5,
        fail_count: 0,
        error_count: 0,
        elapsed_ms: 500,
      });

      expect(selectProgressPercent(useDatasetExecutionStore.getState())).toBe(50);
    });
  });
});
