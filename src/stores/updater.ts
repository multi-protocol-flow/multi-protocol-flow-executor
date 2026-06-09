/**
 * 更新状态管理
 * 使用 Zustand 管理应用更新状态
 */
import { create } from 'zustand';
import type { UpdateInfo, DownloadEvent } from '@/types/updater';
import { checkForUpdates as serviceCheck, downloadAndInstallUpdate as serviceDownload } from '@/services/tauri';

interface UpdaterState {
  checking: boolean;
  updateInfo: UpdateInfo | null;
  downloading: boolean;
  downloadProgress: number;
  error: string | null;
  dismissedVersion: string | null;

  checkForUpdates: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismissUpdate: () => void;
  reset: () => void;
}

const initialState = {
  checking: false,
  updateInfo: null,
  downloading: false,
  downloadProgress: 0,
  error: null,
  dismissedVersion: null,
};

export const useUpdaterStore = create<UpdaterState>((set, get) => ({
  ...initialState,

  checkForUpdates: async () => {
    set({ checking: true, error: null });
    try {
      const info = await serviceCheck();
      if (info?.version && info.version === get().dismissedVersion) {
        set({ checking: false, updateInfo: null });
        return;
      }
      set({ updateInfo: info, checking: false });
    } catch (e) {
      set({ error: String(e), checking: false });
    }
  },

  downloadAndInstall: async () => {
    set({ downloading: true, downloadProgress: 0, error: null });
    try {
      await serviceDownload((event: DownloadEvent) => {
        if (event.event === 'Progress' && event.data?.chunkLength) {
          set((s) => ({ downloadProgress: Math.min(s.downloadProgress + 10, 90) }));
        }
        if (event.event === 'Started') set({ downloadProgress: 5 });
        if (event.event === 'Finished') set({ downloadProgress: 100 });
      });
    } catch (e) {
      set({ error: String(e), downloading: false });
    }
  },

  dismissUpdate: () => {
    const version = get().updateInfo?.version;
    set({ updateInfo: null, dismissedVersion: version || null });
  },

  reset: () => set(initialState),
}));

export const selectUpdateAvailable = (s: UpdaterState) =>
  s.updateInfo?.available && !s.updateInfo?.error;
