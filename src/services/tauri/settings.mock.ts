/**
 * Mock 设置服务 - 用于非 Tauri 环境测试
 * 使用 localStorage 模拟持久化存储
 */

import type { Settings, ThemeMode } from '@/types/settings';

const STORAGE_KEY = 'app-settings';
const DEFAULT_REPORT_DIR = '~/.app/reports/';
const DEFAULT_LOG_DIR = '~/.app/logs/';
const DEFAULT_FLOWS_DIR = '~/.app/flows/';
const DEFAULT_THEME: ThemeMode = 'system';

/**
 * 从 localStorage 获取存储的设置
 */
function getStoredSettings(): Partial<Settings> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch {
    return {};
  }
}

/**
 * 加载设置
 */
export async function loadSettings(): Promise<Settings> {
  const stored = getStoredSettings();
  return {
    reportDir: stored.reportDir ?? DEFAULT_REPORT_DIR,
    logDir: stored.logDir ?? DEFAULT_LOG_DIR,
    flowsDir: stored.flowsDir ?? DEFAULT_FLOWS_DIR,
    theme: stored.theme ?? DEFAULT_THEME,
    locale: stored.locale ?? 'zh-CN',
  };
}

/**
 * 保存设置
 */
export async function saveSettings(settings: Partial<Settings>): Promise<void> {
  const current = getStoredSettings();
  const updated = { ...current, ...settings };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
}

/**
 * 选择目录对话框（Mock: 返回 null 表示用户取消）
 */
export async function selectDirectory(_currentPath: string): Promise<string | null> {
  return null;
}

/**
 * 在文件管理器中打开目录（Mock: 无法真正打开）
 */
export async function openDirectory(_path: string): Promise<void> {
  // Mock 环境下无法真正打开目录
}

/**
 * 获取配置文件路径（Mock）
 */
export async function getConfigPath(): Promise<string> {
  return '浏览器本地存储 (localStorage)';
}

/**
 * 获取应用数据目录（Mock）
 */
export async function getAppDataDir(): Promise<string> {
  return '浏览器环境';
}

/**
 * 保存 CSV 模板文件（Mock: 模拟成功，返回文件名）
 */
export async function saveCsvTemplate(filename: string, _content: string): Promise<string> {
  // Mock 环境下无法真正写入文件，直接返回文件名
  return filename;
}
