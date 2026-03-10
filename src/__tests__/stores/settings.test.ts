/**
 * Settings Store 测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useSettingsStore } from '@/stores/settings';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

describe('SettingsStore', () => {
  beforeEach(() => {
    localStorageMock.clear();
    // 重置 store 状态
    useSettingsStore.setState({
      reportDir: '',
      logDir: '',
      isLoading: true,
      error: null,
    });
  });

  describe('初始状态', () => {
    it('初始 reportDir 为空字符串', () => {
      expect(useSettingsStore.getState().reportDir).toBe('');
    });

    it('初始 logDir 为空字符串', () => {
      expect(useSettingsStore.getState().logDir).toBe('');
    });

    it('初始 isLoading 为 true', () => {
      expect(useSettingsStore.getState().isLoading).toBe(true);
    });

    it('初始 error 为 null', () => {
      expect(useSettingsStore.getState().error).toBeNull();
    });
  });

  describe('loadSettings', () => {
    it('加载设置后更新状态', async () => {
      // 预设一些设置
      localStorageMock.setItem('app-settings', JSON.stringify({
        reportDir: '/custom/reports/',
        logDir: '/custom/logs/',
      }));

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe('/custom/reports/');
      expect(useSettingsStore.getState().logDir).toBe('/custom/logs/');
      expect(useSettingsStore.getState().isLoading).toBe(false);
      expect(useSettingsStore.getState().error).toBeNull();
    });

    it('加载默认设置（无存储数据时）', async () => {
      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe('~/.app/reports/');
      expect(useSettingsStore.getState().logDir).toBe('~/.app/logs/');
      expect(useSettingsStore.getState().isLoading).toBe(false);
    });
  });

  describe('setReportDir', () => {
    it('更新 reportDir 并持久化', async () => {
      await useSettingsStore.getState().setReportDir('/new/reports/');

      expect(useSettingsStore.getState().reportDir).toBe('/new/reports/');

      // 验证已持久化
      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.reportDir).toBe('/new/reports/');
    });
  });

  describe('setLogDir', () => {
    it('更新 logDir 并持久化', async () => {
      await useSettingsStore.getState().setLogDir('/new/logs/');

      expect(useSettingsStore.getState().logDir).toBe('/new/logs/');

      // 验证已持久化
      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.logDir).toBe('/new/logs/');
    });
  });

  describe('openReportDir', () => {
    it('调用 openDirectory 完成无错误', async () => {
      useSettingsStore.setState({ reportDir: '/test/reports/' });
      await useSettingsStore.getState().openReportDir();
      // Mock 环境下函数正常完成即可
      expect(useSettingsStore.getState().error).toBeNull();
    });
  });

  describe('openLogDir', () => {
    it('调用 openDirectory 完成无错误', async () => {
      useSettingsStore.setState({ logDir: '/test/logs/' });
      await useSettingsStore.getState().openLogDir();
      // Mock 环境下函数正常完成即可
      expect(useSettingsStore.getState().error).toBeNull();
    });
  });
});
