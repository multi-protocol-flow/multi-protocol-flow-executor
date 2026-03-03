/**
 * 变量系统类型定义
 * 与后端 src-tauri/src/variable.rs 中的结构体对齐
 */

/**
 * 变量定义
 */
export interface Variable {
  /** 变量名（全局唯一） */
  name: string;
  /** 变量值 */
  value: unknown;
  /** 来源节点 UUID */
  sourceNodeUuid: string;
  /** 变量说明（可选） */
  description?: string;
}

/**
 * 变量池
 */
export interface VariablePool {
  /** 变量列表 */
  variables: Variable[];
}

/**
 * 输出映射配置
 */
export interface OutputMapping {
  /** 源字段路径（如 "status", "body.data.id"） */
  source: string;
  /** 目标变量名 */
  target: string;
  /** 映射说明（可选） */
  description?: string;
  /** 是否启用（默认 true） */
  enabled?: boolean;
}

/**
 * 初始变量配置
 */
export interface InitialVariable {
  /** 变量名 */
  name: string;
  /** 变量值 */
  value: unknown;
  /** 变量说明（可选） */
  description?: string;
}
