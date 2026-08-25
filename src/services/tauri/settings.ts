/**
 * Tauri 设置持久化服务
 * 使用 tauri-plugin-store 和后端 open_directory 命令
 */

import { isTauriEnv } from '@/lib/platform';
import type { Settings, ThemeMode, Locale } from '@/types/settings';

const SETTINGS_FILE = 'settings.bin';

/** 缓存 appDataDir，避免重复 IPC 调用 */
let cachedAppDataDir: string | null = null;

/**
 * 获取应用数据目录（带缓存）
 */
async function getAppDataDirCached(): Promise<string> {
  if (cachedAppDataDir) return cachedAppDataDir;
  const { invoke } = await import('@tauri-apps/api/core');
  cachedAppDataDir = await invoke<string>('get_app_data_dir');
  return cachedAppDataDir;
}

/**
 * 加载设置
 */
export async function loadSettings(): Promise<Settings> {
  if (isTauriEnv()) {
    const [{ LazyStore }] = await Promise.all([
      import('@tauri-apps/plugin-store'),
    ]);
    const appDataDir = await getAppDataDirCached();
    const store = new LazyStore(`${appDataDir}/store/${SETTINGS_FILE}`);
    const reportDir = await store.get<string>('reportDir');
    const logDir = await store.get<string>('logDir');
    const flowsDir = await store.get<string>('flowsDir');
    const pluginDir = await store.get<string>('pluginDir');
    const pluginRegistryUrl = await store.get<string>('pluginRegistryUrl');
    const defaultExecutionTimeoutMs = await store.get<number>('defaultExecutionTimeoutMs');
    const theme = await store.get<string>('theme');
    const locale = await store.get<string>('locale');
    return {
      reportDir: reportDir ?? `${appDataDir}/reports/`,
      logDir: logDir ?? `${appDataDir}/logs/`,
      flowsDir: flowsDir ?? `${appDataDir}/flows/`,
      pluginDir,
      pluginRegistryUrl,
      defaultExecutionTimeoutMs,
      theme: (theme as ThemeMode) ?? 'system',
      locale: (locale as Locale) ?? 'en-US',
    };
  }
  const { loadSettings: loadSettingsImpl } = await import('./settings.mock');
  return loadSettingsImpl();
}

/**
 * 保存设置
 */
export async function saveSettings(settings: Partial<Settings>): Promise<void> {
  if (isTauriEnv()) {
    const [{ LazyStore }] = await Promise.all([
      import('@tauri-apps/plugin-store'),
    ]);
    const appDataDir = await getAppDataDirCached();
    const store = new LazyStore(`${appDataDir}/store/${SETTINGS_FILE}`);
    if (settings.reportDir !== undefined) {
      await store.set('reportDir', settings.reportDir);
    }
    if (settings.logDir !== undefined) {
      await store.set('logDir', settings.logDir);
    }
    if (settings.flowsDir !== undefined) {
      await store.set('flowsDir', settings.flowsDir);
    }
    if (settings.pluginDir !== undefined) {
      await store.set('pluginDir', settings.pluginDir);
    }
    if (settings.pluginRegistryUrl !== undefined) {
      await store.set('pluginRegistryUrl', settings.pluginRegistryUrl);
    }
    if (settings.defaultExecutionTimeoutMs !== undefined) {
      await store.set('defaultExecutionTimeoutMs', settings.defaultExecutionTimeoutMs);
    }
    if (settings.theme !== undefined) {
      await store.set('theme', settings.theme);
    }
    if (settings.locale !== undefined) {
      await store.set('locale', settings.locale);
    }
    await store.save();
    return;
  }
  const { saveSettings: saveSettingsImpl } = await import('./settings.mock');
  return saveSettingsImpl(settings);
}

/**
 * 选择目录对话框
 */
export async function selectDirectory(currentPath: string): Promise<string | null> {
  if (isTauriEnv()) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const result = await open({
      directory: true,
      defaultPath: currentPath,
    });
    return result as string | null;
  }
  const { selectDirectory: selectDirectoryImpl } = await import('./settings.mock');
  return selectDirectoryImpl(currentPath);
}

/**
 * 在文件管理器中打开目录
 */
export async function openDirectory(path: string): Promise<void> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('open_directory', { path });
    return;
  }
  const { openDirectory: openDirectoryImpl } = await import('./settings.mock');
  return openDirectoryImpl(path);
}

/**
 * 获取配置文件路径
 */
export async function getConfigPath(): Promise<string> {
  if (isTauriEnv()) {
    const appDataDir = await getAppDataDirCached();
    return `${appDataDir}/store/${SETTINGS_FILE}`;
  }
  const { getConfigPath: getConfigPathImpl } = await import('./settings.mock');
  return getConfigPathImpl();
}

/**
 * 获取应用数据目录
 */
export async function getAppDataDir(): Promise<string> {
  if (isTauriEnv()) {
    return getAppDataDirCached();
  }
  const { getAppDataDir: getAppDataDirImpl } = await import('./settings.mock');
  return getAppDataDirImpl();
}

/**
 * 保存 CSV 模板文件到应用数据目录
 * @returns 写入的文件绝对路径
 */
export async function saveCsvTemplate(filename: string, content: string): Promise<string> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('save_csv_template', { filename, content });
  }
  const { saveCsvTemplate: saveCsvTemplateImpl } = await import('./settings.mock');
  return saveCsvTemplateImpl(filename, content);
}
