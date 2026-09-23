/**
 * 调试状态管理
 *
 * 包含心跳超时检测、命令超时检测和状态回滚机制
 */
import { create } from 'zustand';
import type { DebugStatus, PauseReason, ExecutionStackEntry, DebugEvent, DebugCallFrame, GrpcConnectionInfo, GrpcMethodCallRecord, GrpcRealtimeStreamMessage, GrpcRealtimeStreamError } from '@/types/debug';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types/report';
import { DEBUG_HEALTH_CONFIG } from '@/types/debug';
import { startDebug, sendDebugCommand, endDebug as endDebugService, saveFlowReport } from '@/services/tauri';
import type { Flow } from '@/types/flow';
import { useSettingsStore } from './settings';
import { useExecutionStore } from './execution';
import { useConsoleStore } from './console';
import { useWsStreamStore } from './wsStream';
import { useUIStore } from './ui';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

const ll = getLL();

// ============================================================================
// 类型定义
// ============================================================================

/** 健康检查状态 */
type HealthStatus = 'healthy' | 'degraded' | 'dead';

interface DebugState {
  /** 调试会话 ID */
  sessionId: string | null;
  /** 调试状态 */
  status: DebugStatus;
  /** 当前暂停的节点 UUID */
  currentNodeUuid: string | null;
  /** 当前暂停原因 */
  pauseReason: PauseReason | null;
  /** 当前上下文变量 */
  variables: Record<string, unknown>;
  /** 上一次暂停时的变量快照（用于计算单步 Diff） */
  previousVariables: Record<string, unknown> | null;
  /** 当前暂停相对上一次暂停的变量差异（随变量更新一次性求值，引用稳定） */
  variableDiff: VariableDiff;
  /** 执行堆栈 */
  executionStack: ExecutionStackEntry[];
  /** 当前节点输出 */
  currentNodeOutput: unknown | null;
  /** 临时节点配置（调试期间生效） */
  tempNodeConfigs: Record<string, unknown>;
  /** 错误信息 */
  error: string | null;
  /** 上次收到心跳的时间戳 */
  lastHeartbeatTime: number | null;
  /** 健康检查状态 */
  healthStatus: HealthStatus;
  /** 是否正在等待命令响应 */
  isCommandPending: boolean;
  /** 上一个状态（用于回滚） */
  previousStatus: DebugStatus | null;
  /** 最后一次调试报告（调试结束时生成） */
  lastDebugReport: FlowExecutionReport | null;
  /** 调试数据是否为只读模式（调试结束后保留数据但禁止编辑） */
  isDebugDataReadOnly: boolean;
  /** 多层子流程嵌套调用栈帧 */
  callFrames: DebugCallFrame[];
  /** 当前用户检视的栈帧索引（默认最新栈顶） */
  activeFrameIndex: number;
  // ===== gRPC 调试数据 =====
  /** gRPC 通道连接信息 (connection_id → 连接信息) */
  grpcConnections: Record<string, GrpcConnectionInfo>;
  /** gRPC 方法调用记录 (connection_id → 调用记录列表) */
  grpcMethodCalls: Record<string, GrpcMethodCallRecord[]>;

  // ===== gRPC 实时流式数据 =====
  /** gRPC 实时流式消息 (call_id → 消息列表) */
  grpcStreamMessages: Record<string, GrpcRealtimeStreamMessage[]>;
  /** gRPC 实时流式错误 (call_id → 错误) */
  grpcStreamErrors: Record<string, GrpcRealtimeStreamError>;

  // ===== Actions =====
  /** 启动调试会话 */
  startDebugSession: (flow: Flow, breakpoints: string[]) => Promise<void>;
  /** 继续执行 */
  continueExecution: () => Promise<void>;
  /** 单步执行 */
  stepOver: () => Promise<void>;
  /** 单步步入（遇到 flow:sub 节点进入其内部） */
  stepInto: () => Promise<void>;
  /** 单步步出（执行完当前子流程并在父流程暂停） */
  stepOut: () => Promise<void>;
  /** 原地重试当前暂停节点（使用修改后的配置或变量重新执行） */
  retryCurrentNode: () => Promise<void>;
  /** 从指定历史节点快照恢复并继续暂停/执行 */
  runFromSnapshot: (nodeUuid: string) => Promise<void>;
  /** 切换检视特定栈帧 */
  setActiveFrame: (index: number) => void;
  /** 跳过节点 */
  skipNode: (selectedPort?: string, simulatedOutput?: unknown) => Promise<void>;
  /** 执行到底 */
  runToEnd: () => Promise<void>;
  /** 结束调试 */
  endDebugSession: () => Promise<void>;
  /** 修改节点配置（临时） */
  modifyNodeConfig: (nodeUuid: string, config: unknown) => Promise<void>;
  /** 动态设置断点（调试期间生效） */
  setBreakpointDynamic: (nodeUuid: string, enabled: boolean) => Promise<void>;
  /** 处理调试事件 */
  handleDebugEvent: (event: DebugEvent) => void;
  /** 检查会话健康状态 */
  checkHealth: () => void;
  /** 清除调试数据（退出只读模式，恢复正常操作） */
  clearDebugData: () => void;
  /** 重置状态 */
  reset: () => void;
}

