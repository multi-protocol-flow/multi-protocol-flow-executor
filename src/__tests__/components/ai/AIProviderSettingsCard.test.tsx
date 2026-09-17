import { describe, it, expect, beforeEach, vi } from 'vitest';
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

  it('5. AI 开关胶囊支持切换并在暗黑模式下具备 emerald 胶囊高亮效果', async () => {
    const ll = getLL();
    useSettingsStore.setState({ aiEnabled: true });
    const { rerender } = render(<AIProviderSettingsCard />);

    // 开启状态验证
    const enabledTitle = ll.ai.settings.enabled();
    const toggleBtn = screen.getByTitle(enabledTitle);
    expect(toggleBtn).toBeInTheDocument();
    expect(toggleBtn.className).toContain('dark:bg-emerald-950/40');
    expect(toggleBtn.className).toContain('dark:border-emerald-600/40');

    // 点击切换为关闭状态
    fireEvent.click(toggleBtn);
    await waitFor(() => {
      expect(useSettingsStore.getState().aiEnabled).toBe(false);
    });

    // 重新渲染验证关闭状态样式
    rerender(<AIProviderSettingsCard />);
    const disabledTitle = ll.ai.settings.disabled();
    const disabledBtn = screen.getByTitle(disabledTitle);
    expect(disabledBtn).toBeInTheDocument();
    expect(disabledBtn.className).toContain('dark:bg-zinc-900/60');
    expect(disabledBtn.className).toContain('dark:border-zinc-800');
  });

  it('6. 多个 Provider 时，点击第一个 Provider 的编辑按钮，表单在第一个 Provider 卡片内就地展开且位于第二个 Provider 上方', () => {
    const ll = getLL();
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'prov-1',
          name: 'Provider One',
          baseUrl: 'https://api.one.com/v1',
          apiKey: 'sk-1',
          model: 'model-1',
        },
        {
          id: 'prov-2',
          name: 'Provider Two',
          baseUrl: 'https://api.two.com/v1',
          apiKey: 'sk-2',
          model: 'model-2',
        },
      ],
      activeAiProviderId: 'prov-1',
    });

    render(<AIProviderSettingsCard />);

    // 确认两个 Provider 都正常展示，且编辑表单初始未展示
    expect(screen.getByText('Provider One')).toBeInTheDocument();
    expect(screen.getByText('Provider Two')).toBeInTheDocument();
    expect(screen.queryByText(ll.ai.settings.form_edit_title())).not.toBeInTheDocument();

    // 点击第一个 Provider 的编辑按钮
    const editButtons = screen.getAllByTitle(ll.ai.settings.edit());
    expect(editButtons).toHaveLength(2);
    fireEvent.click(editButtons[0]);

    // 编辑表单展开
    const editTitle = screen.getByText(ll.ai.settings.form_edit_title());
    expect(editTitle).toBeInTheDocument();

    // 验证 DOM 相对位置：编辑表单位于 Provider Two 之前（随第一个 Provider 就地展开）
    const providerTwo = screen.getByText('Provider Two');
    expect(editTitle.compareDocumentPosition(providerTwo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    // 再次点击第一个 Provider 的编辑/取消按钮支持折叠收起
    const cancelIconBtn = screen.getByTitle(ll.ai.settings.cancel());
    fireEvent.click(cancelIconBtn);
    expect(screen.queryByText(ll.ai.settings.form_edit_title())).not.toBeInTheDocument();
  });

  it('7. 切换编辑不同 Provider 时，前一个 Provider 自动收起，后一个 Provider 就地展开', () => {
    const ll = getLL();
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'prov-1',
          name: 'Provider One',
          baseUrl: 'https://api.one.com/v1',
          apiKey: 'sk-1',
          model: 'model-1',
        },
        {
          id: 'prov-2',
          name: 'Provider Two',
          baseUrl: 'https://api.two.com/v1',
          apiKey: 'sk-2',
          model: 'model-2',
        },
      ],
      activeAiProviderId: 'prov-1',
    });

    render(<AIProviderSettingsCard />);

    // 点击第一个 Provider 编辑
    const editButtons1 = screen.getAllByTitle(ll.ai.settings.edit());
    fireEvent.click(editButtons1[0]);
    expect(screen.getByDisplayValue('Provider One')).toBeInTheDocument();

    // 直接点击第二个 Provider 的编辑按钮
    const editBtn2 = screen.getByTitle(ll.ai.settings.edit());
    fireEvent.click(editBtn2);

    // 现在表单内容切换为 Provider Two，Provider One 收起
    expect(screen.queryByDisplayValue('Provider One')).not.toBeInTheDocument();
    expect(screen.getByDisplayValue('Provider Two')).toBeInTheDocument();

    // 验证 DOM 相对位置：编辑表单在 Provider Two 后面展开，位于 Provider One 之后
    const providerOne = screen.getByText('Provider One');
    const editTitle = screen.getByText(ll.ai.settings.form_edit_title());
    expect(providerOne.compareDocumentPosition(editTitle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('8. 测试连通性时显示标准的 Loader2 旋转加载图标', async () => {
    const ll = getLL();
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'prov-1',
          name: 'Provider One',
          baseUrl: 'https://api.one.com/v1',
          apiKey: 'sk-1',
          model: 'model-1',
        },
      ],
      activeAiProviderId: 'prov-1',
    });

    let resolvePromise: (val: unknown) => void;
    const pendingPromise = new Promise<unknown>((resolve) => {
      resolvePromise = resolve;
    });

    const origFetch = global.fetch;
    global.fetch = vi.fn().mockReturnValue(pendingPromise);

    try {
      render(<AIProviderSettingsCard />);
      const testBtn = screen.getByTitle(ll.ai.settings.test_connection());
      fireEvent.click(testBtn);

      // 正在测试时，按钮被禁用且展示 animate-spin 的 Loader2 图标（类名包含 lucide-loader-2）
      expect(testBtn).toBeDisabled();
      const spinner = testBtn.querySelector('.animate-spin');
      expect(spinner).toBeInTheDocument();
      expect(spinner?.classList.contains('lucide-loader-circle')).toBe(true);
      expect(spinner?.classList.contains('lucide-activity')).toBe(false);
      // 完成后恢复
      resolvePromise!({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await waitFor(() => {
        expect(testBtn).not.toBeDisabled();
      });
    } finally {
      global.fetch = origFetch;
    }
  });
});
