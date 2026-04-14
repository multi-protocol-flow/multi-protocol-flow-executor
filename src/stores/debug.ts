/**
 * 调试状态管理
 *
 * 包含心跳超时检测、命令超时检测和状态回滚机制
 */
import { create } from 'zustand';
import type { DebugStatus, PauseReason, ExecutionStackEntry, DebugEvent } from '@/types/debug';
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

  // ===== Actions =====
  /** 启动调试会话 */
  startDebugSession: (flow: Flow, breakpoints: string[]) => Promise<void>;
  /** 继续执行 */
  continueExecution: () => Promise<void>;
  /** 单步执行 */
  stepOver: () => Promise<void>;
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
// 初始状态
// ============================================================================

const initialState = {
  sessionId: null,
  status: 'idle' as DebugStatus,
  currentNodeUuid: null,
  pauseReason: null,
  variables: {},
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
      console.warn('[调试] 命令超时，回滚状态');
      useDebugStore.setState({
        status: state.previousStatus,
        isCommandPending: false,
        previousStatus: null,
        error: '命令响应超时，已恢复到之前状态',
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
    error: entry.error
      ? { error_kind: 'execution', message: entry.error }
      : undefined,
    resolved_config: entry.resolved_config,
    variable_changes: entry.variable_changes,
    skip_reason: entry.skip_reason,
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
      useUIStore.setState({ isHistoryPanelOpen: true, bottomPanelTab: 'running' });

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
        error: `继续执行失败: ${(error as Error).message}`,
      });
    }
  },

  stepOver: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
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
        error: `单步执行失败: ${(error as Error).message}`,
      });
    }
  },

  skipNode: async (selectedPort?: string, simulatedOutput?: unknown) => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
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
        error: `跳过节点失败: ${(error as Error).message}`,
      });
    }
  },

  runToEnd: async () => {
    const currentStatus = get().status;
    try {
      set({ 
        isCommandPending: true, 
        previousStatus: currentStatus,
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
        error: `执行到底失败: ${(error as Error).message}`,
      });
    }
  },

  endDebugSession: async () => {
    try {
      clearAllTimers();
      await endDebugService();
      // 设置只读标记而不是完全清空，保留调试数据用于查看
      set({ 
        status: 'ended',
        isCommandPending: false,
        previousStatus: null,
        isDebugDataReadOnly: true,
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
      set({ error: `修改配置失败: ${(error as Error).message}` });
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
      set({ error: `设置断点失败: ${(error as Error).message}` });
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

      // 桥接：初始化 executionStore 执行状态
      // 使用动态导入避免循环依赖
      import('./flow').then(({ useFlowStore }) => {
        const totalNodes = useFlowStore.getState().flow?.nodes.length ?? 0;
        useExecutionStore.setState({
          isExecuting: true,
          totalNodes,
          nodeReportsMap: {},
          completedNodeCount: 0,
          currentNodeUuid: null,
          report: null,
          error: null,
        });
      });
    } else if ('PausedAtNode' in event) {
      // 收到暂停事件，清除命令超时
      clearCommandTimeout();
      
      const { node_uuid, reason, variables, execution_stack } = event.PausedAtNode;
      set({
        status: 'paused',
        currentNodeUuid: node_uuid,
        pauseReason: reason,
        variables,
        executionStack: execution_stack,
        // 保留上一个节点的输出，不清空，方便用户查看最近执行的节点结果
        isCommandPending: false,
        previousStatus: null,
        healthStatus: 'healthy',
      });

      // 桥接：根据暂停原因决定 currentNodeUuid
      // - breakpoint: 节点尚未执行，标记为 running（当前正在等待的节点）
      // - step_complete: 节点刚执行完，已通过 execution_stack 同步到 nodeReportsMap
      const isBreakpoint = reason.type === 'breakpoint';
      syncStackToExecutionStore(
        execution_stack,
        isBreakpoint ? node_uuid : null,
      );
    } else if ('NodeExecuted' in event) {
      const { output } = event.NodeExecuted;
      set({ currentNodeOutput: output });

      // 桥接：NodeExecuted 携带的数据有限，仅标记当前正在执行的节点
      // 完整的状态同步会在下一个 PausedAtNode 中完成
      useExecutionStore.setState({
        currentNodeUuid: event.NodeExecuted.node_uuid,
      });
    } else if ('SessionEnded' in event) {
      // 会话结束，清理定时器
      clearAllTimers();

      const { report, execution_stack } = event.SessionEnded;

      set({
        status: 'ended',
        // 保留调试数据用于只读查看，不清空 currentNodeUuid 和 pauseReason
        isCommandPending: false,
        previousStatus: null,
        healthStatus: 'healthy',
        // 设置只读标记
        isDebugDataReadOnly: true,
        // 存储调试报告（如果有）
        lastDebugReport: report || null,
        // 更新执行堆栈（包含所有节点，包括最后一个）
        executionStack: execution_stack,
      });

      // 桥接：从最终执行堆栈同步所有节点报告到 executionStore
      syncStackToExecutionStore(execution_stack, null);

      // 如果有报告，保存并呈现
      if (report) {
        // 1. 保存报告到后端
        const reportDir = useSettingsStore.getState().reportDir;
        saveFlowReport(report, reportDir).catch((err) => {
          console.error('[调试] 保存报告失败:', err);
        });

        // 2. 设置到 executionStore 触发报告弹窗
        useExecutionStore.getState().setReport(report);

        // 3. 显示 toast 通知
        toast.success('调试完成', {
          description: `耗时 ${report.duration_ms}ms · ${report.statistics.success_count} 成功 · ${report.statistics.failed_count} 失败`,
          action: { label: '查看报告', onClick: () => useExecutionStore.getState().openReportDialog() },
          duration: 10000,
        });
      } else {
        // 没有最终报告时也要标记执行结束
        useExecutionStore.setState({ isExecuting: false });
      }
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
        // 设置只读标记，保留调试数据
        isDebugDataReadOnly: true,
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
        console.warn('[调试] 检测到状态不一致，后端为 paused，同步前端状态');
        set({
          status: 'paused',
          currentNodeUuid: current_node_uuid || null,
          pauseReason: { type: 'breakpoint', node_uuid: current_node_uuid || '' },
          isCommandPending: false,
          previousStatus: null,
        });

        // 桥接：心跳修复时也同步当前节点
        useExecutionStore.setState({ currentNodeUuid: current_node_uuid || null });
      }
      
      // 更新当前节点 UUID（如果后端有值且状态一致）
      if (current_node_uuid !== undefined && get().status === backendStatus) {
        set({ currentNodeUuid: current_node_uuid });
      }
    } else if ('WsStreamCreated' in event || 'WsMessageReceived' in event ||
               'WsMessageSent' in event || 'WsSegmentStarted' in event ||
               'WsSegmentEnded' in event || 'WsStreamOwnerChanged' in event ||
               'WsStreamClosed' in event) {
      // 所有 WS 流事件委托到 wsStreamStore 处理
      useWsStreamStore.getState().handleWsEvent(event);
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
      console.error('[调试] 心跳超时，会话可能已死');
      
      // 清理定时器
      clearAllTimers();
      
      set({
        status: 'ended',
        healthStatus: 'dead',
        error: '调试会话失去响应（心跳超时）',
        isCommandPending: false,
        previousStatus: null,
        // 设置只读标记，保留调试数据
        isDebugDataReadOnly: true,
      });
    } else if (timeSinceLastHeartbeat > DEBUG_HEALTH_CONFIG.HEARTBEAT_TIMEOUT_MS / 2) {
      // 心跳延迟，标记为降级状态
      if (state.healthStatus !== 'degraded') {
        set({ healthStatus: 'degraded' });
        console.warn('[调试] 心跳延迟，健康状态降级');
      }
    } else {
      // 心跳正常
      if (state.healthStatus !== 'healthy') {
        set({ healthStatus: 'healthy' });
      }
    }
  },

  clearDebugData: () => {
    // 清除所有调试数据，恢复正常操作模式
    clearAllTimers();
    set(initialState);
    // 同时清理执行状态，确保运行标签完全重置
    useExecutionStore.getState().clearExecutionState();
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
/** 选择是否正在调试 */
export const selectIsDebugging = (state: DebugState) => state.status !== 'idle' && state.status !== 'ended';
/** 选择是否已暂停 */
export const selectIsPaused = (state: DebugState) => state.status === 'paused';
/** 选择健康状态 */
export const selectHealthStatus = (state: DebugState) => state.healthStatus;
/** 选择是否正在等待命令响应 */
export const selectIsCommandPending = (state: DebugState) => state.isCommandPending;
/** 选择调试数据是否为只读模式 */
export const selectIsDebugDataReadOnly = (state: DebugState) => state.isDebugDataReadOnly;
