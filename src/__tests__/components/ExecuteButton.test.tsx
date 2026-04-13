/**
 * ExecuteButton 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { toast } from 'sonner';
import { ExecuteButton } from '@/components/execution/ExecuteButton';
import { useFlowStore, useExecutionStore, useUIStore } from '@/stores';
import { executeFlowWithChannel, saveFlowReport } from '@/services/tauri';

// Mock stores
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
  useExecutionStore: vi.fn(),
  useUIStore: vi.fn(),
}));

// Mock sonner toast - must define fn inside factory due to hoisting
vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

// Mock console store
vi.mock('@/stores/console', () => ({
  useConsoleStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({ addLog: vi.fn(), clearLogs: vi.fn() }),
}));

// Mock settings store
vi.mock('@/stores/settings', () => ({
  useSettingsStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({ reportDir: '/tmp/reports' }),
}));

// Mock tauri service - must define fn inside factory due to hoisting
vi.mock('@/services/tauri', () => ({
  executeFlowWithChannel: vi.fn(),
  saveFlowReport: vi.fn(),
}));

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Play: () => null,
  Loader2: () => null,
  Zap: () => null,
}));

describe('ExecuteButton', () => {
  const mockSetReport = vi.fn();
  const mockSetError = vi.fn();
  const mockSetConfigPanelOpen = vi.fn();
  const mockSetExecutionPanelOpen = vi.fn();
  const mockClearExecutionState = vi.fn();
  const mockOpenReportDialog = vi.fn();
  const mockHandleExecutionEvent = vi.fn();

  const mockFlow = {
    uuid: 'test-flow',
    name: '测试流程',
    nodes: [
      {
        uuid: 'entry-1',
        type: 'entry' as const,
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' as const },
      },
    ],
    connections: [],
    initial_variables: {},
  };

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks for disabled state (no flow)
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: null }) => unknown) => selector({ flow: null })
    );
    (useExecutionStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) => 
        selector({
          setReport: mockSetReport,
          setError: mockSetError,
          clearExecutionState: mockClearExecutionState,
        })
    );
    // Also mock getState for useExecutionStore
    (useExecutionStore as unknown as ReturnType<typeof vi.fn> & { getState: ReturnType<typeof vi.fn> }).getState = vi.fn(() => ({
      handleExecutionEvent: mockHandleExecutionEvent,
      openReportDialog: mockOpenReportDialog,
    }));
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) => 
        selector({ setConfigPanelOpen: mockSetConfigPanelOpen, setExecutionPanelOpen: mockSetExecutionPanelOpen })
    );
  });

  it('当没有流程时按钮被禁用', () => {
    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });
    expect(button).toHaveProperty('disabled', true);
  });

  it('禁用状态下有可见的文字样式', () => {
    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });

    // 验证禁用状态的背景样式
    expect(button.className).toContain('disabled:bg-muted');
    expect(button.className).toContain('disabled:border-border');

    // 验证禁用状态有可见的文字颜色
    expect(button.className).toContain('disabled:text-muted-foreground');
  });

  it('当有有效流程和入口节点时按钮可用', () => {
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow })
    );

    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });
    expect(button).toHaveProperty('disabled', false);
  });

  it('流程没有入口节点时按钮禁用', () => {
    const noEntryFlow = {
      uuid: 'test-flow',
      name: '测试流程',
      nodes: [
        {
          uuid: 'http-1',
          type: 'http' as const,
          name: 'HTTP',
          position: { x: 0, y: 0 },
          data: { type: 'http' as const, url: '', method: 'get' as const },
        },
      ],
      connections: [],
      initial_variables: {},
    };

    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: typeof noEntryFlow }) => unknown) => selector({ flow: noEntryFlow })
    );

    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });
    expect(button).toHaveProperty('disabled', true);
  });

  describe('toast 通知', () => {
    const successResult = {
      success: true,
      flow_report: {
        flow_name: '测试流程',
        duration_ms: 1500,
        statistics: { success_count: 3, failed_count: 0 },
      },
    };

    const failedResult = {
      success: false,
      error: '连接超时',
    };

    beforeEach(() => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow })
      );
    });

    it('执行成功时调用 toast.success 并包含报告信息', async () => {
      (executeFlowWithChannel as ReturnType<typeof vi.fn>).mockResolvedValue(successResult);
      (saveFlowReport as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      render(<ExecuteButton />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalledWith('执行完成', {
          description: '测试流程 · 耗时 1500ms · 3 成功 · 0 失败',
          action: {
            label: '查看报告',
            onClick: expect.any(Function),
          },
          duration: 10000,
        });
      });
    });

    it('toast.success action 的 onClick 调用 openReportDialog', async () => {
      (executeFlowWithChannel as ReturnType<typeof vi.fn>).mockResolvedValue(successResult);
      (saveFlowReport as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      render(<ExecuteButton />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalled();
      });

      const callArgs = (toast.success as ReturnType<typeof vi.fn>).mock.calls[0][1];
      callArgs.action.onClick();

      expect(mockOpenReportDialog).toHaveBeenCalled();
    });

    it('执行失败（result.success=false）时调用 toast.error', async () => {
      (executeFlowWithChannel as ReturnType<typeof vi.fn>).mockResolvedValue(failedResult);

      render(<ExecuteButton />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('执行失败', {
          description: '连接超时',
          duration: 10000,
        });
      });
    });

    it('执行异常（throw）时调用 toast.error', async () => {
      (executeFlowWithChannel as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('网络错误'));

      render(<ExecuteButton />);
      const button = screen.getByRole('button', { name: /执行流程/ });
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('执行失败', {
          description: '网络错误',
          duration: 10000,
        });
      });
    });
  });
});
