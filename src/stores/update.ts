/**
 * 应用版本更新状态管理
 */

import { create } from 'zustand';
import type { UpdateStore, UpdateState, UpdateProgress } from '@/types/update';
import {
  getAppVersion,
  checkAppUpdate,
  downloadUpdatePackage,
  installUpdatePackage,
} from '@/services/tauri/updater';
import { useSettingsStore } from '@/stores/settings';
import { notifyError, notifyInfo } from '@/lib/notify';
import { getLL } from '@/i18n/i18n-lazy';

const initialState: UpdateState = {
  status: 'idle',
  updateInfo: null,
  currentVersion: '',
  progress: null,
  downloadedFilePath: null,
  error: null,
  lastChecked: null,
  showUpdateDialog: false,
};

function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  ...initialState,

  loadCurrentVersion: async () => {
    try {
      const version = await getAppVersion();
      set({ currentVersion: version });
      return version;
    } catch {
      return '';
    }
  },

  checkForUpdates: async (manual = false) => {
    const ll = getLL();
    const settings = useSettingsStore.getState();
    const includePrerelease = settings.includePrerelease;
    const ignoredVersions = settings.ignoredVersions || [];

    set({
      status: 'checking',
      error: null,
    });

    try {
      const info = await checkAppUpdate(includePrerelease);
      const isIgnored = ignoredVersions.includes(info.latestVersion);

      if (info.hasUpdate) {
        set({
          status: 'available',
          updateInfo: info,
          currentVersion: info.currentVersion,
          lastChecked: Date.now(),
          // 手动检查时始终弹窗；自动静默检查仅在未被用户跳过时弹窗
          showUpdateDialog: manual || !isIgnored,
        });
      } else {
        set({
          status: 'up-to-date',
          updateInfo: info,
          currentVersion: info.currentVersion,
          lastChecked: Date.now(),
        });
        if (manual) {
          notifyInfo(ll.settings.update_already_latest?.() ?? 'You are using the latest version.');
        }
      }

      return info;
    } catch (e) {
      const msg = formatErrorMessage(e);
      set({
        status: 'error',
        error: msg,
        lastChecked: Date.now(),
      });
      if (manual) {
        notifyError(ll.settings.update_check_failed(), msg);
      }
      return null;
    }
  },

  startDownload: async () => {
    const { updateInfo, status } = get();
    if (!updateInfo?.downloadUrl) {
      notifyError('Update Error', 'No download URL available for the current platform.');
      return null;
    }

    if (status === 'downloading') {
      return null;
    }

    set({
      status: 'downloading',
      error: null,
      progress: { received: 0, total: null, percent: 0, bytesPerSec: 0 },
    });

    try {
      const filePath = await downloadUpdatePackage(updateInfo.downloadUrl, (progress: UpdateProgress) => {
        set({ progress });
      });

      set({
        status: 'downloaded',
        downloadedFilePath: filePath,
        progress: {
          received: get().progress?.total ?? 100,
          total: get().progress?.total ?? 100,
          percent: 100,
          bytesPerSec: 0,
        },
      });

      return filePath;
    } catch (e) {
      const msg = formatErrorMessage(e);
      set({
        status: 'error',
        error: msg,
      });
      notifyError('Download failed', msg);
      return null;
    }
  },

  installAndRestart: async () => {
    const { downloadedFilePath } = get();
    if (!downloadedFilePath) {
      return;
    }

    try {
      await installUpdatePackage(downloadedFilePath);
    } catch (e) {
      const msg = formatErrorMessage(e);
      set({ error: msg });
      notifyError('Failed to launch installer', msg);
    }
  },

  ignoreCurrentVersion: async () => {
    const { updateInfo } = get();
    if (updateInfo?.latestVersion) {
      await useSettingsStore.getState().addIgnoredVersion?.(updateInfo.latestVersion);
    }
    set({ showUpdateDialog: false });
  },

  openUpdateDialog: () => {
    set({ showUpdateDialog: true });
  },

  closeUpdateDialog: () => {
    set({ showUpdateDialog: false });
  },

  reset: () => {
    set(initialState);
  },
}));