// ============================================================================
// 变量差异计算（纯函数：在变量更新时一次性求值，避免订阅选择器每次返回新对象）
// ============================================================================

/** 变量差异对象 */
export interface VariableDiff {
  added: string[];
  modified: Record<string, { oldVal: unknown; newVal: unknown }>;
  deleted: string[];
}

/** 空差异常量：始终返回同一引用，避免订阅者因新对象反复重渲染 */
export const EMPTY_VARIABLE_DIFF: VariableDiff = Object.freeze({
  added: [],
  modified: {},
  deleted: [],
});

/** 比较两次变量快照，得出新增、修改与删除的变量 */
export function computeVariableDiff(
  prev: Record<string, unknown> | null,
  curr: Record<string, unknown>,
): VariableDiff {
  if (!prev) return EMPTY_VARIABLE_DIFF;

  const added: string[] = [];
  const modified: Record<string, { oldVal: unknown; newVal: unknown }> = {};
  const deleted: string[] = [];

  for (const [k, v] of Object.entries(curr)) {
    if (!(k in prev)) {
      added.push(k);
    } else if (JSON.stringify(prev[k]) !== JSON.stringify(v)) {
      modified[k] = { oldVal: prev[k], newVal: v };
    }
  }

  for (const k of Object.keys(prev)) {
    if (!(k in curr)) {
      deleted.push(k);
    }
  }

  if (added.length === 0 && deleted.length === 0 && Object.keys(modified).length === 0) {
    return EMPTY_VARIABLE_DIFF;
  }
  return { added, modified, deleted };
}

// ============================================================================
// 初始状态
// ============================================================================

const initialState = {
  sessionId: null,
  status: 'idle' as DebugStatus,
  currentNodeUuid: null,
  pauseReason: null,
  variables: {},
  previousVariables: null as Record<string, unknown> | null,
  variableDiff: EMPTY_VARIABLE_DIFF,
  executionStack: [],
  currentNodeOutput: null,
  tempNodeConfigs: {},
  error: null,
  lastHeartbeatTime: null,
  healthStatus: 'healthy' as HealthStatus,
  isCommandPending: false,
  previousStatus: null,
  lastDebugReport: null,
  isDebugDataReadOnly: false,
  grpcConnections: {},
  grpcMethodCalls: {},
  grpcStreamMessages: {},
  grpcStreamErrors: {},
  callFrames: [] as DebugCallFrame[],
  activeFrameIndex: 0,
};

// ============================================================================
// 心跳和健康检查管理
// ============================================================================

/** 心跳检查定时器 ID */
let heartbeatCheckTimer: ReturnType<typeof setInterval> | null = null;

/** 命令超时定时器 ID */
let commandTimeoutTimer: ReturnType<typeof setTimeout> | null = null;

/** 清除所有定时器 */
function clearAllTimers() {
  if (heartbeatCheckTimer) {
    clearInterval(heartbeatCheckTimer);
    heartbeatCheckTimer = null;
  }
  if (commandTimeoutTimer) {
    clearTimeout(commandTimeoutTimer);
    commandTimeoutTimer = null;
  }
}

/** 启动心跳检查定时器 */
function startHeartbeatCheck() {
  // 清除之前的定时器
  if (heartbeatCheckTimer) {
    clearInterval(heartbeatCheckTimer);
  }
  
  heartbeatCheckTimer = setInterval(() => {
    const state = useDebugStore.getState();
    state.checkHealth();
  }, DEBUG_HEALTH_CONFIG.HEALTH_CHECK_INTERVAL_MS);
}

