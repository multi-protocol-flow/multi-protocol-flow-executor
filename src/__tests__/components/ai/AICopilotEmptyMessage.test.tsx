import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { AICopilotPanel } from '@/components/ai/AICopilotPanel';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
describe('MarkdownViewer & AICopilotPanel Enhancements', () => {
  beforeEach(() => {
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
  });

  it('1. MarkdownViewer 正确渲染代码块、复制按钮与表格横向滚动容器', async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    const markdown = `
# 标题
\`\`\`js
const a = 1;
\`\`\`

| Key | Value |
| --- | --- |
| 1   | test  |
`;

    const { container } = render(<MarkdownViewer content={markdown} />);

    // 表格应包含在 max-w-full overflow-x-auto 容器中
    const table = container.querySelector('table');
    expect(table).toBeInTheDocument();
    const tableWrapper = table?.parentElement;
    expect(tableWrapper).toHaveClass('overflow-x-auto');
    expect(tableWrapper).toHaveClass('max-w-full');

    // 代码块应有 Code 标签和复制按钮
    const ll = getLL();
    const copyBtn = screen.getByTitle(ll.ai.code.copy());
    expect(copyBtn).toBeInTheDocument();

    fireEvent.click(copyBtn);
    expect(writeTextMock).toHaveBeenCalled();

    await waitFor(() => {
      expect(screen.getByText(ll.ai.code.copied())).toBeInTheDocument();
    });
  });

  it('2. AICopilotPanel 遇到没有任何内容的空 assistant 消息时，不渲染孤立空白头像', () => {
    // 模拟已配置有效 Provider
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'p1',
          name: 'Test Provider',
          baseUrl: 'https://api.test.com/v1',
          apiKey: 'key',
          model: 'gpt-4o',
        },
      ],
      activeAiProviderId: 'p1',
      aiEnabled: true,
    });

    // 模拟 messages 包含一个工具调用消息，以及一个由于工具执行后模型未回复而产生的空 assistant 消息
    useAIStore.setState({
      isCopilotOpen: true,
      agentStatus: 'idle',
      activeStreamingMessageId: null,
      messages: [
        {
          id: 'user_1',
          role: 'user',
          content: '请帮我创建节点',
          timestamp: 1,
        },
        {
          id: 'asst_1_with_tools',
          role: 'assistant',
          content: '',
          tool_calls: [
            {
              id: 'call_1',
              type: 'function',
              function: {
                name: 'add_node',
                arguments: '{"type":"http:request"}',
              },
            },
          ],
          timestamp: 2,
        },
        // 这个空 assistant 消息：无正文、无思考、无工具
        {
          id: 'asst_2_empty',
          role: 'assistant',
          content: '',
          thought: '',
          tool_calls: [],
          status: 'done',
          timestamp: 3,
        },
      ],
    });

    const { container } = render(<AICopilotPanel />);

    // 页面上应该只渲染 1 个 Bot 头像 (对应 asst_1_with_tools)，而不是 2 个！
    // 用户头像 1 个 (User)
    // asst_2_empty 应该被过滤掉，不渲染为光秃秃的头像！
    const botAvatars = container.querySelectorAll('.bg-primary\\/10.text-primary.rounded-full');
    expect(botAvatars.length).toBe(1);

    // 工具调用项应该正常显示
    expect(screen.getByText('add_node')).toBeInTheDocument();
  });
});
