/**
 * 执行状态和报告管理
 */
import { create } from 'zustand';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types';

interface ExecutionState {
  /** 是否正在执行 */
  isExecuting: boolean;
  /** 执行报告 */
  report: FlowExecutionReport | null;
  /** 错误信息 */
  error: string | null;
  /** 执行历史（最近 10 条） */
  history: FlowExecutionReport[];

  // ===== Actions =====
  /** 开始执行 */
  startExecution: () => void;
  /** 设置执行报告 */
  setReport: (report: FlowExecutionReport | null) => void;
  /** 设置错误 */
  setError: (error: string | null) => void;
  /** 重置执行状态 */
  reset: () => void;
  /** 清除错误 */
  clearError: () => void;
  /** 清除历史记录 */
  clearHistory: () => void;

  // ===== Computed (Selectors) =====
  /** 获取特定节点的执行报告 */
  getNodeReport: (nodeUuid: string) => NodeExecutionReport | null;
  /** 检查是否有执行结果 */
  hasResult: () => boolean;
}

const MAX_HISTORY_SIZE = 10;

const initialState = {
  isExecuting: false,
  report: null,
  error: null,
  history: [],
};

export const useExecutionStore = create<ExecutionState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  startExecution: () =>
    set({
      isExecuting: true,
      report: null,
      error: null,
    }),

  setReport: (report) =>
    set((state) => {
      // 更新历史记录
      const newHistory = report
        ? [report, ...state.history].slice(0, MAX_HISTORY_SIZE)
        : state.history;

      return {
        isExecuting: false,
        report,
        history: newHistory,
      };
    }),

  setError: (error) =>
    set({
      isExecuting: false,
      error,
    }),

  reset: () => set(initialState),

  clearError: () => set({ error: null }),

  clearHistory: () => set({ history: [] }),

  // ===== Computed (Selectors) =====
  getNodeReport: (nodeUuid: string) => {
    const { report } = get();
    if (!report) return null;
    return (
      report.node_reports.find((r) => r.node_uuid === nodeUuid) || null
    );
  },

  hasResult: () => {
    const { report, error } = get();
    return report !== null || error !== null;
  },
}));

// ===== Selectors =====
/** 选择执行状态 */
export const selectIsExecuting = (state: ExecutionState) => state.isExecuting;
/** 选择执行报告 */
export const selectReport = (state: ExecutionState) => state.report;
/** 选择错误信息 */
export const selectError = (state: ExecutionState) => state.error;
/** 选择执行历史 */
export const selectHistory = (state: ExecutionState) => state.history;
/** 选择执行状态文本 */
export const selectExecutionStatus = (state: ExecutionState) => {
  if (state.isExecuting) return 'executing';
  if (state.error) return 'error';
  if (state.report) return state.report.status;
  return 'idle';
};
