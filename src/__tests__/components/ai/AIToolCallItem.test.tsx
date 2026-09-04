import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AIToolCallItem } from '@/components/ai/AIToolCallItem';
import type { AIToolCall } from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';

describe('AIToolCallItem', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  const sampleToolCall: AIToolCall = {
    id: 'call_123',
    type: 'function',
    function: {
      name: 'batch_create_pipeline_long_function_name_for_testing_overflow',
      arguments: JSON.stringify({
        pipeline_name: 'Auth Workflow',
        steps_count: 5,
      }),
    },
  };

  it('1. 渲染长函数名时具有截断与不溢出属性 (truncate, min-w-0, shrink-0)', () => {
    const { container } = render(
      <AIToolCallItem toolCall={sampleToolCall} isExecuted={true} />,
    );

    // 外层容器应具有 overflow-hidden 和 max-w-full
    const outerContainer = container.firstElementChild;
    expect(outerContainer).toHaveClass('overflow-hidden');
    expect(outerContainer).toHaveClass('max-w-full');

    // 函数名应具有 truncate, flex-1, min-w-0 并在 title 中展示全名
    const nameEl = screen.getByText(sampleToolCall.function.name);
    expect(nameEl).toHaveClass('truncate');
    expect(nameEl).toHaveClass('min-w-0');
    expect(nameEl).toHaveClass('flex-1');
    expect(nameEl).toHaveAttribute('title', sampleToolCall.function.name);

    // 状态徽章应有 shrink-0 和 whitespace-nowrap 避免被挤压溢出
    const ll = getLL();
    const doneEl = screen.getByText(ll.ai.tool_call.done());
    const badgeEl = doneEl.parentElement;
    expect(badgeEl).toHaveClass('shrink-0');
    expect(badgeEl).toHaveClass('whitespace-nowrap');
  });

  it('2. isExecuted 为 false 时渲染运行中状态', () => {
    render(<AIToolCallItem toolCall={sampleToolCall} isExecuted={false} />);

    const ll = getLL();
    const runningEl = screen.getByText(ll.ai.tool_call.running());
    expect(runningEl).toBeInTheDocument();
    expect(runningEl).toHaveClass('shrink-0');
    expect(runningEl).toHaveClass('whitespace-nowrap');
  });

  it('3. 点击展开参数并支持一键复制参数', async () => {
    // Mock navigator.clipboard
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    render(<AIToolCallItem toolCall={sampleToolCall} isExecuted={true} />);

    // 初始状态下参数区域不显示
    expect(screen.queryByText('"Auth Workflow"')).not.toBeInTheDocument();

    // 点击按钮展开
    const toggleButton = screen.getByRole('button');
    fireEvent.click(toggleButton);

    // 展开后应显示格式化的参数内容
    await waitFor(() => {
      expect(screen.getByText(/"Auth Workflow"/)).toBeInTheDocument();
    });

    // 点击复制按钮
    const ll = getLL();
    const copyBtn = screen.getByTitle(ll.ai.tool_call.copy_arguments());
    expect(copyBtn).toBeInTheDocument();
    fireEvent.click(copyBtn);

    expect(writeTextMock).toHaveBeenCalledTimes(1);
    expect(writeTextMock).toHaveBeenCalledWith(
      JSON.stringify(
        {
          pipeline_name: 'Auth Workflow',
          steps_count: 5,
        },
        null,
        2,
      ),
    );

    // 复制后应显示复制成功文案
    await waitFor(() => {
      expect(screen.getByText(ll.ai.tool_call.copied())).toBeInTheDocument();
    });
  });
});
