/**
 * 执行状态和报告管理
 */
import { create } from 'zustand';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types';
import type { ExecutionEvent, PluginEventData, NodeStreamData, NodeStreamChunk, SubFlowProgress } from '@/types/execution_event';
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
  /** 插件流事件日志（有界，保留最近 N 条；P1 消费者） */
  pluginEvents: PluginEventData[];
  /** 节点实时流式数据 (nodeUuid -> NodeStreamData) */
  nodeStreamMap: Record<string, NodeStreamData>;
  /**
   * 子流程实时执行进度 (SubFlow 节点 UUID -> 进度)
   *
   * 子流程在顶层流程中只算**一个节点**，其内部节点的执行事件通过
   * `parentNodeUuid` 归属到这里，用于在 SubFlow 节点上展示嵌套进度。
   */
  subFlowProgressMap: Record<string, SubFlowProgress>;

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
  /** 按需从冷通道拉取节点的完整执行详情 */
  fetchNodeDetail: (flowUuid: string, nodeUuid: string) => Promise<void>;
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

/** 插件流事件日志上限（有界内存，P1 消费者再做完整视图） */
const MAX_PLUGIN_EVENTS = 200;

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
  pluginEvents: [] as PluginEventData[],
  nodeStreamMap: {} as Record<string, NodeStreamData>,
  subFlowProgressMap: {} as Record<string, SubFlowProgress>,
};