/** 启动命令超时定时器 */
function startCommandTimeout() {
  // 清除之前的定时器
  if (commandTimeoutTimer) {
    clearTimeout(commandTimeoutTimer);
  }
  
  commandTimeoutTimer = setTimeout(() => {
    const state = useDebugStore.getState();
    
    // 如果命令仍在等待响应，执行回滚
    if (state.isCommandPending && state.previousStatus) {
      console.warn('[Debug] Command timeout, rolling back state');
      useDebugStore.setState({
        status: state.previousStatus,
        isCommandPending: false,
        previousStatus: null,
        error: ll.execution.toast.debug_command_timeout(),
      });
    }
  }, DEBUG_HEALTH_CONFIG.COMMAND_TIMEOUT_MS);
}

/** 清除命令超时定时器 */
function clearCommandTimeout() {
  if (commandTimeoutTimer) {
    clearTimeout(commandTimeoutTimer);
    commandTimeoutTimer = null;
  }
}

// ============================================================================
// 调试事件 → 执行状态桥接辅助函数
// ============================================================================

/**
 * 将 ExecutionStackEntry 转换为 NodeExecutionReport，供 ExecutionPanel 使用
 */
function stackEntryToReport(entry: ExecutionStackEntry): NodeExecutionReport {
  const endTime = entry.ended_at ?? entry.started_at;
  const durationMs = entry.ended_at ? entry.ended_at - entry.started_at : 0;

  let status: NodeExecutionReport['status'] = entry.success ? 'success' : 'failed';
  if (entry.skip_reason) {
    status = 'skipped';
  }

  return {
    node_uuid: entry.node_uuid,
    node_type: entry.node_type,
    node_name: entry.node_name,
    start_time: entry.started_at,
    end_time: endTime,
    duration_ms: durationMs,
    status,
    input_data: entry.input,
    output_data: entry.output,
    used_port_id: entry.used_port_id,
    error: entry.error
      ? { error_kind: 'execution', message: entry.error }
      : undefined,
    resolved_config: entry.resolved_config,
    variable_changes: entry.variable_changes,
    skip_reason: entry.skip_reason,
    sub_flow_report: entry.sub_flow_report,
  };
}

/**
 * 从执行堆栈构建 nodeReportsMap 并同步到 executionStore
 */
function syncStackToExecutionStore(
  executionStack: ExecutionStackEntry[],
  currentNodeUuid?: string | null,
) {
  const nodeReportsMap: Record<string, NodeExecutionReport> = {};
  for (const entry of executionStack) {
    if (entry.ended_at !== undefined) {
      nodeReportsMap[entry.node_uuid] = stackEntryToReport(entry);
    }
  }

  useExecutionStore.setState({
    isExecuting: true, // debug 模式下始终标记为执行中
    nodeReportsMap,
    completedNodeCount: Object.keys(nodeReportsMap).length,
    currentNodeUuid: currentNodeUuid ?? null,
  });
}

// ============================================================================
// Store 实现
// ============================================================================

