/**
 * 设置相关类型定义
 */

// ============================================================================
// 设置数据结构
// ============================================================================

/** 主题模式 */
export type ThemeMode = 'light' | 'dark' | 'system';

/** 语言区域 */
export type Locale = 'zh-CN' | 'en-US';

/** 支持的语言区域列表 */
export const SUPPORTED_LOCALES: Locale[] = ['zh-CN', 'en-US'];

/** 默认语言区域 */
export const DEFAULT_LOCALE: Locale = 'zh-CN';

/**
 * 应用设置
 */
export interface Settings {
  /** 报告目录路径 */
  reportDir: string;
  /** 日志目录路径 */
  logDir: string;
  /** 流程保存目录路径 */
  flowsDir: string;
  /** 主题模式 */
  theme: ThemeMode;
  /** 语言区域 */
  locale?: Locale;
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
  /** 流程保存目录路径 */
  flowsDir: string;
  /** 主题模式 */
  theme: ThemeMode;
  /** 语言区域 */
  locale?: Locale;
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
  /** 设置流程保存目录 */
  setFlowsDir: (dir: string) => Promise<void>;
  /** 设置主题模式 */
  setTheme: (theme: ThemeMode) => Promise<void>;
  /** 设置语言区域 */
  setLocale: (locale: Locale) => Promise<void>;
  /** 打开报告目录 */
  openReportDir: () => Promise<void>;
  /** 打开日志目录 */
  openLogDir: () => Promise<void>;
  /** 打开流程保存目录 */
  openFlowsDir: () => Promise<void>;
}

/**
 * 设置 Store 完整类型（状态 + 操作）
 */
export type SettingsStore = SettingsState & SettingsActions;
