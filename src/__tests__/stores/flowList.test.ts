/**
 * FlowList Store 测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useFlowListStore, type FlowMetadata } from '@/stores/flowList';

/**
 * 创建测试用 FlowMetadata
 */
const createTestFlow = (overrides?: Partial<FlowMetadata>): FlowMetadata => ({
  uuid: crypto.randomUUID(),
  name: 'Test Flow',
  updated_at: Date.now(),
  folder_path: '',
  ...overrides,
});

describe('FlowListStore', () => {
  beforeEach(() => {
    useFlowListStore.getState().reset();
  });

  describe('初始状态', () => {
    it('初始 flows 为空数组', () => {
      expect(useFlowListStore.getState().flows).toEqual([]);
    });

    it('初始 isLoading 为 false', () => {
      expect(useFlowListStore.getState().isLoading).toBe(false);
    });

    it('初始 searchKeyword 为空字符串', () => {
      expect(useFlowListStore.getState().searchKeyword).toBe('');
    });

    it('初始 sortBy 为 name', () => {
      expect(useFlowListStore.getState().sortBy).toBe('name');
    });

    it('初始 sortOrder 为 asc', () => {
      expect(useFlowListStore.getState().sortOrder).toBe('asc');
    });
  });

  describe('setFlows', () => {
    it('可以设置流程列表', () => {
      const flows = [
        createTestFlow({ name: 'Flow 1' }),
        createTestFlow({ name: 'Flow 2' }),
      ];
      useFlowListStore.getState().setFlows(flows);
      expect(useFlowListStore.getState().flows).toEqual(flows);
    });

    it('可以设置为空数组', () => {
      const flows = [createTestFlow()];
      useFlowListStore.getState().setFlows(flows);
      expect(useFlowListStore.getState().flows).toHaveLength(1);

      useFlowListStore.getState().setFlows([]);
      expect(useFlowListStore.getState().flows).toEqual([]);
    });
  });

  describe('addFlow', () => {
    it('添加流程到列表开头', () => {
      const flow1 = createTestFlow({ name: 'Flow 1' });
      const flow2 = createTestFlow({ name: 'Flow 2' });

      useFlowListStore.getState().addFlow(flow1);
      useFlowListStore.getState().addFlow(flow2);

      const flows = useFlowListStore.getState().flows;
      expect(flows).toHaveLength(2);
      expect(flows[0]).toEqual(flow2); // 后添加的在前面
      expect(flows[1]).toEqual(flow1);
    });

    it('向空列表添加流程', () => {
      const flow = createTestFlow({ name: 'New Flow' });
      useFlowListStore.getState().addFlow(flow);

      expect(useFlowListStore.getState().flows).toHaveLength(1);
      expect(useFlowListStore.getState().flows[0]).toEqual(flow);
    });
  });

  describe('updateFlow', () => {
    it('根据 uuid 更新流程', () => {
      const flow = createTestFlow({ name: 'Original Name' });
      useFlowListStore.getState().setFlows([flow]);

      useFlowListStore.getState().updateFlow(flow.uuid, { name: 'Updated Name' });

      const updatedFlow = useFlowListStore.getState().flows[0];
      expect(updatedFlow.name).toBe('Updated Name');
      expect(updatedFlow.uuid).toBe(flow.uuid);
      expect(updatedFlow.updated_at).toBe(flow.updated_at);
    });

    it('更新不存在的 uuid 不影响列表', () => {
      const flow = createTestFlow({ name: 'Test Flow' });
      useFlowListStore.getState().setFlows([flow]);

      useFlowListStore.getState().updateFlow('non-existent-uuid', { name: 'New Name' });

      expect(useFlowListStore.getState().flows).toHaveLength(1);
      expect(useFlowListStore.getState().flows[0].name).toBe('Test Flow');
    });

    it('可以更新多个字段', () => {
      const flow = createTestFlow({ name: 'Original', node_count: 5 });
      useFlowListStore.getState().setFlows([flow]);

      const newTime = Date.now() + 1000;
      useFlowListStore.getState().updateFlow(flow.uuid, {
        name: 'Updated',
        updated_at: newTime,
        node_count: 10,
      });

      const updated = useFlowListStore.getState().flows[0];
      expect(updated.name).toBe('Updated');
      expect(updated.updated_at).toBe(newTime);
      expect(updated.node_count).toBe(10);
    });
  });

  describe('removeFlow', () => {
    it('根据 uuid 移除流程', () => {
      const flow1 = createTestFlow({ name: 'Flow 1' });
      const flow2 = createTestFlow({ name: 'Flow 2' });
      useFlowListStore.getState().setFlows([flow1, flow2]);

      useFlowListStore.getState().removeFlow(flow1.uuid);

      const flows = useFlowListStore.getState().flows;
      expect(flows).toHaveLength(1);
      expect(flows[0].uuid).toBe(flow2.uuid);
    });

    it('移除不存在的 uuid 不影响列表', () => {
      const flow = createTestFlow();
      useFlowListStore.getState().setFlows([flow]);

      useFlowListStore.getState().removeFlow('non-existent-uuid');

      expect(useFlowListStore.getState().flows).toHaveLength(1);
    });

    it('从空列表移除不报错', () => {
      expect(() => {
        useFlowListStore.getState().removeFlow('any-uuid');
      }).not.toThrow();
      expect(useFlowListStore.getState().flows).toEqual([]);
    });
  });

  describe('setLoading', () => {
    it('设置加载状态为 true', () => {
      useFlowListStore.getState().setLoading(true);
      expect(useFlowListStore.getState().isLoading).toBe(true);
    });

    it('设置加载状态为 false', () => {
      useFlowListStore.getState().setLoading(true);
      useFlowListStore.getState().setLoading(false);
      expect(useFlowListStore.getState().isLoading).toBe(false);
    });
  });

  describe('setSearchKeyword', () => {
    it('设置搜索关键词', () => {
      useFlowListStore.getState().setSearchKeyword('test');
      expect(useFlowListStore.getState().searchKeyword).toBe('test');
    });

    it('可以设置为空字符串', () => {
      useFlowListStore.getState().setSearchKeyword('test');
      useFlowListStore.getState().setSearchKeyword('');
      expect(useFlowListStore.getState().searchKeyword).toBe('');
    });
  });

  describe('setSortBy', () => {
    it('设置排序字段为 name', () => {
      useFlowListStore.getState().setSortBy('name');
      expect(useFlowListStore.getState().sortBy).toBe('name');
    });

    it('设置排序字段为 updated_at', () => {
      useFlowListStore.getState().setSortBy('name');
      useFlowListStore.getState().setSortBy('updated_at');
      expect(useFlowListStore.getState().sortBy).toBe('updated_at');
    });
  });

  describe('setSortOrder', () => {
    it('设置排序方向为 asc', () => {
      useFlowListStore.getState().setSortOrder('asc');
      expect(useFlowListStore.getState().sortOrder).toBe('asc');
    });

    it('设置排序方向为 desc', () => {
      useFlowListStore.getState().setSortOrder('asc');
      useFlowListStore.getState().setSortOrder('desc');
      expect(useFlowListStore.getState().sortOrder).toBe('desc');
    });
  });

  describe('reset', () => {
    it('重置所有状态到初始值', () => {
      const flows = [createTestFlow()];
      useFlowListStore.getState().setFlows(flows);
      useFlowListStore.getState().setLoading(true);
      useFlowListStore.getState().setSearchKeyword('test');
      useFlowListStore.getState().setSortBy('name');
      useFlowListStore.getState().setSortOrder('asc');

      useFlowListStore.getState().reset();

      const state = useFlowListStore.getState();
      expect(state.flows).toEqual([]);
      expect(state.isLoading).toBe(false);
      expect(state.searchKeyword).toBe('');
      expect(state.sortBy).toBe('name');
      expect(state.sortOrder).toBe('asc');
    });
  });

  describe('getFilteredFlows', () => {
    it('无过滤条件时返回所有流程', () => {
      const flows = [
        createTestFlow({ name: 'Flow 1' }),
        createTestFlow({ name: 'Flow 2' }),
      ];
      useFlowListStore.getState().setFlows(flows);

      const filtered = useFlowListStore.getState().getFilteredFlows();
      expect(filtered).toHaveLength(2);
    });

    it('空列表返回空数组', () => {
      const filtered = useFlowListStore.getState().getFilteredFlows();
      expect(filtered).toEqual([]);
    });

    describe('搜索过滤', () => {
      it('根据名称过滤（大小写不敏感）', () => {
        const flows = [
          createTestFlow({ name: 'API Test Flow' }),
          createTestFlow({ name: 'User Management' }),
          createTestFlow({ name: 'api validation' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSearchKeyword('api');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered).toHaveLength(2);
        expect(filtered.map(f => f.name)).toContain('API Test Flow');
        expect(filtered.map(f => f.name)).toContain('api validation');
      });

      it('无匹配时返回空数组', () => {
        const flows = [
          createTestFlow({ name: 'Flow 1' }),
          createTestFlow({ name: 'Flow 2' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSearchKeyword('nonexistent');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered).toEqual([]);
      });

      it('空格关键词不过滤', () => {
        const flows = [
          createTestFlow({ name: 'Flow 1' }),
          createTestFlow({ name: 'Flow 2' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSearchKeyword('   ');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered).toHaveLength(2);
      });

      it('部分匹配有效', () => {
        const flows = [
          createTestFlow({ name: 'Get User Data' }),
          createTestFlow({ name: 'Post Comment' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSearchKeyword('user');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered).toHaveLength(1);
        expect(filtered[0].name).toBe('Get User Data');
      });
    });

    describe('排序 - 按名称', () => {
      it('按名称升序排序', () => {
        const flows = [
          createTestFlow({ name: 'Zebra Flow' }),
          createTestFlow({ name: 'Alpha Flow' }),
          createTestFlow({ name: 'Beta Flow' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSortBy('name');
        useFlowListStore.getState().setSortOrder('asc');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered[0].name).toBe('Alpha Flow');
        expect(filtered[1].name).toBe('Beta Flow');
        expect(filtered[2].name).toBe('Zebra Flow');
      });

      it('按名称降序排序', () => {
        const flows = [
          createTestFlow({ name: 'Alpha Flow' }),
          createTestFlow({ name: 'Zebra Flow' }),
          createTestFlow({ name: 'Beta Flow' }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSortBy('name');
        useFlowListStore.getState().setSortOrder('desc');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered[0].name).toBe('Zebra Flow');
        expect(filtered[1].name).toBe('Beta Flow');
        expect(filtered[2].name).toBe('Alpha Flow');
      });
    });

    describe('排序 - 按更新时间', () => {
      it('按更新时间升序排序（旧到新）', () => {
        const now = Date.now();
        const flows = [
          createTestFlow({ name: 'Newest', updated_at: now + 2000 }),
          createTestFlow({ name: 'Oldest', updated_at: now }),
          createTestFlow({ name: 'Middle', updated_at: now + 1000 }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSortBy('updated_at');
        useFlowListStore.getState().setSortOrder('asc');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered[0].name).toBe('Oldest');
        expect(filtered[1].name).toBe('Middle');
        expect(filtered[2].name).toBe('Newest');
      });

      it('按更新时间降序排序（新到旧）', () => {
        const now = Date.now();
        const flows = [
          createTestFlow({ name: 'Oldest', updated_at: now }),
          createTestFlow({ name: 'Newest', updated_at: now + 2000 }),
          createTestFlow({ name: 'Middle', updated_at: now + 1000 }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSortBy('updated_at');
        useFlowListStore.getState().setSortOrder('desc');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered[0].name).toBe('Newest');
        expect(filtered[1].name).toBe('Middle');
        expect(filtered[2].name).toBe('Oldest');
      });
    });

    describe('组合过滤和排序', () => {
      it('先过滤后排序', () => {
        const now = Date.now();
        const flows = [
          createTestFlow({ name: 'API Alpha', updated_at: now }),
          createTestFlow({ name: 'User Flow', updated_at: now + 1000 }),
          createTestFlow({ name: 'API Beta', updated_at: now + 2000 }),
        ];
        useFlowListStore.getState().setFlows(flows);
        useFlowListStore.getState().setSearchKeyword('api');
        useFlowListStore.getState().setSortBy('name');
        useFlowListStore.getState().setSortOrder('asc');

        const filtered = useFlowListStore.getState().getFilteredFlows();
        expect(filtered).toHaveLength(2);
        expect(filtered[0].name).toBe('API Alpha');
        expect(filtered[1].name).toBe('API Beta');
      });
    });
  });

  describe('Selectors', () => {
    it('selectFlows 返回流程列表', () => {
      const flows = [createTestFlow()];
      useFlowListStore.getState().setFlows(flows);
      expect(useFlowListStore.getState().flows).toEqual(flows);
    });

    it('selectIsLoading 返回加载状态', () => {
      useFlowListStore.getState().setLoading(true);
      expect(useFlowListStore.getState().isLoading).toBe(true);
    });

    it('selectSearchKeyword 返回搜索关键词', () => {
      useFlowListStore.getState().setSearchKeyword('test');
      expect(useFlowListStore.getState().searchKeyword).toBe('test');
    });
  });
});
