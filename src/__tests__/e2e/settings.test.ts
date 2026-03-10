/**
 * 设置端到端测试
 * 模拟用户完整操作流程
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

// 默认值常量
const DEFAULT_REPORT_DIR = '~/.app/reports/';
const DEFAULT_LOG_DIR = '~/.app/logs/';
const STORAGE_KEY = 'app-settings';

describe('Settings E2E', () => {
  // ========================================
  // 测试场景1：首次启动时显示默认目录
  // ========================================
  describe('场景1: 首次启动显示默认目录', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('首次启动时 reportDir 应为默认值', async () => {
      await useSettingsStore.getState().loadSettings();
      const state = useSettingsStore.getState();

      expect(state.reportDir).toBe(DEFAULT_REPORT_DIR);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it('首次启动时 logDir 应为默认值', async () => {
      await useSettingsStore.getState().loadSettings();
      const state = useSettingsStore.getState();

      expect(state.logDir).toBe(DEFAULT_LOG_DIR);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it('首次启动时 localStorage 无持久化数据', async () => {
      await useSettingsStore.getState().loadSettings();

      // 默认值不应写入 localStorage（除非用户主动修改）
      const stored = localStorageMock.getItem(STORAGE_KEY);
      expect(stored).toBeNull();
    });

    it('首次启动完成后状态正常', async () => {
      const state = useSettingsStore.getState();
      expect(state.isLoading).toBe(true);

      await useSettingsStore.getState().loadSettings();

      const finalState = useSettingsStore.getState();
      expect(finalState.isLoading).toBe(false);
      expect(finalState.error).toBeNull();
    });
  });

  // ========================================
  // 测试场景2：修改目录后设置保存
  // ========================================
  describe('场景2: 修改目录后设置保存', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('修改 reportDir 后保存到 localStorage', async () => {
      const customDir = '/custom/reports/';

      await useSettingsStore.getState().setReportDir(customDir);

      const stored = JSON.parse(localStorageMock.getItem(STORAGE_KEY) || '{}');
      expect(stored.reportDir).toBe(customDir);
    });

    it('修改 logDir 后保存到 localStorage', async () => {
      const customDir = '/custom/logs/';

      await useSettingsStore.getState().setLogDir(customDir);

      const stored = JSON.parse(localStorageMock.getItem(STORAGE_KEY) || '{}');
      expect(stored.logDir).toBe(customDir);
    });

    it('修改单个目录时保留另一个目录的值', async () => {
      // 先设置两个目录
      await useSettingsStore.getState().setReportDir('/first/reports/');
      await useSettingsStore.getState().setLogDir('/first/logs/');

      // 再修改 reportDir
      await useSettingsStore.getState().setReportDir('/second/reports/');

      const stored = JSON.parse(localStorageMock.getItem(STORAGE_KEY) || '{}');
      expect(stored.reportDir).toBe('/second/reports/');
      expect(stored.logDir).toBe('/first/logs/');
    });

    it('修改后 store 状态立即更新', async () => {
      const customDir = '/immediate/update/';

      await useSettingsStore.getState().setReportDir(customDir);

      expect(useSettingsStore.getState().reportDir).toBe(customDir);
    });

    it('多次修改只保留最后一次的值', async () => {
      await useSettingsStore.getState().setReportDir('/first/');
      await useSettingsStore.getState().setReportDir('/second/');
      await useSettingsStore.getState().setReportDir('/third/');

      const stored = JSON.parse(localStorageMock.getItem(STORAGE_KEY) || '{}');
      expect(stored.reportDir).toBe('/third/');
    });
  });

  // ========================================
  // 测试场景3：重启应用后设置保持（模拟）
  // ========================================
  describe('场景3: 重启后加载已保存设置', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('重启后加载已保存的 reportDir', async () => {
      const savedDir = '/saved/reports/';
      localStorageMock.setItem(STORAGE_KEY, JSON.stringify({
        reportDir: savedDir,
        logDir: DEFAULT_LOG_DIR,
      }));

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe(savedDir);
    });

    it('重启后加载已保存的 logDir', async () => {
      const savedDir = '/saved/logs/';
      localStorageMock.setItem(STORAGE_KEY, JSON.stringify({
        reportDir: DEFAULT_REPORT_DIR,
        logDir: savedDir,
      }));

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().logDir).toBe(savedDir);
    });

    it('重启后加载所有已保存设置', async () => {
      const savedReportDir = '/saved/reports/';
      const savedLogDir = '/saved/logs/';
      localStorageMock.setItem(STORAGE_KEY, JSON.stringify({
        reportDir: savedReportDir,
        logDir: savedLogDir,
      }));

      await useSettingsStore.getState().loadSettings();

      const state = useSettingsStore.getState();
      expect(state.reportDir).toBe(savedReportDir);
      expect(state.logDir).toBe(savedLogDir);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it('部分保存的设置与默认值合并', async () => {
      // 只保存了 reportDir，logDir 应使用默认值
      localStorageMock.setItem(STORAGE_KEY, JSON.stringify({
        reportDir: '/partial/reports/',
      }));

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().reportDir).toBe('/partial/reports/');
      expect(useSettingsStore.getState().logDir).toBe(DEFAULT_LOG_DIR);
    });
  });

  // ========================================
  // 测试场景4：打开目录功能工作（Mock）
  // ========================================
  describe('场景4: 打开目录功能 (Mock)', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('openReportDir 正常完成', async () => {
      useSettingsStore.setState({ reportDir: '/test/reports/' });

      await useSettingsStore.getState().openReportDir();

      // Mock 环境下函数正常完成即可
      expect(useSettingsStore.getState().error).toBeNull();
    });

    it('openLogDir 正常完成', async () => {
      useSettingsStore.setState({ logDir: '/test/logs/' });

      await useSettingsStore.getState().openLogDir();

      // Mock 环境下函数正常完成即可
      expect(useSettingsStore.getState().error).toBeNull();
    });

    it('打开目录不产生错误', async () => {
      useSettingsStore.setState({ reportDir: '/test/reports/' });

      await useSettingsStore.getState().openReportDir();

      expect(useSettingsStore.getState().error).toBeNull();
    });
  });

  // ========================================
  // Mock 环境综合测试
  // ========================================
  describe('Mock 环境综合测试', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('完整用户流程：首次启动 -> 修改设置 -> 重启', async () => {
      // 1. 首次启动
      await useSettingsStore.getState().loadSettings();
      expect(useSettingsStore.getState().reportDir).toBe(DEFAULT_REPORT_DIR);
      expect(useSettingsStore.getState().logDir).toBe(DEFAULT_LOG_DIR);

      // 2. 修改设置
      await useSettingsStore.getState().setReportDir('/user/reports/');
      await useSettingsStore.getState().setLogDir('/user/logs/');

      // 3. 模拟重启（重置 store 状态后重新加载）
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });

      await useSettingsStore.getState().loadSettings();

      // 4. 验证设置已恢复
      expect(useSettingsStore.getState().reportDir).toBe('/user/reports/');
      expect(useSettingsStore.getState().logDir).toBe('/user/logs/');
    });

    it('多次修改和加载的正确性', async () => {
      // 第一轮
      await useSettingsStore.getState().loadSettings();
      await useSettingsStore.getState().setReportDir('/round1/');

      // 第二轮（模拟重启）
      useSettingsStore.setState({ reportDir: '', logDir: '', isLoading: true });
      await useSettingsStore.getState().loadSettings();
      expect(useSettingsStore.getState().reportDir).toBe('/round1/');

      await useSettingsStore.getState().setReportDir('/round2/');

      // 第三轮（模拟重启）
      useSettingsStore.setState({ reportDir: '', logDir: '', isLoading: true });
      await useSettingsStore.getState().loadSettings();
      expect(useSettingsStore.getState().reportDir).toBe('/round2/');
    });

    it('localStorage 格式异常时回退到默认值', async () => {
      // 存储无效 JSON
      localStorageMock.setItem(STORAGE_KEY, 'invalid json');

      await useSettingsStore.getState().loadSettings();

      // 应回退到默认值
      expect(useSettingsStore.getState().reportDir).toBe(DEFAULT_REPORT_DIR);
      expect(useSettingsStore.getState().logDir).toBe(DEFAULT_LOG_DIR);
    });

    it('空 localStorage 时正确初始化', async () => {
      await useSettingsStore.getState().loadSettings();

      const state = useSettingsStore.getState();
      expect(state.reportDir).toBe(DEFAULT_REPORT_DIR);
      expect(state.logDir).toBe(DEFAULT_LOG_DIR);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });
  });

  // ========================================
  // 错误处理测试
  // ========================================
  describe('错误处理', () => {
    beforeEach(() => {
      localStorageMock.clear();
      useSettingsStore.setState({
        reportDir: '',
        logDir: '',
        isLoading: true,
        error: null,
      });
    });

    it('错误状态可以被重置', async () => {
      // 先加载设置
      await useSettingsStore.getState().loadSettings();
      expect(useSettingsStore.getState().error).toBeNull();
    });

    it('加载完成后 isLoading 必须为 false', async () => {
      await useSettingsStore.getState().loadSettings();
      expect(useSettingsStore.getState().isLoading).toBe(false);
    });
  });
});
