/**
 * BreakpointMarker 组件测试
 */
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { BreakpointMarker } from '@/components/canvas/BreakpointMarker';

describe('BreakpointMarker', () => {
  it('有断点时显示红色实心圆', () => {
    render(
      <BreakpointMarker
        hasBreakpoint={true}
        onToggle={() => {}}
      />
    );

    const button = screen.getByRole('button', { name: '取消断点' });
    expect(button).toBeInTheDocument();
    expect(button).toHaveClass('bg-red-500');
    expect(button).toHaveClass('border-red-600');
  });

  it('无断点时显示灰色空心圆', () => {
    render(
      <BreakpointMarker
        hasBreakpoint={false}
        onToggle={() => {}}
      />
    );

    const button = screen.getByRole('button', { name: '设置断点' });
    expect(button).toBeInTheDocument();
    expect(button).toHaveClass('bg-background');
    expect(button).toHaveClass('border-border');
  });

  it('点击时调用 onToggle 回调', async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <BreakpointMarker
        hasBreakpoint={false}
        onToggle={onToggle}
      />
    );

    const button = screen.getByRole('button');
    await user.click(button);

    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('点击时阻止事件冒泡', async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    const onParentClick = vi.fn();

    render(
      <div onClick={onParentClick}>
        <BreakpointMarker
          hasBreakpoint={false}
          onToggle={onToggle}
        />
      </div>
    );

    const button = screen.getByRole('button');
    await user.click(button);

    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(onParentClick).not.toHaveBeenCalled();
  });

  it('有断点时悬停显示更深的红色', () => {
    render(
      <BreakpointMarker
        hasBreakpoint={true}
        onToggle={() => {}}
      />
    );

    const button = screen.getByRole('button');
    expect(button).toHaveClass('hover:bg-red-600');
    expect(button).toHaveClass('hover:border-red-700');
  });

  it('无断点时悬停显示红色边框', () => {
    render(
      <BreakpointMarker
        hasBreakpoint={false}
        onToggle={() => {}}
      />
    );

    const button = screen.getByRole('button');
    expect(button).toHaveClass('hover:bg-red-50');
    expect(button).toHaveClass('hover:border-red-500');
  });

  it('有正确的 title 属性', () => {
    const { rerender } = render(
      <BreakpointMarker
        hasBreakpoint={true}
        onToggle={() => {}}
      />
    );

    let button = screen.getByRole('button');
    expect(button).toHaveAttribute('title', '点击取消断点');

    rerender(
      <BreakpointMarker
        hasBreakpoint={false}
        onToggle={() => {}}
      />
    );

    button = screen.getByRole('button');
    expect(button).toHaveAttribute('title', '点击设置断点');
  });

  describe('locked 模式', () => {
    it('locked 模式显示红色实心圆（不可点击）', () => {
      render(
        <BreakpointMarker
          hasBreakpoint={true}
          locked
        />
      );

      // locked 模式渲染 div，不是 button
      const marker = screen.getByTitle('入口节点断点（固定）');
      expect(marker).toBeInTheDocument();
      expect(marker).toHaveClass('bg-red-500');
      expect(marker).toHaveClass('border-red-600');
      expect(marker).toHaveClass('pointer-events-none');
      // 不应该是按钮（不可点击）
      expect(marker.tagName).toBe('DIV');
    });

    it('locked 模式没有按钮角色', () => {
      render(
        <BreakpointMarker
          hasBreakpoint={true}
          locked
        />
      );

      expect(screen.queryByRole('button')).not.toBeInTheDocument();
    });

    it('locked 模式忽略 onToggle 回调', async () => {
      const onToggle = vi.fn();
      render(
        <BreakpointMarker
          hasBreakpoint={true}
          locked
          onToggle={onToggle}
        />
      );

      // locked 模式下 pointer-events-none，且是 div 不是 button
      const marker = screen.getByTitle('入口节点断点（固定）');
      expect(marker).toHaveClass('pointer-events-none');
      expect(onToggle).not.toHaveBeenCalled();
    });
  });
});
