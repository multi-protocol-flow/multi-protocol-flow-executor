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
      engine_desc: () => '引擎配置',
      plugins_manager_link: () => '打开插件管理',
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
      },
    },
    plugins: {
      settings_title: () => '插件设置',
      source_default: () => '管理插件',
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
  });
});
