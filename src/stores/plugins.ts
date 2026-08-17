/**
 * 插件设置 / 插件市场状态管理
 *
 * 覆盖插件目录信息、已安装插件列表与市场列表的加载/重载。
 * 目录更改走 settings store 持久化（`pluginDir`），这里只负责后端重载与列表刷新。
 */

import { create } from 'zustand';
import { isTauriEnv } from '@/lib/platform';
import type { PluginDirInfo, InstalledPlugin, MarketplacePlugin } from '@/types/plugins';

// 动态加载服务（Tauri 环境用真实 IPC，否则用 mock）
async function getPluginsService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/plugins');
  }
  return import('@/services/tauri/mock');
}

/**
 * 格式化错误信息
 */
function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

interface PluginsState {
  /** 当前生效的插件目录信息（null = 尚未加载） */
  pluginDirInfo: PluginDirInfo | null;
  /** 已安装插件列表 */
  installed: InstalledPlugin[];
  /** 市场插件列表 */
  marketplace: MarketplacePlugin[];
  /** 是否正在加载/重载 */
  loading: boolean;
  /** 错误信息 */
  error: string | null;

  // Actions
  /** 加载插件目录信息 */
  loadPluginDirInfo: () => Promise<void>;
  /** 加载已安装插件列表 */
  loadInstalled: () => Promise<void>;
  /** 重载插件目录（成功后再刷新已安装列表） */
  reloadPlugins: (dir: string) => Promise<void>;
  /** 从市场拉取插件列表 */
  loadMarketplace: (url: string) => Promise<void>;
  /** 从市场安装插件（成功后刷新已安装列表） */
  installMarketplacePlugin: (
    url: string,
    name: string,
    version?: string,
    force?: boolean,
  ) => Promise<void>;
  /** 清空市场列表 */
  clearMarketplace: () => void;
  /** 重置（测试隔离） */
  reset: () => void;
}

const initialState = {
  pluginDirInfo: null as PluginDirInfo | null,
  installed: [] as InstalledPlugin[],
  marketplace: [] as MarketplacePlugin[],
  loading: false,
  error: null as string | null,
};

export const usePluginsStore = create<PluginsState>((set) => ({
  ...initialState,

  loadPluginDirInfo: async () => {
    set({ loading: true, error: null });
    try {
      const service = await getPluginsService();
      const pluginDirInfo = await service.getPluginDirInfo();
      set({ pluginDirInfo, loading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
    }
  },

  loadInstalled: async () => {
    try {
      const service = await getPluginsService();
      const installed = await service.listInstalledPlugins();
      set({ installed });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  reloadPlugins: async (dir: string) => {
    set({ loading: true, error: null });
    try {
      const service = await getPluginsService();
      await service.reloadPlugins(dir);
      // 重载成功后刷新已安装列表与目录信息
      const [installed, pluginDirInfo] = await Promise.all([
        service.listInstalledPlugins(),
        service.getPluginDirInfo(),
      ]);
      set({ installed, pluginDirInfo, loading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
      throw e;
    }
  },

  loadMarketplace: async (url: string) => {
    set({ loading: true, error: null });
    try {
      const service = await getPluginsService();
      const marketplace = await service.listMarketplacePlugins(url);
      set({ marketplace, loading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
      throw e;
    }
  },

  installMarketplacePlugin: async (
    url: string,
    name: string,
    version?: string,
    force?: boolean,
  ) => {
    set({ loading: true, error: null });
    try {
      const service = await getPluginsService();
      await service.installMarketplacePlugin(url, name, version, force);
      // 安装后热重载插件目录，让新插件通过 describe 登记进注册表；
      // 重载成功后再刷新列表与目录信息（避免用旧快照展示中间态 error）
      const info = await service.getPluginDirInfo();
      if (info?.effectiveDir) {
        await service.reloadPlugins(info.effectiveDir);
      }
      const [installed, pluginDirInfo] = await Promise.all([
        service.listInstalledPlugins(),
        service.getPluginDirInfo(),
      ]);
      set({ installed, pluginDirInfo, loading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
      throw e;
    }
  },

  clearMarketplace: () => set({ marketplace: [] }),

  reset: () => set(initialState),
}));
