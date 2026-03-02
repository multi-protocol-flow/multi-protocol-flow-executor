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
    };

    expect(settings.reportDir).toBe('/path/to/reports');
    expect(settings.logDir).toBe('/path/to/logs');
  });

  it('SettingsState interface should have correct structure', () => {
    const state: SettingsState = {
      reportDir: '/path/to/reports',
      logDir: '/path/to/logs',
      isLoading: false,
      error: null,
    };

    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('SettingsState should accept error string', () => {
    const state: SettingsState = {
      reportDir: '',
      logDir: '',
      isLoading: true,
      error: 'Failed to load settings',
    };

    expect(state.error).toBe('Failed to load settings');
  });

  it('SettingsActions interface should have correct methods', () => {
    const actions: SettingsActions = {
      loadSettings: async () => {},
      setReportDir: async (_dir: string) => {},
      setLogDir: async (_dir: string) => {},
      openReportDir: async () => {},
      openLogDir: async () => {},
    };

    expect(typeof actions.loadSettings).toBe('function');
    expect(typeof actions.setReportDir).toBe('function');
    expect(typeof actions.setLogDir).toBe('function');
    expect(typeof actions.openReportDir).toBe('function');
    expect(typeof actions.openLogDir).toBe('function');
  });

  it('SettingsStore should combine state and actions', () => {
    const store: SettingsStore = {
      reportDir: '/path/to/reports',
      logDir: '/path/to/logs',
      isLoading: false,
      error: null,
      loadSettings: async () => {},
      setReportDir: async (_dir: string) => {},
      setLogDir: async (_dir: string) => {},
      openReportDir: async () => {},
      openLogDir: async () => {},
    };

    expect(store.reportDir).toBe('/path/to/reports');
    expect(typeof store.loadSettings).toBe('function');
  });
});
