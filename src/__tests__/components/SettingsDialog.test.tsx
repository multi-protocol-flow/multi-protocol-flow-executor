import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { PathSettingItem } from '@/components/settings/PathSettingItem';
import { useUIStore } from '@/stores/ui';
import { useSettingsStore } from '@/stores/settings';
import { Folder } from 'lucide-react';

// Mock getLL
vi.mock('@/i18n/i18n-lazy', () => ({
  getLL: () => ({
    settings: {
      dialog_title: () => '全局设置',
      saved_badge: () => '已自动同步',
      tab_general: () => '常规与外观',
      tab_storage: () => '存储与路径',
      tab_ai: () => 'AI 助手模型',
      tab_engine: () => '引擎与性能',
      appearance: () => '外观主题',
      appearance_desc: () => '选择明暗模式',
      theme_light: () => '浅色',
      theme_dark: () => '暗黑',
      theme_system: () => '跟随系统',
      language: () => '语言',
      language_desc: () => '选择界面语言',
      config_file: () => '配置文件',
      config_file_desc: () => '持久化存储全局首选项',
      flows_dir: () => '流程保存目录',
      flows_dir_desc: () => '本地流程保存路径',
      reports_dir: () => '报告目录',
      reports_dir_desc: () => '执行报告存储路径',
      logs_dir: () => '日志目录',
      logs_dir_desc: () => '应用日志存储路径',
      default_timeout: () => '节点默认执行超时',
      default_timeout_tip: () => '超时兜底',
      plugin_max_concurrency: () => '插件最大并发数',
      plugin_max_concurrency_tip: () => '并发请求数',
      plugin_queue_capacity: () => '插件通信队列容量',
      plugin_queue_capacity_tip: () => '队列深度',
      timeout_presets: () => '快捷预设',
      preset_recommended: () => '推荐',
      engine_desc: () => '引擎配置',
      plugins_manager_link: () => '打开插件管理',
      plugins_dir: () => '插件存储目录',
      plugins_dir_desc: () => '扩展插件保存路径',
      copy_path: () => '复制路径',
      path_copied: () => '路径已复制到剪贴板',
      copy_failed: () => '复制失败',
      open_file: () => '打开文件',
      open_in_folder: () => '打开所在目录',
      open_dir: () => '打开目录',
      select_dir: () => '选择目录',
      loading: () => '加载中...',
      not_set: () => '未设置',
    },
    ai: {
      settings: {
        title: () => 'AI Provider 配置',
        description: () => '配置大模型服务商',
        enabled: () => '已启用',
        disabled: () => '已禁用',
        providers_list: () => '服务商列表',
        add_provider: () => '添加 Provider',
        no_providers: () => '尚未配置任何 AI Provider',
        no_providers_desc: () => '请添加一个 OpenAI 兼容服务',
        form_create_title: () => '添加新 AI Provider',
        form_edit_title: () => '编辑 AI Provider',
        quick_fill_presets: () => '快速填充预设:',
        cancel: () => '取消',
        save: () => '保存配置',
        name_label: () => 'Provider 显示名称',
        base_url_label: () => 'API Base URL',
        api_key_label: () => 'API Key',
        model_label: () => '模型名称',
        show: () => '显示',
        hide: () => '隐藏',
        fetch_models: () => '获取模型列表',
        test_connection: () => '测试连接',
        custom_system_prompt_label: () => '自定义全局 System Prompt',
        custom_system_prompt_placeholder: () => '输入附加的系统指令...',
        custom_system_prompt_tip: () => '提示词说明',
      },
    },
    plugins: {
      settings_title: () => '插件设置',
      source_default: () => '管理插件',
      source_env: () => '环境变量指定',
      source_config: () => '配置指定',
      reload_success: () => '插件已重载',
      reload_failed: () => '插件重载失败',
    },
    panels: {
      close: () => '关闭',
    },
    common: {
      label_browser_storage: () => '浏览器存储',
    },
  }),
}));

// Mock platform
vi.mock('@/lib/platform', () => ({
  isTauriEnv: () => true,
}));

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('PathSettingItem', () => {
  it('正确渲染路径、标签与操作按钮', () => {
    const handleSelect = vi.fn();
    const handleOpen = vi.fn();

    render(
      <PathSettingItem
        label="测试目录"
        description="测试目录描述"
        path="/data/test-dir"
        icon={Folder}
        onSelectDirectory={handleSelect}
        onOpenDirectory={handleOpen}
      />
    );

    expect(screen.getByText('测试目录')).toBeInTheDocument();
    expect(screen.getByText('测试目录描述')).toBeInTheDocument();
    expect(screen.getByText('/data/test-dir')).toBeInTheDocument();

    const selectBtn = screen.getByTitle('选择目录');
    fireEvent.click(selectBtn);
    expect(handleSelect).toHaveBeenCalled();

    const openBtn = screen.getByTitle('打开目录');
    fireEvent.click(openBtn);
    expect(handleOpen).toHaveBeenCalled();
  });
  it('支持渲染徽标与扩展操作按钮', () => {
    render(
      <PathSettingItem
        label="插件目录"
        description="插件路径"
        path="/data/plugins"
        icon={Folder}
        badge={<span data-testid="test-badge">自定义徽标</span>}
        extraHeaderAction={<button type="button">管理</button>}
      />
    );

    expect(screen.getByTestId('test-badge')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '管理' })).toBeInTheDocument();
  });

  it('文件模式下渲染打开文件与打开所在目录按钮', () => {
    const handleOpenFile = vi.fn();
    const handleOpenFolder = vi.fn();

    render(
      <PathSettingItem
        label="配置文件"
        description="系统配置"
        path="/path/to/settings.bin"
        icon={Folder}
        isFile
        onOpenFile={handleOpenFile}
        onOpenDirectory={handleOpenFolder}
      />
    );

    const openFileBtn = screen.getByTitle('打开文件');
    fireEvent.click(openFileBtn);
    expect(handleOpenFile).toHaveBeenCalled();

    const openFolderBtn = screen.getByTitle('打开所在目录');
    fireEvent.click(openFolderBtn);
    expect(handleOpenFolder).toHaveBeenCalled();
  });
});

