/**
 * Settings Mock 服务测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
  loadSettings,
  saveSettings,
  selectDirectory,
  openDirectory,
} from '@/services/tauri/settings.mock';

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

describe('Settings Mock Service', () => {
  beforeEach(() => {
    localStorageMock.clear();
  });

  describe('loadSettings', () => {
    it('返回默认设置（无存储数据时）', async () => {
      const settings = await loadSettings();

      expect(settings.reportDir).toBe('~/.app/reports/');
      expect(settings.logDir).toBe('~/.app/logs/');
      expect(settings.flowsDir).toBe('~/.app/flows/');
    });

    it('返回已存储的设置', async () => {
      localStorageMock.setItem('app-settings', JSON.stringify({
        reportDir: '/custom/reports/',
        logDir: '/custom/logs/',
        flowsDir: '/custom/flows/',
      }));

      const settings = await loadSettings();

      expect(settings.reportDir).toBe('/custom/reports/');
      expect(settings.logDir).toBe('/custom/logs/');
      expect(settings.flowsDir).toBe('/custom/flows/');
    });

    it('部分设置使用默认值', async () => {
      localStorageMock.setItem('app-settings', JSON.stringify({
        reportDir: '/custom/reports/',
      }));

      const settings = await loadSettings();

      expect(settings.reportDir).toBe('/custom/reports/');
      expect(settings.logDir).toBe('~/.app/logs/');
      expect(settings.flowsDir).toBe('~/.app/flows/');
    });

    it('处理无效 JSON', async () => {
      localStorageMock.setItem('app-settings', 'invalid-json');

      const settings = await loadSettings();

      expect(settings.reportDir).toBe('~/.app/reports/');
      expect(settings.logDir).toBe('~/.app/logs/');
    });
  });

  describe('saveSettings', () => {
    it('保存单个设置', async () => {
      await saveSettings({ reportDir: '/new/reports/' });

      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.reportDir).toBe('/new/reports/');
    });

    it('保存多个设置', async () => {
      await saveSettings({
        reportDir: '/new/reports/',
        logDir: '/new/logs/',
      });

      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.reportDir).toBe('/new/reports/');
      expect(stored.logDir).toBe('/new/logs/');
    });

    it('合并现有设置', async () => {
      localStorageMock.setItem('app-settings', JSON.stringify({
        reportDir: '/old/reports/',
        logDir: '/old/logs/',
      }));

      await saveSettings({ reportDir: '/new/reports/' });

      const stored = JSON.parse(localStorageMock.getItem('app-settings') || '{}');
      expect(stored.reportDir).toBe('/new/reports/');
      expect(stored.logDir).toBe('/old/logs/');
    });
  });

  describe('selectDirectory', () => {
    it('返回 null（Mock 环境）', async () => {
      const result = await selectDirectory('/some/path/');
      expect(result).toBeNull();
    });
  });

  describe('openDirectory', () => {
    it('不抛出错误', async () => {
      await expect(openDirectory('/some/path/')).resolves.not.toThrow();
    });
  });
});
