/**
 * 压力测试状态管理
 *
 * 管理压力测试的配置、进度和报告状态
 */
import { create } from 'zustand';
import type {
  StressTestStatus,
  StressTestConfig,
  StressTestProgress,
  StressTestReport,
  StressTestEvent,
  StressTestReportMetadata,
} from '@/types/stressTest';
import type { Flow } from '@/types/flow';
import { startStressTest as startStressTestService, stopStressTest as stopStressTestService } from '@/services/tauri';
import { stressTestStorage } from '@/services/storage/stressTestStorage';
import { useSettingsStore } from './settings';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

const ll = getLL();

/** 当前压测事件监听器的清理函数（模块级，与组件生命周期解耦） */
let currentStressUnlisten: (() => void) | null = null;

/** 清理当前事件监听器 */
function cleanupStressListener() {
  if (currentStressUnlisten) {
    currentStressUnlisten();
    currentStressUnlisten = null;
  }
}

// ============================================================================
// 类型定义
// ============================================================================

interface StressTestState {
  // ===== State =====
  /** 压力测试状态 */
  status: StressTestStatus;
  /** 压力测试配置 */
  config: StressTestConfig | null;
  /** 压力测试进度 */
  progress: StressTestProgress | null;
  /** 进度历史数据（用于图表渲染） */
  progressHistory: StressTestProgress[];
  /** 压力测试报告 */
  report: StressTestReport | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 报告 Dialog 是否打开 */
  isReportDialogOpen: boolean;
  /** 压测报告历史列表 */
  history: StressTestReportMetadata[];
  /** 是否正在加载历史 */
  isLoadingHistory: boolean;

  // ===== Actions =====
  /** 启动压力测试 */
  startStressTest: (flow: Flow, config: StressTestConfig) => Promise<void>;
  /** 停止压力测试 */
  stopStressTest: () => Promise<void>;
  /** 重置状态 */
  reset: () => void;
  /** 打开报告 Dialog */
  openReportDialog: () => void;
  /** 关闭报告 Dialog（不清除报告数据） */
  closeReportDialog: () => void;
  /** 从文件加载压测报告并打开弹窗 */
  loadAndOpenReport: (filePath: string) => Promise<void>;
  /** 加载压测报告历史列表 */
  loadHistory: () => Promise<void>;

  // ===== Event handlers (internal) =====
  /** 处理压力测试事件 */
  handleStressTestEvent: (event: StressTestEvent) => Promise<void>;
}

// ============================================================================
// 初始状态
// ============================================================================

const initialState = {
  status: 'idle' as StressTestStatus,
  config: null as StressTestConfig | null,
  progress: null as StressTestProgress | null,
  progressHistory: [] as StressTestProgress[],
  report: null as StressTestReport | null,
  isLoading: false,
  error: null as string | null,
  isReportDialogOpen: false,
  history: [] as StressTestReportMetadata[],
  isLoadingHistory: false,
};

// ============================================================================
// Store 实现
// ============================================================================

