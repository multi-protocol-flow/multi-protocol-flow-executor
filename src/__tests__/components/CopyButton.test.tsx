/**
 * CopyButton 组件测试
 */
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { CopyButton } from '@/components/common/CopyButton';
import * as clipboard from '@/lib/clipboard';

describe('CopyButton', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('正常渲染按钮和标签', () => {
    render(<CopyButton value="test-content" label="复制" />);

    const button = screen.getByRole('button', { name: '复制' });
    expect(button).toBeInTheDocument();
  });

  it('点击按钮复制字符串内容', async () => {
    const copySpy = vi.spyOn(clipboard, 'copyToClipboard').mockResolvedValue(true);

    render(<CopyButton value="hello world" label="复制文本" />);

    const button = screen.getByRole('button', { name: '复制文本' });
    await act(async () => {
      fireEvent.click(button);
    });

    expect(copySpy).toHaveBeenCalledWith('hello world');
  });

  it('点击按钮以 JSON 格式复制对象', async () => {
    const copySpy = vi.spyOn(clipboard, 'copyToClipboard').mockResolvedValue(true);
    const data = { user: 'Alice', age: 30 };

    render(<CopyButton value={data} format="json" label="复制 JSON" />);

    const button = screen.getByRole('button', { name: '复制 JSON' });
    await act(async () => {
      fireEvent.click(button);
    });

    expect(copySpy).toHaveBeenCalledWith(JSON.stringify(data, null, 2));
  });

  it('支持函数懒求值', async () => {
    const copySpy = vi.spyOn(clipboard, 'copyToClipboard').mockResolvedValue(true);
    const getValue = vi.fn(() => 'dynamic-value');

    render(<CopyButton value={getValue} label="复制动态内容" />);

    const button = screen.getByRole('button', { name: '复制动态内容' });
    await act(async () => {
      fireEvent.click(button);
    });

    expect(getValue).toHaveBeenCalled();
    expect(copySpy).toHaveBeenCalledWith('dynamic-value');
  });

  it('复制成功后切换标签与成功状态', async () => {
    vi.spyOn(clipboard, 'copyToClipboard').mockResolvedValue(true);

    render(
      <CopyButton
        value="success-test"
        label="点击复制"
        successLabel="复制完毕"
      />
    );

    const button = screen.getByRole('button', { name: '点击复制' });
    await act(async () => {
      fireEvent.click(button);
    });

    await waitFor(() => {
      expect(screen.getByText('复制完毕')).toBeInTheDocument();
    });
  });

  it('阻止冒泡默认有效', async () => {
    vi.spyOn(clipboard, 'copyToClipboard').mockResolvedValue(true);
    const parentClick = vi.fn();

    render(
      <div onClick={parentClick}>
        <CopyButton value="bubble-test" label="测试冒泡" />
      </div>
    );

    const button = screen.getByRole('button', { name: '测试冒泡' });
    await act(async () => {
      fireEvent.click(button);
    });

    expect(parentClick).not.toHaveBeenCalled();
  });
});
