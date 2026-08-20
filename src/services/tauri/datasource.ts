/**
 * Tauri 数据源插件服务
 *
 * 封装数据源插件 5 个 IPC 命令（列表 / browse / get_item / get_templates /
 * validate_config），并负责数据源连接配置的本地持久化
 * （`{appDataDir}/store/datasources.bin`，照搬 settings.ts 的 LazyStore 模式）。
 * 非 Tauri 环境：IPC 返回空列表，持久化 no-op（不新增 mock 文件）。
 */

import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';
import type {
  DatasourceConnection,
  DatasourceDescription,
  DataItem,
  NodeTemplate,
  ValidateConfigResult,
} from '@/types/datasource';

/** 连接配置文件（相对 store 目录） */
const DATASOURCES_FILE = 'datasources.bin';

/** 缓存 appDataDir，避免重复 IPC 调用（settings.ts 的同名 helper 是模块内私有） */
let cachedAppDataDir: string | null = null;

/**
 * 获取应用数据目录（带缓存）
 */
async function getAppDataDirCached(): Promise<string> {
  if (cachedAppDataDir) return cachedAppDataDir;
  const { invoke } = await import('@tauri-apps/api/core');
  cachedAppDataDir = await invoke<string>('get_app_data_dir');
  return cachedAppDataDir;
}

/**
 * 列出全部已注册数据源（插件 describe 注册）
 */
export async function listDatasources(): Promise<DatasourceDescription[]> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<DatasourceDescription[]>('list_datasources');
    } catch (error) {
      throw formatError(error, 'list_datasources');
    }
  }
  return [];
}

/**
 * 浏览数据源某资源（项目列表 / 接口列表）
 *
 * @param datasourceId - 数据源 id
 * @param resourceId - 资源 id
 * @param parentId - 父条目 id（浏览子资源时；顶层资源为 null）
 * @param config - 连接配置（保存的 api_key 等，宿主原样转发给插件）
 */
export async function datasourceBrowse(
  datasourceId: string,
  resourceId: string,
  parentId: string | null,
  config: Record<string, unknown>,
): Promise<DataItem[]> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<DataItem[]>('datasource_browse', {
        datasourceId,
        resourceId,
        parentId,
        page: null,
        size: null,
        config,
      });
    } catch (error) {
      throw formatError(error, 'datasource_browse');
    }
  }
  return [];
}

/**
 * 获取数据源单个条目详情
 */
export async function datasourceGetItem(
  datasourceId: string,
  resourceId: string,
  itemId: string,
  config: Record<string, unknown>,
): Promise<DataItem> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<DataItem>('datasource_get_item', {
        datasourceId,
        resourceId,
        itemId,
        config,
      });
    } catch (error) {
      throw formatError(error, 'datasource_get_item');
    }
  }
  throw new Error('datasource_get_item is unavailable outside Tauri');
}

/**
 * 把平台条目转换成可落画布的节点模板
 */
export async function datasourceGetTemplates(
  datasourceId: string,
  itemIds: string[],
  config: Record<string, unknown>,
): Promise<NodeTemplate[]> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<NodeTemplate[]>('datasource_get_templates', {
        datasourceId,
        itemIds,
        config,
      });
    } catch (error) {
      throw formatError(error, 'datasource_get_templates');
    }
  }
  return [];
}

/**
 * 验证数据源连接配置（保存连接前调用）
 */
export async function datasourceValidateConfig(
  datasourceId: string,
  config: Record<string, unknown>,
): Promise<ValidateConfigResult> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<ValidateConfigResult>('datasource_validate_config', {
        datasourceId,
        config,
      });
    } catch (error) {
      throw formatError(error, 'datasource_validate_config');
    }
  }
  return { valid: false, message: 'unavailable outside Tauri' };
}

/**
 * 加载已保存的数据源连接列表（整表读取）
 */
export async function loadConnections(): Promise<DatasourceConnection[]> {
  if (isTauriEnv()) {
    const [{ LazyStore }] = await Promise.all([import('@tauri-apps/plugin-store')]);
    const appDataDir = await getAppDataDirCached();
    const store = new LazyStore(`${appDataDir}/store/${DATASOURCES_FILE}`);
    const raw = await store.get<DatasourceConnection[]>('connections');
    return raw ?? [];
  }
  return [];
}

/**
 * 保存数据源连接列表（整表写入 + save）
 */
export async function saveConnections(list: DatasourceConnection[]): Promise<void> {
  if (isTauriEnv()) {
    const [{ LazyStore }] = await Promise.all([import('@tauri-apps/plugin-store')]);
    const appDataDir = await getAppDataDirCached();
    const store = new LazyStore(`${appDataDir}/store/${DATASOURCES_FILE}`);
    await store.set('connections', list);
    await store.save();
  }
}
