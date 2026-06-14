/**
 * Tauri IPC 输入/输出类型定义
 * 与后端 src-tauri/src/command.rs 中的结构体对齐
 */

import type { Flow } from './flow';
import type { FlowExecutionReport } from './report';

// ============================================================================
// 数据库连接命令
// ============================================================================

/**
 * 测试数据库连接结果
 */
export interface TestDbConnectionResult {
  /** 是否成功 */
  success: boolean;
  /** 错误信息（如果失败） */
  error?: string;
}

// ============================================================================
// 加载流程命令
// ============================================================================

/**
 * 加载流程命令的输入参数
 */
export interface LoadFlowCommandInput {
  /** 流程文件路径 */
  path: string;
}

/**
 * 加载流程命令的输出结果
 */
export interface LoadFlowCommandOutput {
  /** 加载的流程数据 */
  flow: Flow;
}

// ============================================================================
// 保存流程命令
// ============================================================================

/**
 * 保存流程命令的输入参数
 */
export interface SaveFlowCommandInput {
  /** 要保存的流程数据 */
  flow: Flow;
  /** 保存路径 */
  path: string;
}

/**
 * 保存流程命令的输出结果
 */
export interface SaveFlowCommandOutput {
  /** 是否保存成功 */
  success: boolean;
}

// ============================================================================
// 创建流程命令
// ============================================================================

/**
 * 创建流程命令的输入参数
 */
export interface CreateFlowCommandInput {
  /** 流程名称 */
  name: string;
}

/**
 * 创建流程命令的输出结果
 */
export interface CreateFlowCommandOutput {
  /** 创建的流程数据 */
  flow: Flow;
}

// ============================================================================
// 执行流程命令
// ============================================================================

/**
 * 执行流程命令的输入参数
 */
export interface ExecuteFlowCommandInput {
  /** 流程 ID */
  flow_id: string;
  /** 执行上下文变量（可选） */
  variables?: unknown;
}

/**
 * 执行流程命令的输出结果
 */
export interface ExecuteFlowCommandOutput {
  /** 执行结果 */
  success: boolean;
  /** 执行报告（成功时返回） */
  flow_report?: FlowExecutionReport;
  /** 错误信息（可选） */
  error?: string;
  /** 执行时间（毫秒） */
  execution_time: number;
}

// ============================================================================
// 更新节点命令
// ============================================================================

/**
 * 更新节点命令的输入参数
 */
export interface UpdateNodeCommandInput {
  /** 流程 ID */
  flow_id: string;
  /** 节点 ID */
  node_id: string;
  /** 节点数据 */
  data: unknown;
}

/**
 * 更新节点命令的输出结果
 */
export interface UpdateNodeCommandOutput {
  /** 是否更新成功 */
  success: boolean;
  /** 错误信息（可选） */
  error?: string;
}

// ============================================================================
// 删除节点命令
// ============================================================================

/**
 * 删除节点命令的输入参数
 */
export interface DeleteNodeCommandInput {
  /** 流程 ID */
  flow_id: string;
  /** 节点 ID */
  node_id: string;
}

/**
 * 删除节点命令的输出结果
 */
export interface DeleteNodeCommandOutput {
  /** 是否删除成功 */
  success: boolean;
  /** 错误信息（可选） */
  error?: string;
}

// ============================================================================
// 流程验证
// ============================================================================

/**
 * 验证错误条目（结构化，含错误码、详细消息和可选字段名）
 * 对应后端 flow-engine-executor 中的 ValidationEntry
 */
export interface ValidationEntry {
  /** 错误码（如 E_VAL_MISSING_ENTRY，用于前端 i18n 映射） */
  code: string;
  /** 详细错误消息（后端英文原始描述） */
  message: string;
  /** 相关字段名（可选） */
  field?: string;
}

/**
 * 流程验证结果
 */
export interface ValidationResult {
  /** 验证是否通过 */
  valid: boolean;
  /** 结构化错误列表 */
  errors: ValidationEntry[];
  /** 警告信息列表 */
  warnings: string[];
}

// ============================================================================
// 报告元数据
// ============================================================================

/**
 * 执行模式
 */
export type ExecutionMode = 'normal' | 'debug';

/**
 * 报告元数据（用于历史列表）
 */
export interface ReportMetadata {
  /** 报告唯一 ID */
  uuid: string;
  /** 关联的流程 ID */
  flow_uuid: string;
  /** 流程名称 */
  flow_name: string;
  /** 执行状态 */
  status: 'success' | 'failed';
  /** 执行开始时间（毫秒时间戳） */
  timestamp: number;
  /** 执行时长（毫秒） */
  duration_ms: number;
  /** 执行模式（正常/调试） */
  execution_mode?: ExecutionMode;
}

