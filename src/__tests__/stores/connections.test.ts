/**
 * Connections Store 测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useConnectionsStore } from '@/stores/connections';
import type { DbConnection } from '@/types/database';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

// 删除 __TAURI__ 属性以模拟非 Tauri 环境（使用 Mock 服务）
// @ts-expect-error - 测试环境需要删除 __TAURI__
delete window.__TAURI__;

const mockConnection: DbConnection = {
  id: 'conn-001',
  name: 'Test PostgreSQL',
  driver: 'postgres',
  host: 'localhost',
  port: 5432,
  database: 'testdb',
  username: 'testuser',
  password: 'testpass',
  min_connections: 1,
  max_connections: 10,
  connect_timeout_ms: 10000,
  idle_timeout_ms: 300000,
  status: 'disconnected',
};

const mockConnection2: DbConnection = {
  id: 'conn-002',
  name: 'Test MySQL',
  driver: 'mysql',
  host: 'localhost',
  port: 3306,
  database: 'testdb',
  username: 'root',
  min_connections: 1,
  max_connections: 5,
  connect_timeout_ms: 10000,
  idle_timeout_ms: 300000,
  status: 'disconnected',
};

describe('ConnectionsStore', () => {
  beforeEach(() => {
    localStorageMock.clear();
    // 重置 store 状态
    useConnectionsStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('初始 connections 为空数组', () => {
      expect(useConnectionsStore.getState().connections).toEqual([]);
    });

    it('初始 activeConnectionId 为 null', () => {
      expect(useConnectionsStore.getState().activeConnectionId).toBeNull();
    });

    it('初始 isLoading 为 false', () => {
      expect(useConnectionsStore.getState().isLoading).toBe(false);
    });

    it('初始 error 为 null', () => {
      expect(useConnectionsStore.getState().error).toBeNull();
    });
  });

  describe('loadConnections', () => {
    it('加载连接后更新状态', async () => {
      // 预设连接数据
      localStorageMock.setItem(
        'app-db-connections',
        JSON.stringify([mockConnection])
      );

      await useConnectionsStore.getState().loadConnections();

      expect(useConnectionsStore.getState().connections).toHaveLength(1);
      expect(useConnectionsStore.getState().connections[0].id).toBe('conn-001');
      expect(useConnectionsStore.getState().isLoading).toBe(false);
      expect(useConnectionsStore.getState().error).toBeNull();
    });

    it('加载默认设置（无存储数据时）', async () => {
      await useConnectionsStore.getState().loadConnections();

      expect(useConnectionsStore.getState().connections).toEqual([]);
      expect(useConnectionsStore.getState().isLoading).toBe(false);
    });
  });

  describe('saveConnection', () => {
    it('新增连接', async () => {
      await useConnectionsStore.getState().saveConnection(mockConnection);

      expect(useConnectionsStore.getState().connections).toHaveLength(1);
      expect(useConnectionsStore.getState().connections[0].id).toBe('conn-001');

      // 验证已持久化
      const stored = JSON.parse(
        localStorageMock.getItem('app-db-connections') || '[]'
      );
      expect(stored).toHaveLength(1);
      expect(stored[0].id).toBe('conn-001');
    });

    it('更新现有连接', async () => {
      // 先添加连接
      await useConnectionsStore.getState().saveConnection(mockConnection);

      // 更新连接
      const updated: DbConnection = {
        ...mockConnection,
        name: 'Updated PostgreSQL',
        port: 5433,
      };
      await useConnectionsStore.getState().saveConnection(updated);

      expect(useConnectionsStore.getState().connections).toHaveLength(1);
      expect(useConnectionsStore.getState().connections[0].name).toBe(
        'Updated PostgreSQL'
      );
      expect(useConnectionsStore.getState().connections[0].port).toBe(5433);
    });
  });

  describe('deleteConnection', () => {
    it('删除连接', async () => {
      // 先添加两个连接
      await useConnectionsStore.getState().saveConnection(mockConnection);
      await useConnectionsStore.getState().saveConnection(mockConnection2);

      expect(useConnectionsStore.getState().connections).toHaveLength(2);

      // 删除第一个
      await useConnectionsStore.getState().deleteConnection('conn-001');

      expect(useConnectionsStore.getState().connections).toHaveLength(1);
      expect(useConnectionsStore.getState().connections[0].id).toBe('conn-002');
    });

    it('删除当前活跃连接时清除 activeConnectionId', async () => {
      await useConnectionsStore.getState().saveConnection(mockConnection);
      useConnectionsStore.getState().setActiveConnection('conn-001');

      expect(useConnectionsStore.getState().activeConnectionId).toBe('conn-001');

      await useConnectionsStore.getState().deleteConnection('conn-001');

      expect(useConnectionsStore.getState().activeConnectionId).toBeNull();
    });
  });

  describe('testConnection', () => {
    it('测试连接成功', async () => {
      await useConnectionsStore.getState().saveConnection(mockConnection);

      const result =
        await useConnectionsStore.getState().testConnection('conn-001');

      expect(result.success).toBe(true);
      expect(useConnectionsStore.getState().connections[0].status).toBe(
        'connected'
      );
    });

    it('测试连接失败（主机名包含 fail）', async () => {
      const failConnection: DbConnection = {
        ...mockConnection,
        id: 'conn-fail',
        host: 'fail.example.com',
      };
      await useConnectionsStore.getState().saveConnection(failConnection);

      const result =
        await useConnectionsStore.getState().testConnection('conn-fail');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(useConnectionsStore.getState().connections[0].status).toBe(
        'error'
      );
      expect(useConnectionsStore.getState().connections[0].last_error).toBeDefined();
    });

    it('测试不存在的连接返回失败', async () => {
      const result =
        await useConnectionsStore.getState().testConnection('non-existent');
      expect(result.success).toBe(false);
      expect(result.error).toContain('不存在');
    });
  });

  describe('setActiveConnection', () => {
    it('设置活跃连接', () => {
      useConnectionsStore.getState().setActiveConnection('conn-001');
      expect(useConnectionsStore.getState().activeConnectionId).toBe('conn-001');
    });

    it('清除活跃连接', () => {
      useConnectionsStore.getState().setActiveConnection('conn-001');
      useConnectionsStore.getState().setActiveConnection(null);
      expect(useConnectionsStore.getState().activeConnectionId).toBeNull();
    });
  });

  describe('reset', () => {
    it('重置状态到初始值', async () => {
      // 修改状态
      await useConnectionsStore.getState().saveConnection(mockConnection);
      useConnectionsStore.getState().setActiveConnection('conn-001');

      // 重置
      useConnectionsStore.getState().reset();

      expect(useConnectionsStore.getState().connections).toEqual([]);
      expect(useConnectionsStore.getState().activeConnectionId).toBeNull();
      expect(useConnectionsStore.getState().isLoading).toBe(false);
      expect(useConnectionsStore.getState().error).toBeNull();
    });
  });
});
