/**
 * 数据源插件状态管理
 *
 * 覆盖数据源描述列表（listDatasources）、已保存连接（本地持久化）与浏览缓存
 * （browseCache，key = `${connectionId}/${resourceId}/${parentId ?? ''}`）。
 * 支持同一个数据源插件配置多个连接实例（如多个 Eolink 空间、多个 Postman 工作空间等）。
 */

import { create } from 'zustand';
import {
  datasourceBrowse,
  listDatasources,
  loadConnections,
  saveConnections,
} from '@/services/tauri/datasource';
import type {
  DatasourceConnection,
  DatasourceDescription,
  DataItem,
} from '@/types/datasource';

/**
 * 格式化错误信息
 */
function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

interface DatasourceState {
  /** 数据源描述列表（listDatasources 结果） */
  descriptions: DatasourceDescription[];
  /** 已保存连接列表（loadConnections 结果） */
  connections: DatasourceConnection[];
  /** 浏览缓存：key = `${datasourceId}/${resourceId}/${parentId ?? ''}` */
  browseCache: Record<string, DataItem[]>;
  /** 是否正在加载 */
  loading: boolean;
  /** 错误信息（browse/validate 失败等） */
  error: string | null;

  // Actions
  /** 加载数据源描述列表 */
  loadDescriptions: () => Promise<void>;
  /** 加载已保存连接列表 */
  loadConnections: () => Promise<void>;
  /** 保存一个连接（写 store + 持久化整表） */
  saveConnection: (conn: DatasourceConnection) => Promise<void>;
  /** 删除一个连接（写 store + 持久化整表） */
  deleteConnection: (id: string) => Promise<void>;
  /**
   * 浏览数据源某资源；有缓存且非 force 直接返回缓存，否则 IPC 拉取并写缓存。
   * 失败时设置 error 并抛出（面板据此显示行内错误）。
   */
  browse: (
    datasourceId: string,
    resourceId: string,
    parentId: string | null,
    force?: boolean,
    connectionId?: string,
  ) => Promise<DataItem[]>;
  /** 清空浏览缓存（刷新按钮用；可选按连接 id 或数据源 id 过滤） */
  clearCache: (id?: string) => void;
  /** 重置（测试隔离） */
  reset: () => void;
}

const initialState = {
  descriptions: [] as DatasourceDescription[],
  connections: [] as DatasourceConnection[],
  browseCache: {} as Record<string, DataItem[]>,
  loading: false,
  error: null as string | null,
};

export const useDatasourceStore = create<DatasourceState>((set, get) => ({
  ...initialState,

  loadDescriptions: async () => {
    set({ loading: true, error: null });
    try {
      const descriptions = await listDatasources();
      set({ descriptions, loading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
    }
  },

  loadConnections: async () => {
    try {
      const connections = await loadConnections();
      set({ connections });
    } catch (e) {
      set({ error: formatErrorMessage(e) });
    }
  },

  saveConnection: async (conn) => {
    // 按 conn.id 进行 upsert：存在同 id 则更新，不存在则追加新连接
    const existing = get().connections.filter((c) => c.id !== conn.id);
    const connections = [...existing, conn];
    set({ connections });
    get().clearCache(conn.id);
    await saveConnections(connections);
  },

  deleteConnection: async (id) => {
    const connections = get().connections.filter((c) => c.id !== id);
    set({ connections });
    get().clearCache(id);
    await saveConnections(connections);
  },

  browse: async (datasourceId, resourceId, parentId, force = false, connectionId) => {
    const conn = connectionId
      ? connectionFor(connectionId)
      : connectionFor(datasourceId);
    if (!conn) {
      const err = `datasource ${datasourceId} has no saved connection`;
      set({ error: err, loading: false });
      throw new Error(err);
    }

    const sameDsConnections = get().connections.filter((c) => c.datasource_id === datasourceId);
    const isSingleConnection = sameDsConnections.length <= 1;

    const key = `${conn.id}/${resourceId}/${parentId ?? ''}`;
    const legacyKey = `${datasourceId}/${resourceId}/${parentId ?? ''}`;
    // 多连接时严格按 conn.id 隔离缓存，绝不跨连接读取共享的 legacyKey 造成数据串味
    const cached = get().browseCache[key] ?? (isSingleConnection ? get().browseCache[legacyKey] : undefined);
    if (cached && !force) {
      return cached;
    }
    set({ loading: true, error: null });
    try {
      const items = await datasourceBrowse(
        datasourceId,
        resourceId,
        parentId,
        conn.config,
      );
      set((state) => ({
        browseCache: {
          ...state.browseCache,
          [key]: items,
          ...(isSingleConnection ? { [legacyKey]: items } : {}),
        },
        loading: false,
      }));
      return items;
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
      throw e;
    }
  },

  clearCache: (id) => {
    if (!id) {
      set({ browseCache: {} });
      return;
    }
    // id 可能是 connectionId 或 datasourceId
    const matchingConnIds = new Set(
      get()
        .connections.filter((c) => c.datasource_id === id || c.id === id)
        .map((c) => c.id),
    );
    const matchingDsIds = new Set(
      get()
        .connections.filter((c) => c.datasource_id === id || c.id === id)
        .map((c) => c.datasource_id),
    );
    matchingDsIds.add(id);

    set((state) => {
      const browseCache: Record<string, DataItem[]> = {};
      for (const [key, value] of Object.entries(state.browseCache)) {
        const prefix = key.split('/')[0];
        if (!matchingConnIds.has(prefix) && !matchingDsIds.has(prefix)) {
          browseCache[key] = value;
        }
      }
      return { browseCache };
    });
  },

  reset: () => set(initialState),
}));

/**
 * 查找指定连接。
 * 优先按 connectionId 精准匹配；若未匹配到或入参为 datasourceId 则回退按 datasource_id 查找第一个匹配连接。
 */
export function connectionFor(idOrDatasourceId: string): DatasourceConnection | undefined {
  const connections = useDatasourceStore.getState().connections;
  return (
    connections.find((c) => c.id === idOrDatasourceId) ??
    connections.find((c) => c.datasource_id === idOrDatasourceId)
  );
}

/**
 * 查找某个数据源类型下的所有已保存连接
 */
export function connectionsForDatasource(datasourceId: string): DatasourceConnection[] {
  return useDatasourceStore
    .getState()
    .connections.filter((c) => c.datasource_id === datasourceId);
}
