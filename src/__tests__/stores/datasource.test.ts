/**
 * Datasource Store 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useDatasourceStore } from '@/stores/datasource';
import type { DatasourceConnection } from '@/types/datasource';

// Mock datasource 服务（静态 + 动态 import 均被拦截）
vi.mock('@/services/tauri/datasource', () => ({
  listDatasources: vi.fn(),
  datasourceBrowse: vi.fn(),
  datasourceGetItem: vi.fn(),
  datasourceGetTemplates: vi.fn(),
  datasourceValidateConfig: vi.fn(),
  loadConnections: vi.fn(),
  saveConnections: vi.fn(),
}));

import {
  listDatasources,
  datasourceBrowse,
  loadConnections,
  saveConnections,
} from '@/services/tauri/datasource';

const mockedListDatasources = vi.mocked(listDatasources);
const mockedDatasourceBrowse = vi.mocked(datasourceBrowse);
const mockedLoadConnections = vi.mocked(loadConnections);
const mockedSaveConnections = vi.mocked(saveConnections);

function makeConnection(overrides: Partial<DatasourceConnection> = {}): DatasourceConnection {
  return {
    id: 'conn-1',
    datasource_id: 'eolink',
    display_name: 'Eolink',
    config: { api_key: 'secret' },
    created_at: '2026-08-05T00:00:00Z',
    ...overrides,
  };
}

describe('DatasourceStore', () => {
  beforeEach(() => {
    useDatasourceStore.getState().reset();
    vi.clearAllMocks();
  });

  it('初始状态正确', () => {
    const state = useDatasourceStore.getState();
    expect(state.descriptions).toEqual([]);
    expect(state.connections).toEqual([]);
    expect(state.browseCache).toEqual({});
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  describe('loadDescriptions', () => {
    it('拉取数据源描述列表并写入 store', async () => {
      mockedListDatasources.mockResolvedValue([
        {
          id: 'eolink',
          display_name: 'Eolink',
          auth: { type: 'api_key' },
          resources: [],
        },
      ]);

      await useDatasourceStore.getState().loadDescriptions();

      const state = useDatasourceStore.getState();
      expect(state.descriptions).toHaveLength(1);
      expect(state.descriptions[0].id).toBe('eolink');
      expect(state.loading).toBe(false);
    });

    it('失败时写入 error', async () => {
      mockedListDatasources.mockRejectedValue(new Error('boom'));

      await useDatasourceStore.getState().loadDescriptions();

      expect(useDatasourceStore.getState().error).toBe('boom');
      expect(useDatasourceStore.getState().loading).toBe(false);
    });
  });

  describe('连接持久化', () => {
    it('loadConnections 填充已保存连接', async () => {
      mockedLoadConnections.mockResolvedValue([makeConnection()]);

      await useDatasourceStore.getState().loadConnections();

      expect(useDatasourceStore.getState().connections).toHaveLength(1);
      expect(useDatasourceStore.getState().connections[0].datasource_id).toBe('eolink');
    });

    it('saveConnection 追加连接并在存在同 id 时替换（upsert）', async () => {
      mockedSaveConnections.mockResolvedValue(undefined);

      await useDatasourceStore.getState().saveConnection(makeConnection({ config: { api_key: 'old' } }));
      expect(useDatasourceStore.getState().connections).toHaveLength(1);

      // 更新同 datasource_id 的连接
      await useDatasourceStore.getState().saveConnection(makeConnection({ config: { api_key: 'new' } }));
      const state = useDatasourceStore.getState();
      expect(state.connections).toHaveLength(1);
      expect(state.connections[0].config.api_key).toBe('new');
    });

    it('deleteConnection 删除连接并整表持久化', async () => {
      mockedSaveConnections.mockResolvedValue(undefined);
      const store = useDatasourceStore.getState();
      await store.saveConnection(makeConnection({ id: 'a' }));
      await store.saveConnection(makeConnection({ id: 'b' }));

      await useDatasourceStore.getState().deleteConnection('a');

      const state = useDatasourceStore.getState();
      expect(state.connections.map((c) => c.id)).toEqual(['b']);
      expect(mockedSaveConnections).toHaveBeenLastCalledWith([makeConnection({ id: 'b' })]);
    });
  });

  describe('browse 缓存', () => {
    beforeEach(async () => {
      // 预置连接（browse 依赖）
      mockedSaveConnections.mockResolvedValue(undefined);
      await useDatasourceStore.getState().saveConnection(makeConnection());
    });

    it('无连接时报错并写入 error', async () => {
      await useDatasourceStore.getState().deleteConnection('conn-1');

      await expect(
        useDatasourceStore.getState().browse('eolink', 'project', null),
      ).rejects.toThrow('no saved connection');
      expect(useDatasourceStore.getState().error).toContain('no saved connection');
    });

    it('首次 browse 走 IPC 并写缓存；缓存命中不再请求', async () => {
      mockedDatasourceBrowse.mockResolvedValue([
        { id: 'p1', name: 'Project 1', protocol: 'other', extra: [] },
      ]);

      const first = await useDatasourceStore.getState().browse('eolink', 'project', null);
      expect(first).toHaveLength(1);
      expect(mockedDatasourceBrowse).toHaveBeenCalledTimes(1);
      // 携带保存的连接配置
      expect(mockedDatasourceBrowse).toHaveBeenCalledWith(
        'eolink',
        'project',
        null,
        { api_key: 'secret' },
      );

      const second = await useDatasourceStore.getState().browse('eolink', 'project', null);
      expect(second).toHaveLength(1);
      expect(mockedDatasourceBrowse).toHaveBeenCalledTimes(1); // 命中缓存，未再请求
    });

    it('force=true 绕过缓存重新拉取', async () => {
      mockedDatasourceBrowse.mockResolvedValue([
        { id: 'p1', name: 'Project 1', protocol: 'other', extra: [] },
      ]);

      await useDatasourceStore.getState().browse('eolink', 'project', null);
      await useDatasourceStore.getState().browse('eolink', 'project', null, true);

      expect(mockedDatasourceBrowse).toHaveBeenCalledTimes(2);
    });

    it('父条目不同 → 缓存 key 不同，分别请求', async () => {
      mockedDatasourceBrowse.mockResolvedValue([]);

      await useDatasourceStore.getState().browse('eolink', 'api', null);
      await useDatasourceStore.getState().browse('eolink', 'api', 'p1');

      expect(mockedDatasourceBrowse).toHaveBeenCalledTimes(2);
      expect(mockedDatasourceBrowse).toHaveBeenLastCalledWith(
        'eolink',
        'api',
        'p1',
        { api_key: 'secret' },
      );
    });

    it('browse 失败写入 error 并抛出', async () => {
      mockedDatasourceBrowse.mockRejectedValue(new Error('auth failed'));

      await expect(
        useDatasourceStore.getState().browse('eolink', 'project', null),
      ).rejects.toThrow('auth failed');
      expect(useDatasourceStore.getState().error).toBe('auth failed');
    });
  });

  describe('clearCache', () => {
    it('无参清空全部缓存', async () => {
      mockedDatasourceBrowse.mockResolvedValue([]);
      const store = useDatasourceStore.getState();
      await store.saveConnection(makeConnection());
      await store.browse('eolink', 'project', null);
      await store.browse('eolink', 'api', 'p1');

      useDatasourceStore.getState().clearCache();
      expect(useDatasourceStore.getState().browseCache).toEqual({});
    });

    it('带数据源 id 只清该数据源缓存', async () => {
      mockedDatasourceBrowse.mockResolvedValue([]);
      const store = useDatasourceStore.getState();
      await store.saveConnection(makeConnection());
      await store.saveConnection(makeConnection({ id: 'conn-2', datasource_id: 'other' }));
      await store.browse('eolink', 'project', null);
      await store.browse('other', 'project', null);

      useDatasourceStore.getState().clearCache('eolink');

      const cache = useDatasourceStore.getState().browseCache;
      expect(cache['eolink/project/']).toBeUndefined();
      expect(cache['other/project/']).toBeDefined();
    });
  });
});
