/**
 * Tauri 插件设置服务
 *
 * 封装插件目录信息 / 热重载 / 已安装列表 / 市场列表四个 IPC 命令。
 * 非 Tauri 环境走 mock.ts 的占位实现。
 */

import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';
import type { PluginDirInfo, InstalledPlugin, MarketplacePlugin } from '@/types/plugins';

/** 插件重载结果（`reload_plugins` 返回，camelCase） */
export interface ReloadPluginsResult {
  /** 新快照中已登记（describe 成功）的插件数 */
  pluginsRegistered: number;
  /** 新快照的节点类型总数（内置 + 插件） */
  nodeTypes: number;
}

/**
 * 获取当前生效的插件目录信息（含来源与设置文件中的配置值）
 */
export async function getPluginDirInfo(): Promise<PluginDirInfo> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<PluginDirInfo>('get_plugin_dir_info');
    } catch (error) {
      throw formatError(error, 'get_plugin_dir_info');
    }
  }
  const { getPluginDirInfo: mock } = await import('./mock');
  return mock();
}

/**
 * 重载插件目录（运行时热重载，立即生效）
 *
 * @param dir - 目标插件目录
 */
export async function reloadPlugins(dir: string): Promise<ReloadPluginsResult> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<ReloadPluginsResult>('reload_plugins', { dir });
    } catch (error) {
      throw formatError(error, 'reload_plugins');
    }
  }
  const { reloadPlugins: mock } = await import('./mock');
  return mock(dir);
}

/**
 * 列出已安装插件（扫描插件目录 + 加载状态）
 */
export async function listInstalledPlugins(): Promise<InstalledPlugin[]> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<InstalledPlugin[]>('list_installed_plugins');
    } catch (error) {
      throw formatError(error, 'list_installed_plugins');
    }
  }
  const { listInstalledPlugins: mock } = await import('./mock');
  return mock();
}

/**
 * 从插件市场拉取插件列表（`GET {base}/plugins`）
 *
 * @param url - 市场地址
 */
export async function listMarketplacePlugins(url: string): Promise<MarketplacePlugin[]> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<MarketplacePlugin[]>('list_marketplace_plugins', { url });
    } catch (error) {
      throw formatError(error, 'list_marketplace_plugins');
    }
  }
  const { listMarketplacePlugins: mock } = await import('./mock');
  return mock(url);
}
