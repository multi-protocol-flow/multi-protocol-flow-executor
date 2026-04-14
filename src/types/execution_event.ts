/**
 * 执行事件类型定义
 *
 * 与 Rust ExecutionEvent 对齐，使用 camelCase 序列化
 */

import type { NodeExecutionReport, FlowExecutionReport } from './report';

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
  | { flowCompleted: { report: FlowExecutionReport } };
