/**
 * 数据集执行类型定义
 * 与后端 dataset_executor types/events + flow-engine-core dataset.rs 对齐
 */

// ============================================================================
// 执行配置
// ============================================================================

/** 失败策略 */
export type FailureStrategy = 'continue' | 'stop_on_first';

/** 数据集执行配置 */
export interface DatasetExecutionConfig {
  /** 流程 UUID */
  flow_uuid: string;
  /** CSV 文件路径 */
  dataset_path: string;
  /** CSV 编码（可选，自动检测） */
  dataset_encoding?: string;
  /** CSV 分隔符（可选，默认逗号） */
  delimiter?: string;
  /** 并发数（默认 1，即顺序执行） */
  concurrency: number;
  /** 失败策略 */
  failure_strategy: FailureStrategy;
}

// ============================================================================
// CSV 验证
// ============================================================================

/** CSV 验证信息 */
export interface CsvInfo {
  /** 数据行数（不含表头） */
  row_count: number;
  /** 列名列表 */
  column_names: string[];
  /** 检测到的编码名称 */
  detected_encoding: string;
}

// ============================================================================
// 行状态与摘要
// ============================================================================

/** 行执行状态 */
export type RowStatus = 'success' | 'fail' | 'error';

/** 单行执行摘要 */
export interface RowSummary {
  /** 行索引 */
  row_index: number;
  /** 输入数据（CSV 行） */
  input_data: Record<string, unknown>;
  /** 执行状态 */
  status: RowStatus;
  /** 执行耗时（毫秒） */
  duration_ms: number;
  /** 错误信息（失败/错误时有值） */
  error_message?: string;
  /** 是否有详细报告文件 */
  has_detail_report: boolean;
}

// ============================================================================
// 执行报告
// ============================================================================

/** 数据集执行报告 */
export interface DatasetExecutionReport {
  /** 报告 UUID */
  uuid: string;
  /** 流程 UUID */
  flow_uuid: string;
  /** 流程名称 */
  flow_name: string;
  /** 数据集文件路径 */
  dataset_path: string;
  /** 数据集名称（文件名） */
  dataset_name: string;
  /** 编码 */
  encoding: string;
  /** 执行时间戳（毫秒） */
  executed_at: number;
  /** 总耗时（毫秒） */
  total_duration_ms: number;
  /** 总行数 */
  total_rows: number;
  /** 成功数 */
  success_count: number;
  /** 失败数（断言未通过） */
  fail_count: number;
  /** 错误数（执行异常） */
  error_count: number;
  /** 平均耗时（毫秒） */
  avg_duration_ms: number;
  /** P50 耗时（毫秒） */
  p50_duration_ms: number;
  /** P95 耗时（毫秒） */
  p95_duration_ms: number;
  /** P99 耗时（毫秒） */
  p99_duration_ms: number;
  /** 行级执行摘要列表 */
  row_summaries: RowSummary[];
}

// ============================================================================
// 执行事件
// ============================================================================

/** 数据集执行事件（从后端发送到前端的 tagged union） */
export type DatasetEvent =
  | { type: 'started'; total_rows: number; dataset_name: string }
  | { type: 'row_completed'; row_index: number; status: RowStatus; duration_ms: number }
  | { type: 'row_failed'; row_index: number; error: string; duration_ms: number }
  | {
      type: 'progress';
      completed: number;
      total: number;
      success_count: number;
      fail_count: number;
      error_count: number;
      elapsed_ms: number;
      estimated_remaining_ms?: number;
    }
  | { type: 'ended'; report: DatasetExecutionReport }
  | { type: 'cancelled'; report: DatasetExecutionReport };

// ============================================================================
// 报告列表项
// ============================================================================

/** 数据集报告摘要（用于历史列表） */
export interface DatasetReportSummary {
  /** 报告 ID（UUID） */
  report_id: string;
  /** 流程名称 */
  flow_name: string;
  /** 数据集名称 */
  dataset_name: string;
  /** 执行时间戳（毫秒） */
  executed_at: number;
  /** 总行数 */
  total_rows: number;
  /** 成功数 */
  success_count: number;
  /** 失败数 */
  fail_count: number;
  /** 错误数 */
  error_count: number;
}
