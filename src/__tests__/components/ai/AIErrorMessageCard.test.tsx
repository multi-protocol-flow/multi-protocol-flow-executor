import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { AIErrorMessageCard } from '@/components/ai/AIErrorMessageCard';
import { useAIStore } from '@/stores/ai';
import type { AIMessage } from '@/types/ai';

describe('AIErrorMessageCard', () => {
  it('渲染流式传输中断错误，并显示可重试按钮与诊断说明', () => {
    const message: AIMessage = {
      id: 'msg-err-1',
      role: 'assistant',
      content: '[Data stream interrupted] connection reset by peer',
      status: 'error',
      error: '[Data stream interrupted] connection reset by peer',
      timestamp: Date.now(),
    };

    const handleRetry = vi.fn();
    render(<AIErrorMessageCard message={message} onRetry={handleRetry} />);

    // 诊断文案存在
    expect(screen.getByText(/AI 请求异常|AI Request Failed/)).toBeDefined();

    // 重试按钮可点击
    const retryBtn = screen.getByRole('button', { name: /重试|Retry/ });
    expect(retryBtn).toBeDefined();

    fireEvent.click(retryBtn);
    expect(handleRetry).toHaveBeenCalledTimes(1);
  });

  it('401 权限错误时不展示重试按钮', () => {
    const message: AIMessage = {
      id: 'msg-err-2',
      role: 'assistant',
      content: 'HTTP 401: Unauthorized invalid api key',
      status: 'error',
      error: 'HTTP 401: Unauthorized invalid api key',
      timestamp: Date.now(),
    };

    render(<AIErrorMessageCard message={message} />);

    // 不应有重试按钮
    expect(screen.queryByRole('button', { name: /重试|Retry/ })).toBeNull();
  });

  it('点击展开/收起技术错误详情', () => {
    const message: AIMessage = {
      id: 'msg-err-3',
      role: 'assistant',
      content: 'Detailed raw rust reqwest error message chain',
      status: 'error',
      error: 'Detailed raw rust reqwest error message chain',
      timestamp: Date.now(),
    };

    render(<AIErrorMessageCard message={message} />);

    // 默认详情折叠，不直接显示代码块
    expect(screen.queryByText('Detailed raw rust reqwest error message chain')).toBeNull();

    // 点击技术详情展开按钮
    const detailsToggle = screen.getByText(/技术错误详情|Technical Details/);
    fireEvent.click(detailsToggle);

    // 展开后显示
    expect(screen.getByText('Detailed raw rust reqwest error message chain')).toBeDefined();
  });

  it('默认点击重试调用 store 的 retryLastMessage', async () => {
    const retrySpy = vi.spyOn(useAIStore.getState(), 'retryLastMessage').mockResolvedValue();

    const message: AIMessage = {
      id: 'msg-err-4',
      role: 'assistant',
      content: 'timed out',
      status: 'error',
      error: 'timed out',
      timestamp: Date.now(),
    };

    render(<AIErrorMessageCard message={message} />);

    const retryBtn = screen.getByRole('button', { name: /重试|Retry/ });
    await act(async () => {
      fireEvent.click(retryBtn);
    });
    expect(retrySpy).toHaveBeenCalledTimes(1);
    retrySpy.mockRestore();
  });
});
