/**
 * 数据源插件状态管理
 *
 * 覆盖数据源描述列表（listDatasources）、已保存连接（本地持久化）与浏览缓存
 * （browseCache，key = `${datasourceId}/${resourceId}/${parentId ?? ''}`）。
 * 一个数据源一个连接（P0 不支持多连接）：`connectionFor` 由面板从 connections
 * 中查找。
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
  ) => Promise<DataItem[]>;
  /** 清空浏览缓存（刷新按钮用；可选按数据源过滤） */
  clearCache: (datasourceId?: string) => void;
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
    const existing = get().connections.filter(
      (c) => c.datasource_id !== conn.datasource_id,
    );
    const connections = [...existing, conn];
    set({ connections });
    get().clearCache(conn.datasource_id);
    await saveConnections(connections);
  },

  deleteConnection: async (id) => {
    const connections = get().connections.filter((c) => c.id !== id);
    set({ connections });
    await saveConnections(connections);
  },

  browse: async (datasourceId, resourceId, parentId, force = false) => {
    const key = `${datasourceId}/${resourceId}/${parentId ?? ''}`;
    const cached = get().browseCache[key];
    if (cached && !force) {
      return cached;
    }
    set({ loading: true, error: null });
    try {
      const conn = connectionFor(datasourceId);
      if (!conn) {
        throw new Error(`datasource ${datasourceId} has no saved connection`);
      }
      const items = await datasourceBrowse(
        datasourceId,
        resourceId,
        parentId,
        conn.config,
      );
      set((state) => ({
        browseCache: { ...state.browseCache, [key]: items },
        loading: false,
      }));
      return items;
    } catch (e) {
      set({ error: formatErrorMessage(e), loading: false });
      throw e;
    }
  },

  clearCache: (datasourceId) => {
    if (!datasourceId) {
      set({ browseCache: {} });
      return;
    }
    set((state) => {
      const browseCache: Record<string, DataItem[]> = {};
      for (const [key, value] of Object.entries(state.browseCache)) {
        if (!key.startsWith(`${datasourceId}/`)) {
          browseCache[key] = value;
        }
      }
      return { browseCache };
    });
  },

  reset: () => set(initialState),
}));

/**
 * 查找数据源对应的已保存连接（一个数据源一个连接；P0 不支持多连接）。
 */
export function connectionFor(datasourceId: string): DatasourceConnection | undefined {
  return useDatasourceStore
    .getState()
    .connections.find((c) => c.datasource_id === datasourceId);
}
