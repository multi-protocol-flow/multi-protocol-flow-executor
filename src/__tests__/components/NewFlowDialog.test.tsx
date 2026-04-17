/**
 * NewFlowDialog 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { useUIStore, useFlowStore, useFlowListStore } from '@/stores';
import * as tauriService from '@/services/tauri';

// Mock stores
vi.mock('@/stores', () => ({
  useUIStore: vi.fn(),
  useFlowStore: vi.fn(),
  useFlowListStore: vi.fn(),
}));

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  createFlow: vi.fn(),
}));

// Mock flowStorage (now async IPC-based)
vi.mock('@/services/storage', () => ({
  flowStorage: {
    saveFlow: vi.fn().mockResolvedValue(undefined),
    loadFlow: vi.fn(),
    listFlows: vi.fn(),
    deleteFlow: vi.fn(),
    copyFlow: vi.fn(),
    refreshFlows: vi.fn(),
    listFolders: vi.fn().mockResolvedValue([]),
    moveFlowToFolder: vi.fn().mockResolvedValue(undefined),
  },
}));

describe('NewFlowDialog', () => {
  const mockCloseNewFlowDialog = vi.fn();
  const mockSetFlow = vi.fn();
  const mockAddFlow = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Setup store mocks
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { showNewFlowDialog: boolean; closeNewFlowDialog: typeof mockCloseNewFlowDialog; newFlowDialogFolderPath: string }) => unknown) => {
        const state = { showNewFlowDialog: true, closeNewFlowDialog: mockCloseNewFlowDialog, newFlowDialogFolderPath: '' };
        return selector(state);
      }
    );

    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { setFlow: typeof mockSetFlow }) => unknown) => {
        const state = { setFlow: mockSetFlow };
        return selector(state);
      }
    );

    (useFlowListStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { addFlow: typeof mockAddFlow }) => unknown) => {
        const state = { addFlow: mockAddFlow };
        return selector(state);
      }
    );
  });

  it('当对话框关闭时不渲染', () => {
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { showNewFlowDialog: boolean; closeNewFlowDialog: typeof mockCloseNewFlowDialog; newFlowDialogFolderPath: string }) => unknown) => {
        const state = { showNewFlowDialog: false, closeNewFlowDialog: mockCloseNewFlowDialog, newFlowDialogFolderPath: '' };
        return selector(state);
      }
    );

    const { container } = render(<NewFlowDialog />);
    expect(container.firstChild).toBeNull();
  });

  it('对话框打开时显示标题和输入框', () => {
    render(<NewFlowDialog />);

    expect(screen.getByText('新建流程')).toBeDefined();
    expect(screen.getByPlaceholderText('请输入流程名称')).toBeDefined();
  });

  it('显示确认和取消按钮', () => {
    render(<NewFlowDialog />);

    expect(screen.getByRole('button', { name: '取消' })).toBeDefined();
    expect(screen.getByRole('button', { name: '创建' })).toBeDefined();
  });

  it('点击取消按钮关闭对话框', async () => {
    const user = userEvent.setup();
    render(<NewFlowDialog />);

    await user.click(screen.getByRole('button', { name: '取消' }));
    expect(mockCloseNewFlowDialog).toHaveBeenCalled();
  });

  it('输入流程名称并点击创建', async () => {
    const user = userEvent.setup();
    const mockFlow = {
      uuid: 'flow-123',
      name: '新流程',
      nodes: [],
      connections: [],
      initial_variables: {},
    };

    vi.mocked(tauriService.createFlow).mockResolvedValue(mockFlow);

    render(<NewFlowDialog />);

    const input = screen.getByPlaceholderText('请输入流程名称');
    await user.type(input, '新流程');

    await user.click(screen.getByRole('button', { name: '创建' }));

    await waitFor(() => {
      expect(tauriService.createFlow).toHaveBeenCalledWith('新流程');
      expect(mockSetFlow).toHaveBeenCalled();
      expect(mockAddFlow).toHaveBeenCalled();
      expect(mockCloseNewFlowDialog).toHaveBeenCalled();
    });
  });

  it('空名称时创建按钮禁用', () => {
    render(<NewFlowDialog />);

    const createButton = screen.getByRole('button', { name: '创建' });
    expect(createButton).toHaveProperty('disabled', true);
  });
});
