import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AICopilotInput } from '@/components/ai/AICopilotInput';
import { useSettingsStore } from '@/stores/settings';
import type { AIProviderConfig } from '@/types/ai';

describe('AICopilotInput', () => {
  const mockProvider: AIProviderConfig = {
    id: 'prov-1',
    name: 'DeepSeek Chat',
    baseUrl: 'https://api.deepseek.com/v1',
    apiKey: 'sk-1',
    model: 'deepseek-chat',
  };

  const mockProvider2: AIProviderConfig = {
    id: 'prov-2',
    name: 'OpenAI GPT-4o',
    baseUrl: 'https://api.openai.com/v1',
    apiKey: 'sk-2',
    model: 'gpt-4o',
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    useSettingsStore.setState({
      aiProviders: [mockProvider, mockProvider2],
      activeAiProviderId: 'prov-1',
    });
  });

  it('1. 输入框底部工具栏渲染供应商快速切换器并可直接切换供应商', async () => {
    const setActiveAiProviderIdSpy = vi.fn().mockImplementation(async (id: string | null) => {
      useSettingsStore.setState({ activeAiProviderId: id });
    });
    useSettingsStore.setState({ setActiveAiProviderId: setActiveAiProviderIdSpy });

    const handleSend = vi.fn();
    const handleStop = vi.fn();

    render(
      <AICopilotInput
        activeProvider={mockProvider}
        isGenerating={false}
        onSendMessage={handleSend}
        onStopGeneration={handleStop}
        onSelectWorkspaceFolder={vi.fn()}
        onUnlinkWorkspace={vi.fn()}
      />
    );

    // 供应商切换按钮渲染在输入框工具栏中
    const providerBtn = screen.getByRole('button', { name: /DeepSeek Chat/i });
    expect(providerBtn).toBeInTheDocument();

    // 点击弹出供应商选择菜单
    fireEvent.click(providerBtn);
    expect(screen.getByText('OpenAI GPT-4o')).toBeInTheDocument();

    // 点击切换为 OpenAI
    fireEvent.click(screen.getByRole('option', { name: /OpenAI GPT-4o/i }));

    await waitFor(() => {
      expect(setActiveAiProviderIdSpy).toHaveBeenCalledWith('prov-2');
    });
  });

  it('2. 输入文本并按 Enter 发送消息', () => {
    const handleSend = vi.fn();
    const handleStop = vi.fn();

    render(
      <AICopilotInput
        activeProvider={mockProvider}
        isGenerating={false}
        onSendMessage={handleSend}
        onStopGeneration={handleStop}
        onSelectWorkspaceFolder={vi.fn()}
        onUnlinkWorkspace={vi.fn()}
      />
    );

    const textarea = screen.getByPlaceholderText(/给 AI 助手发送指令/i);
    fireEvent.change(textarea, { target: { value: '分析当前流程' } });

    // 按 Enter 键发送
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    expect(handleSend).toHaveBeenCalledWith('分析当前流程', undefined);
  });
});
