/**
 * PluginSettingsDialog 组件测试
 */
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PluginSettingsDialog } from '@/components/dialogs/PluginSettingsDialog';
import { useUIStore, usePluginsStore } from '@/stores';
import type { InstalledPlugin, MarketplacePlugin, PluginDirInfo } from '@/types/plugins';

vi.mock('@/services/tauri/settings', () => ({
  selectDirectory: vi.fn(),
}));

vi.mock('@/stores', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/stores')>();
  return {
    ...actual,
    useUIStore: vi.fn(),
    usePluginsStore: vi.fn(),
  };
});

describe('PluginSettingsDialog', () => {
  const mockClosePluginSettingsDialog = vi.fn();
  const mockLoadPluginDirInfo = vi.fn();
  const mockLoadInstalledPlugins = vi.fn();
  const mockLoadMarketplacePlugins = vi.fn();
  const mockClearMarketplace = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    (useUIStore as unknown as Mock).mockImplementation(
      (selector?: (state: { showPluginSettingsDialog: boolean; closePluginSettingsDialog: () => void }) => unknown) => {
        const state = {
          showPluginSettingsDialog: true,
          closePluginSettingsDialog: mockClosePluginSettingsDialog,
        };
        return typeof selector === 'function' ? selector(state) : state;
      }
    );

    (usePluginsStore as unknown as Mock).mockImplementation(
      (selector?: (state: {
        pluginDirInfo: PluginDirInfo | null;
        installed: InstalledPlugin[];
        marketplace: MarketplacePlugin[];
        loading: boolean;
        error: string | null;
        loadPluginDirInfo: () => Promise<void>;
        loadInstalled: () => Promise<void>;
        loadMarketplace: (url: string) => Promise<void>;
        reloadPlugins: () => Promise<void>;
        installMarketplacePlugin: (url: string, name: string, version?: string, overwrite?: boolean) => Promise<unknown>;
        clearMarketplace: () => void;
      }) => unknown) => {
        const state = {
          pluginDirInfo: null,
          installed: [
            {
              name: 'flow-engine-plugin-redis',
              version: '0.1.0',
              description: 'Redis connector plugin',
              path: '/plugins/redis',
              status: 'loaded' as const,
              error: null,
            },
          ],
          marketplace: [] as MarketplacePlugin[],
          loading: false,
          error: null,
          loadPluginDirInfo: mockLoadPluginDirInfo,
          loadInstalled: mockLoadInstalledPlugins,
          loadMarketplace: mockLoadMarketplacePlugins,
          reloadPlugins: vi.fn(),
          installMarketplacePlugin: vi.fn(),
          clearMarketplace: mockClearMarketplace,
        };
        return typeof selector === 'function' ? selector(state) : state;
      }
    );
  });
  it('渲染已安装和插件市场 Tab 切换按钮并展示安装数量', () => {
    render(<PluginSettingsDialog />);

    expect(screen.getByRole('tab', { name: /已安装/i })).toBeDefined();
    expect(screen.getByRole('tab', { name: /插件市场/i })).toBeDefined();
    expect(screen.getByText('1')).toBeDefined();
  });

  it('点击切换到插件市场 Tab', async () => {
    const user = userEvent.setup();
    render(<PluginSettingsDialog />);

    const marketTab = screen.getByRole('tab', { name: /插件市场/i });
    await user.click(marketTab);

    expect(screen.getByPlaceholderText(/https:\/\//i)).toBeDefined();
  });
});
