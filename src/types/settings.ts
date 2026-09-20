import type { AIProviderConfig } from './ai';

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
  /** 环境保存目录路径（可选，未配置时跟随流程目录: <flowsDir>/environments） */
  environmentsDir?: string;
  /** 插件目录路径（可选；未配置时用后端默认解析） */
  pluginDir?: string;
  /** 插件市场地址（可选；未配置时 GUI 市场 Tab 显示引导文案） */
  pluginRegistryUrl?: string;
  /** 节点默认执行超时时间（毫秒，未设置时兜底 30000） */
  defaultExecutionTimeoutMs?: number;
  /** 插件最大并发请求数（未设置时兜底 256） */
  pluginMaxConcurrency?: number;
  /** 插件通信队列深度（未设置时兜底 2048） */
  pluginQueueCapacity?: number;
  /** 主题模式 */
  theme: ThemeMode;
  /** 语言区域 */
  locale?: Locale;
  /** AI Provider 配置列表 */
  aiProviders?: AIProviderConfig[];
  /** 当前选中的激活 AI Provider ID */
  activeAiProviderId?: string | null;
  /** AI 助手全局启用开关 */
  aiEnabled?: boolean;
  /** AI 窗口模式 */
  aiWindowMode?: 'dock' | 'spotlight';
  /** 自定义全局 System Prompt */
  aiCustomSystemPrompt?: string;
  /** 启动时是否自动检查更新（默认 true） */
  autoCheckUpdate?: boolean;
  /** 是否接收预发布版本（Beta / Alpha） */
  includePrerelease?: boolean;
  /** 跳过提醒的版本号列表 */
  ignoredVersions?: string[];
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
  /** 环境保存目录路径（可选，未配置时跟随流程目录: <flowsDir>/environments） */
  environmentsDir?: string;
  /** 插件目录路径（可选；未配置时用后端默认解析） */
  pluginDir?: string;
  /** 插件市场地址（可选） */
  pluginRegistryUrl?: string;
  /** 节点默认执行超时时间（毫秒） */
  defaultExecutionTimeoutMs?: number;
  /** 插件最大并发请求数 */
  pluginMaxConcurrency?: number;
  /** 插件通信队列深度 */
  pluginQueueCapacity?: number;
  /** 主题模式 */
  theme: ThemeMode;
  /** 语言区域 */
  locale?: Locale;
  /** AI Provider 配置列表 */
  aiProviders: AIProviderConfig[];
  /** 当前选中的激活 AI Provider ID */
  activeAiProviderId: string | null;
  /** AI 助手全局启用开关 */
  aiEnabled: boolean;
  /** AI 窗口模式 */
  aiWindowMode: 'dock' | 'spotlight';
  /** 自定义全局 System Prompt */
  aiCustomSystemPrompt?: string;
  /** 启动时是否自动检查更新 */
  autoCheckUpdate?: boolean;
  /** 是否接收预发布版本 */
  includePrerelease?: boolean;
  /** 跳过提醒的版本号列表 */
  ignoredVersions?: string[];
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
  /** 设置环境保存目录 */
  setEnvironmentsDir: (dir: string) => Promise<void>;
  /** 设置插件目录 */
  setPluginDir: (dir: string) => Promise<void>;
  /** 设置插件市场地址 */
  setPluginRegistryUrl: (url: string) => Promise<void>;
  /** 设置主题模式 */
  setTheme: (theme: ThemeMode) => Promise<void>;
  /** 设置语言区域 */
  setLocale: (locale: Locale) => Promise<void>;
  /** 设置节点默认执行超时时间（毫秒） */
  setDefaultExecutionTimeoutMs: (ms: number) => Promise<void>;
  /** 设置插件最大并发请求数 */
  setPluginMaxConcurrency: (concurrency: number) => Promise<void>;
  /** 设置插件通信队列深度 */
  setPluginQueueCapacity: (capacity: number) => Promise<void>;
  /** 打开报告目录 */
  openReportDir: () => Promise<void>;
  /** 打开日志目录 */
  openLogDir: () => Promise<void>;
  /** 打开流程保存目录 */
  openFlowsDir: () => Promise<void>;
  /** 打开环境保存目录 */
  openEnvironmentsDir: () => Promise<void>;
  /** 设置 AI Providers */
  setAiProviders: (providers: AIProviderConfig[]) => Promise<void>;
  /** 设置当前激活的 AI Provider ID */
  setActiveAiProviderId: (id: string | null) => Promise<void>;
  /** 设置 AI 助手启用状态 */
  setAiEnabled: (enabled: boolean) => Promise<void>;
  /** 设置 AI 窗口展示模式 */
  setAiWindowMode: (mode: 'dock' | 'spotlight') => Promise<void>;
  /** 设置自定义全局 System Prompt */
  setAiCustomSystemPrompt: (prompt: string | undefined) => Promise<void>;
  /** 设置启动时自动检查更新 */
  setAutoCheckUpdate?: (enabled: boolean) => Promise<void>;
  /** 设置是否接收预发布版本 */
  setIncludePrerelease?: (enabled: boolean) => Promise<void>;
  /** 记录跳过某个版本的更新提醒 */
  addIgnoredVersion?: (version: string) => Promise<void>;
}

/**
 * 设置 Store 完整类型（状态 + 操作）
 */
export type SettingsStore = SettingsState & SettingsActions;