export const useDebugStore = create<DebugState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  startDebugSession: async (flow: Flow, breakpoints: string[]) => {
    try {
      // 清理之前的定时器
      clearAllTimers();
      
      // 重置所有调试数据并清除只读标记
      set({ 
        ...initialState,
        status: 'running',
        error: null,
        lastHeartbeatTime: Date.now(),
        healthStatus: 'healthy',
      });

      // 清空执行状态（含 report、nodeReportsMap 等），确保不显示上次运行结果
      useExecutionStore.getState().clearExecutionState();

      // 打开底部面板并切换到运行标签
      useUIStore.setState({ isBottomPanelExpanded: true, bottomPanelTab: 'running' });

      // 启动心跳检查
      startHeartbeatCheck();

      // 清空控制台日志
      useConsoleStore.getState().clearLogs();

      await startDebug(
        flow, 
        breakpoints, 
        (event) => {
          get().handleDebugEvent(event);
        },
        (logEvent) => {
          // 将日志转发到 console store
          useConsoleStore.getState().addLog(logEvent);
        }
      );
    } catch (error) {
      clearAllTimers();
      set({
        status: 'ended',
        error: (error as Error).message,
        healthStatus: 'dead',
      });
    }
  },

  continueExecution: async () => {
    const currentStatus = get().status;
    try {
      // 保存当前状态用于回滚
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand('Continue');
      set({ status: 'running' });
      
      // 启动命令超时检测
      startCommandTimeout();
    } catch (error) {
      // 命令失败，回滚状态
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: `${ll.execution.toast.debug_continue_failed()}: ${(error as Error).message}`,
      });
    }
  },

  stepOver: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand('StepOver');
      set({ status: 'running' });
      
      startCommandTimeout();
    } catch (error) {
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: `${ll.execution.toast.debug_step_over_failed()}: ${(error as Error).message}`,
      });
    }
  },

  stepInto: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand('StepInto');
      set({ status: 'running' });
      
      startCommandTimeout();
    } catch (error) {
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: (error as Error).message,
      });
    }
  },

  stepOut: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand('StepOut');
      set({ status: 'running' });
      
      startCommandTimeout();
    } catch (error) {
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: (error as Error).message,
      });
    }
  },

  retryCurrentNode: async () => {
    const currentStatus = get().status;
    try {
      set({
        isCommandPending: true,
        previousStatus: currentStatus,
        pauseReason: null,
        error: null,
      });

      await sendDebugCommand('RetryCurrentNode');
      set({ status: 'running' });
      startCommandTimeout();
    } catch (error) {
      set({
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: (error as Error).message,
      });
    }
  },

  runFromSnapshot: async (nodeUuid: string) => {
    const currentStatus = get().status;
    try {
      set({
        isCommandPending: true,
        previousStatus: currentStatus,
        pauseReason: null,
        error: null,
      });

      await sendDebugCommand({ RunFromSnapshot: { node_uuid: nodeUuid } });
      startCommandTimeout();
    } catch (error) {
      set({
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: (error as Error).message,
      });
    }
  },
  setActiveFrame: (index: number) => {
    const { callFrames } = get();
    if (index >= 0 && index < callFrames.length) {
      set({ activeFrameIndex: index });
      const targetFrame = callFrames[index];
      import('./flow').then(({ useFlowStore }) => {
        const flowState = useFlowStore.getState();
        if (targetFrame.frame_id === 0) {
          flowState.clearDebugSubflows?.();
          flowState.centerOnNode?.(targetFrame.node_uuid);
        } else {
          if (flowState.flow?.uuid === targetFrame.flow_uuid) {
            flowState.centerOnNode?.(targetFrame.node_uuid);
          } else {
            import('@/services/storage/flowStorage').then(({ flowStorage }) => {
              flowStorage.loadFlow(targetFrame.flow_uuid).then((loadedFlow) => {
                useFlowStore.getState().pushDebugSubflow?.(loadedFlow, targetFrame.parent_subflow_node_uuid);
                useFlowStore.getState().centerOnNode?.(targetFrame.node_uuid);
              }).catch(() => {
              });
            });
          }
        }
      });
    }
  },

  skipNode: async (selectedPort?: string, simulatedOutput?: unknown) => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand({
        SkipNode: {
          selected_port: selectedPort,
          simulated_output: simulatedOutput,
        },
      });
      set({ status: 'running' });
      
      startCommandTimeout();
    } catch (error) {
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: `${ll.execution.toast.debug_skip_node_failed()}: ${(error as Error).message}`,
      });
    }
  },

  runToEnd: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
        pauseReason: null,
        error: null 
      });
      
      await sendDebugCommand('RunToEnd');
      set({ status: 'running' });
      
      startCommandTimeout();
    } catch (error) {
      set({ 
        status: currentStatus,
        isCommandPending: false,
        previousStatus: null,
        error: `${ll.execution.toast.debug_run_to_end_failed()}: ${(error as Error).message}`,
      });
    }
  },

  endDebugSession: async () => {
    try {
      clearAllTimers();
      await endDebugService();
      // 调试结束，保留执行数据用于查看（与普通执行模式一致）
      set({ 
        status: 'ended',
        isCommandPending: false,
        previousStatus: null,
      });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  modifyNodeConfig: async (nodeUuid: string, config: unknown) => {
    try {
      await sendDebugCommand({
        ModifyNodeConfig: { node_uuid: nodeUuid, config },
      });
      // 更新本地状态
      set((state) => ({
        tempNodeConfigs: {
          ...state.tempNodeConfigs,
          [nodeUuid]: config,
        },
      }));
    } catch (error) {
      set({ error: `${ll.execution.toast.debug_modify_config_failed()}: ${(error as Error).message}` });
    }
  },

  setBreakpointDynamic: async (nodeUuid: string, enabled: boolean) => {
    try {
      await sendDebugCommand({
        SetBreakpoint: { node_uuid: nodeUuid, enabled },
      });
      // 注意：本地断点状态由 flow store 管理，这里只发送命令到后端
      // 后端会返回 BreakpointModified 事件，但前端断点显示由 flow store 控制
    } catch (error) {
      set({ error: `${ll.execution.toast.debug_set_breakpoint_failed()}: ${(error as Error).message}` });
    }
  },

  handleDebugEvent: (event: DebugEvent) => {
    // 收到任何事件都更新心跳时间
    set({ lastHeartbeatTime: Date.now() });

    if ('SessionStarted' in event) {
      set({
        sessionId: event.SessionStarted.flow_uuid,
        status: 'running',
        healthStatus: 'healthy',
      });

      // 桥接：同步初始化 executionStore 执行状态
      // isExecuting 必须同步设置，否则 PausedAtNode 到达时面板仍认为未在执行
      useExecutionStore.setState({
        isExecuting: true,
        nodeReportsMap: {},
        completedNodeCount: 0,
        currentNodeUuid: null,
        report: null,
        error: null,
      });

      // 异步获取 totalNodes（使用动态导入避免循环依赖）
      import('./flow').then(({ useFlowStore, getStageGraph }) => {
        const flowStore = useFlowStore.getState();
        const flow = flowStore.flow;
        // 调试覆盖三个生命周期阶段：总节点数与后端报告口径一致（三阶段节点数之和）
        const totalNodes = flow
          ? getStageGraph(flow, 'setup').nodes.length +
            flow.nodes.length +
            getStageGraph(flow, 'teardown').nodes.length
          : 0;
        useExecutionStore.setState({ totalNodes });
      });
    } else if ('PausedAtNode' in event) {
      // 收到暂停事件，清除命令超时
      clearCommandTimeout();
      
      const { node_uuid, reason, variables, execution_stack, call_frames } = event.PausedAtNode;
      const frames = call_frames ?? [];
      const topIndex = frames.length > 0 ? frames.length - 1 : 0;
      const currentVars = get().variables;
      const previousVariables = Object.keys(currentVars).length > 0 ? currentVars : null;
      set({
        status: 'paused',
        previousVariables,
        variableDiff: computeVariableDiff(previousVariables, variables),
        currentNodeUuid: node_uuid,
        pauseReason: reason,
        variables,
        executionStack: execution_stack,
        callFrames: frames,
        activeFrameIndex: topIndex,
        // 保留上一个节点的输出，不清空，方便用户查看最近执行的节点结果
        isCommandPending: false,
        previousStatus: null,
        healthStatus: 'healthy',
      });

      // 驱动当前活跃流程的切换与画布展示
      import('./flow').then(({ useFlowStore, getNodeStage }) => {
        const flowStore = useFlowStore.getState();
        // 暂停节点可能位于 Setup/Teardown 阶段：自动联动切换画布阶段
        // （仅顶层流程；子流程帧由下方 debugFlowStack 逻辑处理）
        if (frames.length <= 1 && flowStore.flow) {
          const stage = getNodeStage(flowStore.flow, node_uuid);
          if (stage && stage !== flowStore.activeStage) {
            flowStore.setActiveStage(stage);
          }
        }
        if (frames.length > 1) {
          const topFrame = frames[topIndex];
          if (flowStore.flow?.uuid !== topFrame.flow_uuid) {
            import('@/services/storage/flowStorage').then(({ flowStorage }) => {
              flowStorage.loadFlow(topFrame.flow_uuid).then((subflow) => {
                useFlowStore.getState().pushDebugSubflow?.(subflow, topFrame.parent_subflow_node_uuid);
                useFlowStore.getState().centerOnNode?.(node_uuid);
              }).catch((err) => {
                console.warn('[Debug] Could not load subflow to display on canvas:', err);
              });
            });
          } else {
            flowStore.centerOnNode?.(node_uuid);
          }
        } else if (frames.length <= 1 && (flowStore.debugFlowStack?.length ?? 0) > 0) {
          flowStore.clearDebugSubflows?.();
          flowStore.centerOnNode?.(node_uuid);
        }
      });

      // 桥接：将当前暂停节点标记为 executionStore 的 currentNodeUuid
      // 无论是 breakpoint 还是 step_complete，暂停的节点都应该显示为 running
      syncStackToExecutionStore(execution_stack, node_uuid);
    } else if ('NodeExecuted' in event) {
      const { entry } = event.NodeExecuted;
      set({ currentNodeOutput: entry.output });

      // 将刚完成的节点追加到 executionStack（去重），使连续执行时边能实时更新
      set((state) => {
        const alreadyInStack = state.executionStack.some(
          (e) => e.node_uuid === entry.node_uuid,
        );
        if (alreadyInStack) return {};
        return { executionStack: [...state.executionStack, entry] };
      });

      // 桥接：增量同步已完成节点到 executionStore
      // 使执行面板实时显示每个节点完成状态（而非等待 SessionEnded 批量更新）
      const nodeReport = stackEntryToReport(entry);
      const currentMap = useExecutionStore.getState().nodeReportsMap;
      const newMap = { ...currentMap, [entry.node_uuid]: nodeReport };

      // RunToEnd 模式：不清空 currentNodeUuid，由 Heartbeat handler 管理，避免闪烁
      // Step/暂停模式：清空，由下一个 PausedAtNode 设置
      const { status: debugStatus, pauseReason } = get();
      const isRunToEnd = debugStatus === 'running' && pauseReason === null;

      // 在 debug 模式下，如果已经暂停且 executionStore 有 currentNodeUuid（断点节点），
      // 不覆盖 currentNodeUuid，保留断点节点的 "running" 状态
      // 解决事件顺序问题：PausedAtNode 先到达设置 currentNodeUuid，NodeExecuted 后到达不应覆盖
      const currentExecutionUuid = useExecutionStore.getState().currentNodeUuid;
      const shouldPreserveCurrentNode = debugStatus === 'paused' && currentExecutionUuid !== null;

      useExecutionStore.setState({
        nodeReportsMap: newMap,
        completedNodeCount: Object.keys(newMap).length,
        ...(isRunToEnd || shouldPreserveCurrentNode ? {} : { currentNodeUuid: null }),
      });
    } else if ('SessionEnded' in event) {
      // 会话结束，清理定时器
      clearAllTimers();

      const { report, execution_stack } = event.SessionEnded;

      set({
        status: 'ended',
        // 保留调试数据用于只读查看，不清空 currentNodeUuid 和 pauseReason
        previousStatus: null,
        healthStatus: 'healthy',
        callFrames: [],
        activeFrameIndex: 0,
        // 存储调试报告（如果有）
        lastDebugReport: report || null,
        // 更新执行堆栈（包含所有节点，包括最后一个）
        executionStack: execution_stack,
        // 同步最终变量，供调试结束后的变量面板与节点查看
        ...(report?.final_variables && typeof report.final_variables === 'object'
          ? { variables: report.final_variables as Record<string, unknown> }
          : {}),
      });

      // 桥接：从最终执行堆栈同步所有节点报告到 executionStore
      syncStackToExecutionStore(execution_stack, null);

      // 如果有报告，保存并呈现
      if (report) {
        // 1. 保存报告到后端
        const reportDir = useSettingsStore.getState().reportDir;
        saveFlowReport(report, reportDir).catch((err) => {
          console.error('[Debug] Failed to save report:', err);
        });

        // 2. 设置到 executionStore 触发报告弹窗
        useExecutionStore.getState().setReport(report);

        // 3. 显示 toast 通知
        toast.success(ll.execution.toast.debug_completed(), {
          description: `${ll.execution.toast.duration_label()} ${report.duration_ms}ms · ${report.statistics.success_count} ${ll.execution.status.success()} · ${report.statistics.failed_count} ${ll.execution.status.failed()}`,
          action: { label: ll.execution.toast.view_report(), onClick: () => useExecutionStore.getState().openReportDialog() },
          duration: 10000,
        });
      } else {
        // 没有最终报告时也要标记执行结束
        useExecutionStore.setState({ isExecuting: false });
      }
    } else if ('SnapshotRestored' in event) {
      clearCommandTimeout();
      const { node_uuid, restored_variables, stack_len } = event.SnapshotRestored;
      set((state) => ({
        currentNodeUuid: node_uuid,
        variables: restored_variables,
        executionStack: state.executionStack.slice(0, stack_len),
        isCommandPending: false,
      }));
    } else if ('VariableModified' in event) {
      const { name, value } = event.VariableModified;
      set((state) => ({
        variables: {
          ...state.variables,
          [name]: value,
        },
      }));
    } else if ('NodeConfigModified' in event) {
      const { node_uuid, config } = event.NodeConfigModified;
      set((state) => ({
        tempNodeConfigs: {
          ...state.tempNodeConfigs,
          [node_uuid]: config,
        },
      }));
    } else if ('BreakpointModified' in event) {
      // 断点修改事件 - 同步到 flow store
      const { node_uuid, enabled } = event.BreakpointModified;
      // 使用动态导入避免循环依赖
      import('./flow').then(({ useFlowStore }) => {
        useFlowStore.getState().setBreakpoint(node_uuid, enabled);
      });
    } else if ('Error' in event) {
      // 收到错误事件，清除命令超时
      clearCommandTimeout();
      
      set({
        status: 'ended',
        error: event.Error.message,
        isCommandPending: false,
        previousStatus: null,
        healthStatus: 'dead',
      });

      // 桥接：标记执行结束
      useExecutionStore.setState({ isExecuting: false });
    } else if ('Heartbeat' in event) {
      // 心跳事件 - 更新心跳时间（已在顶部更新）
      // 可以在这里验证后端状态是否与前端一致
      const { status: backendStatus, current_node_uuid } = event.Heartbeat;
      const currentStatus = get().status;
      
      // 如果前端状态是 running 但后端状态是 paused，可能是 PausedAtNode 事件丢失
      // 此时需要同步前端状态，让工具栏变为可用
      if (currentStatus === 'running' && backendStatus === 'paused') {
        console.warn('[Debug] State mismatch detected, backend is paused, syncing frontend state');
        set({
          status: 'paused',
          currentNodeUuid: current_node_uuid || null,
          pauseReason: { type: 'breakpoint', node_uuid: current_node_uuid || '' },
          isCommandPending: false,
          previousStatus: null,
        });

        // 桥接：心跳修复时也同步当前节点与阶段
        useExecutionStore.setState({ currentNodeUuid: current_node_uuid || null });
        if (current_node_uuid) {
          import('./flow').then(({ useFlowStore, getNodeStage }) => {
            const flowStore = useFlowStore.getState();
            if (flowStore.flow) {
              const stage = getNodeStage(flowStore.flow, current_node_uuid);
              if (stage && stage !== flowStore.activeStage) {
                flowStore.setActiveStage(stage);
              }
            }
          });
        }
      }
      
      // 更新当前节点 UUID（如果后端有值且状态一致）
      if (current_node_uuid !== undefined && get().status === backendStatus) {
        set({ currentNodeUuid: current_node_uuid });

        // 后端为 running（RunToEnd 或 Step 间过渡）时，同步到 executionStore
        // 确保底部面板和画布高亮能实时反映当前执行节点
        if (backendStatus === 'running' && current_node_uuid) {
          useExecutionStore.setState({ currentNodeUuid: current_node_uuid });
          import('./flow').then(({ useFlowStore, getNodeStage }) => {
            const flowStore = useFlowStore.getState();
            if (flowStore.flow) {
              const stage = getNodeStage(flowStore.flow, current_node_uuid);
              if (stage && stage !== flowStore.activeStage) {
                flowStore.setActiveStage(stage);
              }
            }
          });
        }
      }
    } else if ('WsStreamCreated' in event || 'WsMessageReceived' in event ||
                'WsMessageSent' in event || 'WsSegmentStarted' in event ||
                'WsSegmentEnded' in event || 'WsStreamOwnerChanged' in event ||
                'WsStreamClosed' in event) {
      // 所有 WS 流事件委托到 wsStreamStore 处理
      useWsStreamStore.getState().handleWsEvent(event);
    } else if ('GrpcChannelCreated' in event) {
      const { connection_id, url, service_count } = event.GrpcChannelCreated;
      set((state) => ({
        grpcConnections: {
          ...state.grpcConnections,
          [connection_id]: { url, service_count },
        },
      }));
    } else if ('GrpcMethodCalled' in event) {
      const { connection_id, node_uuid, service, method, pattern, duration_ms } = event.GrpcMethodCalled;
      set((state) => ({
        grpcMethodCalls: {
          ...state.grpcMethodCalls,
          [connection_id]: [
            ...(state.grpcMethodCalls[connection_id] || []),
            { node_uuid, service, method, pattern, duration_ms },
          ],
        },
      }));
    } else if ('GrpcChannelClosed' in event) {
      // gRPC 通道关闭 — 保留连接信息用于查看，不删除
    } else if ('GrpcStreamMessage' in event) {
      // gRPC 实时流式消息
      const msg = event.GrpcStreamMessage;
      set((state) => ({
        grpcStreamMessages: {
          ...state.grpcStreamMessages,
          [msg.call_id]: [
            ...(state.grpcStreamMessages[msg.call_id] || []),
            msg,
          ],
        },
      }));
    } else if ('GrpcStreamError' in event) {
      // gRPC 实时流式错误
      const err = event.GrpcStreamError;
      set((state) => ({
        grpcStreamErrors: {
          ...state.grpcStreamErrors,
          [err.call_id]: err,
        },
      }));
    }
  },

  checkHealth: () => {
    const state = get();
    
    // 如果不在调试中，跳过检查
    if (state.status === 'idle' || state.status === 'ended') {
      return;
    }
    
    const now = Date.now();
    const lastHeartbeat = state.lastHeartbeatTime;
    
    if (!lastHeartbeat) {
      // 没有心跳记录，可能刚启动
      return;
    }
    
    const timeSinceLastHeartbeat = now - lastHeartbeat;
    
    if (timeSinceLastHeartbeat > DEBUG_HEALTH_CONFIG.HEARTBEAT_TIMEOUT_MS) {
      // 心跳超时，会话可能已死
      console.error('[Debug] Heartbeat timeout, session may be dead');
      
      // 清理定时器
      clearAllTimers();
      
      set({
        status: 'ended',
        healthStatus: 'dead',
        error: ll.execution.toast.debug_heartbeat_timeout(),
        isCommandPending: false,
        previousStatus: null,
      });
    } else if (timeSinceLastHeartbeat > DEBUG_HEALTH_CONFIG.HEARTBEAT_TIMEOUT_MS / 2) {
      // 心跳延迟，标记为降级状态
      if (state.healthStatus !== 'degraded') {
        set({ healthStatus: 'degraded' });
        console.warn('[Debug] Heartbeat delayed, health degraded');
      }
    } else {
      // 心跳正常
      if (state.healthStatus !== 'healthy') {
        set({ healthStatus: 'healthy' });
      }
    }
  },

  clearDebugData: () => {
    // 清除调试数据，恢复正常操作模式
    // 不清理 executionStore，保留执行结果供用户查看（与普通执行模式一致）
    clearAllTimers();
    set(initialState);
  },

  reset: () => {
    clearAllTimers();
    set(initialState);
  },
}));

