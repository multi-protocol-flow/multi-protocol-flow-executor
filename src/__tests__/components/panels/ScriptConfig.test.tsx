/**
 * ScriptConfig 组件测试
 * - 渲染脚本内容、执行时机、超时时间、内存限制当前值
 * - 编辑字段 + 卸载保存 (onSave)
 * - 执行时机选择即时保存
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ScriptConfig } from '@/components/panels/ScriptConfig';
import type { ScriptNodeConfig } from '@/types';

// jsdom 未实现 Pointer Capture / scrollIntoView，Radix Select 依赖它们
if (typeof Element !== 'undefined') {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => undefined;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
}

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock CodeMirror 编辑器为简单 textarea（jsdom 不支持真实 CodeMirror 布局测量）
vi.mock('@uiw/react-codemirror', () => ({
  default: (props: { value?: string; onChange?: (value: string) => void }) => (
    <textarea
      data-testid="script-editor"
      value={props.value ?? ''}
      onChange={(e) => props.onChange?.(e.target.value)}
    />
  ),
}));

/**
 * 创建基础 ScriptNodeConfig 数据
 */
function createBaseConfig(overrides: Partial<ScriptNodeConfig> = {}): ScriptNodeConfig {
  return {
    type: 'script',
    script: 'console.log("hello");',
    timeout_ms: 5000,
    memory_limit_mb: 10,
    execution_phase: 'post_request',
    ...overrides,
  };
}

describe('ScriptConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染脚本内容', () => {
      render(<ScriptConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const editor = screen.getByTestId('script-editor') as HTMLTextAreaElement;
      expect(editor.value).toBe('console.log("hello");');
    });

    it('渲染执行时机、超时时间和内存限制当前值', () => {
      render(
        <ScriptConfig
          data={createBaseConfig({ timeout_ms: 8000, memory_limit_mb: 20, execution_phase: 'pre_request' })}
          onSave={mockOnSave}
        />,
      );

      // 执行时机 select 显示当前选中项（触发按钮文本）
      const phaseTrigger = screen.getByRole('combobox', { name: '执行时机' });
      expect(phaseTrigger.textContent).toContain('请求前执行 (Pre-Request)');
      const timeoutInput = document.getElementById('timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('8000');
      const memoryInput = document.getElementById('memory_limit_mb') as HTMLInputElement;
      expect(memoryInput?.value).toBe('20');
    });
  });

  describe('编辑与保存', () => {
    it('修改超时时间后卸载保存（onSave 收到新值）', async () => {
      const user = userEvent.setup();
      const { unmount } = render(<ScriptConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const timeoutInput = document.getElementById('timeout_ms') as HTMLInputElement;
      await user.clear(timeoutInput);
      await user.type(timeoutInput, '12000');
      unmount();

      expect(mockOnSave).toHaveBeenCalledTimes(1);
      expect(mockOnSave).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'script',
          script: 'console.log("hello");',
          timeout_ms: 12000,
          execution_phase: 'post_request',
        }),
      );
    });

    it('修改脚本内容后卸载保存', async () => {
      const { unmount } = render(<ScriptConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const editor = screen.getByTestId('script-editor') as HTMLTextAreaElement;
      fireEvent.change(editor, { target: { value: 'const x = 1;' } });
      unmount();

      expect(mockOnSave).toHaveBeenCalledTimes(1);
      expect(mockOnSave).toHaveBeenCalledWith(
        expect.objectContaining({ script: 'const x = 1;' }),
      );
    });

    it('选择执行时机后立即保存（onValueChange 路径）', async () => {
      const user = userEvent.setup();
      render(<ScriptConfig data={createBaseConfig()} onSave={mockOnSave} />);

      await user.click(document.getElementById('execution_phase') as HTMLElement);
      await user.click(screen.getByRole('option', { name: '请求前执行 (Pre-Request)' }));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ execution_phase: 'pre_request' }),
        );
      });
    });
  });
});
