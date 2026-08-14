/**
 * Updater Store 单元测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useUpdaterStore, selectUpdateAvailable } from '@/stores/updater';
import { checkForUpdates, downloadAndInstallUpdate } from '@/services/tauri';
import type { UpdateInfo } from '@/types/updater';

vi.mock('@/services/tauri', () => ({
  checkForUpdates: vi.fn(),
  downloadAndInstallUpdate: vi.fn(),
}));

const mockCheck = vi.mocked(checkForUpdates);
const mockDownload = vi.mocked(downloadAndInstallUpdate);

const availableUpdate: UpdateInfo = {
  available: true,
  version: '1.2.0',
  currentVersion: '1.1.0',
  body: '修复若干问题',
};

describe('UpdaterStore', () => {
  beforeEach(() => {
    useUpdaterStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('初始状态正确', () => {
      const state = useUpdaterStore.getState();
      expect(state.checking).toBe(false);
      expect(state.updateInfo).toBeNull();
      expect(state.downloading).toBe(false);
      expect(state.downloadProgress).toBe(0);
      expect(state.error).toBeNull();
      expect(state.dismissedVersion).toBeNull();
    });
  });

  describe('checkForUpdates', () => {
    it('检查成功时写入更新信息', async () => {
      mockCheck.mockResolvedValue(availableUpdate);
      await useUpdaterStore.getState().checkForUpdates();

      const state = useUpdaterStore.getState();
      expect(state.checking).toBe(false);
      expect(state.updateInfo).toEqual(availableUpdate);
      expect(state.error).toBeNull();
    });

    it('无可用更新时 updateInfo 为 null', async () => {
      mockCheck.mockResolvedValue(null);
      await useUpdaterStore.getState().checkForUpdates();
      expect(useUpdaterStore.getState().updateInfo).toBeNull();
      expect(useUpdaterStore.getState().checking).toBe(false);
    });

    it('检查失败时记录错误', async () => {
      mockCheck.mockRejectedValue(new Error('网络错误'));
      await useUpdaterStore.getState().checkForUpdates();

      const state = useUpdaterStore.getState();
      expect(state.checking).toBe(false);
      expect(state.error).toContain('网络错误');
      expect(state.updateInfo).toBeNull();
    });

    it('已忽略的版本不再提示', async () => {
      mockCheck.mockResolvedValue(availableUpdate);
      await useUpdaterStore.getState().checkForUpdates();
      useUpdaterStore.getState().dismissUpdate();

      await useUpdaterStore.getState().checkForUpdates();
      expect(useUpdaterStore.getState().updateInfo).toBeNull();
      expect(useUpdaterStore.getState().dismissedVersion).toBe('1.2.0');
    });
  });

  describe('downloadAndInstall', () => {
    it('按进度事件推进下载进度', async () => {
      mockDownload.mockImplementation(async (onEvent) => {
        onEvent?.({ event: 'Started' });
        onEvent?.({ event: 'Progress', data: { chunkLength: 1024 } });
        onEvent?.({ event: 'Finished' });
      });
      await useUpdaterStore.getState().downloadAndInstall();

      const state = useUpdaterStore.getState();
      expect(state.downloading).toBe(true);
      expect(state.downloadProgress).toBe(100);
    });

    it('Progress 事件每次最多累加到 90', async () => {
      mockDownload.mockImplementation(async (onEvent) => {
        for (let i = 0; i < 20; i++) {
          onEvent?.({ event: 'Progress', data: { chunkLength: 1024 } });
        }
      });
      await useUpdaterStore.getState().downloadAndInstall();
      expect(useUpdaterStore.getState().downloadProgress).toBe(90);
    });

    it('下载失败时记录错误', async () => {
      mockDownload.mockRejectedValue(new Error('下载中断'));
      await useUpdaterStore.getState().downloadAndInstall();

      const state = useUpdaterStore.getState();
      expect(state.downloading).toBe(false);
      expect(state.error).toContain('下载中断');
    });
  });

  describe('dismissUpdate / reset', () => {
    it('dismissUpdate 记录已忽略版本并清空更新信息', async () => {
      mockCheck.mockResolvedValue(availableUpdate);
      await useUpdaterStore.getState().checkForUpdates();
      useUpdaterStore.getState().dismissUpdate();

      const state = useUpdaterStore.getState();
      expect(state.updateInfo).toBeNull();
      expect(state.dismissedVersion).toBe('1.2.0');
    });

    it('reset 还原初始状态', async () => {
      mockCheck.mockResolvedValue(availableUpdate);
      await useUpdaterStore.getState().checkForUpdates();
      useUpdaterStore.getState().dismissUpdate();
      useUpdaterStore.getState().reset();

      const state = useUpdaterStore.getState();
      expect(state.updateInfo).toBeNull();
      expect(state.dismissedVersion).toBeNull();
      expect(state.checking).toBe(false);
      expect(state.downloadProgress).toBe(0);
    });

    it('selectUpdateAvailable 选择器按 available/error 判断', async () => {
      mockCheck.mockResolvedValue(availableUpdate);
      await useUpdaterStore.getState().checkForUpdates();
      expect(selectUpdateAvailable(useUpdaterStore.getState())).toBe(true);

      useUpdaterStore.getState().reset();
      expect(selectUpdateAvailable(useUpdaterStore.getState())).toBeFalsy();

      mockCheck.mockResolvedValue({ ...availableUpdate, error: '校验失败' });
      await useUpdaterStore.getState().checkForUpdates();
      expect(selectUpdateAvailable(useUpdaterStore.getState())).toBe(false);
    });
  });
});
