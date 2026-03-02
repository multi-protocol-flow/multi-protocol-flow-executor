/**
 * 执行报告类型定义
 * 与后端 src-tauri/src/report.rs 和 src-tauri/src/statistics.rs 中的结构体对齐
 */

import type { ErrorDetail } from './error';

// ============================================================================
// 执行状态枚举
// ============================================================================

/**
 * 执行状态枚举
 */
export type ExecutionStatus = 'success' | 'failed' | 'skipped';

// ============================================================================
// 节点执行报告
// ============================================================================

/**
 * 节点执行报告
 */
export interface NodeExecutionReport {
  /** 节点 UUID */
  node_uuid: string;
  /** 节点类型 */
  node_type: string;
  /** 节点名称 */
  node_name: string;
  /** 开始时间（毫秒时间戳） */
  start_time: number;
  /** 结束时间（毫秒时间戳） */
  end_time: number;
  /** 执行时长（毫秒） */
  duration_ms: number;
  /** 执行状态 */
  status: ExecutionStatus;
  /** 输入数据（完整存储，不截断） */
  input_data?: unknown;
  /** 输出数据（完整存储，不截断） */
  output_data?: unknown;
  /**
   * 实际使用的输出端口 ID（仅已执行节点有值）
   * - Entry 节点：固定为 "out"
   * - Http 节点：固定为 "out"
   * - Conditional 节点：根据条件评估结果为 "true" 或 "false"
   */
  used_port_id?: string;
  /** 错误详情 */
  error?: ErrorDetail;
}

// ============================================================================
// 流程统计信息
// ============================================================================

/**
 * 流程执行统计信息
 *
 * 提供流程执行的各项统计指标，包括成功/失败节点数量、执行时间、
 * 数据大小和执行效率等。
 */
export interface FlowStatistics {
  // ===== 基础统计 =====
  /** 成功的节点数量 */
  success_count: number;
  /** 失败的节点数量 */
  failed_count: number;

  // ===== 时间统计 =====
  /** 平均节点执行时间（毫秒） */
  avg_node_duration_ms?: number;
  /** 最长节点执行时间（毫秒） */
  max_node_duration_ms?: number;
  /** 最短节点执行时间（毫秒） */
  min_node_duration_ms?: number;

  // ===== 数据统计 =====
  /** 总输入数据大小（字节，估算） */
  total_input_size_bytes?: number;
  /** 总输出数据大小（字节，估算） */
  total_output_size_bytes?: number;

  // ===== 执行效率 =====
  /** 错误率（0.0 - 1.0） */
  error_rate?: number;
  /** 完成率（executed_count / total_nodes） */
  completion_rate?: number;
}

// ============================================================================
// 流程执行报告
// ============================================================================

/**
 * 流程执行报告
 */
export interface FlowExecutionReport {
  /** 报告唯一 ID（新增） */
  uuid: string;
  /** 流程 UUID */
  flow_uuid: string;
  flow_name: string;
  /** 开始时间（毫秒时间戳） */
  start_time: number;
  /** 结束时间（毫秒时间戳） */
  end_time: number;
  /** 总执行时长（毫秒） */
  duration_ms: number;
  /** 总节点数 */
  total_nodes: number;
  /** 已执行节点数 */
  executed_count: number;
  /** 整体执行状态 */
  status: ExecutionStatus;
  /** 节点执行报告列表（扁平结构） */
  node_reports: NodeExecutionReport[];
  /** 流程统计信息 */
  statistics: FlowStatistics;
  /** 流程级错误详情（可选） */
  flow_error?: ErrorDetail;
}
