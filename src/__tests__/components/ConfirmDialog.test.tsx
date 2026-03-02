/**
 * ConfirmDialog 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ConfirmDialog } from '@/components/dialogs/ConfirmDialog';

describe('ConfirmDialog', () => {
  const mockOnConfirm = vi.fn();
  const mockOnCancel = vi.fn();

  const defaultProps = {
    open: true,
    title: '删除确认',
    onConfirm: mockOnConfirm,
    onCancel: mockOnCancel,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('当 open=false 时不渲染对话框内容', () => {
    render(<ConfirmDialog {...defaultProps} open={false} />);

    // Dialog 关闭时不应该显示标题
    expect(screen.queryByText('删除确认')).toBeNull();
  });

  it('对话框打开时显示标题', () => {
    render(<ConfirmDialog {...defaultProps} />);

    expect(screen.getByText('删除确认')).toBeDefined();
  });

  it('显示可选的描述文本', () => {
    render(
      <ConfirmDialog
        {...defaultProps}
        description="确定要删除此项目吗？删除后无法恢复。"
      />
    );

    expect(screen.getByText('确定要删除此项目吗？删除后无法恢复。')).toBeDefined();
  });

  it('没有描述时不显示描述区域', () => {
    render(<ConfirmDialog {...defaultProps} />);

    // 默认不传 description，不应显示描述
    expect(screen.queryByText(/确定要删除/)).toBeNull();
  });

  it('显示默认的确认和取消按钮文本', () => {
    render(<ConfirmDialog {...defaultProps} />);

    expect(screen.getByRole('button', { name: '取消' })).toBeDefined();
    expect(screen.getByRole('button', { name: '确认' })).toBeDefined();
  });

  it('支持自定义按钮文本', () => {
    render(
      <ConfirmDialog
        {...defaultProps}
        confirmText="删除"
        cancelText="返回"
      />
    );

    expect(screen.getByRole('button', { name: '返回' })).toBeDefined();
    expect(screen.getByRole('button', { name: '删除' })).toBeDefined();
  });

  it('点击取消按钮调用 onCancel', async () => {
    const user = userEvent.setup();
    render(<ConfirmDialog {...defaultProps} />);

    await user.click(screen.getByRole('button', { name: '取消' }));
    expect(mockOnCancel).toHaveBeenCalledTimes(1);
  });

  it('点击确认按钮调用 onConfirm', async () => {
    const user = userEvent.setup();
    render(<ConfirmDialog {...defaultProps} />);

    await user.click(screen.getByRole('button', { name: '确认' }));
    expect(mockOnConfirm).toHaveBeenCalledTimes(1);
  });

  it('加载中时禁用按钮并显示加载文本', () => {
    render(<ConfirmDialog {...defaultProps} loading={true} />);

    const confirmButton = screen.getByRole('button', { name: '处理中...' });
    const cancelButton = screen.getByRole('button', { name: '取消' });

    expect(confirmButton).toHaveProperty('disabled', true);
    expect(cancelButton).toHaveProperty('disabled', true);
  });

  it('加载中时不显示默认确认文本', () => {
    render(<ConfirmDialog {...defaultProps} loading={true} />);

    // 加载中时显示"处理中..."，不显示"确认"
    expect(screen.queryByRole('button', { name: '确认' })).toBeNull();
    expect(screen.getByRole('button', { name: '处理中...' })).toBeDefined();
  });

  it('点击确认按钮后异步回调正常执行', async () => {
    const user = userEvent.setup();
    const asyncOnConfirm = vi.fn().mockResolvedValue(undefined);

    render(
      <ConfirmDialog
        {...defaultProps}
        onConfirm={asyncOnConfirm}
      />
    );

    await user.click(screen.getByRole('button', { name: '确认' }));

    await waitFor(() => {
      expect(asyncOnConfirm).toHaveBeenCalledTimes(1);
    });
  });
});
