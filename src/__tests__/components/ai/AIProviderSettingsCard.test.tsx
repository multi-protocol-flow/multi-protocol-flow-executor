import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AIProviderSettingsCard } from '@/components/ai/AIProviderSettingsCard';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { i18nObject } from '@/i18n/i18n-util';
import zh_CN_ai from '@/i18n/zh-CN/ai';
import en_US_ai from '@/i18n/en-US/ai';

describe('AIProviderSettingsCard i18n & styling', () => {
  beforeEach(() => {
    useSettingsStore.setState({
      aiProviders: [],
      activeAiProviderId: null,
      aiEnabled: true,
      aiCustomSystemPrompt: undefined,
    });
    localStorage.setItem('locale', 'zh-CN');
  });

  it('1. 国际化字典中包含快速填充预设与保存按钮的正确多语言定义', () => {
    // 中文环境
    expect(zh_CN_ai.settings.quick_fill_presets).toBe('快速填充预设:');
    expect(zh_CN_ai.settings.save).toBe('保存配置');

    // 英文环境
    expect(en_US_ai.settings.quick_fill_presets).toBe('Quick Fill Presets:');
    expect(en_US_ai.settings.save).toBe('Save Provider');

    // getLL 运行时
    const ll = getLL();
    expect(typeof ll.ai.settings.quick_fill_presets).toBe('function');
    expect(ll.ai.settings.quick_fill_presets()).toBe('快速填充预设:');
    const enLL = i18nObject('en-US');
    expect(enLL.ai.settings.quick_fill_presets()).toBe('Quick Fill Presets:');
    expect(enLL.ai.settings.save()).toBe('Save Provider');
  });

  it('2. 新建 Provider 时快速填充标签使用多语言文案渲染', () => {
    render(<AIProviderSettingsCard />);

    // 空状态下点击快速填充或点击新建 Provider
    const deepSeekBtn = screen.getByText('DeepSeek Official');
    expect(deepSeekBtn).toBeInTheDocument();

    // 点击该预设进入新建状态
    fireEvent.click(deepSeekBtn);

    // 快速填充预设标签应通过多语言正常渲染，而非硬编码
    const ll = getLL();
    expect(screen.getByText(ll.ai.settings.quick_fill_presets())).toBeInTheDocument();
  });

  it('3. Save Provider 按钮包含 hover:bg-primary/90 避免 hover 时变白导致看不清底色', () => {
    render(<AIProviderSettingsCard />);

    // 点击 DeepSeek 快速预设进入新建
    fireEvent.click(screen.getByText('DeepSeek Official'));

    const ll = getLL();
    const saveButton = screen.getByRole('button', { name: ll.ai.settings.save() });
    expect(saveButton).toBeInTheDocument();

    // 验证类名包含 hover:bg-primary/90，防止默认 variant 的 hover:bg-accent 覆盖使其变成白色
    expect(saveButton.className).toContain('bg-primary');
    expect(saveButton.className).toContain('text-primary-foreground');
    expect(saveButton.className).toContain('hover:bg-primary/90');
  });

  it('4. 点击预设可以快速填充表单并成功保存 Provider', async () => {
    render(<AIProviderSettingsCard />);

    // 点击新建
    fireEvent.click(screen.getByText('DeepSeek Official'));

    // 点击预设中的 "Local Ollama"
    const ollamaChip = screen.getByRole('button', { name: 'Local Ollama' });
    fireEvent.click(ollamaChip);

    // 检查表单已被填充为 Ollama 预设
    const nameInput = screen.getByDisplayValue('Local Ollama');
    expect(nameInput).toBeInTheDocument();

    const baseUrlInput = screen.getByDisplayValue('http://localhost:11434/v1');
    expect(baseUrlInput).toBeInTheDocument();

    // 保存 Provider
    const ll = getLL();
    const saveButton = screen.getByRole('button', { name: ll.ai.settings.save() });
    fireEvent.click(saveButton);

    // 验证 store 中保存了该 Provider
    await waitFor(() => {
      const providers = useSettingsStore.getState().aiProviders;
      expect(providers).toHaveLength(1);
      expect(providers[0].name).toBe('Local Ollama');
      expect(providers[0].baseUrl).toBe('http://localhost:11434/v1');
      expect(providers[0].model).toBe('qwen2.5:latest');
    });
  });
});
