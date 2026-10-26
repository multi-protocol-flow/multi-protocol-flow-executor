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
      flowsDir: '',
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

    it('初始 flowsDir 为空字符串', () => {
      expect(useSettingsStore.getState().flowsDir).toBe('');
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
        flowsDir: '/custom/flows/',
      }));

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe('/custom/reports/');
      expect(useSettingsStore.getState().logDir).toBe('/custom/logs/');
      expect(useSettingsStore.getState().flowsDir).toBe('/custom/flows/');
      expect(useSettingsStore.getState().isLoading).toBe(false);
      expect(useSettingsStore.getState().error).toBeNull();
    });

    it('加载默认设置（无存储数据时）', async () => {
      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe('~/.app/reports/');
      expect(useSettingsStore.getState().logDir).toBe('~/.app/logs/');
      expect(useSettingsStore.getState().flowsDir).toBe('~/.app/flows/');
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

  describe('setFlowsDir', () => {
    it('更新 flowsDir 并持久化', async () => {
      await useSettingsStore.getState().setFlowsDir('/new/flows/');

      expect(useSettingsStore.getState().flowsDir).toBe('/new/flows/');

      // 验证已持久化
      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.flowsDir).toBe('/new/flows/');
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

  describe('openFlowsDir', () => {
    it('调用 openDirectory 完成无错误', async () => {
      useSettingsStore.setState({ flowsDir: '/test/flows/' });
      await useSettingsStore.getState().openFlowsDir();
      // Mock 环境下函数正常完成即可
      expect(useSettingsStore.getState().error).toBeNull();
    });
  });
  describe('setAiMaxIterations', () => {
    it('更新 aiMaxIterations 并 clamp 在 5 到 100 范围内', async () => {
      await useSettingsStore.getState().setAiMaxIterations(35);
      expect(useSettingsStore.getState().aiMaxIterations).toBe(35);

      // 小于 5 (含 0) 时 clamp 到 5
      await useSettingsStore.getState().setAiMaxIterations(2);
      expect(useSettingsStore.getState().aiMaxIterations).toBe(5);

      await useSettingsStore.getState().setAiMaxIterations(0);
      expect(useSettingsStore.getState().aiMaxIterations).toBe(5);

      // 大于 100 时 clamp 到 100
      await useSettingsStore.getState().setAiMaxIterations(150);
      expect(useSettingsStore.getState().aiMaxIterations).toBe(100);
    });
  });
});
