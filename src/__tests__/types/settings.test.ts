/**
 * Settings 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import type {
  Settings,
  SettingsState,
  SettingsActions,
  SettingsStore,
} from '@/types/settings';

describe('Settings types', () => {
  it('Settings interface should have correct structure', () => {
    const settings: Settings = {
      reportDir: '/path/to/reports',
      logDir: '/path/to/logs',
      flowsDir: '/path/to/flows',
      theme: 'system',
    };

    expect(settings.reportDir).toBe('/path/to/reports');
    expect(settings.logDir).toBe('/path/to/logs');
    expect(settings.flowsDir).toBe('/path/to/flows');
    expect(settings.theme).toBe('system');
  });

  it('SettingsState interface should have correct structure', () => {
    const state: SettingsState = {
      reportDir: '/path/to/reports',
      logDir: '/path/to/logs',
      flowsDir: '/path/to/flows',
      theme: 'system',
      isLoading: false,
      error: null,
    };

    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.theme).toBe('system');
  });

  it('SettingsState should accept error string', () => {
    const state: SettingsState = {
      reportDir: '',
      logDir: '',
      flowsDir: '',
      theme: 'dark',
      isLoading: true,
      error: 'Failed to load settings',
    };

    expect(state.error).toBe('Failed to load settings');
    expect(state.theme).toBe('dark');
  });

  it('SettingsActions interface should have correct methods', () => {
    const actions: SettingsActions = {
      loadSettings: async () => {},
      setReportDir: async (_dir: string) => {},
      setLogDir: async (_dir: string) => {},
      setFlowsDir: async (_dir: string) => {},
      setTheme: async (_theme: 'light' | 'dark' | 'system') => {},
      openReportDir: async () => {},
      openLogDir: async () => {},
      openFlowsDir: async () => {},
      setLocale: async (_locale: 'zh-CN' | 'en-US') => {},
    };

    expect(typeof actions.loadSettings).toBe('function');
    expect(typeof actions.setReportDir).toBe('function');
    expect(typeof actions.setLogDir).toBe('function');
    expect(typeof actions.setFlowsDir).toBe('function');
    expect(typeof actions.setTheme).toBe('function');
    expect(typeof actions.setLocale).toBe('function');
    expect(typeof actions.openReportDir).toBe('function');
    expect(typeof actions.openLogDir).toBe('function');
    expect(typeof actions.openFlowsDir).toBe('function');
  });

  it('SettingsStore should combine state and actions', () => {
    const store: SettingsStore = {
      reportDir: '/path/to/reports',
      logDir: '/path/to/logs',
      flowsDir: '/path/to/flows',
      theme: 'light',
      isLoading: false,
      error: null,
      loadSettings: async () => {},
      setReportDir: async (_dir: string) => {},
      setLogDir: async (_dir: string) => {},
      setFlowsDir: async (_dir: string) => {},
      setTheme: async (_theme: 'light' | 'dark' | 'system') => {},
      openReportDir: async () => {},
      openLogDir: async () => {},
      openFlowsDir: async () => {},
      setLocale: async (_locale: 'zh-CN' | 'en-US') => {},
    };

    expect(store.reportDir).toBe('/path/to/reports');
    expect(store.flowsDir).toBe('/path/to/flows');
    expect(store.theme).toBe('light');
    expect(typeof store.loadSettings).toBe('function');
    expect(typeof store.setFlowsDir).toBe('function');
    expect(typeof store.setTheme).toBe('function');
  });
});
