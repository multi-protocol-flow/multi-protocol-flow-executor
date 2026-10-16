/**
 * Tauri 设置持久化服务
 * 使用 tauri-plugin-store 和后端 open_directory 命令
 */

import { isTauriEnv } from '@/lib/platform';
import type { Settings, ThemeMode, Locale } from '@/types/settings';
import type { AIProviderConfig } from '@/types/ai';


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
    const environmentsDir = await store.get<string>('environmentsDir');
    const pluginDir = await store.get<string>('pluginDir');
    const pluginRegistryUrl = await store.get<string>('pluginRegistryUrl');
    const defaultExecutionTimeoutMs = await store.get<number>('defaultExecutionTimeoutMs');
    const pluginMaxConcurrency = await store.get<number>('pluginMaxConcurrency');
    const pluginQueueCapacity = await store.get<number>('pluginQueueCapacity');
    const defaultIngressPort = await store.get<number>('defaultIngressPort');
    const theme = await store.get<string>('theme');
    const locale = await store.get<string>('locale');
    const aiProviders = await store.get<AIProviderConfig[]>('aiProviders');
    const activeAiProviderId = await store.get<string | null>('activeAiProviderId');
    const aiEnabled = await store.get<boolean>('aiEnabled');
    const aiWindowMode = await store.get<'dock' | 'spotlight'>('aiWindowMode');
    const aiCustomSystemPrompt = await store.get<string>('aiCustomSystemPrompt');
    const autoCheckUpdate = await store.get<boolean>('autoCheckUpdate');
    const includePrerelease = await store.get<boolean>('includePrerelease');
    const ignoredVersions = await store.get<string[]>('ignoredVersions');
    return {
      reportDir: reportDir ?? `${appDataDir}/reports/`,
      logDir: logDir ?? `${appDataDir}/logs/`,
      flowsDir: flowsDir ?? `${appDataDir}/flows/`,
      environmentsDir: environmentsDir ?? '',
      pluginDir,
      pluginRegistryUrl,
      defaultExecutionTimeoutMs,
      pluginMaxConcurrency,
      pluginQueueCapacity,
      defaultIngressPort: defaultIngressPort ?? 9527,
      theme: (theme as ThemeMode) ?? 'system',
      locale: (locale as Locale) ?? 'en-US',
      aiProviders: aiProviders ?? [],
      activeAiProviderId: activeAiProviderId ?? null,
      aiEnabled: aiEnabled ?? true,
      aiWindowMode: aiWindowMode ?? 'dock',
      aiCustomSystemPrompt: aiCustomSystemPrompt ?? undefined,
      autoCheckUpdate: autoCheckUpdate ?? true,
      includePrerelease: includePrerelease ?? false,
      ignoredVersions: ignoredVersions ?? [],
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
    if (settings.environmentsDir !== undefined) {
      await store.set('environmentsDir', settings.environmentsDir);
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
    if (settings.pluginMaxConcurrency !== undefined) {
      await store.set('pluginMaxConcurrency', settings.pluginMaxConcurrency);
    }
    if (settings.pluginQueueCapacity !== undefined) {
      await store.set('pluginQueueCapacity', settings.pluginQueueCapacity);
    }
    if (settings.defaultIngressPort !== undefined) {
      await store.set('defaultIngressPort', settings.defaultIngressPort);
    }
    if (settings.theme !== undefined) {
      await store.set('theme', settings.theme);
    }
    if (settings.locale !== undefined) {
      await store.set('locale', settings.locale);
    }
    if (settings.aiProviders !== undefined) {
      await store.set('aiProviders', settings.aiProviders);
    }
    if (settings.activeAiProviderId !== undefined) {
      await store.set('activeAiProviderId', settings.activeAiProviderId);
    }
    if (settings.aiEnabled !== undefined) {
      await store.set('aiEnabled', settings.aiEnabled);
    }
    if (settings.aiWindowMode !== undefined) {
      await store.set('aiWindowMode', settings.aiWindowMode);
    }
    if (settings.aiCustomSystemPrompt !== undefined) {
      await store.set('aiCustomSystemPrompt', settings.aiCustomSystemPrompt);
    }
    if (settings.autoCheckUpdate !== undefined) {
      await store.set('autoCheckUpdate', settings.autoCheckUpdate);
    }
    if (settings.includePrerelease !== undefined) {
      await store.set('includePrerelease', settings.includePrerelease);
    }
    if (settings.ignoredVersions !== undefined) {
      await store.set('ignoredVersions', settings.ignoredVersions);
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
 * 在系统文件管理器中显示并高亮定位文件或目录
 */
export async function revealItemInDir(path: string): Promise<void> {
  if (isTauriEnv()) {
    try {
      const { revealItemInDir: reveal } = await import('@tauri-apps/plugin-opener');
      await reveal(path);
      return;
    } catch {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('reveal_in_file_manager', { path });
      return;
    }
  }
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
