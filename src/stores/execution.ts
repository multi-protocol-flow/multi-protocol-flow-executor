/**
 * 执行状态和报告管理
 */
import { create } from 'zustand';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types';
import type { ExecutionEvent } from '@/types/execution_event';
import type { DebugEvent } from '@/types/debug';
import type { WsMessage, WsConnectionStream, WsStreamSegment } from '@/types/ws_stream';

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
  /** 处理执行事件（增量更新状态） */
  handleExecutionEvent: (event: ExecutionEvent) => void;
  /** 清空增量执行状态 */
  clearExecutionState: () => void;
  /** 打开报告对话框 */
  openReportDialog: (report?: FlowExecutionReport) => void;
  /** 关闭报告对话框 */
  closeReportDialog: () => void;

  // ===== WebSocket 流状态 =====
  /** WebSocket 连接流（connection_id → WsConnectionStream） */
  wsStreams: Map<string, WsConnectionStream>;
  /** 当前活动 WS 连接 ID */
  activeWsConnectionId: string | null;

  // ===== WS 事件处理 =====
  /** 处理 WS 流事件（DebugEvent 中的 WS 变体） */
  handleWsEvent: (event: DebugEvent) => void;

  // ===== Computed (Selectors) =====
  /** 获取特定节点的执行报告 */
  getNodeReport: (nodeUuid: string) => NodeExecutionReport | null;
  /** 检查是否有执行结果 */
  hasResult: () => boolean;
  /** 获取节点执行状态 */
  getNodeExecutionStatus: (nodeUuid: string) => 'running' | 'success' | 'failed' | 'pending';
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
  wsStreams: new Map<string, WsConnectionStream>(),
  activeWsConnectionId: null as string | null,
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
    }
  },

  clearExecutionState: () => set({
    nodeReportsMap: {},
    currentNodeUuid: null,
    completedNodeCount: 0,
    totalNodes: 0,
    wsStreams: new Map(),
    activeWsConnectionId: null,
  }),

  handleWsEvent: (event: DebugEvent) => {
    if ('WsStreamCreated' in event) {
      // WebSocket 连接创建事件
      const { connection_id, url, owner_node_uuid, owner_node_name } = event.WsStreamCreated;
      const stream: WsConnectionStream = {
        connection_id,
        url,
        current_owner_node_uuid: owner_node_uuid,
        segments: [{
          node_uuid: owner_node_uuid,
          node_name: owner_node_name,
          node_type: 'ws_connect',
          started_at: Date.now(),
          ended_at: null,
          messages: [],
          max_messages: 1000,
          dropped_message_count: 0,
        }],
        max_messages_per_segment: 1000,
        max_total_messages: 10000,
        is_closed: false,
      };
      set((state) => {
        const newStreams = new Map(state.wsStreams);
        newStreams.set(stream.connection_id, stream);
        return {
          wsStreams: newStreams,
          activeWsConnectionId: state.activeWsConnectionId || stream.connection_id,
        };
      });
    } else if ('WsMessageReceived' in event) {
      // WebSocket 消息接收事件（后端 #[serde(flatten)] 展平 message 字段）
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageReceived;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      void _owner;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = [...stream.segments];
        const lastSegment = segments[segments.length - 1];
        if (lastSegment && lastSegment.ended_at === null) {
          lastSegment.messages.push(message);
          if (lastSegment.messages.length > lastSegment.max_messages) {
            lastSegment.dropped_message_count += lastSegment.messages.length - lastSegment.max_messages;
            lastSegment.messages = lastSegment.messages.slice(-lastSegment.max_messages);
          }
        }

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsMessageSent' in event) {
      // WebSocket 消息发送事件（后端 #[serde(flatten)] 展平 message 字段）
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageSent;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      void _owner;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = [...stream.segments];
        const lastSegment = segments[segments.length - 1];
        if (lastSegment && lastSegment.ended_at === null) {
          lastSegment.messages.push(message);
          if (lastSegment.messages.length > lastSegment.max_messages) {
            lastSegment.dropped_message_count += lastSegment.messages.length - lastSegment.max_messages;
            lastSegment.messages = lastSegment.messages.slice(-lastSegment.max_messages);
          }
        }

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsSegmentStarted' in event) {
      // WebSocket 流段开始事件
      const { connection_id, segment } = event.WsSegmentStarted;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = [...stream.segments, segment];
        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments,
          current_owner_node_uuid: segment.node_uuid,
        });
        return { wsStreams: newStreams };
      });
    } else if ('WsSegmentEnded' in event) {
      // WebSocket 流段结束事件
      const { connection_id, segment } = event.WsSegmentEnded;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const segments = stream.segments.map(s =>
          s.node_uuid === segment.node_uuid && s.started_at === segment.started_at
            ? segment
            : s
        );
        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, { ...stream, segments });
        return { wsStreams: newStreams };
      });
    } else if ('WsStreamOwnerChanged' in event) {
      // WebSocket 流所有权变更事件
      const { connection_id, to_node_uuid } = event.WsStreamOwnerChanged;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        const now = Date.now();

        // 结束上一个 segment
        const updatedSegments = stream.segments.map((s, idx) => {
          if (idx === stream.segments.length - 1 && s.ended_at === null) {
            return { ...s, ended_at: now };
          }
          return s;
        });

        // 添加新 segment
        const newSegment: WsStreamSegment = {
          node_uuid: to_node_uuid,
          node_name: '',
          node_type: 'ws_send_collect',
          started_at: now,
          ended_at: null,
          messages: [],
          max_messages: 1000,
          dropped_message_count: 0,
        };

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments: [...updatedSegments, newSegment],
          current_owner_node_uuid: to_node_uuid,
        });
        return { wsStreams: newStreams };
      });
    } else if ('WsStreamClosed' in event) {
      // WebSocket 流关闭事件
      const { connection_id } = event.WsStreamClosed;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) return {};

        // 结束当前活动的 segment
        const segments = [...stream.segments];
        const lastSegment = segments[segments.length - 1];
        if (lastSegment && lastSegment.ended_at === null) {
          lastSegment.ended_at = Date.now();
        }

        const newStreams = new Map(state.wsStreams);
        newStreams.set(connection_id, {
          ...stream,
          segments,
          is_closed: true,
          current_owner_node_uuid: null,
        });

        const newActiveWsConnectionId = state.activeWsConnectionId === connection_id
          ? null
          : state.activeWsConnectionId;

        return {
          wsStreams: newStreams,
          activeWsConnectionId: newActiveWsConnectionId,
        };
      });
    }
    // 其他 DebugEvent 变体（PausedAtNode、Heartbeat 等）忽略
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
