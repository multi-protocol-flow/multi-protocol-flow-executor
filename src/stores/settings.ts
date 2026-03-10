/**
 * 设置状态管理
 * 使用 Zustand 管理设置状态，自动检测 Tauri 环境
 */

import { create } from 'zustand';
import type { SettingsStore } from '@/types/settings';

// 检测是否在 Tauri 环境中
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

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
}));
