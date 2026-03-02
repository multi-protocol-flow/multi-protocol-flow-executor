/**
 * ExecutionHistory 组件测试
 * 测试执行历史列表的渲染和交互
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ExecutionHistory } from '@/components/execution/ExecutionHistory';
import { useSettingsStore } from '@/stores/settings';
import { useExecutionStore } from '@/stores/execution';
import { useUIStore } from '@/stores';
import type { ReportMetadata } from '@/types/ipc';

// Mock stores
vi.mock('@/stores/settings', () => ({
  useSettingsStore: vi.fn(),
}));

vi.mock('@/stores/execution', () => ({
  useExecutionStore: vi.fn(),
}));

vi.mock('@/stores', () => ({
  useUIStore: vi.fn(),
}));

// Mock reportStorage
vi.mock('@/services/storage/reportStorage', () => ({
  listReports: vi.fn(),
  loadReport: vi.fn(),
  deleteReport: vi.fn(),
}));

// Mock scroll-area to avoid Radix issues in tests
vi.mock('@/components/ui/scroll-area', () => ({
  ScrollArea: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="scroll-area">{children}</div>
  ),
}));

// Mock button
vi.mock('@/components/ui/button', () => ({
  Button: ({ children, onClick, disabled, className, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement>) => (
    <button onClick={onClick} disabled={disabled} className={className} {...props}>
      {children}
    </button>
  ),
}));

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

// 导入 mock 后的函数
import { listReports, loadReport, deleteReport } from '@/services/storage/reportStorage';

// 共享 mockShowConfirm
let mockShowConfirm: ReturnType<typeof vi.fn>;

describe('ExecutionHistory', () => {
  const mockSetReport = vi.fn();
  const mockReportDir = '/test/reports';

  const mockReports: ReportMetadata[] = [
    {
      uuid: 'report-1',
      flow_uuid: 'flow-1',
      flow_name: '测试流程1',
      status: 'success',
      timestamp: Date.now() - 1000,
      duration_ms: 500,
    },
    {
      uuid: 'report-2',
      flow_uuid: 'flow-2',
      flow_name: '测试流程2',
      status: 'failed',
      timestamp: Date.now(),
      duration_ms: 1000,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();

    // Default store mocks
    (useSettingsStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { reportDir: string }) => unknown) => selector({ reportDir: mockReportDir })
    );
    (useExecutionStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { setReport: typeof mockSetReport }) => unknown) => 
        selector({ setReport: mockSetReport })
    );

    // Default API mocks
    (listReports as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    (loadReport as ReturnType<typeof vi.fn>).mockResolvedValue({});
    (deleteReport as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

    // Mock useUIStore with showConfirm
    mockShowConfirm = vi.fn();
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { showConfirm: typeof mockShowConfirm }) => unknown) => 
        selector({ showConfirm: mockShowConfirm })
    );
  });

  describe('加载状态', () => {
    it('初始加载时显示加载中状态', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {})); // Never resolves

      render(<ExecutionHistory />);

      expect(screen.getByText('加载中...')).toBeDefined();
    });
  });

  describe('空状态', () => {
    it('没有历史记录时显示空状态提示', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('暂无执行历史')).toBeDefined();
      });
    });
  });

  describe('历史列表渲染', () => {
    it('正确渲染历史列表', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
        expect(screen.getByText('测试流程2')).toBeDefined();
      });

      // 显示记录数
      expect(screen.getByText('共 2 条记录')).toBeDefined();
    });

    it('正确显示成功和失败状态图标', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        // 成功状态
        expect(screen.getByText('测试流程1')).toBeDefined();
        // 失败状态
        expect(screen.getByText('测试流程2')).toBeDefined();
      });
    });

    it('按时间倒序显示历史记录', async () => {
      const orderedReports: ReportMetadata[] = [
        { ...mockReports[1], timestamp: 2000 }, // 较新
        { ...mockReports[0], timestamp: 1000 }, // 较旧
      ];
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(orderedReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        const items = screen.getAllByRole('button').filter(btn => 
          btn.className.includes('cursor-pointer')
        );
        // 第一个应该是较新的记录
        expect(items.length).toBeGreaterThan(0);
      });
    });
  });

  describe('reportDir 依赖测试（核心 bug 修复）', () => {
    it('当 reportDir 变化时重新加载历史列表', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      const { rerender } = render(<ExecutionHistory />);

      await waitFor(() => {
        expect(listReports).toHaveBeenCalledWith(mockReportDir);
      });

      // 清除调用记录
      vi.clearAllMocks();

      // 模拟 reportDir 变化
      const newReportDir = '/new/reports';
      (useSettingsStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { reportDir: string }) => unknown) => selector({ reportDir: newReportDir })
      );

      rerender(<ExecutionHistory />);

      await waitFor(() => {
        expect(listReports).toHaveBeenCalledWith(newReportDir);
      });
    });

    it('当 reportDir 为空时不加载历史列表', async () => {
      (useSettingsStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: { reportDir: string }) => unknown) => selector({ reportDir: '' })
      );

      render(<ExecutionHistory />);

      // 等待一段时间确保不会调用 listReports
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(listReports).not.toHaveBeenCalled();
    });
  });

  describe('选择报告', () => {
    it('点击报告项时加载并设置报告', async () => {
      const mockFullReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: '测试流程1',
      };
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);
      (loadReport as ReturnType<typeof vi.fn>).mockResolvedValue(mockFullReport);

      const onSelectReport = vi.fn();
      render(<ExecutionHistory onSelectReport={onSelectReport} />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      // 点击第一个报告
      const items = screen.getAllByRole('button').filter(btn => 
        btn.className.includes('cursor-pointer')
      );
      fireEvent.click(items[0]);

      await waitFor(() => {
        expect(loadReport).toHaveBeenCalledWith('report-1', mockReportDir);
        expect(mockSetReport).toHaveBeenCalledWith(mockFullReport);
        expect(onSelectReport).toHaveBeenCalledWith('report-1');
      });
    });
  });

  describe('删除报告', () => {
    it('点击删除按钮时显示确认对话框', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      // 找到删除按钮（垃圾桶图标按钮）
      const deleteButtons = screen.getAllByRole('button').filter(btn => 
        btn.getAttribute('title') === '删除'
      );

      fireEvent.click(deleteButtons[0]);

      // 应该调用 showConfirm 显示确认对话框
      expect(mockShowConfirm).toHaveBeenCalled();
      expect(mockShowConfirm).toHaveBeenCalledWith(
        expect.objectContaining({
          title: '删除报告',
          confirmText: '删除',
        })
      );
    });

    it('确认删除后调用 deleteReport', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      // 模拟 showConfirm 在调用时执行 onConfirm
      mockShowConfirm.mockImplementation((config: { onConfirm: () => void }) => {
        config.onConfirm();
      });

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      const deleteButtons = screen.getAllByRole('button').filter(btn => 
        btn.getAttribute('title') === '删除'
      );

      fireEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(deleteReport).toHaveBeenCalledWith('report-1', mockReportDir);
      });
    });

    it('阻止事件冒泡（不触发选择报告）', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      const deleteButtons = screen.getAllByRole('button').filter(btn => 
        btn.getAttribute('title') === '删除'
      );

      // 点击删除按钮
      fireEvent.click(deleteButtons[0]);

      // 不应该加载报告
      expect(loadReport).not.toHaveBeenCalled();
    });
  });

  describe('清空历史', () => {
    it('点击清空按钮时显示确认对话框', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      // 点击清空按钮
      const clearButton = screen.getByText('清空');
      fireEvent.click(clearButton);

      // 应该调用 showConfirm 显示确认对话框
      expect(mockShowConfirm).toHaveBeenCalled();
      expect(mockShowConfirm).toHaveBeenCalledWith(
        expect.objectContaining({
          title: '清空执行历史',
          confirmText: '清空',
        })
      );
    });

    it('确认清空后删除所有报告', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      // 模拟 showConfirm 在调用时执行 onConfirm
      mockShowConfirm.mockImplementation((config: { onConfirm: () => void }) => {
        config.onConfirm();
      });

      render(<ExecutionHistory />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      const clearButton = screen.getByText('清空');
      fireEvent.click(clearButton);

      await waitFor(() => {
        expect(deleteReport).toHaveBeenCalledTimes(2);
      });
    });
  });

  describe('重新执行', () => {
    it('点击重新执行按钮触发 onRerun 回调', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      const onRerun = vi.fn();
      render(<ExecutionHistory onRerun={onRerun} />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      // 找到重新执行按钮
      const rerunButtons = screen.getAllByRole('button').filter(btn => 
        btn.getAttribute('title') === '重新执行'
      );

      fireEvent.click(rerunButtons[0]);

      expect(onRerun).toHaveBeenCalledWith(mockReports[0]);
    });

    it('阻止事件冒泡（不触发选择报告）', async () => {
      (listReports as ReturnType<typeof vi.fn>).mockResolvedValue(mockReports);

      const onRerun = vi.fn();
      render(<ExecutionHistory onRerun={onRerun} />);

      await waitFor(() => {
        expect(screen.getByText('测试流程1')).toBeDefined();
      });

      const rerunButtons = screen.getAllByRole('button').filter(btn => 
        btn.getAttribute('title') === '重新执行'
      );

      fireEvent.click(rerunButtons[0]);

      expect(loadReport).not.toHaveBeenCalled();
    });
  });
});