export const useStressTestStore = create<StressTestState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  startStressTest: async (flow: Flow, config: StressTestConfig) => {
    try {
      // 清理之前残留的监听器
      cleanupStressListener();

      // 设置加载状态，清除之前的错误和报告
      set({
        isLoading: true,
        error: null,
        report: null,
        progress: null,
        progressHistory: [],
      });

      // 调用 IPC 服务，传入事件处理器回调，返回 unlisten 函数
      const unlisten = await startStressTestService(flow, config, (event) => {
        get().handleStressTestEvent(event);
      });
      currentStressUnlisten = unlisten;

      // IPC 调用返回后设置 isLoading 为 false
      // 注意：状态变化由事件处理器负责
      set({ isLoading: false });
    } catch (error) {
      const errorMsg = (error as Error).message;
      set({
        isLoading: false,
        status: 'stopped',
        error: errorMsg,
      });

      // 启动失败时 show toast，让用户感知到错误
      cleanupStressListener();
      toast.error(getLL().execution.toast.flow_failed(), {
        description: errorMsg,
        duration: 8000,
      });
    }
  },

  stopStressTest: async () => {
    try {
      await stopStressTestService();
      // 保持当前状态，'ended' 事件会更新状态为 'stopped'
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  reset: () => {
    cleanupStressListener();
    set(initialState);
  },

  openReportDialog: () => set({ isReportDialogOpen: true }),
  closeReportDialog: () => set({ isReportDialogOpen: false }),

  /** 从文件加载压测报告并打开弹窗 */
  loadAndOpenReport: async (filePath: string) => {
    try {
      const report = await stressTestStorage.loadReport(filePath);
      set({
        report: report as StressTestReport,
        progressHistory: (report as StressTestReport).progressHistory ?? [],
        isReportDialogOpen: true,
      });
    } catch (e) {
      console.error(ll.execution.history.toast_load_report_failed(), e);
      toast.error(getLL().execution.history.toast_load_report_failed(), {
        description: (e as Error).message,
        duration: 8000,
      });
    }
  },

  loadHistory: async () => {
    set({ isLoadingHistory: true });
    try {
      const reportsDir = useSettingsStore.getState().reportDir;
      if (reportsDir) {
        const history = await stressTestStorage.listReports(reportsDir);
        set({ history, isLoadingHistory: false });
      } else {
        set({ isLoadingHistory: false });
      }
    } catch (e) {
      set({ isLoadingHistory: false });
    }
  },

  // ===== Event handlers =====
  handleStressTestEvent: async (event: StressTestEvent) => {
    if (event.type === 'started') {
      // 压测开始事件：设置状态为爬升阶段，存储配置
      set({
        status: 'rampUp',
        config: event.config,
        error: null,
      });
    } else if (event.type === 'progress') {
      // 进度更新事件：更新状态和进度信息，累积完整历史数据（安全上限 36000 点/10小时）
      set((state) => ({
        status: event.progress.status,
        progress: event.progress,
        progressHistory: state.progressHistory.length >= 36000
          ? [...state.progressHistory.slice(1), event.progress]
          : [...state.progressHistory, event.progress],
      }));
    } else if (event.type === 'ended') {
      // 压测结束事件：设置状态为已停止，存储报告
      set({
        status: 'stopped',
        report: event.report,
        progress: null,
      });

      // 清理事件监听器
      cleanupStressListener();

      // 自动保存压测报告
      try {
        const reportsDir = useSettingsStore.getState().reportDir;
        if (reportsDir) {
          await stressTestStorage.saveReport(event.report, reportsDir);
          // 重新加载历史列表
          const history = await stressTestStorage.listReports(reportsDir);
          set({ history });
        }
      } catch (e) {
        console.error(ll.execution.toast.report_save_failed(), e);
      }

      // 显示 toast 通知
      const successRate = event.report.statistics.successRate;
      const toastFn = successRate < 0.99 ? toast.error : toast.success;
      toastFn(ll.execution.toast.stress_completed(), {
        description: `${event.report.statistics.totalIterations} ${ll.execution.toast.iterations()} · ${ll.execution.toast.success_rate()} ${(successRate * 100).toFixed(1)}%`,
        action: { label: ll.execution.toast.view_report(), onClick: () => get().openReportDialog() },
        duration: 10000,
      });
    }
  },
}));

// ============================================================================
// Selectors
// ============================================================================

/** 选择压力测试状态 */
export const selectStressTestStatus = (state: StressTestState) => state.status;

/** 选择压力测试进度 */
export const selectStressTestProgress = (state: StressTestState) => state.progress;

/** 选择压力测试报告 */
export const selectStressTestReport = (state: StressTestState) => state.report;

/** 选择压力测试是否正在运行（爬升或运行中） */
export const selectIsStressTestRunning = (state: StressTestState) =>
  state.status === 'rampUp' || state.status === 'running';

/** 选择压力测试错误信息 */
export const selectStressTestError = (state: StressTestState) => state.error;

/** 选择压力测试进度历史（用于图表） */
export const selectStressTestProgressHistory = (state: StressTestState) => state.progressHistory;

/** 选择压测报告历史列表 */
export const selectStressTestHistory = (state: StressTestState) => state.history;
