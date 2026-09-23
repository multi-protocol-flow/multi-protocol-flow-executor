import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { Switch } from '@/components/ui/switch';

describe('Switch 组件', () => {
  it('正确渲染未选中状态', () => {
    render(<Switch data-testid="switch" checked={false} onCheckedChange={() => {}} />);
    const input = screen.getByRole('checkbox');
    expect(input).not.toBeChecked();
  });

  it('正确渲染选中状态', () => {
    render(<Switch data-testid="switch" checked={true} onCheckedChange={() => {}} />);
    const input = screen.getByRole('checkbox');
    expect(input).toBeChecked();
  });

  it('点击时触发 onCheckedChange 回调', async () => {
    const user = userEvent.setup();
    const onCheckedChange = vi.fn();
    render(<Switch checked={false} onCheckedChange={onCheckedChange} />);
    const input = screen.getByRole('checkbox');

    await user.click(input);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it('禁用状态下不触发变更且有禁用样式', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(
      <Switch checked={false} disabled onCheckedChange={onCheckedChange} />
    );
    const label = container.querySelector('label');
    expect(label).toHaveClass('cursor-not-allowed', 'opacity-50');

    const input = screen.getByRole('checkbox');
    expect(input).toBeDisabled();

    fireEvent.click(input);
    expect(onCheckedChange).not.toHaveBeenCalled();
  });

  it('支持不同变体 (default, emerald, amber)', () => {
    const { container, rerender } = render(
      <Switch checked={true} variant="default" onCheckedChange={() => {}} />
    );
    let track = container.querySelector('div');
    expect(track?.className).toContain('peer-checked:bg-primary');
    expect(track?.className).toContain('dark:peer-checked:after:bg-zinc-950');

    rerender(<Switch checked={true} variant="emerald" onCheckedChange={() => {}} />);
    track = container.querySelector('div');
    expect(track?.className).toContain('peer-checked:bg-emerald-600');
    expect(track?.className).toContain('peer-checked:after:bg-white');

    rerender(<Switch checked={true} variant="amber" onCheckedChange={() => {}} />);
    track = container.querySelector('div');
    expect(track?.className).toContain('peer-checked:bg-amber-500');
  });
});
