/**
 * 环境与变量体系类型定义
 */

/**
 * 环境变量集
 */
export interface EnvironmentSet {
  /** 环境名称（如 "dev"、"staging"、"prod"） */
  name: string;
  /** 环境描述 */
  description?: string;
  /** 环境变量字典 */
  variables: Record<string, unknown>;
}

/**
 * 环境元数据
 */
export interface EnvironmentMetadata {
  /** 环境名称 */
  name: string;
  /** 绝对文件路径 */
  filePath: string;
  /** 变量总数 */
  variableCount: number;
  /** 是否存在本地私有覆盖文件（.local.yaml） */
  hasLocalOverride: boolean;
  /** 更新时间戳（毫秒） */
  updatedAt: number;
}

/**
 * 环境对比项（画布对比矩阵使用）
 */
export interface EnvironmentComparisonItem {
  /** 变量名 */
  key: string;
  /** 最终生效值 */
  effectiveValue: unknown;
  /** 变量来源 */
  source: 'flow_fallback' | 'environment' | 'local_override';
  /** 环境定义的值（若存在） */
  envValue?: unknown;
  /** 本地私有覆盖的值（若存在） */
  localValue?: unknown;
  /** 流程内置兜底默认值（若存在） */
  fallbackValue?: unknown;
  /** 是否被覆盖 */
  isOverridden: boolean;
}
