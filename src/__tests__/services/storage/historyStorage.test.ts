/**
 * 执行历史存储服务测试
 * 使用 Mock localStorage 测试执行历史的 CRUD 操作
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  saveExecutionHistory,
  listExecutionHistory,
  clearExecutionHistory,
  deleteExecutionHistoryByFlow,
  type ExecutionHistoryItem,
} from '@/services/storage/historyStorage';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: vi.fn((key: string) => store[key] || null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
    removeItem: vi.fn((key: string) => {
      delete store[key];
    }),
    clear: vi.fn(() => {
      store = {};
    }),
    get store() {
      return store;
    },
  };
})();

Object.defineProperty(global, 'localStorage', { value: localStorageMock });

/** 创建测试用的执行历史项 */
function createHistoryItem(
  overrides: Partial<ExecutionHistoryItem> = {}
): ExecutionHistoryItem {
  return {
    uuid: crypto.randomUUID(),
    flow_uuid: crypto.randomUUID(),
    flow_name: 'Test Flow',
    status: 'success',
    duration_ms: 1000,
    timestamp: Date.now(),
    ...overrides,
  };
}

describe('historyStorage', () => {
  beforeEach(() => {
    localStorageMock.clear();
    vi.clearAllMocks();
  });

  describe('saveExecutionHistory', () => {
    it('可以保存执行历史到 localStorage', () => {
      const item = createHistoryItem({ uuid: 'history-1' });
      saveExecutionHistory(item);

      expect(localStorageMock.setItem).toHaveBeenCalled();
      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].uuid).toBe('history-1');
    });

    it('保存的历史记录按时间倒序排列（最新在前）', () => {
      const item1 = createHistoryItem({
        uuid: 'history-1',
        flow_name: 'Flow 1',
        timestamp: 1000,
      });
      const item2 = createHistoryItem({
        uuid: 'history-2',
        flow_name: 'Flow 2',
        timestamp: 2000,
      });

      saveExecutionHistory(item1);
      saveExecutionHistory(item2);

      const history = listExecutionHistory();
      expect(history).toHaveLength(2);
      expect(history[0].uuid).toBe('history-2'); // 最新的在前
      expect(history[1].uuid).toBe('history-1');
    });

    it('限制历史记录数量为 100 条', () => {
      for (let i = 0; i < 150; i++) {
        saveExecutionHistory(
          createHistoryItem({
            uuid: `history-${i}`,
            timestamp: i,
          })
        );
      }

      const history = listExecutionHistory();
      expect(history).toHaveLength(100);
      // 最新的记录应该保留（149, 148, ..., 50）
      expect(history[0].uuid).toBe('history-149');
      expect(history[99].uuid).toBe('history-50');
    });

    it('可以保存成功状态的历史记录', () => {
      const item = createHistoryItem({
        uuid: 'success-history',
        status: 'success',
      });
      saveExecutionHistory(item);

      const history = listExecutionHistory();
      expect(history[0].status).toBe('success');
    });

    it('可以保存失败状态的历史记录', () => {
      const item = createHistoryItem({
        uuid: 'failed-history',
        status: 'failed',
      });
      saveExecutionHistory(item);

      const history = listExecutionHistory();
      expect(history[0].status).toBe('failed');
    });

    it('保存完整的历史记录数据', () => {
      const item = createHistoryItem({
        uuid: 'full-history',
        flow_uuid: 'flow-123',
        flow_name: '完整测试流程',
        status: 'success',
        duration_ms: 5000,
        timestamp: 1700000000000,
      });
      saveExecutionHistory(item);

      const history = listExecutionHistory();
      expect(history[0]).toEqual(item);
    });
  });

  describe('listExecutionHistory', () => {
    it('空存储返回空数组', () => {
      expect(listExecutionHistory()).toEqual([]);
    });

    it('返回所有历史记录', () => {
      saveExecutionHistory(createHistoryItem({ uuid: 'history-1' }));
      saveExecutionHistory(createHistoryItem({ uuid: 'history-2' }));
      saveExecutionHistory(createHistoryItem({ uuid: 'history-3' }));

      const history = listExecutionHistory();
      expect(history).toHaveLength(3);
    });

    it('处理 localStorage 中的无效 JSON 返回空数组', () => {
      localStorageMock.store['mpe_history'] = 'invalid json string';

      const history = listExecutionHistory();
      expect(history).toEqual([]);
    });

    it('处理 localStorage 中不存在的 key 返回空数组', () => {
      localStorageMock.store['other_key'] = '[]';

      const history = listExecutionHistory();
      expect(history).toEqual([]);
    });
  });

  describe('clearExecutionHistory', () => {
    it('可以清空所有执行历史', () => {
      saveExecutionHistory(createHistoryItem({ uuid: 'history-1' }));
      saveExecutionHistory(createHistoryItem({ uuid: 'history-2' }));

      expect(listExecutionHistory()).toHaveLength(2);

      clearExecutionHistory();

      expect(listExecutionHistory()).toHaveLength(0);
    });

    it('清空后可以重新保存', () => {
      saveExecutionHistory(createHistoryItem({ uuid: 'history-1' }));
      clearExecutionHistory();
      saveExecutionHistory(createHistoryItem({ uuid: 'history-2' }));

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].uuid).toBe('history-2');
    });

    it('清空空存储不会报错', () => {
      expect(() => clearExecutionHistory()).not.toThrow();
    });
  });

  describe('deleteExecutionHistoryByFlow', () => {
    it('可以删除指定流程的执行历史', () => {
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-1',
          flow_uuid: 'flow-1',
        })
      );
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-2',
          flow_uuid: 'flow-2',
        })
      );

      deleteExecutionHistoryByFlow('flow-1');

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].flow_uuid).toBe('flow-2');
    });

    it('删除不存在的流程历史不会报错', () => {
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-1',
          flow_uuid: 'flow-1',
        })
      );

      expect(() => deleteExecutionHistoryByFlow('non-existent-flow')).not.toThrow();
      expect(listExecutionHistory()).toHaveLength(1);
    });

    it('删除指定流程的所有历史记录', () => {
      // 同一个流程的多条历史记录
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-1',
          flow_uuid: 'flow-1',
          timestamp: 1000,
        })
      );
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-2',
          flow_uuid: 'flow-1',
          timestamp: 2000,
        })
      );
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-3',
          flow_uuid: 'flow-2',
          timestamp: 3000,
        })
      );

      deleteExecutionHistoryByFlow('flow-1');

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].flow_uuid).toBe('flow-2');
    });

    it('删除后不影响其他流程的历史', () => {
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-1',
          flow_uuid: 'flow-1',
          flow_name: '流程1',
        })
      );
      saveExecutionHistory(
        createHistoryItem({
          uuid: 'history-2',
          flow_uuid: 'flow-2',
          flow_name: '流程2',
        })
      );

      deleteExecutionHistoryByFlow('flow-1');

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].flow_name).toBe('流程2');
    });
  });

  describe('集成场景', () => {
    it('保存→列出→删除→清空完整流程', () => {
      // 1. 保存多条历史
      saveExecutionHistory(createHistoryItem({ uuid: 'history-1' }));
      saveExecutionHistory(createHistoryItem({ uuid: 'history-2' }));

      // 2. 列出历史
      expect(listExecutionHistory()).toHaveLength(2);

      // 3. 删除单条
      deleteExecutionHistoryByFlow(
        listExecutionHistory()[0].flow_uuid
      );
      expect(listExecutionHistory()).toHaveLength(1);

      // 4. 清空所有
      clearExecutionHistory();
      expect(listExecutionHistory()).toHaveLength(0);
    });

    it('处理边界情况：正好 100 条记录', () => {
      for (let i = 0; i < 100; i++) {
        saveExecutionHistory(
          createHistoryItem({
            uuid: `history-${i}`,
            timestamp: i,
          })
        );
      }

      const history = listExecutionHistory();
      expect(history).toHaveLength(100);
    });

    it('处理边界情况：101 条记录（触发截断）', () => {
      for (let i = 0; i < 101; i++) {
        saveExecutionHistory(
          createHistoryItem({
            uuid: `history-${i}`,
            timestamp: i,
          })
        );
      }

      const history = listExecutionHistory();
      expect(history).toHaveLength(100);
      // 最新的 100 条应该保留 (100, 99, ..., 1)
      expect(history[0].uuid).toBe('history-100');
      expect(history[99].uuid).toBe('history-1');
    });
  });
});
