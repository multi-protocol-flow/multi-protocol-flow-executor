/**
 * 调试模式类型定义
 * 与后端 src-tauri/crates/flow-engine/src/debug/types.rs 中的结构体对齐
 */

import type { FlowExecutionReport, VariableChange } from './report';
import type { WsMessage, WsStreamSegment, WsConnectionStream } from './ws_stream';

// ============================================================================
// 调试状态枚举
// ============================================================================
// 调试状态枚举
// ============================================================================
// 调试状态枚举
// ============================================================================

/**
 * 调试会话状态
 */
export type DebugStatus = 'idle' | 'running' | 'paused' | 'ended';

/**
 * 暂停原因
 */
export type PauseReason =
  | { type: 'breakpoint'; node_uuid: string }
  | { type: 'step_complete'; node_uuid: string }
  | { type: 'error'; message: string };

/**
 * 调试结束原因
 */
export type EndReason =
  | { type: 'completed' }
  | { type: 'user_abort' }
  | { type: 'error'; message: string };

// ============================================================================
// 调试命令枚举
// ============================================================================

/**
 * 调试命令（从前端发送到后端）
 */
export type DebugCommand =
  | { StartDebug: { flow_uuid: string } }
  | 'Continue'
  | 'StepOver'
  | { SkipNode: { selected_port?: string; simulated_output?: unknown } }
  | 'RunToEnd'
  | 'EndDebug'
  | { ModifyVariable: { name: string; value: unknown } }
  | { ModifyNodeConfig: { node_uuid: string; config: unknown } }
  | { SetBreakpoint: { node_uuid: string; enabled: boolean } };

// ============================================================================
// 调试事件枚举
// ============================================================================

/**
 * 执行堆栈条目
 */
export interface ExecutionStackEntry {
  /** 节点 UUID */
  node_uuid: string;
  /** 节点名称 */
  node_name: string;
  /** 节点类型 */
  node_type: string;
  /** 执行开始时间（Unix 时间戳，毫秒） */
  started_at: number;
  /** 执行结束时间（Unix 时间戳，毫秒） */
  ended_at?: number;
  /** 输入数据 */
  input?: unknown;
  /** 输出数据 */
  output?: unknown;
  /** 是否执行成功 */
  success: boolean;
  /** 错误信息（如果失败） */
  error?: string;
  /** 解析后的配置（与 NodeExecutionReport 对齐） */
  resolved_config?: unknown;
  /** 当前节点修改变量的记录（与 NodeExecutionReport 对齐） */
  variable_changes?: VariableChange[];
  /** 跳过原因（与 NodeExecutionReport 对齐） */
  skip_reason?: string;
}

/**
 * 调试事件（从后端发送到前端）
 */
export type DebugEvent =
  | { SessionStarted: { flow_uuid: string } }
  | {
      PausedAtNode: {
        node_uuid: string;
        reason: PauseReason;
        variables: Record<string, unknown>;
        execution_stack: ExecutionStackEntry[];
      };
    }
  | { NodeExecuted: { node_uuid: string; output?: unknown } }
  | { SessionEnded: { reason: EndReason; report?: FlowExecutionReport; execution_stack: ExecutionStackEntry[] } }
  | { VariableModified: { name: string; value: unknown } }
  | { NodeConfigModified: { node_uuid: string; config: unknown } }
  | { BreakpointModified: { node_uuid: string; enabled: boolean } }
  | { Error: { message: string } }
  | {
      Heartbeat: {
        status: DebugStatus;
        current_node_uuid?: string;
        timestamp: number;
      };
    }
  // WebSocket 流事件
  | {
      WsMessageReceived: {
        connection_id: string;
        message: WsMessage;
      };
    }
  | {
      WsMessageSent: {
        connection_id: string;
        message: WsMessage;
      };
    }
  | {
      WsSegmentStarted: {
        connection_id: string;
        segment: WsStreamSegment;
      };
    }
  | {
      WsSegmentEnded: {
        connection_id: string;
        segment: WsStreamSegment;
      };
    }
  | {
      WsConnectionOpened: {
        stream: WsConnectionStream;
      };
    }
  | {
      WsConnectionClosed: {
        connection_id: string;
      };
    };

// ============================================================================
// 调试配置常量
// ============================================================================

/**
 * 调试会话健康检查配置
 */
export const DEBUG_HEALTH_CONFIG = {
  /** 心跳超时时间（毫秒）- 超过此时间未收到心跳则认为会话已死 */
  HEARTBEAT_TIMEOUT_MS: 15000,
  /** 命令响应超时时间（毫秒）- 发送命令后等待响应的超时时间 */
  COMMAND_TIMEOUT_MS: 30000,
  /** 心跳检查间隔（毫秒） */
  HEALTH_CHECK_INTERVAL_MS: 5000,
} as const;

// ============================================================================
// 调试会话状态
// ============================================================================

/**
 * 调试会话状态
 */
export interface DebugSessionState {
  /** 会话 ID */
  session_id: string;
  /** 流程 UUID */
  flow_uuid: string;
  /** 当前调试状态 */
  status: DebugStatus;
  /** 当前暂停的节点 UUID（如果暂停） */
  current_node_uuid?: string;
  /** 当前暂停原因（如果暂停） */
  pause_reason?: PauseReason;
  /** 当前上下文变量 */
  variables: Record<string, unknown>;
  /** 执行堆栈 */
  execution_stack: ExecutionStackEntry[];
  /** 断点列表（节点 UUID） */
  breakpoints: string[];
  /** 是否处于单步模式 */
  is_stepping: boolean;
  /** 是否跳过后续断点（执行到底模式） */
  skip_breakpoints: boolean;
}
