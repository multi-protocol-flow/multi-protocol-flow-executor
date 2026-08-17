/**
 * 设置状态管理
 * 使用 Zustand 管理设置状态，自动检测 Tauri 环境
 */

import { create } from 'zustand';
import type { SettingsStore, ThemeMode, Locale } from '@/types/settings';
import { switchLocale } from '@/i18n';

import { isTauriEnv } from '@/lib/platform';

// 检测是否在 Tauri 环境中

// 动态导入服务
async function getSettingsService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/settings');
  }
  return import('@/services/tauri/settings.mock');
}

/**
 * 格式化错误信息
 */
function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  reportDir: '',
  logDir: '',
  flowsDir: '',
  pluginDir: undefined,
  pluginRegistryUrl: undefined,
  theme: 'system' as ThemeMode,
  locale: 'en-US' as Locale,
  isLoading: true,
  error: null,

  loadSettings: async () => {
    set({ isLoading: true, error: null });
    try {
      const service = await getSettingsService();
      const settings = await service.loadSettings();
      set({
        reportDir: settings.reportDir,
        logDir: settings.logDir,
        flowsDir: settings.flowsDir,
        pluginDir: settings.pluginDir,
        pluginRegistryUrl: settings.pluginRegistryUrl,
        theme: settings.theme,
        locale: settings.locale,
        isLoading: false,
      });
    } catch (e) {
      set({
        error: formatErrorMessage(e),
        isLoading: false,
      });
    }
  },

  setReportDir: async (dir: string) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ reportDir: dir });
      set({ reportDir: dir });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  setLogDir: async (dir: string) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ logDir: dir });
      set({ logDir: dir });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  setFlowsDir: async (dir: string) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ flowsDir: dir });
      set({ flowsDir: dir });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  setPluginDir: async (dir: string) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ pluginDir: dir });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
      throw e;
    }
  },

  setPluginRegistryUrl: async (url: string) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ pluginRegistryUrl: url });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
      throw e;
    }
  },

  setTheme: async (theme: ThemeMode) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ theme });
      set({ theme });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  setLocale: async (locale: Locale) => {
    try {
      const service = await getSettingsService();
      await service.saveSettings({ locale });
      switchLocale(locale);
      set({ locale });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  openReportDir: async () => {
    try {
      const service = await getSettingsService();
      await service.openDirectory(get().reportDir);
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  openLogDir: async () => {
    try {
      const service = await getSettingsService();
      await service.openDirectory(get().logDir);
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  openFlowsDir: async () => {
    try {
      const service = await getSettingsService();
      await service.openDirectory(get().flowsDir);
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },
}));
