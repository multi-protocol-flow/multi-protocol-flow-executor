/**
 * 调试状态管理
 *
 * 包含心跳超时检测、命令超时检测和状态回滚机制
 */
import { create } from 'zustand';
import type { DebugStatus, PauseReason, ExecutionStackEntry, DebugEvent } from '@/types/debug';
import type { FlowExecutionReport } from '@/types/report';
import type { WsConnectionStream, WsMessage, WsStreamSegment } from '@/types/ws_stream';
import { DEBUG_HEALTH_CONFIG } from '@/types/debug';
import { startDebug, sendDebugCommand, endDebug as endDebugService, saveFlowReport } from '@/services/tauri';
import type { Flow } from '@/types/flow';
import { useSettingsStore } from './settings';
import { useExecutionStore } from './execution';
import { useConsoleStore } from './console';

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
  /** WebSocket 连接流映射 (connection_id -> WsConnectionStream) */
  wsStreams: Map<string, WsConnectionStream>;
  /** 当前活动的 WebSocket 连接 ID */
  activeConnectionId: string | null;

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
  // ===== WebSocket 事件处理 =====
  /** 处理 WebSocket 连接创建事件 */
  handleWsStreamCreated: (stream: WsConnectionStream) => void;
  /** 处理 WebSocket 消息接收事件 */
  handleWsMessageReceived: (connectionId: string, message: WsMessage) => void;
  /** 处理 WebSocket 消息发送事件 */
  handleWsMessageSent: (connectionId: string, message: WsMessage) => void;
  /** 处理 WebSocket 流所有者变更事件 */
  handleWsStreamOwnerChanged: (connectionId: string, ownerNodeUuid: string | null) => void;
  /** 处理 WebSocket 流关闭事件 */
  handleWsStreamClosed: (connectionId: string) => void;
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
  wsStreams: new Map<string, WsConnectionStream>(),
  activeConnectionId: null,
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
    } else if ('NodeExecuted' in event) {
      const { output } = event.NodeExecuted;
      set({ currentNodeOutput: output });
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

      // 如果有报告，保存并呈现
      if (report) {
        // 1. 保存报告到后端
        const reportDir = useSettingsStore.getState().reportDir;
        saveFlowReport(report, reportDir).catch((err) => {
          console.error('[调试] 保存报告失败:', err);
        });

        // 2. 设置到 executionStore 触发报告弹窗
        useExecutionStore.getState().setReport(report);
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
      }
      
      // 更新当前节点 UUID（如果后端有值且状态一致）
      if (current_node_uuid !== undefined && get().status === backendStatus) {
        set({ currentNodeUuid: current_node_uuid });
      }
    } else if ('WsStreamCreated' in event) {
      // WebSocket 连接创建事件（后端发送，需转换为前端格式）
      // 后端发送: { connection_id, url, owner_node_uuid, owner_node_name }
      // 前端期望: WsConnectionStream
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
      get().handleWsStreamCreated(stream);
    } else if ('WsMessageReceived' in event) {
      // WebSocket 消息接收事件
      // 后端使用 #[serde(flatten)] 展平了 message 字段，需要重新构建
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageReceived;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      get().handleWsMessageReceived(connection_id, message);
    } else if ('WsMessageSent' in event) {
      // WebSocket 消息发送事件
      // 后端使用 #[serde(flatten)] 展平了 message 字段，需要重新构建
      const { connection_id, owner_node_uuid: _owner, timestamp, direction, content, is_binary, size } = event.WsMessageSent;
      const message: WsMessage = { timestamp, direction, content, is_binary, size };
      get().handleWsMessageSent(connection_id, message);
    } else if ('WsSegmentStarted' in event) {
      // WebSocket 流段开始事件 - 更新 segments 和 owner
      const { connection_id, segment } = event.WsSegmentStarted;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) {
          console.warn(`[调试] 未知连接 ${connection_id} 的流段开始`);
          return {};
        }

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
        if (!stream) {
          console.warn(`[调试] 未知连接 ${connection_id} 的流段结束`);
          return {};
        }

        // 更新对应的 segment
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
      // WebSocket 流所有权变更事件（后端发送）
      const { connection_id, to_node_uuid } = event.WsStreamOwnerChanged;
      set((state) => {
        const stream = state.wsStreams.get(connection_id);
        if (!stream) {
          console.warn(`[调试] 未知连接 ${connection_id} 的所有者变更`);
          return {};
        }

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
          node_name: '', // 后端没有发送名称，前端需要从 flow 中获取
          node_type: 'ws_send_wait', // 假设是 send_wait 节点
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
      // WebSocket 流关闭事件（后端发送）
      const { connection_id } = event.WsStreamClosed;
      get().handleWsStreamClosed(connection_id);
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
  },

  reset: () => {
    clearAllTimers();
    set(initialState);
  },

  // ===== WebSocket 事件处理 =====
  handleWsStreamCreated: (stream: WsConnectionStream) => {
    set((state) => {
      const newStreams = new Map(state.wsStreams);
      newStreams.set(stream.connection_id, stream);
      return {
        wsStreams: newStreams,
        // 如果没有活动连接，设置为新创建的连接
        activeConnectionId: state.activeConnectionId || stream.connection_id,
      };
    });
  },

  handleWsMessageReceived: (connectionId: string, message: WsMessage) => {
    set((state) => {
      const stream = state.wsStreams.get(connectionId);
      if (!stream) {
        console.warn(`[调试] 收到未知连接 ${connectionId} 的消息`);
        return {};
      }

      // 找到当前活动的 segment（最后一个未结束的）
      const segments = [...stream.segments];
      const lastSegment = segments[segments.length - 1];
      
      if (lastSegment && lastSegment.ended_at === null) {
        // 添加消息到当前 segment
        lastSegment.messages.push(message);
        // 检查是否超过最大消息数
        if (lastSegment.messages.length > lastSegment.max_messages) {
          lastSegment.dropped_message_count += lastSegment.messages.length - lastSegment.max_messages;
          lastSegment.messages = lastSegment.messages.slice(-lastSegment.max_messages);
        }
      }

      const newStreams = new Map(state.wsStreams);
      newStreams.set(connectionId, { ...stream, segments });
      return { wsStreams: newStreams };
    });
  },

  handleWsMessageSent: (connectionId: string, message: WsMessage) => {
    set((state) => {
      const stream = state.wsStreams.get(connectionId);
      if (!stream) {
        console.warn(`[调试] 发送到未知连接 ${connectionId} 的消息`);
        return {};
      }

      // 找到当前活动的 segment（最后一个未结束的）
      const segments = [...stream.segments];
      const lastSegment = segments[segments.length - 1];
      
      if (lastSegment && lastSegment.ended_at === null) {
        // 添加消息到当前 segment
        lastSegment.messages.push(message);
        // 检查是否超过最大消息数
        if (lastSegment.messages.length > lastSegment.max_messages) {
          lastSegment.dropped_message_count += lastSegment.messages.length - lastSegment.max_messages;
          lastSegment.messages = lastSegment.messages.slice(-lastSegment.max_messages);
        }
      }

      const newStreams = new Map(state.wsStreams);
      newStreams.set(connectionId, { ...stream, segments });
      return { wsStreams: newStreams };
    });
  },

  handleWsStreamOwnerChanged: (connectionId: string, ownerNodeUuid: string | null) => {
    set((state) => {
      const stream = state.wsStreams.get(connectionId);
      if (!stream) {
        console.warn(`[调试] 未知连接 ${connectionId} 的所有者变更`);
        return {};
      }

      const newStreams = new Map(state.wsStreams);
      newStreams.set(connectionId, {
        ...stream,
        current_owner_node_uuid: ownerNodeUuid,
      });
      return { wsStreams: newStreams };
    });
  },

  handleWsStreamClosed: (connectionId: string) => {
    set((state) => {
      const stream = state.wsStreams.get(connectionId);
      if (!stream) {
        console.warn(`[调试] 关闭未知连接 ${connectionId}`);
        return {};
      }

      // 结束当前活动的 segment
      const segments = [...stream.segments];
      const lastSegment = segments[segments.length - 1];
      if (lastSegment && lastSegment.ended_at === null) {
        lastSegment.ended_at = Date.now();
      }

      const newStreams = new Map(state.wsStreams);
      newStreams.set(connectionId, {
        ...stream,
        segments,
        is_closed: true,
        current_owner_node_uuid: null,
      });

      // 如果关闭的是当前活动连接，清除活动连接
      const newActiveConnectionId = state.activeConnectionId === connectionId 
        ? null 
        : state.activeConnectionId;

      return {
        wsStreams: newStreams,
        activeConnectionId: newActiveConnectionId,
      };
    });
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
