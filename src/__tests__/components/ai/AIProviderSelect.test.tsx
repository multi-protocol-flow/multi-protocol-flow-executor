import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AIProviderSelect } from '@/components/ai/AIProviderSelect';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIProviderConfig } from '@/types/ai';

describe('AIProviderSelect', () => {
  const mockProviders: AIProviderConfig[] = [
    {
      id: 'provider-1',
      name: 'DeepSeek Official',
      baseUrl: 'https://api.deepseek.com/v1',
      apiKey: 'sk-1',
      model: 'deepseek-chat',
    },
    {
      id: 'provider-2',
      name: 'OpenAI GPT-4o',
      baseUrl: 'https://api.openai.com/v1',
      apiKey: 'sk-2',
      model: 'gpt-4o',
    },
    {
      id: 'provider-3',
      name: 'Local Ollama',
      baseUrl: 'http://localhost:11434/v1',
      apiKey: 'sk-3',
      model: 'qwen2.5:14b',
    },
  ];

  beforeEach(() => {
    vi.restoreAllMocks();
    useSettingsStore.setState({
      aiProviders: mockProviders,
      activeAiProviderId: 'provider-1',
    });
  });

  it('1. 渲染当前生效的 AI 供应商名称与模型', () => {
    render(<AIProviderSelect />);

    expect(screen.getByText('DeepSeek Official')).toBeInTheDocument();
    const button = screen.getByRole('button', { name: /DeepSeek Official/i });
    expect(button).toHaveAttribute('title', 'DeepSeek Official (deepseek-chat)');
  });

  it('2. 点击按钮打开下拉菜单，列出所有可用供应商并高亮当前激活项', async () => {
    render(<AIProviderSelect />);

    const trigger = screen.getByRole('button', { name: /DeepSeek Official/i });
    fireEvent.click(trigger);

    const ll = getLL();
    // 菜单标题
    expect(screen.getByText(ll.ai.copilot.switch_provider())).toBeInTheDocument();

    // 列表中所有供应商
    expect(screen.getByText('OpenAI GPT-4o')).toBeInTheDocument();
    expect(screen.getByText('gpt-4o')).toBeInTheDocument();
    expect(screen.getByText('Local Ollama')).toBeInTheDocument();
    expect(screen.getByText('qwen2.5:14b')).toBeInTheDocument();

    // 当前项带有 aria-selected="true"
    const selectedOption = screen.getByRole('option', { name: /DeepSeek Official/i });
    expect(selectedOption).toHaveAttribute('aria-selected', 'true');
  });

  it('3. 在对话中直接切换供应商，调用 setActiveAiProviderId 并更新 store', async () => {
    const setActiveAiProviderIdSpy = vi.fn().mockImplementation(async (id: string | null) => {
      useSettingsStore.setState({ activeAiProviderId: id });
    });
    useSettingsStore.setState({ setActiveAiProviderId: setActiveAiProviderIdSpy });

    render(<AIProviderSelect />);

    // 打开下拉
    fireEvent.click(screen.getByRole('button', { name: /DeepSeek Official/i }));

    // 点击切换为 OpenAI GPT-4o
    const targetOption = screen.getByRole('option', { name: /OpenAI GPT-4o/i });
    fireEvent.click(targetOption);

    await waitFor(() => {
      expect(setActiveAiProviderIdSpy).toHaveBeenCalledWith('provider-2');
    });

    // 激活项已切换
    expect(useSettingsStore.getState().activeAiProviderId).toBe('provider-2');
  });

  it('4. 点击"管理供应商..."按钮打开设置弹窗的 AI 标签页', () => {
    const openSettingsDialogSpy = vi.fn();
    useUIStore.setState({ openSettingsDialog: openSettingsDialogSpy });

    render(<AIProviderSelect />);

    fireEvent.click(screen.getByRole('button', { name: /DeepSeek Official/i }));

    const ll = getLL();
    const manageBtn = screen.getByText(ll.ai.copilot.manage_providers());
    fireEvent.click(manageBtn);

    expect(openSettingsDialogSpy).toHaveBeenCalledWith('ai');
  });

  it('5. 未配置任何供应商时展示提示和配置引导按钮', () => {
    useSettingsStore.setState({
      aiProviders: [],
      activeAiProviderId: null,
    });

    const openSettingsDialogSpy = vi.fn();
    useUIStore.setState({ openSettingsDialog: openSettingsDialogSpy });

    render(<AIProviderSelect />);

    const ll = getLL();
    expect(screen.getByText(ll.ai.copilot.select_provider())).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: ll.ai.copilot.select_provider() }));

    expect(screen.getByText(ll.ai.copilot.no_active_provider())).toBeInTheDocument();
    const configBtn = screen.getByText(ll.ai.copilot.configure_provider());
    fireEvent.click(configBtn);

    expect(openSettingsDialogSpy).toHaveBeenCalledWith('ai');
  });

  it('6. 按 Escape 键或点击外部关闭下拉列表', () => {
    render(
      <div>
        <div data-testid="outside">Outside</div>
        <AIProviderSelect />
      </div>,
    );

    const trigger = screen.getByRole('button', { name: /DeepSeek Official/i });
    fireEvent.click(trigger);
    expect(screen.getByRole('listbox')).toBeInTheDocument();

    // 点击外部
    fireEvent.mouseDown(screen.getByTestId('outside'));
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();

    // 重新打开并按 Escape
    fireEvent.click(trigger);
    expect(screen.getByRole('listbox')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });
});
