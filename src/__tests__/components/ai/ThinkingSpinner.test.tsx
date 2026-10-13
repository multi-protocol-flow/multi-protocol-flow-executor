import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ThinkingDots, ThinkingSpinner } from '@/components/ai/ThinkingSpinner';
import { AIMessageRow } from '@/components/ai/AIMessageRow';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIMessage } from '@/types/ai';

describe('ThinkingDots & AI Thinking i18n', () => {
  it('ThinkingDots 正常渲染 3 颗下落堆叠指示点，包含 thinking-dots testid 与各自的动画', () => {
    const { container } = render(<ThinkingDots className="my-custom-class" />);
    const dotsWrapper = screen.getByTestId('thinking-dots');
    expect(dotsWrapper).toBeInTheDocument();
    expect(dotsWrapper.className).toContain('my-custom-class');

    const dots = screen.getAllByTestId('thinking-dot');
    expect(dots).toHaveLength(3);
    expect(dots[0].className).toContain('animate-thinking-dot-1');
    expect(dots[1].className).toContain('animate-thinking-dot-2');
    expect(dots[2].className).toContain('animate-thinking-dot-3');

    // 验证 style 标签中定义了向下平移的下落关键帧 thinking-dot-fall
    const styleEl = container.querySelector('style');
    expect(styleEl?.textContent).toContain('thinking-dot-fall');
    expect(styleEl?.textContent).toContain('translateY(2px)');
    expect(ThinkingSpinner).toBe(ThinkingDots);
  });
  it('中文模式下思考字数统计展示“xxx 字”且流式思考状态展示同心旋转指示器', () => {
    localStorage.setItem('locale', 'zh-CN');
    const msg: AIMessage = {
      id: 'msg-1',
      role: 'assistant',
      content: '',
      thought: '这是深度思考过程测试文本',
      timestamp: Date.now(),
    };

    render(
      <AIMessageRow
        message={msg}
        isStreaming={true}
        isGenerating={true}
        isLatestAssistant={true}
      />
    );

    const charCount = msg.thought!.length;
    const expectedCountText = `${charCount} 字`;
    expect(screen.getByText(expectedCountText)).toBeInTheDocument();
    expect(screen.getByText(`(${getLL().ai.copilot.thinking()})`)).toBeInTheDocument();
  });

  it('英文模式下思考字数统计多语言化为“xxx chars”', () => {
    localStorage.setItem('locale', 'en-US');
    const msg: AIMessage = {
      id: 'msg-2',
      role: 'assistant',
      content: '',
      thought: 'Testing reasoning process',
      timestamp: Date.now(),
    };

    render(
      <AIMessageRow
        message={msg}
        isStreaming={false}
        isGenerating={false}
      />
    );

    const charCount = msg.thought!.length;
    const expectedCountText = `${charCount} chars`;
    expect(screen.getByText(expectedCountText)).toBeInTheDocument();
  });
});
