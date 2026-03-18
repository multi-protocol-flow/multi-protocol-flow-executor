/**
 * 数据库连接状态管理
 * 使用 Zustand 管理连接状态，自动检测 Tauri 环境
 */

import { create } from 'zustand';
import type { DbConnection } from '@/types/database';

// 检测是否在 Tauri 环境中
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

// 动态导入服务
async function getConnectionService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/connection');
  }
  return import('@/services/tauri/connection.mock');
}

/**
 * 格式化错误信息
 */
function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export interface ConnectionsState {
  connections: DbConnection[];
  activeConnectionId: string | null;
  isLoading: boolean;
  error: string | null;
}

export interface ConnectionsActions {
  loadConnections: () => Promise<void>;
  saveConnection: (connection: DbConnection) => Promise<void>;
  deleteConnection: (id: string) => Promise<void>;
  testConnection: (id: string) => Promise<{ success: boolean; error?: string }>;
  setActiveConnection: (id: string | null) => void;
  reset: () => void;
}

export type ConnectionsStore = ConnectionsState & ConnectionsActions;

const initialState: ConnectionsState = {
  connections: [],
  activeConnectionId: null,
  isLoading: false,
  error: null,
};

export const useConnectionsStore = create<ConnectionsStore>((set, get) => ({
  ...initialState,

  loadConnections: async () => {
    set({ isLoading: true, error: null });
    try {
      const service = await getConnectionService();
      const connections = await service.loadConnections();
      set({ connections, isLoading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), isLoading: false });
    }
  },

  saveConnection: async (connection: DbConnection) => {
    set({ isLoading: true, error: null });
    try {
      const service = await getConnectionService();
      await service.saveConnection(connection);
      // 更新本地状态
      const connections = get().connections;
      const existingIndex = connections.findIndex((c) => c.id === connection.id);
      if (existingIndex >= 0) {
        const updated = [...connections];
        updated[existingIndex] = connection;
        set({ connections: updated, isLoading: false });
      } else {
        set({ connections: [...connections, connection], isLoading: false });
      }
    } catch (e) {
      set({ error: formatErrorMessage(e), isLoading: false });
    }
  },

  deleteConnection: async (id: string) => {
    set({ isLoading: true, error: null });
    try {
      const service = await getConnectionService();
      await service.deleteConnection(id);
      const connections = get().connections.filter((c) => c.id !== id);
      // 如果删除的是当前活跃连接，清除活跃连接
      const activeConnectionId =
        get().activeConnectionId === id ? null : get().activeConnectionId;
      set({ connections, activeConnectionId, isLoading: false });
    } catch (e) {
      set({ error: formatErrorMessage(e), isLoading: false });
    }
  },

  testConnection: async (id: string) => {
    set({ error: null });
    try {
      const service = await getConnectionService();
      const result = await service.testConnection(id);
      // 更新连接状态
      const connections = get().connections.map((c) => {
        if (c.id === id) {
          return {
            ...c,
            status: result.success ? 'connected' : 'error',
            last_tested_at: Date.now(),
            last_error: result.success ? undefined : result.error || '连接测试失败',
          } as DbConnection;
        }
        return c;
      });
      set({ connections });
      return { success: result.success, error: result.error };
    } catch (e) {
      const errorMessage = formatErrorMessage(e);
      // 更新连接状态为错误
      const connections = get().connections.map((c) => {
        if (c.id === id) {
          return {
            ...c,
            status: 'error' as const,
            last_tested_at: Date.now(),
            last_error: errorMessage,
          } as DbConnection;
        }
        return c;
      });
      set({ connections, error: errorMessage });
      return { success: false, error: errorMessage };
    }
  },

  setActiveConnection: (id: string | null) => {
    set({ activeConnectionId: id });
  },

  reset: () => set(initialState),
}));

// Selectors (导出用于性能优化)
export const selectDbConnections = (state: ConnectionsStore) => state.connections;
export const selectActiveDbConnectionId = (state: ConnectionsStore) =>
  state.activeConnectionId;
export const selectDbConnectionsLoading = (state: ConnectionsStore) => state.isLoading;
export const selectDbConnectionsError = (state: ConnectionsStore) => state.error;
