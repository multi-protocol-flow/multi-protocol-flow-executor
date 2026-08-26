/**
 * 执行事件类型定义
 *
 * 与 Rust ExecutionEvent 对齐，使用 camelCase 序列化
 */

import type { NodeExecutionReport, FlowExecutionReport } from './report';

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
 * 执行事件（正常模式）
 *
 * Rust 使用 #[serde(rename_all = "camelCase")] 序列化为 tagged union
 * 前端接收到的 JSON 格式为：{ "flowStarted": {...} } 或 { "nodeCompleted": {...} } 等
 */
export type ExecutionEvent =
  | { flowStarted: { flowUuid: string; flowName: string; totalNodes: number } }
  | { nodeStarted: { nodeUuid: string; nodeName: string; nodeType: string } }
  | { nodeCompleted: { report: NodeExecutionReport } }
  | { flowCompleted: { report: FlowExecutionReport } }
  | { flowCancelled: { flowUuid: string; reason: string } }
  | { pluginEvent: PluginEventData };
