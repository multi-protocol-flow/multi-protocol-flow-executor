/**
 * 数据集执行状态管理
 *
 * 管理数据驱动执行的状态、进度和报告
 */
import { create } from 'zustand';
import { getLL } from '@/i18n/i18n-lazy';
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

const ll = getLL();

/** 当前数据集执行事件监听器的清理函数（模块级） */
let currentDatasetUnlisten: (() => void) | null = null;

/** 清理当前事件监听器 */
function cleanupDatasetListener() {
  if (currentDatasetUnlisten) {
    currentDatasetUnlisten();
    currentDatasetUnlisten = null;
  }
}

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
  /** 当前选中的报告 ID（Dialog 用） */
  selectedReportId: string | null;

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
  openReportDialog: (reportId?: string) => void;
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
  selectedReportId: null as string | null,
};

// ============================================================================
// Store 实现
// ============================================================================

export const useDatasetExecutionStore = create<DatasetExecutionState>((set, get) => ({
  ...initialState,

  startExecution: async (flow: Flow, config: DatasetExecutionConfig) => {
    try {
      // 清理之前残留的监听器
      cleanupDatasetListener();

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

      const unlisten = await executeFlowWithDatasetService(flow, config, (event) => {
        get().handleEvent(event);
      });
      currentDatasetUnlisten = unlisten;
    } catch (err) {
      const errorMsg = (err as Error).message;
      set({
        isExecuting: false,
        error: errorMsg,
      });
      cleanupDatasetListener();
      toast.error(getLL().execution.toast.flow_failed(), {
        description: errorMsg,
        duration: 8000,
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
        // 清理事件监听器
        cleanupDatasetListener();
        set({
          isExecuting: false,
          currentReport: event.report,
        });
        // 保存报告到磁盘（异步，不阻塞通知）
        (async () => {
          try {
            const reportsDir = useSettingsStore.getState().reportDir;
            if (reportsDir) {
              const { saveDatasetReport } = await import('@/services/tauri');
              await saveDatasetReport(event.report, reportsDir);
            }
          } catch (err) {
            console.error('保存数据集报告失败:', err);
          }
        })();
        // 显示完成通知
        toast.success(ll.execution.toast.dataset_completed(), {
          description: `${event.report.total_rows} ${ll.execution.dataset.label_row()} · ${ll.execution.dataset.success()} ${event.report.success_count} · ${ll.execution.dataset.fail()} ${event.report.fail_count + event.report.error_count}`,
          action: { label: ll.execution.toast.view_report(), onClick: () => get().openReportDialog(event.report.uuid) },
          duration: 10000,
        });
        break;
      case 'cancelled':
        cleanupDatasetListener();
        set({
          isExecuting: false,
          currentReport: event.report,
        });
        toast.info(ll.execution.toast.dataset_cancelled());
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

  openReportDialog: (reportId?: string) => set({
    isReportDialogOpen: true,
    selectedReportId: reportId ?? null,
  }),
  closeReportDialog: () => set({
    isReportDialogOpen: false,
    selectedReportId: null,
  }),

  reset: () => {
    cleanupDatasetListener();
    set(initialState);
  },
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
