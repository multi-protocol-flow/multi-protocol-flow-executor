import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AISpotlightDialog } from '@/components/ai/AISpotlightDialog';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import type { AIProviderConfig } from '@/types/ai';

describe('AISpotlightDialog', () => {
  const mockProvider1: AIProviderConfig = {
    id: 'prov-1',
    name: 'DeepSeek Official',
    baseUrl: 'https://api.deepseek.com/v1',
    apiKey: 'sk-1',
    model: 'deepseek-chat',
  };

  const mockProvider2: AIProviderConfig = {
    id: 'prov-2',
    name: 'Claude 3.5 Sonnet',
    baseUrl: 'https://api.anthropic.com/v1',
    apiKey: 'sk-2',
    model: 'claude-3-5-sonnet',
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    useAIStore.setState({
      isSpotlightOpen: true,
      messages: [],
      agentStatus: 'idle',
      pendingActions: [],
    });
    useSettingsStore.setState({
      aiProviders: [mockProvider1, mockProvider2],
      activeAiProviderId: 'prov-1',
    });
  });

  it('Spotlight 对话框中渲染供应商快捷选择器并可直接切换', async () => {
    const setActiveAiProviderIdSpy = vi.fn().mockImplementation(async (id: string | null) => {
      useSettingsStore.setState({ activeAiProviderId: id });
    });
    useSettingsStore.setState({ setActiveAiProviderId: setActiveAiProviderIdSpy });

    render(<AISpotlightDialog />);

    // 找到顶部供应商选择按钮
    const providerTrigger = screen.getByRole('button', { name: /DeepSeek Official/i });
    expect(providerTrigger).toBeInTheDocument();

    // 打开下拉
    fireEvent.click(providerTrigger);

    // 选项中展示 Claude 3.5 Sonnet
    const claudeOption = screen.getByRole('option', { name: /Claude 3.5 Sonnet/i });
    expect(claudeOption).toBeInTheDocument();

    // 点击切换
    fireEvent.click(claudeOption);

    await waitFor(() => {
      expect(setActiveAiProviderIdSpy).toHaveBeenCalledWith('prov-2');
    });
  });
});
