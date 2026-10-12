/**
 * Tauri 插件设置服务
 *
 * 封装插件目录信息 / 热重载 / 已安装列表 / 市场列表四个 IPC 命令。
 * 非 Tauri 环境走 mock.ts 的占位实现。
 */

import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';
import type {
  InstalledPlugin,
  InstalledPluginInfo,
  MarketplacePlugin,
  PluginDirInfo,
} from '@/types/plugins';

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

/**
 * 从插件市场安装插件（下载 + sha256 校验 + 解压到生效插件目录）
 *
 * @param url - 市场地址
 * @param name - 插件名
 * @param version - 精确版本（缺省取市场列表第一个匹配项）
 * @param force - 覆盖已存在的安装
 */
export async function installMarketplacePlugin(
  url: string,
  name: string,
  version?: string,
  force = false,
): Promise<InstalledPluginInfo> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<InstalledPluginInfo>('install_marketplace_plugin', {
        url,
        name,
        version: version ?? null,
        force,
      });
    } catch (error) {
      throw formatError(error, 'install_marketplace_plugin');
    }
  }
  const { installMarketplacePlugin: mock } = await import('./mock');
  return mock(url, name, version, force);
}

/**
 * 调用插件提供的 UI Call 方法（如人机协同 submit_human_response 等）
 */
export async function pluginUiCall<T = unknown>(
  nodeType: string,
  method: string,
  payload?: unknown
): Promise<T> {
  if (isTauriEnv()) {
    // 动态导入：Tauri API 在纯浏览器/测试环境中不存在
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<T>('plugin_ui_call', {
        nodeType,
        method,
        payload: payload ?? null,
      });
    } catch (error) {
      throw formatError(error, 'plugin_ui_call');
    }
  }
  return null as T;
}
