/**
 * ExecuteButton 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ExecuteButton } from '@/components/execution/ExecuteButton';
import { useFlowStore, useExecutionStore, useUIStore } from '@/stores';

// Mock stores
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
  useExecutionStore: vi.fn(),
  useUIStore: vi.fn(),
}));

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  executeFlow: vi.fn(),
}));

describe('ExecuteButton', () => {
  const mockSetReport = vi.fn();
  const mockSetError = vi.fn();
  const mockSetConfigPanelOpen = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks for disabled state (no flow)
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: null }) => unknown) => selector({ flow: null })
    );
    (useExecutionStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) => 
        selector({ setReport: mockSetReport, setError: mockSetError })
    );
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) => 
        selector({ setConfigPanelOpen: mockSetConfigPanelOpen })
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
    expect(button.className).toContain('disabled:from-slate-400');
    expect(button.className).toContain('disabled:to-slate-500');

    // 验证禁用状态有可见的文字颜色
    expect(button.className).toMatch(/disabled:text-(slate|white)/);
  });

  it('当有有效流程和入口节点时按钮可用', () => {
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

    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow })
    );

    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });
    expect(button).toHaveProperty('disabled', false);
  });

  it('流程没有入口节点时按钮禁用', () => {
    const mockFlow = {
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
      (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow })
    );

    render(<ExecuteButton />);
    const button = screen.getByRole('button', { name: /执行流程/ });
    expect(button).toHaveProperty('disabled', true);
  });
});
