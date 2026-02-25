/**
 * 存储服务测试
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  saveFlow,
  loadFlow,
  listFlows,
  deleteFlow,
  clearFlows,
  saveExecutionHistory,
  listExecutionHistory,
  clearExecutionHistory,
  deleteExecutionHistoryByFlow,
  type FlowMetadata,
  type ExecutionHistoryItem,
} from '@/services/storage';
import type { Flow } from '@/types';

/** 创建测试用的空流程 */
function createTestFlow(uuid: string, name: string): Flow {
  return {
    uuid,
    name,
    nodes: [],
    connections: [],
  };
}

describe('FlowStorage', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe('saveFlow', () => {
    it('可以保存流程', () => {
      const flow = createTestFlow('test-1', '测试流程');
      saveFlow(flow);
      const loaded = loadFlow('test-1');
      expect(loaded).not.toBeNull();
      expect(loaded?.uuid).toBe('test-1');
      expect(loaded?.name).toBe('测试流程');
    });

    it('更新已存在的流程', () => {
      saveFlow(createTestFlow('test-1', '流程1'));
      saveFlow(createTestFlow('test-1', '更新后的流程'));

      const loaded = loadFlow('test-1');
      expect(loaded?.name).toBe('更新后的流程');
    });
  });

  describe('loadFlow', () => {
    it('加载不存在的流程返回 null', () => {
      expect(loadFlow('not-exist')).toBeNull();
    });
  });

  describe('listFlows', () => {
    it('空存储返回空数组', () => {
      expect(listFlows()).toEqual([]);
    });

    it('可以列出流程元数据', () => {
      saveFlow(createTestFlow('test-1', '流程1'));
      saveFlow(createTestFlow('test-2', '流程2'));

      const list = listFlows();
      expect(list).toHaveLength(2);
      expect(list.map((f) => f.name)).toContain('流程1');
      expect(list.map((f) => f.name)).toContain('流程2');
    });

    it('列表只包含元数据', () => {
      saveFlow(createTestFlow('test-1', '流程1'));

      const list = listFlows() as FlowMetadata[];
      expect(list[0]).toHaveProperty('uuid');
      expect(list[0]).toHaveProperty('name');
      expect(list[0]).toHaveProperty('updated_at');
      expect(list[0]).not.toHaveProperty('nodes');
      expect(list[0]).not.toHaveProperty('connections');
    });
  });

  describe('deleteFlow', () => {
    it('可以删除流程', () => {
      saveFlow(createTestFlow('test-1', '流程1'));

      deleteFlow('test-1');

      expect(loadFlow('test-1')).toBeNull();
      expect(listFlows()).toHaveLength(0);
    });

    it('删除不存在的流程不会报错', () => {
      expect(() => deleteFlow('not-exist')).not.toThrow();
    });
  });

  describe('clearFlows', () => {
    it('可以清空所有流程', () => {
      saveFlow(createTestFlow('test-1', '流程1'));
      saveFlow(createTestFlow('test-2', '流程2'));

      clearFlows();

      expect(loadFlow('test-1')).toBeNull();
      expect(loadFlow('test-2')).toBeNull();
      expect(listFlows()).toHaveLength(0);
    });
  });
});

describe('HistoryStorage', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe('saveExecutionHistory', () => {
    it('可以保存执行历史', () => {
      const item: ExecutionHistoryItem = {
        uuid: 'history-1',
        flow_uuid: 'flow-1',
        flow_name: '测试流程',
        status: 'success',
        duration_ms: 100,
        timestamp: Date.now(),
      };

      saveExecutionHistory(item);

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].uuid).toBe('history-1');
    });

    it('历史记录按时间倒序排列', () => {
      const item1: ExecutionHistoryItem = {
        uuid: 'history-1',
        flow_uuid: 'flow-1',
        flow_name: '流程1',
        status: 'success',
        duration_ms: 100,
        timestamp: 1000,
      };
      const item2: ExecutionHistoryItem = {
        uuid: 'history-2',
        flow_uuid: 'flow-2',
        flow_name: '流程2',
        status: 'failed',
        duration_ms: 200,
        timestamp: 2000,
      };

      saveExecutionHistory(item1);
      saveExecutionHistory(item2);

      const history = listExecutionHistory();
      expect(history[0].uuid).toBe('history-2'); // 最新的在前
      expect(history[1].uuid).toBe('history-1');
    });

    it('限制历史记录数量为100条', () => {
      for (let i = 0; i < 150; i++) {
        saveExecutionHistory({
          uuid: `history-${i}`,
          flow_uuid: 'flow-1',
          flow_name: '流程',
          status: 'success',
          duration_ms: 100,
          timestamp: i,
        });
      }

      const history = listExecutionHistory();
      expect(history).toHaveLength(100);
      // 最新的记录应该保留
      expect(history[0].uuid).toBe('history-149');
    });
  });

  describe('listExecutionHistory', () => {
    it('空存储返回空数组', () => {
      expect(listExecutionHistory()).toEqual([]);
    });
  });

  describe('clearExecutionHistory', () => {
    it('可以清空执行历史', () => {
      saveExecutionHistory({
        uuid: 'history-1',
        flow_uuid: 'flow-1',
        flow_name: '流程1',
        status: 'success',
        duration_ms: 100,
        timestamp: Date.now(),
      });

      clearExecutionHistory();

      expect(listExecutionHistory()).toHaveLength(0);
    });
  });

  describe('deleteExecutionHistoryByFlow', () => {
    it('可以删除指定流程的执行历史', () => {
      saveExecutionHistory({
        uuid: 'history-1',
        flow_uuid: 'flow-1',
        flow_name: '流程1',
        status: 'success',
        duration_ms: 100,
        timestamp: 1000,
      });
      saveExecutionHistory({
        uuid: 'history-2',
        flow_uuid: 'flow-2',
        flow_name: '流程2',
        status: 'success',
        duration_ms: 100,
        timestamp: 2000,
      });

      deleteExecutionHistoryByFlow('flow-1');

      const history = listExecutionHistory();
      expect(history).toHaveLength(1);
      expect(history[0].flow_uuid).toBe('flow-2');
    });
  });
});
