import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AboutSettingsTab } from '@/components/settings/AboutSettingsTab';
import { useUpdateStore } from '@/stores/update';
import { useSettingsStore } from '@/stores/settings';

// Mock getLL with real-like return values
vi.mock('@/i18n/i18n-lazy', () => ({
  getLL: () => ({
    settings: {
      app_name: () => 'MPE Flow',
      app_subtitle: () => '多协议流程编排与执行引擎',
      version_current: () => '当前版本',
      version_latest: () => '最新版本',
      check_updates: () => '检查更新',
      checking_updates: () => '正在检查更新...',
      update_available: () => '发现新版本',
      update_already_latest: () => '当前已是最新版本',
      update_check_failed: () => '检查更新失败',
      auto_check_updates: () => '启动时自动检查更新',
      auto_check_updates_desc: () => '在应用程序每次启动时在后台静默检查是否有新版本发布',
      include_prerelease: () => '接收预发布版本 (Beta / RC)',
      include_prerelease_desc: () => '抢先体验包含最新特性但可能不够稳定的测试版本',
      download_update: () => '立即下载更新',
      downloading_update: () => '正在下载更新包...',
      download_completed: () => '下载完成，已准备安装',
      install_and_restart: () => '立即安装并重启',
      installing_update: () => '正在启动安装程序...',
      skip_version: () => '跳过此版本',
      remind_later: () => '稍后提醒',
      open_in_browser: () => '在浏览器中下载',
      view_changelog: () => '查看完整更新日志',
      hide_changelog: () => '收起更新日志',
      retry_update: () => '重试更新',
      appearance: () => '外观与偏好',
      system_platform: () => '运行平台',
      official_website: () => '官方网站',
      open_source_repo: () => '开源代码仓库',
    },
    panels: {
      close: () => '关闭',
    },
  }),
}));

describe('AboutSettingsTab - 设置页内更新流程', () => {
  beforeEach(() => {
    useUpdateStore.getState().reset();
    useSettingsStore.setState({
      autoCheckUpdate: true,
      includePrerelease: false,
      ignoredVersions: [],
    });
    vi.clearAllMocks();
  });

  it('初始状态下渲染当前版本信息与检查更新按钮', () => {
    useUpdateStore.setState({ currentVersion: '0.1.1' });
    render(<AboutSettingsTab />);

    expect(screen.getByText('MPE Flow')).toBeInTheDocument();
    expect(screen.getAllByText(/v0.1.1/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('检查更新')).toBeInTheDocument();
  });

  it('发现可用更新时渲染更新详情、下载按钮以及折叠/展开更新日志', () => {
    useUpdateStore.setState({
      status: 'available',
      currentVersion: '0.1.1',
      updateInfo: {
        hasUpdate: true,
        currentVersion: '0.1.1',
        latestVersion: '0.1.2',
        releaseDate: '2026-09-01T00:00:00Z',
        releaseNotes: '### v0.1.2\n- 修复若干已知问题',
        isPrerelease: false,
        downloadUrl: 'https://example.com/download',
        platform: 'linux',
      },
    });

    render(<AboutSettingsTab />);

    expect(screen.getByText(/发现新版本/)).toBeInTheDocument();
    expect(screen.getByText('v0.1.2')).toBeInTheDocument();
    expect(screen.getByText('立即下载更新')).toBeInTheDocument();
    expect(screen.getByText('跳过此版本')).toBeInTheDocument();

    // 点击查看更新日志（卡片内的第一个按钮）
    const changelogBtn = screen.getAllByRole('button', { name: /查看完整更新日志/ })[0];
    fireEvent.click(changelogBtn);
    expect(screen.getByText(/修复若干已知问题/)).toBeInTheDocument();
    expect(screen.getByText('收起更新日志')).toBeInTheDocument();

    // 点击收起
    fireEvent.click(screen.getByText('收起更新日志'));
    expect(screen.queryByText(/修复若干已知问题/)).not.toBeInTheDocument();
  });

  it('点击立即下载更新触发 startDownload', () => {
    const mockStartDownload = vi.fn().mockResolvedValue('/tmp/update.pkg');
    useUpdateStore.setState({
      status: 'available',
      currentVersion: '0.1.1',
      updateInfo: {
        hasUpdate: true,
        currentVersion: '0.1.1',
        latestVersion: '0.1.2',
        releaseDate: null,
        releaseNotes: null,
        isPrerelease: false,
        downloadUrl: 'https://example.com/download',
        platform: 'linux',
      },
      startDownload: mockStartDownload,
    });

    render(<AboutSettingsTab />);

    const downloadBtn = screen.getByText('立即下载更新');
    fireEvent.click(downloadBtn);

    expect(mockStartDownload).toHaveBeenCalled();
  });

  it('下载中展示进度条与百分比', () => {
    useUpdateStore.setState({
      status: 'downloading',
      currentVersion: '0.1.1',
      updateInfo: {
        hasUpdate: true,
        currentVersion: '0.1.1',
        latestVersion: '0.1.2',
        releaseDate: null,
        releaseNotes: null,
        isPrerelease: false,
        downloadUrl: 'https://example.com/download',
        platform: 'linux',
      },
      progress: {
        received: 52428800,
        total: 104857600,
        percent: 50,
        bytesPerSec: 10485760,
      },
    });

    render(<AboutSettingsTab />);

    expect(screen.getAllByText('正在下载更新包...').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByText(/50.0 MB \/ 100.0 MB/)).toBeInTheDocument();
  });

  it('下载完成后展示安装并重启按钮，点击触发 installAndRestart', () => {
    const mockInstall = vi.fn();
    useUpdateStore.setState({
      status: 'downloaded',
      currentVersion: '0.1.1',
      updateInfo: {
        hasUpdate: true,
        currentVersion: '0.1.1',
        latestVersion: '0.1.2',
        releaseDate: null,
        releaseNotes: null,
        isPrerelease: false,
        downloadUrl: 'https://example.com/download',
        platform: 'linux',
      },
      installAndRestart: mockInstall,
    });

    render(<AboutSettingsTab />);

    expect(screen.getByText('下载完成，已准备安装')).toBeInTheDocument();
    const installBtn = screen.getByText('立即安装并重启');
    fireEvent.click(installBtn);

    expect(mockInstall).toHaveBeenCalled();
  });

  it('点击跳过此版本触发 ignoreCurrentVersion', () => {
    const mockIgnore = vi.fn();
    useUpdateStore.setState({
      status: 'available',
      currentVersion: '0.1.1',
      updateInfo: {
        hasUpdate: true,
        currentVersion: '0.1.1',
        latestVersion: '0.1.2',
        releaseDate: null,
        releaseNotes: null,
        isPrerelease: false,
        downloadUrl: 'https://example.com/download',
        platform: 'linux',
      },
      ignoreCurrentVersion: mockIgnore,
    });

    render(<AboutSettingsTab />);

    const skipBtn = screen.getByText('跳过此版本');
    fireEvent.click(skipBtn);

    expect(mockIgnore).toHaveBeenCalled();
  });
});
