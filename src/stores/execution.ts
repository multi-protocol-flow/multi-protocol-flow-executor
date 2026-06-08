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

export const useExecutionStore = create<ExecutionState>((set, get) => {
  // T21: setState 批量更新缓冲（预留 — 切换 handleExecutionEvent 时启用）
  // ════════════════════════════════════════════════════════════════════════
  // 通过 queueMicrotask 将多个 handleExecutionEvent 合并为一次 setState 调用，
  // 减少高频事件下 React 重渲染次数。
  //
  //   启用步骤：
  //   1. 取消下面 3 行注释（类型 + 变量 + merge 函数）
  //   2. 取消 scheduleFlush 函数的注释
  //   3. 将 handleExecutionEvent 中各分支的 set(...) 替换为
  //      eventBuffer.push({ updater: ... }) + scheduleFlush()
  //
  //   // 缓冲条目类型
  //   interface EventBufferEntry {
  //     updater: Partial<ExecutionState> | ((state: ExecutionState) => Partial<ExecutionState>);
  //   }
  //   let eventBuffer: EventBufferEntry[] = [];
  //   let flushScheduled = false;
  //
  //   // 合并所有缓冲条目为单个 Partial<ExecutionState>
  //   function mergeBufferedEvents(state: ExecutionState, entries: EventBufferEntry[]): Partial<ExecutionState> {
  //     let merged: Partial<ExecutionState> = {};
  //     for (const entry of entries) {
  //       const partial = typeof entry.updater === 'function'
  //         ? entry.updater(state)
  //         : entry.updater;
  //       Object.assign(merged, partial);
  //     }
  //     return merged;
  //   }
  //
  //   // 在下一个微任务执行一次合并 set
  //   const scheduleFlush = () => {
  //     if (!flushScheduled) {
  //       flushScheduled = true;
  //       queueMicrotask(() => {
  //         flushScheduled = false;
  //         if (eventBuffer.length === 0) return;
  //         const currentState = get();
  //         const merged = mergeBufferedEvents(currentState, eventBuffer);
  //         eventBuffer = [];
  //         set(merged);
  //       });
  //     }
  //   };
  // ════════════════════════════════════════════════════════════════════════

  return {
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
      // TODO: 切换为批量缓冲模式时，将以下 set 调用替换为 eventBuffer.push + scheduleFlush()
      // 示例：
      //   eventBuffer.push({ updater: { isExecuting: true, ... } });
      //   scheduleFlush();
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
          currentNodeUuid: null,
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
      const { nodeReportsMap, currentNodeUuid, isExecuting, report } = get();
      // 1. 优先从增量 map 读取（节流期间可能缺失最新条目）
      if (nodeReportsMap[nodeUuid]) {
        const status = nodeReportsMap[nodeUuid].status;
        if (status === 'skipped') return 'pending';
        return status;
      }
      // 2. 正在执行中的节点
      if (isExecuting && currentNodeUuid === nodeUuid) {
        return 'running';
      }
      // 3. fallback 到最终报告（flowCompleted 后 report 已完整但 nodeReportsMap 可能尚未 catch up）
      if (report) {
        const nodeReport = report.node_reports.find((r) => r.node_uuid === nodeUuid);
        if (nodeReport) {
          const status = nodeReport.status;
          if (status === 'skipped') return 'pending';
          return status;
        }
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
  };
});

// ===== Selectors =====
/** 选择执行状态 */
export const selectIsExecuting = (state: ExecutionState) => state.isExecuting;
/** 选择当前执行中的节点 UUID（用于与节点 ID 做相等性比较，替代 selectIsNodeRunning 工厂 selector） */
export const selectRunningNodeUuid = (state: ExecutionState) => state.currentNodeUuid;
/** 选择是否正在执行中（布尔标志，与 selectRunningNodeUuid 配合使用） */
export const selectIsExecutingFlag = (state: ExecutionState) => state.isExecuting;
/** 选择指定节点是否正在执行中（isExecuting && currentNodeUuid === nodeId） */
export const selectIsNodeRunning = (nodeId: string) => (state: ExecutionState) =>
  state.isExecuting && state.currentNodeUuid === nodeId;
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
/** 按 UUID 选择特定节点的执行报告（工厂选择器，用于需要单节点报告的组件） */
export const selectNodeReportByUuid = (uuid: string) => (state: ExecutionState) =>
  state.nodeReportsMap[uuid];