describe('SettingsDialog', () => {
  beforeEach(() => {
    useUIStore.setState({
      showSettingsDialog: true,
      settingsDialogTab: 'general',
    });
    useSettingsStore.setState({
      theme: 'dark',
      locale: 'zh-CN',
      defaultExecutionTimeoutMs: 30000,
      pluginMaxConcurrency: 256,
      pluginQueueCapacity: 2048,
      flowsDir: '/test/flows',
      reportDir: '/test/reports',
      logDir: '/test/logs',
    });
  });

  it('对话框打开时渲染标题和分类导航项', () => {
    render(<SettingsDialog />);

    expect(screen.getByText('全局设置')).toBeInTheDocument();
    expect(screen.getAllByText('常规与外观').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('存储与路径').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('AI 助手模型')).toBeInTheDocument();
    expect(screen.getByText('引擎与性能')).toBeInTheDocument();
  });

  it('切换到存储 Tab 时展示存储与路径配置项', () => {
    render(<SettingsDialog />);

    const storageTabBtn = screen.getAllByText('存储与路径')[0];
    fireEvent.click(storageTabBtn);

    expect(screen.getByText('流程保存目录')).toBeInTheDocument();
    expect(screen.getByText('插件存储目录')).toBeInTheDocument();
    expect(screen.getByText('报告目录')).toBeInTheDocument();
    expect(screen.getByText('日志目录')).toBeInTheDocument();
  });

  it('切换到引擎 Tab 时展示超时与并发配置', () => {
    render(<SettingsDialog />);

    const engineTabBtn = screen.getByText('引擎与性能');
    fireEvent.click(engineTabBtn);

    expect(screen.getByText('节点默认执行超时')).toBeInTheDocument();
    expect(screen.getByText('插件最大并发数')).toBeInTheDocument();
    expect(screen.getByText('插件通信队列容量')).toBeInTheDocument();
    expect(screen.getByText('10s')).toBeInTheDocument();
    expect(screen.getByText('256 (推荐)')).toBeInTheDocument();
    expect(screen.getByText('2048 (推荐)')).toBeInTheDocument();
  });

  it('切换到 AI Tab 时展示 AI Provider 配置，点击添加 Provider 后展示快速填充预设与正确样式的保存按钮', () => {
    render(<SettingsDialog />);

    const aiTabBtn = screen.getByText('AI 助手模型');
    fireEvent.click(aiTabBtn);

    expect(screen.getByText('AI Provider 配置')).toBeInTheDocument();
    expect(screen.getByText('服务商列表')).toBeInTheDocument();

    const addBtn = screen.getByText('添加 Provider');
    fireEvent.click(addBtn);

    // 验证快速填充预设多语言文案显示
    expect(screen.getByText('快速填充预设:')).toBeInTheDocument();

    // 验证保存按钮存在且包含 hover:bg-primary/90 防止变白
    const saveBtn = screen.getByText('保存配置');
    expect(saveBtn).toBeInTheDocument();
    expect(saveBtn.className).toContain('bg-primary');
    expect(saveBtn.className).toContain('hover:bg-primary/90');
  });

  it('在引擎 Tab 中点击打开插件管理应关闭全局设置弹窗并打开插件弹窗（消除弹窗叠弹窗）', () => {
    render(<SettingsDialog />);

    const engineTabBtn = screen.getByText('引擎与性能');
    fireEvent.click(engineTabBtn);

    const openPluginsBtn = screen.getByRole('button', { name: /打开插件管理/ });
    fireEvent.click(openPluginsBtn);

    expect(useUIStore.getState().showSettingsDialog).toBe(false);
    expect(useUIStore.getState().showPluginSettingsDialog).toBe(true);
  });

  it('在存储 Tab 中点击打开插件管理应关闭全局设置弹窗并打开插件弹窗（消除弹窗叠弹窗）', () => {
    useUIStore.setState({
      showSettingsDialog: true,
      showPluginSettingsDialog: false,
    });
    render(<SettingsDialog />);

    const storageTabBtn = screen.getAllByText('存储与路径')[0];
    fireEvent.click(storageTabBtn);

    const openPluginsBtn = screen.getByRole('button', { name: /打开插件管理/ });
    fireEvent.click(openPluginsBtn);

    expect(useUIStore.getState().showSettingsDialog).toBe(false);
    expect(useUIStore.getState().showPluginSettingsDialog).toBe(true);
  });
});
