/**
 * 设置相关类型定义
 */

// ============================================================================
// 设置数据结构
// ============================================================================

/**
 * 应用设置
 */
export interface Settings {
  /** 报告目录路径 */
  reportDir: string;
  /** 日志目录路径 */
  logDir: string;
}

// ============================================================================
// 设置状态
// ============================================================================

/**
 * 设置状态
 */
export interface SettingsState {
  /** 报告目录路径 */
  reportDir: string;
  /** 日志目录路径 */
  logDir: string;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
}

// ============================================================================
// 设置操作
// ============================================================================

/**
 * 设置操作方法
 */
export interface SettingsActions {
  /** 加载设置 */
  loadSettings: () => Promise<void>;
  /** 设置报告目录 */
  setReportDir: (dir: string) => Promise<void>;
  /** 设置日志目录 */
  setLogDir: (dir: string) => Promise<void>;
  /** 打开报告目录 */
  openReportDir: () => Promise<void>;
  /** 打开日志目录 */
  openLogDir: () => Promise<void>;
}

/**
 * 设置 Store 完整类型（状态 + 操作）
 */
export type SettingsStore = SettingsState & SettingsActions;
