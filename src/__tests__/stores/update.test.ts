/**
 * Update Store 测试
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useUpdateStore } from '@/stores/update';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { toast } from 'sonner';

vi.mock('sonner', () => ({
  toast: vi.fn(),
}));
describe('UpdateStore', () => {
  beforeEach(() => {
    useUpdateStore.getState().reset();
    useSettingsStore.setState({
      autoCheckUpdate: true,
      includePrerelease: false,
      ignoredVersions: [],
    });
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('状态应为 idle 且无更新信息', () => {
      const state = useUpdateStore.getState();
      expect(state.status).toBe('idle');
      expect(state.updateInfo).toBeNull();
      expect(state.progress).toBeNull();
      expect(state.downloadedFilePath).toBeNull();
      expect(state.showUpdateDialog).toBe(false);
    });
  });

  describe('检查更新流程', () => {
    it('调用 loadCurrentVersion 加载版本号', async () => {
      const version = await useUpdateStore.getState().loadCurrentVersion();
      expect(version).toBe('0.1.1-beta.2');
      expect(useUpdateStore.getState().currentVersion).toBe('0.1.1-beta.2');
    });

    it('检查更新发现新版本后进入 available 且触发右上角通知，不弹出大模态弹窗', async () => {
      const info = await useUpdateStore.getState().checkForUpdates(true);
      expect(info).not.toBeNull();
      expect(info?.hasUpdate).toBe(true);

      const state = useUpdateStore.getState();
      expect(state.status).toBe('available');
      expect(state.showUpdateDialog).toBe(false);
      expect(state.updateInfo?.latestVersion).toBe('0.1.2');
      expect(toast).toHaveBeenCalled();
    });

    it('如果新版本在跳过列表中，静默检查不触发通知且不弹窗', async () => {
      useSettingsStore.setState({
        ignoredVersions: ['0.1.2'],
      });

      await useUpdateStore.getState().checkForUpdates(false);
      const state = useUpdateStore.getState();
      expect(state.status).toBe('available');
      expect(state.showUpdateDialog).toBe(false);
      expect(toast).not.toHaveBeenCalled();
    });

    it('如果新版本在跳过列表中，手动检查仍然触发右上角通知', async () => {
      useSettingsStore.setState({
        ignoredVersions: ['0.1.2'],
      });

      await useUpdateStore.getState().checkForUpdates(true);
      const state = useUpdateStore.getState();
      expect(state.status).toBe('available');
      expect(state.showUpdateDialog).toBe(false);
      expect(toast).toHaveBeenCalled();
    });

    it('通知 Action 点击能够打开全局设置关于 Tab', async () => {
      useUIStore.setState({ showSettingsDialog: false, settingsDialogTab: 'general' });
      await useUpdateStore.getState().checkForUpdates(true);

      expect(toast).toHaveBeenCalled();
      const lastCall = vi.mocked(toast).mock.calls[0];
      const options = lastCall[1] as { action?: { onClick?: () => void } };
      expect(options.action?.onClick).toBeDefined();
      options.action?.onClick?.();

      expect(useUIStore.getState().showSettingsDialog).toBe(true);
      expect(useUIStore.getState().settingsDialogTab).toBe('about');
    });
  });

  describe('忽略版本与弹窗控制', () => {
    it('ignoreCurrentVersion 将版本加入 settings 忽略列表并关闭弹窗', async () => {
      await useUpdateStore.getState().checkForUpdates(true);
      expect(useUpdateStore.getState().showUpdateDialog).toBe(false);
      await useUpdateStore.getState().ignoreCurrentVersion();
      expect(useUpdateStore.getState().showUpdateDialog).toBe(false);
      expect(useSettingsStore.getState().ignoredVersions).toContain('0.1.2');
    });

    it('openUpdateDialog 和 closeUpdateDialog 正常切换', () => {
      useUpdateStore.getState().openUpdateDialog();
      expect(useUpdateStore.getState().showUpdateDialog).toBe(true);

      useUpdateStore.getState().closeUpdateDialog();
      expect(useUpdateStore.getState().showUpdateDialog).toBe(false);
    });
  });

  describe('下载流程', () => {
    it('startDownload 在 Mock 环境下完成下载并标记 downloaded', async () => {
      await useUpdateStore.getState().checkForUpdates(true);
      const filePath = await useUpdateStore.getState().startDownload();

      expect(filePath).toBeTruthy();
      const state = useUpdateStore.getState();
      expect(state.status).toBe('downloaded');
      expect(state.downloadedFilePath).toBe(filePath);
      expect(state.progress?.percent).toBe(100);
    });
  });
});
