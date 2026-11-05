/**
 * 执行事件类型定义
 *
 * 与 Rust ExecutionEvent 对齐，使用 camelCase 序列化
 */

import type { NodeExecutionReport, FlowExecutionReport, IterationSummary } from './report';

/**
 * 插件节点流式事件数据（SidecarPlugin 运行时推送）
 * 字段与 Rust `ExecutionEvent::PluginEvent` 的 camelCase serde 名称对齐
 */
export interface PluginEventData {
  /** 流程唯一标识 */
  flowUuid: string;
  /** 节点唯一标识 */
  nodeUuid: string;
  /** 节点类型 */
  nodeType: string;
  /** 事件类型（如 "log"、"data"、"status"） */
  eventType: string;
  /** 事件数据（任意 JSON 值） */
  data: unknown;
  /** 事件时间戳 */
  timestamp: string;
}

/**
 * 插件人机协同交互请求 (HITL: Human-in-the-Loop)
 */
export interface PendingInteraction {
  interaction_id: string;
  execution_id: string;
  /** "ask_human" | "confirmation" | "step_pause" */
  interaction_type: 'ask_human' | 'confirmation' | 'step_pause' | string;
  prompt: string;
  tool_id?: string;
  tool_name?: string;
  input_args?: Record<string, unknown>;
  step_index?: number;
  created_at_ms: number;
}

/** 人机协同用户响应载荷 */
export interface HumanResponsePayload {
  text?: string;
  approved?: boolean;
  reason?: string;
  action?: 'next_step' | 'run_to_end' | 'cancel' | string;
}

/** 单个流式分块数据 */
export interface NodeStreamChunk {
  kind?: 'content' | 'reasoning' | 'data' | string;
  delta_content?: string;
  delta_reasoning?: string;
  text?: string;
  timestamp: string;
  raw?: unknown;
  [key: string]: unknown;
}

/** 节点实时流式输出聚合数据 */
export interface NodeStreamData {
  fullContent: string;
  fullReasoning: string;
  lastUpdated: number;
  chunks: NodeStreamChunk[];
}

/**
 * 子流程实时执行进度（宿主把子流程视为**单个节点**，进度挂在该 SubFlow 节点上）
 */
export interface SubFlowProgress {
  /** 子流程内已完成（执行结束）的节点数量 */
  completed: number;
  /** 子流程节点总数（来自嵌套 flowStarted） */
  total: number;
  /** 当前正在执行的子节点名称 */
  currentNodeName?: string;
}

/**
 * loop 节点运行中的实时进度（按 loop 节点 UUID 归集）
 *
 * 与 Rust `ExecutionEvent::LoopIteration` 对齐：`flow:loop` 的最终报告要等整个
 * 循环结束才产生，运行中的报告靠这份增量数据渲染迭代列表。
 */
export interface LoopLiveProgress {
  /** 各轮摘要（与最终报告的 `iteration_summary` 同口径，永不裁剪） */
  iterations: IterationSummary[];
  /** 各轮保留的 body 节点报告（与 `sub_flow_report.node_reports` 同口径） */
  reports: NodeExecutionReport[];
  /** 只保留摘要的轮次数 */
  truncated: number;
}

/**
 * 执行事件（正常模式）
 *
 * Rust 使用 #[serde(rename_all = "camelCase")] 序列化为 tagged union
 * 前端接收到的 JSON 格式为：{ "flowStarted": {...} } 或 { "nodeCompleted": {...} } 等
 *
 * `parentNodeUuid` 标记事件来自子流程内部：其值为触发该子流程的 SubFlow 节点 UUID。
 * 顶层流程的事件不含该字段。消费方必须据此把嵌套事件归属到 SubFlow 节点，
 * 而不能让它们重置顶层流程的运行状态与节点计数。
 */
export type ExecutionEvent =
  | {
      flowStarted: {
        flowUuid: string;
        flowName: string;
        totalNodes: number;
        parentNodeUuid?: string;
      };
    }
  | {
      nodeStarted: {
        nodeUuid: string;
        nodeName: string;
        nodeType: string;
        parentNodeUuid?: string;
      };
    }
  | { nodeCompleted: { report: NodeExecutionReport; parentNodeUuid?: string } }
  | { flowCompleted: { report: FlowExecutionReport; parentNodeUuid?: string } }
  | { flowCancelled: { flowUuid: string; reason: string; parentNodeUuid?: string } }
  | {
      loopIteration: {
        /** loop 节点 UUID */
        nodeUuid: string;
        /** 该轮摘要 */
        iteration: IterationSummary;
        /** 该轮 body 节点报告（被裁剪时缺省） */
        reports?: NodeExecutionReport[];
        /** 该轮是否只保留了摘要 */
        truncated?: boolean;
      };
    }
  | { pluginEvent: PluginEventData }
  | { batch: ExecutionEvent[] };
