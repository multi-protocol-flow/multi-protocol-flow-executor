/**
 * Mock 设置服务 - 用于非 Tauri 环境测试
 * 使用 localStorage 模拟持久化存储
 */

import type { Settings } from '@/types/settings';

const STORAGE_KEY = 'app-settings';
const DEFAULT_REPORT_DIR = '~/.app/reports/';
const DEFAULT_LOG_DIR = '~/.app/logs/';

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
export async function selectDirectory(currentPath: string): Promise<string | null> {
  console.log('Mock: selectDirectory called with', currentPath);
  return null;
}

/**
 * 在文件管理器中打开目录（Mock: 仅打印日志）
 */
export async function openDirectory(path: string): Promise<void> {
  console.log('Mock: openDirectory called with', path);
  // Mock 环境下无法真正打开目录，仅打印日志
  console.log(`[Mock] 将在文件管理器中打开: ${path}`);
}
