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
} from '@/types/stressTest';
import type { Flow } from '@/types/flow';
import { startStressTest as startStressTestService, stopStressTest as stopStressTestService } from '@/services/tauri';
import { toast } from 'sonner';

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
  /** 压力测试报告 */
  report: StressTestReport | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 报告 Dialog 是否打开 */
  isReportDialogOpen: boolean;

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

  // ===== Event handlers (internal) =====
  /** 处理压力测试事件 */
  handleStressTestEvent: (event: StressTestEvent) => void;
}

// ============================================================================
// 初始状态
// ============================================================================

const initialState = {
  status: 'idle' as StressTestStatus,
  config: null as StressTestConfig | null,
  progress: null as StressTestProgress | null,
  report: null as StressTestReport | null,
  isLoading: false,
  error: null as string | null,
  isReportDialogOpen: false,
};

// ============================================================================
// Store 实现
// ============================================================================

export const useStressTestStore = create<StressTestState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  startStressTest: async (flow: Flow, config: StressTestConfig) => {
    try {
      // 设置加载状态，清除之前的错误和报告
      set({
        isLoading: true,
        error: null,
        report: null,
        progress: null,
      });

      // 调用 IPC 服务，传入事件处理器回调
      await startStressTestService(flow, config, (event) => {
        get().handleStressTestEvent(event);
      });

      // IPC 调用返回后设置 isLoading 为 false
      // 注意：状态变化由事件处理器负责
      set({ isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        status: 'stopped',
        error: (error as Error).message,
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
    set(initialState);
  },

  openReportDialog: () => set({ isReportDialogOpen: true }),
  closeReportDialog: () => set({ isReportDialogOpen: false }),

  // ===== Event handlers =====
  handleStressTestEvent: (event: StressTestEvent) => {
    if (event.type === 'started') {
      // 压测开始事件：设置状态为爬升阶段，存储配置
      set({
        status: 'rampUp',
        config: event.config,
        error: null,
      });
    } else if (event.type === 'progress') {
      // 进度更新事件：更新状态和进度信息
      set({
        status: event.progress.status,
        progress: event.progress,
      });
    } else if (event.type === 'ended') {
      // 压测结束事件：设置状态为已停止，存储报告
      set({
        status: 'stopped',
        report: event.report,
        progress: null,
      });

      // 显示 toast 通知
      const successRate = event.report.statistics.successRate;
      const toastFn = successRate < 0.99 ? toast.error : toast.success;
      toastFn('压力测试完成', {
        description: `${event.report.statistics.totalIterations} 次迭代 · 成功率 ${(successRate * 100).toFixed(1)}%`,
        action: { label: '查看报告', onClick: () => get().openReportDialog() },
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
