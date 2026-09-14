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
import { useUIStore } from '@/stores/ui';
import { toast } from 'sonner';
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
          showUpdateDialog: false,
        });

        // 无论是静默检测（未跳过时）还是手动检测，均不再弹大模态窗，改为右上角 Toast 通知并引导至全局设置
        if (manual || !isIgnored) {
          toast(ll.settings.update_available(), {
            description: `${ll.settings.update_available_toast_desc()} (v${info.latestVersion})`,
            action: {
              label: ll.settings.go_to_settings(),
              onClick: () => {
                useUIStore.getState().openSettingsDialog('about');
              },
            },
            duration: 8000,
          });
        }
      } else {
        set({
          status: 'up-to-date',
          updateInfo: info,
          currentVersion: info.currentVersion,
          lastChecked: Date.now(),
        });
        if (manual) {
          notifyInfo(ll.settings.update_already_latest());
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

    const ll = getLL();
    toast.info(ll.settings.installing_update());

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
