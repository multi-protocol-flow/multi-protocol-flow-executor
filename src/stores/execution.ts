/**
 * 执行状态和报告管理
 */
import { create } from 'zustand';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types';
import type { ExecutionEvent } from '@/types/execution_event';
import { useWsStreamStore } from './wsStream';

interface ExecutionState {
  /** 是否正在执行 */
  isExecuting: boolean;
  /** 执行报告 */
  report: FlowExecutionReport | null;
  /** 错误信息 */
  error: string | null;
  /** 执行历史（最近 10 条） */
  history: FlowExecutionReport[];

  // ===== Real-time execution tracking =====
  /** 增量累积的节点报告 (nodeUuid → NodeExecutionReport) */
  nodeReportsMap: Record<string, NodeExecutionReport>;
  /** 当前正在执行的节点 UUID */
  currentNodeUuid: string | null;
  /** 已完成节点数 */
  completedNodeCount: number;
  /** 总节点数 */
  totalNodes: number;
  /** 报告对话框是否打开 */
  isReportDialogOpen: boolean;
  /** 选中的历史报告（用于对话框展示） */
  selectedHistoryReport: FlowExecutionReport | null;

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
  /** 取消正在执行的流程 */
  cancelExecution: () => Promise<void>;
  /** 处理执行事件（增量更新状态） */
  handleExecutionEvent: (event: ExecutionEvent) => void;
  /** 清空增量执行状态 */
  clearExecutionState: () => void;
  /** 打开报告对话框 */
  openReportDialog: (report?: FlowExecutionReport) => void;
  /** 关闭报告对话框 */
  closeReportDialog: () => void;

  // ===== Computed (Selectors) =====
  /** 获取特定节点的执行报告 */
  getNodeReport: (nodeUuid: string) => NodeExecutionReport | null;
  /** 检查是否有执行结果 */
  hasResult: () => boolean;
  /** 获取节点执行状态 */
  getNodeExecutionStatus: (nodeUuid: string) => 'running' | 'success' | 'failed' | 'cancelled' | 'pending';
  /** 获取节点增量执行报告 */
  getNodeExecutionReport: (nodeUuid: string) => NodeExecutionReport | null;
}

const MAX_HISTORY_SIZE = 10;

const initialState = {
  isExecuting: false,
  report: null,
  error: null,
  history: [],
  nodeReportsMap: {} as Record<string, NodeExecutionReport>,
  currentNodeUuid: null as string | null,
  completedNodeCount: 0,
  totalNodes: 0,
  isReportDialogOpen: false,
  selectedHistoryReport: null as FlowExecutionReport | null,
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

  cancelExecution: async () => {
    try {
      const { cancelFlow } = await import('@/services/tauri');
      await cancelFlow();
    } catch {
      // Session may already be cleaned up, ignore
    }
    set({ isExecuting: false });
  },

  handleExecutionEvent: (event: ExecutionEvent) => {
    if ('flowStarted' in event) {
      const { totalNodes } = event.flowStarted;
      set({
        isExecuting: true,
        nodeReportsMap: {},
        currentNodeUuid: null,
        completedNodeCount: 0,
        totalNodes,
        report: null,
        error: null,
      });
    } else if ('nodeStarted' in event) {
      const { nodeUuid } = event.nodeStarted;
      set({ currentNodeUuid: nodeUuid });
    } else if ('nodeCompleted' in event) {
      const { report } = event.nodeCompleted;
      set((state) => ({
        nodeReportsMap: {
          ...state.nodeReportsMap,
          [report.node_uuid]: report,
        },
        completedNodeCount: state.completedNodeCount + 1,
        currentNodeUuid: null, // Node finished, no longer running
      }));
    } else if ('flowCompleted' in event) {
      const { report } = event.flowCompleted;
      set((state) => {
        const newHistory = [report, ...state.history].slice(0, MAX_HISTORY_SIZE);
        return {
          isExecuting: false,
          report,
          history: newHistory,
        };
      });
    } else if ('flowCancelled' in event) {
      set({ isExecuting: false });
    }
  },

  clearExecutionState: () => {
    useWsStreamStore.getState().resetWsStreams();
    set({
      nodeReportsMap: {},
      currentNodeUuid: null,
      completedNodeCount: 0,
      totalNodes: 0,
      report: null,
      error: null,
    });
  },

  openReportDialog: (report) =>
    set(report !== undefined
      ? { isReportDialogOpen: true, selectedHistoryReport: report }
      : { isReportDialogOpen: true }
    ),

  closeReportDialog: () =>
    set({
      isReportDialogOpen: false,
      selectedHistoryReport: null,
    }),

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

  getNodeExecutionStatus: (nodeUuid: string) => {
    const { nodeReportsMap, currentNodeUuid, isExecuting } = get();
    if (nodeReportsMap[nodeUuid]) {
      const status = nodeReportsMap[nodeUuid].status;
      // 'skipped' 也是终止状态，映射为 'pending'（未执行）
      if (status === 'skipped') return 'pending';
      return status;
    }
    if (isExecuting && currentNodeUuid === nodeUuid) {
      return 'running';
    }
    return 'pending';
  },

  getNodeExecutionReport: (nodeUuid: string) => {
    const { nodeReportsMap, report } = get();
    // First check incremental map, then fall back to final report
    if (nodeReportsMap[nodeUuid]) {
      return nodeReportsMap[nodeUuid];
    }
    if (report) {
      return report.node_reports.find((r) => r.node_uuid === nodeUuid) || null;
    }
    return null;
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
/** 选择增量节点报告 */
export const selectNodeReportsMap = (state: ExecutionState) => state.nodeReportsMap;
/** 选择已完成节点数 */
export const selectCompletedNodeCount = (state: ExecutionState) => state.completedNodeCount;
/** 选择总节点数 */
export const selectTotalNodes = (state: ExecutionState) => state.totalNodes;
