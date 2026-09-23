import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { Checkbox } from '@/components/ui/checkbox';

describe('Checkbox 组件', () => {
  it('正确渲染未选中状态', () => {
    render(<Checkbox checked={false} onCheckedChange={() => {}} />);
    const input = screen.getByRole('checkbox');
    expect(input).not.toBeChecked();
  });

  it('正确渲染选中状态', () => {
    render(<Checkbox checked={true} onCheckedChange={() => {}} />);
    const input = screen.getByRole('checkbox');
    expect(input).toBeChecked();
  });

  it('点击时触发 onCheckedChange 回调', async () => {
    const user = userEvent.setup();
    const onCheckedChange = vi.fn();
    render(<Checkbox checked={false} onCheckedChange={onCheckedChange} />);
    const input = screen.getByRole('checkbox');

    await user.click(input);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it('支持半选 indeterminate 状态', () => {
    const { container } = render(
      <Checkbox checked={false} indeterminate onCheckedChange={() => {}} />
    );
    const input = screen.getByRole('checkbox') as HTMLInputElement;
    expect(input.indeterminate).toBe(true);

    const box = container.querySelector('div');
    expect(box?.className).toContain('bg-primary');
  });

  it('禁用状态下不触发变更', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(
      <Checkbox checked={false} disabled onCheckedChange={onCheckedChange} />
    );
    const label = container.querySelector('label');
    expect(label).toHaveClass('cursor-not-allowed', 'opacity-50');

    const input = screen.getByRole('checkbox');
    expect(input).toBeDisabled();

    fireEvent.click(input);
    expect(onCheckedChange).not.toHaveBeenCalled();
  });
});
