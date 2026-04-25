/**
 * 数据集执行状态管理
 *
 * 管理数据驱动执行的状态、进度和报告
 */
import { create } from 'zustand';
import type {
  DatasetExecutionReport,
  DatasetEvent,
  DatasetExecutionConfig,
  DatasetReportSummary,
} from '@/types/dataset';
import type { Flow } from '@/types/flow';
import {
  executeFlowWithDataset as executeFlowWithDatasetService,
  cancelDatasetExecution as cancelDatasetExecutionService,
  listDatasetReports as listDatasetReportsService,
} from '@/services/tauri';
import { useSettingsStore } from './settings';
import { toast } from 'sonner';

// ============================================================================
// 类型定义
// ============================================================================

interface DatasetExecutionState {
  // ===== 状态 =====
  /** 是否正在执行 */
  isExecuting: boolean;
  /** 总行数 */
  totalRows: number;
  /** 已完成行数 */
  completedRows: number;
  /** 成功数 */
  successCount: number;
  /** 失败数 */
  failCount: number;
  /** 错误数 */
  errorCount: number;
  /** 已消耗时间（毫秒） */
  elapsedTimeMs: number;
  /** 预计剩余时间（毫秒） */
  estimatedRemainingMs: number | null;
  /** 当前报告 */
  currentReport: DatasetExecutionReport | null;
  /** 错误信息 */
  error: string | null;
  /** 报告历史列表 */
  history: DatasetReportSummary[];
  /** 是否正在加载历史 */
  isLoadingHistory: boolean;
  /** 报告详情 Dialog 是否打开 */
  isReportDialogOpen: boolean;

  // ===== 操作 =====
  /** 启动数据驱动执行 */
  startExecution: (flow: Flow, config: DatasetExecutionConfig) => Promise<void>;
  /** 取消数据驱动执行 */
  cancelExecution: () => Promise<void>;
  /** 处理执行事件 */
  handleEvent: (event: DatasetEvent) => void;
  /** 加载报告历史列表 */
  loadHistory: () => Promise<void>;
  /** 打开报告 Dialog */
  openReportDialog: () => void;
  /** 关闭报告 Dialog（不清除报告数据） */
  closeReportDialog: () => void;
  /** 重置状态 */
  reset: () => void;
}

// ============================================================================
// 初始状态
// ============================================================================

const initialState = {
  isExecuting: false,
  totalRows: 0,
  completedRows: 0,
  successCount: 0,
  failCount: 0,
  errorCount: 0,
  elapsedTimeMs: 0,
  estimatedRemainingMs: null as number | null,
  currentReport: null as DatasetExecutionReport | null,
  error: null as string | null,
  history: [] as DatasetReportSummary[],
  isLoadingHistory: false,
  isReportDialogOpen: false,
};

// ============================================================================
// Store 实现
// ============================================================================

export const useDatasetExecutionStore = create<DatasetExecutionState>((set, get) => ({
  ...initialState,

  startExecution: async (flow: Flow, config: DatasetExecutionConfig) => {
    try {
      set({
        isExecuting: true,
        totalRows: 0,
        completedRows: 0,
        successCount: 0,
        failCount: 0,
        errorCount: 0,
        elapsedTimeMs: 0,
        estimatedRemainingMs: null,
        currentReport: null,
        error: null,
      });

      await executeFlowWithDatasetService(flow, config, (event) => {
        get().handleEvent(event);
      });
    } catch (err) {
      set({
        isExecuting: false,
        error: (err as Error).message,
      });
    }
  },

  cancelExecution: async () => {
    try {
      await cancelDatasetExecutionService();
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  handleEvent: (event: DatasetEvent) => {
    switch (event.type) {
      case 'started':
        set({ totalRows: event.total_rows });
        break;
      case 'row_completed':
        set((s) => ({
          completedRows: s.completedRows + 1,
          successCount: s.successCount + 1,
        }));
        break;
      case 'row_failed':
        set((s) => ({
          completedRows: s.completedRows + 1,
          failCount: s.failCount + 1,
        }));
        break;
      case 'progress':
        set({
          totalRows: event.total,
          completedRows: event.completed,
          successCount: event.success_count,
          failCount: event.fail_count,
          errorCount: event.error_count,
          elapsedTimeMs: event.elapsed_ms,
          estimatedRemainingMs: event.estimated_remaining_ms ?? null,
        });
        break;
      case 'ended':
        set({
          isExecuting: false,
          currentReport: event.report,
        });
        // 显示完成通知
        toast.success('数据集执行完成', {
          description: `${event.report.total_rows} 行 · 成功 ${event.report.success_count} · 失败 ${event.report.fail_count + event.report.error_count}`,
          action: { label: '查看报告', onClick: () => get().openReportDialog() },
          duration: 10000,
        });
        break;
      case 'cancelled':
        set({
          isExecuting: false,
          currentReport: event.report,
        });
        toast.info('数据集执行已取消');
        break;
    }
  },

  loadHistory: async () => {
    set({ isLoadingHistory: true });
    try {
      const reportsDir = useSettingsStore.getState().reportDir;
      if (reportsDir) {
        const history = await listDatasetReportsService(reportsDir);
        set({ history, isLoadingHistory: false });
      } else {
        set({ isLoadingHistory: false });
      }
    } catch {
      set({ isLoadingHistory: false });
    }
  },

  openReportDialog: () => set({ isReportDialogOpen: true }),
  closeReportDialog: () => set({ isReportDialogOpen: false }),

  reset: () => set(initialState),
}));

// ============================================================================
// Selectors
// ============================================================================

/** 选择是否正在执行 */
export const selectIsExecuting = (state: DatasetExecutionState) => state.isExecuting;

/** 选择执行进度百分比 */
export const selectProgressPercent = (state: DatasetExecutionState) =>
  state.totalRows > 0 ? Math.round((state.completedRows / state.totalRows) * 100) : 0;

/** 选择当前报告 */
export const selectCurrentReport = (state: DatasetExecutionState) => state.currentReport;

/** 选择错误信息 */
export const selectError = (state: DatasetExecutionState) => state.error;

/** 选择报告历史列表 */
export const selectHistory = (state: DatasetExecutionState) => state.history;
