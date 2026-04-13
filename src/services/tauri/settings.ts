/**
 * Tauri 设置持久化服务
 * 使用 tauri-plugin-store 和后端 open_directory 命令
 */

import { LazyStore } from '@tauri-apps/plugin-store';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import type { Settings, ThemeMode } from '@/types/settings';
const SETTINGS_FILE = 'settings.bin';

let store: LazyStore | null = null;

/**
 * 获取 Store 实例（延迟加载）
 */
async function getStore(): Promise<LazyStore> {
  if (!store) {
    const appDataDir = await invoke<string>('get_app_data_dir');
    store = new LazyStore(`${appDataDir}/store/${SETTINGS_FILE}`);
  }
  return store;
}

/**
 * 加载设置
 */
export async function loadSettings(): Promise<Settings> {
  const s = await getStore();
  const reportDir = await s.get<string>('reportDir');
  const logDir = await s.get<string>('logDir');
  const theme = await s.get<string>('theme');
  const appDataDir = await invoke<string>('get_app_data_dir');
  return {
    reportDir: reportDir ?? `${appDataDir}/reports/`,
    logDir: logDir ?? `${appDataDir}/logs/`,
    theme: (theme as ThemeMode) ?? 'system',
  };
}

/**
 * 保存设置
 */
export async function saveSettings(settings: Partial<Settings>): Promise<void> {
  const s = await getStore();
  if (settings.reportDir !== undefined) {
    await s.set('reportDir', settings.reportDir);
  }
  if (settings.logDir !== undefined) {
    await s.set('logDir', settings.logDir);
  }
  if (settings.theme !== undefined) {
    await s.set('theme', settings.theme);
  }
  await s.save();
}

/**
 * 选择目录对话框
 */
export async function selectDirectory(currentPath: string): Promise<string | null> {
  const result = await open({
    directory: true,
    defaultPath: currentPath,
  });
  return result as string | null;
}

/**
 * 在文件管理器中打开目录
 */
/**
 * 在文件管理器中打开目录
 */
export async function openDirectory(path: string): Promise<void> {
  await invoke('open_directory', { path });
}

/**
 * 获取配置文件路径
 */
export async function getConfigPath(): Promise<string> {
  const appDataDir = await invoke<string>('get_app_data_dir');
  return `${appDataDir}/store/${SETTINGS_FILE}`;
}

/**
 * 获取应用数据目录
 */
export async function getAppDataDir(): Promise<string> {
  return invoke<string>('get_app_data_dir');
}