export const useExecutionStore = create<ExecutionState>((set, get) => {
  // T21: setState 批量更新缓冲
  // ════════════════════════════════════════════════════════════════════════
  // 通过 queueMicrotask 将多个 handleExecutionEvent 合并为一次 setState 调用，
  // 减少高频事件（流式 token / 连续节点状态）下 React 重渲染次数。
  interface EventBufferEntry {
    updater: Partial<ExecutionState> | ((state: ExecutionState) => Partial<ExecutionState>);
  }
  let eventBuffer: EventBufferEntry[] = [];
  let flushScheduled = false;
  const isTest = typeof process !== 'undefined' && Boolean(process.env.VITEST);

  function applyBufferedEvents(state: ExecutionState, entries: EventBufferEntry[]): Partial<ExecutionState> {
    let acc = { ...state };
    for (const entry of entries) {
      const partial = typeof entry.updater === 'function' ? entry.updater(acc) : entry.updater;
      acc = { ...acc, ...partial };
    }
    return acc;
  }

  const flushBuffer = () => {
    if (eventBuffer.length === 0) return;
    const currentState = get();
    const merged = applyBufferedEvents(currentState, eventBuffer);
    eventBuffer = [];
    set(merged);
  };

  const scheduleFlush = () => {
    if (isTest) {
      flushBuffer();
      return;
    }
    if (!flushScheduled) {
      flushScheduled = true;
      queueMicrotask(() => {
        flushScheduled = false;
        flushBuffer();
      });
    }
  };

  const queueEventUpdate = (updater: Partial<ExecutionState> | ((state: ExecutionState) => Partial<ExecutionState>)) => {
    eventBuffer.push({ updater });
    scheduleFlush();
  };
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
          // 从最终报告同步增量计数器，确保面板计数与报告一致
          completedNodeCount: report?.node_reports.length ?? state.completedNodeCount,
          totalNodes: report?.total_nodes ?? state.totalNodes,
        };
      }),

    setError: (error) =>
      set({
        isExecuting: false,
        error,
      }),

    reset: () => {
      eventBuffer = [];
      set(initialState);
    },

    clearError: () => set({ error: null }),

    clearHistory: () => set({ history: [] }),

    cancelExecution: async () => {
      try {
        const { cancelFlow } = await import('@/services/tauri');
        await cancelFlow();
      } catch {
        // Session may already be cleaned up, ignore
      }
      flushBuffer();
      set({ isExecuting: false });
    },

    handleExecutionEvent: (event: ExecutionEvent) => {
      if ('batch' in event) {
        for (const subEvent of event.batch) {
          get().handleExecutionEvent(subEvent);
        }
        return;
      }
      if ('flowStarted' in event) {
        const { totalNodes, parentNodeUuid } = event.flowStarted;
        if (parentNodeUuid) {
          // 嵌套流程开始：只初始化该 SubFlow 节点的进度，不重置顶层运行状态
          queueEventUpdate((state) => ({
            subFlowProgressMap: {
              ...state.subFlowProgressMap,
              [parentNodeUuid]: {
                completed: 0,
                total: totalNodes,
                currentNodeName: undefined,
              },
            },
          }));
          return;
        }
        flushBuffer();
        set({
          isExecuting: true,
          nodeReportsMap: {},
          currentNodeUuid: null,
          completedNodeCount: 0,
          totalNodes,
          report: null,
          error: null,
          pluginEvents: [],
          nodeStreamMap: {},
          subFlowProgressMap: {},
        });
      } else if ('nodeStarted' in event) {
        const { nodeUuid, nodeName, parentNodeUuid } = event.nodeStarted;
        if (parentNodeUuid) {
          // 子流程内部节点开始：SubFlow 节点保持"运行中"，并记录当前子节点名称
          queueEventUpdate((state) => {
            const prev = state.subFlowProgressMap[parentNodeUuid];
            return {
              subFlowProgressMap: {
                ...state.subFlowProgressMap,
                [parentNodeUuid]: {
                  completed: prev?.completed ?? 0,
                  total: prev?.total ?? 0,
                  currentNodeName: nodeName,
                },
              },
            };
          });
          return;
        }
        queueEventUpdate({ currentNodeUuid: nodeUuid });
      } else if ('nodeCompleted' in event) {
        const { report, parentNodeUuid } = event.nodeCompleted;
        if (parentNodeUuid) {
          // 子流程内部节点完成：只累加该 SubFlow 节点的进度，顶层计数不受影响
          queueEventUpdate((state) => {
            const prev = state.subFlowProgressMap[parentNodeUuid];
            return {
              subFlowProgressMap: {
                ...state.subFlowProgressMap,
                [parentNodeUuid]: {
                  completed: (prev?.completed ?? 0) + 1,
                  total: prev?.total ?? 0,
                  currentNodeName: prev?.currentNodeName,
                },
              },
            };
          });
          return;
        }
        queueEventUpdate((state) => ({
          nodeReportsMap: {
            ...state.nodeReportsMap,
            [report.node_uuid]: report,
          },
          completedNodeCount: state.completedNodeCount + 1,
          currentNodeUuid: null,
        }));
      } else if ('flowCompleted' in event) {
        // 嵌套流程结束：顶层流程仍在运行，报告树随 SubFlow 节点的 NodeCompleted 到达
        if (event.flowCompleted.parentNodeUuid) return;
        flushBuffer();
        const { report } = event.flowCompleted;
        set((state) => {
          const newHistory = [report, ...state.history].slice(0, MAX_HISTORY_SIZE);
          return {
            isExecuting: false,
            report,
            history: newHistory,
            // 从最终报告同步增量计数器，保证面板计数与报告保持一致
            completedNodeCount: report.node_reports.length,
            totalNodes: report.total_nodes,
          };
        });
      } else if ('flowCancelled' in event) {
        // 嵌套流程取消：顶层流程的取消事件随后到达，不在此提前结束顶层状态
        if (event.flowCancelled.parentNodeUuid) return;
        flushBuffer();
        set({ isExecuting: false });
      } else if ('pluginEvent' in event) {
        const pluginEvt = event.pluginEvent;
        const isStream = pluginEvt.eventType === 'stream';
        queueEventUpdate((state) => {
          let nextNodeStreamMap = state.nodeStreamMap;
          if (isStream) {
            const dataObj =
              typeof pluginEvt.data === 'object' && pluginEvt.data !== null
                ? (pluginEvt.data as Record<string, unknown>)
                : {};
            const deltaContent = typeof dataObj.delta_content === 'string' ? dataObj.delta_content : '';
            const deltaReasoning = typeof dataObj.delta_reasoning === 'string' ? dataObj.delta_reasoning : '';
            const text =
              typeof dataObj.text === 'string'
                ? dataObj.text
                : typeof pluginEvt.data === 'string'
                  ? pluginEvt.data
                  : '';
            const kind =
              typeof dataObj.kind === 'string'
                ? dataObj.kind
                : deltaReasoning
                  ? 'reasoning'
                  : 'content';

            const prev = state.nodeStreamMap[pluginEvt.nodeUuid] || {
              fullContent: '',
              fullReasoning: '',
              lastUpdated: Date.now(),
              chunks: [],
            };

            const newFullContent = prev.fullContent + (deltaContent || (kind === 'content' ? text : ''));
            const newFullReasoning =
              prev.fullReasoning + (deltaReasoning || (kind === 'reasoning' ? text : ''));

            const chunkEntry: NodeStreamChunk = {
              ...dataObj,
              kind,
              delta_content: deltaContent || undefined,
              delta_reasoning: deltaReasoning || undefined,
              text: text || undefined,
              timestamp: pluginEvt.timestamp || new Date().toISOString(),
              raw: pluginEvt.data,
            };
            nextNodeStreamMap = {
              ...state.nodeStreamMap,
              [pluginEvt.nodeUuid]: {
                fullContent: newFullContent,
                fullReasoning: newFullReasoning,
                lastUpdated: Date.now(),
                chunks: [...prev.chunks, chunkEntry].slice(-500),
              },
            };
          }

          return {
            pluginEvents: [...state.pluginEvents, pluginEvt].slice(-MAX_PLUGIN_EVENTS),
            nodeStreamMap: nextNodeStreamMap,
          };
        });
      }
    },

    clearExecutionState: () => {
      eventBuffer = [];
      useWsStreamStore.getState().resetWsStreams();
      set({
        nodeReportsMap: {},
        currentNodeUuid: null,
        completedNodeCount: 0,
        totalNodes: 0,
        report: null,
        error: null,
        pluginEvents: [],
        nodeStreamMap: {},
        subFlowProgressMap: {},
      });
    },

    fetchNodeDetail: async (flowUuid: string, nodeUuid: string) => {
      const current = get().nodeReportsMap[nodeUuid];
      if (!current || !current.is_detail_stripped) return;
      try {
        const { getNodeExecutionDetail } = await import('@/services/tauri');
        const detail = await getNodeExecutionDetail(flowUuid, nodeUuid);
        if (detail) {
          set((state) => ({
            nodeReportsMap: {
              ...state.nodeReportsMap,
              [nodeUuid]: detail,
            },
          }));
        }
      } catch (err) {
        console.error('Failed to fetch node execution detail:', err);
      }
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
      const incReport = nodeReportsMap[nodeUuid];
      if (incReport && !incReport.is_detail_stripped) {
        return incReport;
      }
      if (report) {
        const full = report.node_reports.find((r) => r.node_uuid === nodeUuid);
        if (full) return full;
      }
      return incReport || null;
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
/** 选择插件流事件日志 */
export const selectPluginEvents = (state: ExecutionState) => state.pluginEvents;
/** 选择节点实时流式数据映射表 */
export const selectNodeStreamMap = (state: ExecutionState) => state.nodeStreamMap;
/** 按 UUID 选择特定节点的实时流式数据 */
export const selectNodeStream = (state: ExecutionState, nodeUuid?: string | null) =>
  nodeUuid ? state.nodeStreamMap[nodeUuid] : undefined;
/** 工厂选择器：选择特定节点的实时流式数据 */
export const selectNodeStreamByUuid = (nodeUuid: string) => (state: ExecutionState) =>
  state.nodeStreamMap[nodeUuid];
/** 工厂选择器：选择特定 SubFlow 节点的嵌套执行进度 */
export const selectSubFlowProgress = (nodeUuid: string) => (state: ExecutionState) =>
  state.subFlowProgressMap[nodeUuid];