// ===== Selectors =====
/** 选择调试状态 */
export const selectDebugStatus = (state: DebugState) => state.status;
/** 选择当前节点 UUID */
export const selectCurrentNodeUuid = (state: DebugState) => state.currentNodeUuid;
/** 选择调试当前节点 UUID（命名别名，用于与节点 ID 做相等性比较） */
export const selectDebugCurrentNodeUuid = (state: DebugState) => state.currentNodeUuid;
/** 选择是否正在调试 */
export const selectIsDebugging = (state: DebugState) => state.status !== 'idle' && state.status !== 'ended';
/** 选择是否已暂停 */
export const selectIsPaused = (state: DebugState) => state.status === 'paused';
/** 选择指定节点是否处于调试暂停状态（isPaused && currentNodeUuid === nodeId） */
export const selectIsNodeDebugPaused = (nodeId: string) => (state: DebugState) =>
  state.status === 'paused' && state.currentNodeUuid === nodeId;
/** 选择健康状态 */
export const selectHealthStatus = (state: DebugState) => state.healthStatus;
/** 选择是否正在等待命令响应 */
export const selectIsCommandPending = (state: DebugState) => state.isCommandPending;
/** 选择调试数据是否为只读模式 */
export const selectIsDebugDataReadOnly = (state: DebugState) => state.isDebugDataReadOnly;

/** 选择调用栈帧列表 */
export const selectCallFrames = (state: DebugState) => state.callFrames;
/** 选择当前检视栈帧索引 */
export const selectActiveFrameIndex = (state: DebugState) => state.activeFrameIndex;
/** 选择当前检视的栈帧 */
export const selectActiveCallFrame = (state: DebugState) =>
  state.callFrames[state.activeFrameIndex] ?? null;

/** 选择当前单步变量与上一暂停状态的 Diff（直接取 store 中已求值的稳定字段） */
export const selectVariableDiff = (state: DebugState): VariableDiff => state.variableDiff;
