/**
 * 压力测试类型定义
 * 与后端 src-tauri/crates/flow-engine/src/stress_test/types.rs 中的结构体对齐
 */

import type { Flow } from './flow';
import type { ErrorDetail } from './error';
import type { FlowStagesReport } from './report';
// ============================================================================
// 数据集策略
// ============================================================================

/**
 * CSV 数据取行策略
 */
export type DataPickStrategy = 'round_robin' | 'random' | 'unique';

// ============================================================================
// 压力测试状态枚举
// ============================================================================

/**
 * 压力测试状态
 */
export type StressTestStatus = 'idle' | 'rampUp' | 'running' | 'stopped';

// ============================================================================
// 压力测试配置
// ============================================================================

/**
 * 压力测试配置
 */
export interface StressTestConfig {
  /** 并发用户数 */
  concurrentUsers: number;
  /** 爬升时间（秒） */
  rampUpSeconds: number;
  /** 持续时间（秒） */
  durationSeconds: number;
  /** 流程 UUID */
  flowUuid: string;
  /** CSV 数据源路径（可选，设置后启用数据驱动模式） */
  datasetPath?: string;
  /** CSV 编码 */
  datasetEncoding?: string;
  /** 数据取行策略 */
  datasetStrategy?: DataPickStrategy;
}

// ============================================================================
// 压力测试进度
// ============================================================================

/**
 * 压力测试进度
 */
export interface StressTestProgress {
  /** 当前状态 */
  status: StressTestStatus;
  /** 已运行时间（秒） */
  elapsedSeconds: number;
  /** 总迭代次数 */
  totalIterations: number;
  /** 成功迭代次数 */
  successfulIterations: number;
  /** 失败迭代次数 */
  failedIterations: number;
  /** 当前 RPS（每秒请求数） */
  currentRps: number;
  /** 平均延迟（毫秒） */
  avgLatencyMs: number;
  /** 节点级实时指标 */
  nodeMetrics: NodeMetrics[];
}

// ============================================================================
// 压力测试统计信息
// ============================================================================

/**
 * 压力测试统计信息
 */
export interface StressTestStatistics {
  /** 总迭代次数 */
  totalIterations: number;
  /** 成功次数 */
  successful: number;
  /** 失败次数 */
  failed: number;
  /** 成功率 (0.0 - 1.0) */
  successRate: number;
  /** RPS（每秒请求数） */
  rps: number;
}

// ============================================================================
// 响应时间分布
// ============================================================================

/**
 * 响应时间分布
 */
export interface ResponseTimeDistribution {
  /** 平均响应时间（毫秒） */
  avgMs: number;
  /** 最小响应时间（毫秒） */
  minMs: number;
  /** 最大响应时间（毫秒） */
  maxMs: number;
  /** P50 响应时间（毫秒） */
  p50Ms: number;
  /** P90 响应时间（毫秒） */
  p90Ms: number;
  /** P95 响应时间（毫秒） */
  p95Ms: number;
  /** P99 响应时间（毫秒） */
  p99Ms: number;
}

// ============================================================================
// 错误样本
// ============================================================================

/**
 * 错误样本
 */
export interface ErrorSample {
  /** 迭代序号 */
  iteration: number;
  /** 发生时间戳（毫秒） */
  timestamp?: number;
  /** 节点 UUID */
  nodeUuid: string;
  /** 节点名称 */
  nodeName: string;
  /** 节点类型 */
  nodeType?: string;
  /** 错误消息 */
  errorMessage: string;
  /** 结构化错误详情 */
  errorDetail?: ErrorDetail;
  /** 执行耗时（毫秒） */
  durationMs?: number;
  /** 输入数据（完整存储） */
  inputData?: unknown;
  /** 输出数据（完整存储） */
  outputData?: unknown;
  /** 解析后的配置 */
  resolvedConfig?: unknown;
}
// ============================================================================
// 错误统计
// ============================================================================

/**
 * 错误统计
 */
export interface ErrorStatistics {
  /** 错误类型 */
  errorKind: string;
  /** 出现次数 */
  count: number;
  /** 错误样本（最多保留若干条） */
  samples: ErrorSample[];
}

// ============================================================================
// 节点级指标与统计
// ============================================================================

/** 节点级实时指标（压测过程中实时更新） */
export interface NodeMetrics {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  totalExecutions: number;
  successfulExecutions: number;
  failedExecutions: number;
  successRate: number;
  avgLatencyMs: number;
  minLatencyMs: number;
  maxLatencyMs: number;
  p50LatencyMs: number;
  p90LatencyMs: number;
  p95LatencyMs: number;
  p99LatencyMs: number;
}

/** 节点级别统计（最终报告用） */
export interface NodeLevelStatistics {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  totalExecutions: number;
  successfulExecutions: number;
  failedExecutions: number;
  successRate: number;
  responseTimeDistribution: ResponseTimeDistribution;
  errorStatistics: ErrorStatistics[];
}

/** 节点类型聚合统计 */
export interface NodeTypeStatistics {
  nodeType: string;
  nodeCount: number;
  totalExecutions: number;
  successfulExecutions: number;
  failedExecutions: number;
  successRate: number;
  responseTimeDistribution: ResponseTimeDistribution;
}

/** 压测报告元数据（用于历史列表） */
export interface StressTestReportMetadata {
  uuid: string;
  flowUuid: string;
  flowName: string;
  startTime: number;
  /** 报告文件完整路径（用于加载报告） */
  filePath: string;
}

// ============================================================================
// 压力测试报告
// ============================================================================

/**
 * 压力测试报告
 */
export interface StressTestReport {
  /** 报告唯一 ID */
  uuid: string;
  /** 关联的流程 UUID */
  flowUuid: string;
  /** 流程名称 */
  flowName: string;
  /** 测试开始时间（毫秒时间戳） */
  startTime: number;
  /** 测试结束时间（毫秒时间戳） */
  endTime: number;
  /** 总执行时长（毫秒） */
  durationMs: number;
  /** 测试配置 */
  config: StressTestConfig;
  /** 统计信息 */
  statistics: StressTestStatistics;
  /** 响应时间分布 */
  responseTimeDistribution: ResponseTimeDistribution;
  /** 错误统计列表（按错误类型分组，无错误时省略） */
  errorStatistics?: ErrorStatistics[];
  /** 节点级别统计（无数据时省略） */
  nodeStatistics?: NodeLevelStatistics[];
  /** 节点类型聚合统计（无数据时省略） */
  nodeTypeStatistics?: NodeTypeStatistics[];
  /** 进度历史数据（用于折线图等可视化，无数据时省略） */
  progressHistory?: StressTestProgress[];
  /** Flow 快照（执行时的流程完整副本） */
  flowSnapshot?: Flow;
  /** 生命周期执行报告（Setup & Teardown 阶段） */
  lifecycle?: FlowStagesReport;
}

// ============================================================================
// 压力测试事件
// ============================================================================

/**
 * 压力测试事件（从后端发送到前端）
 */
export type StressTestEvent =
  | { type: 'started'; flowUuid: string; config: StressTestConfig }
  | { type: 'progress'; progress: StressTestProgress }
  | { type: 'ended'; report: StressTestReport };

// ============================================================================
// IPC 命令类型
// ============================================================================

/**
 * 获取压力测试状态命令的输出结果
 */
export interface GetStressTestStatusOutput {
  /** 当前状态 */
  status: StressTestStatus;
  /** 进度信息（运行中时有值） */
  progress?: StressTestProgress;
}
