/**
 * 流程存储 Mock 服务测试
 * 测试 flowStorage.mock CRUD 及文件夹操作
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
  saveFlow,
  loadFlow,
  listFlows,
  deleteFlow,
  copyFlow,
  refreshFlows,
  clearMockFlows,
  getMockFlowCount,
  createFolder,
  deleteFolder,
  renameFolder,
  listFolders,
  moveFlowToFolder,
  moveFolder,
} from '@/services/storage/flowStorage.mock';
import type { Flow } from '@/types/flow';
import type { FlowMeta } from '@/services/storage/flowStorage';

/** 创建测试用的空流程 */
function createTestFlow(uuid: string, name: string): Flow {
  return {
    uuid,
    name,
    nodes: [
      {
        uuid: `entry-${uuid}`,
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
    ],
    connections: [],
  };
}

describe('FlowStorage Mock', () => {
  beforeEach(() => {
    clearMockFlows();
  });

  describe('saveFlow', () => {
    it('可以保存流程', async () => {
      const flow = createTestFlow('flow-1', '测试流程');
      await saveFlow(flow);

      expect(getMockFlowCount()).toBe(1);
    });

    it('可以保存带 _meta 的流程', async () => {
      const flow = createTestFlow('flow-1', '测试流程') as Flow & { _meta?: FlowMeta };
      flow._meta = { version: 2, isBuiltin: true };
      await saveFlow(flow);

      expect(getMockFlowCount()).toBe(1);
      const loaded = await loadFlow('flow-1');
      const loadedWithMeta = loaded as Flow & { _meta?: FlowMeta };
      expect(loadedWithMeta._meta).toEqual({ version: 2, isBuiltin: true });
    });

    it('可以保存多个流程', async () => {
      await saveFlow(createTestFlow('flow-1', '流程1'));
      await saveFlow(createTestFlow('flow-2', '流程2'));
      await saveFlow(createTestFlow('flow-3', '流程3'));

      expect(getMockFlowCount()).toBe(3);
    });

    it('保存相同 UUID 覆盖已有流程', async () => {
      const flow1 = createTestFlow('flow-1', '原始名称');
      await saveFlow(flow1);

      const flow2 = createTestFlow('flow-1', '更新后的名称');
      await saveFlow(flow2);

      expect(getMockFlowCount()).toBe(1);
      const loaded = await loadFlow('flow-1');
      expect(loaded.name).toBe('更新后的名称');
    });
  });

  describe('loadFlow', () => {
    it('可以加载已保存的流程', async () => {
      const flow = createTestFlow('flow-1', '测试流程');
      await saveFlow(flow);

      const loaded = await loadFlow('flow-1');

      expect(loaded).toBeDefined();
      expect(loaded.uuid).toBe('flow-1');
      expect(loaded.name).toBe('测试流程');
      expect(loaded.nodes).toHaveLength(1);
    });

    it('加载不存在的流程抛出错误', async () => {
      await expect(loadFlow('non-existent')).rejects.toThrow('Flow not found: non-existent');
    });

    it('加载的流程包含完整节点和连接数据', async () => {
      const flow = createTestFlow('flow-1', '完整流程');
      flow.nodes.push({
        uuid: 'http-node',
        type: 'http:request',
        name: 'HTTP 请求',
        position: { x: 200, y: 0 },
        data: { type: 'http:request', url: 'https://example.com', method: 'get' },
      });
      flow.connections.push({
        id: 'conn-1',
        source_node_uuid: `entry-flow-1`,
        target_node_uuid: 'http-node',
        source_port_id: 'out',
        target_port_id: 'in',
      });
      await saveFlow(flow);

      const loaded = await loadFlow('flow-1');

      expect(loaded.nodes).toHaveLength(2);
      expect(loaded.connections).toHaveLength(1);
      expect(loaded.nodes[1].type).toBe('http:request');
    });

    it('保存时不带 _meta，加载时不会附加 _meta', async () => {
      const flow = createTestFlow('flow-1', '无元数据');
      await saveFlow(flow);

      const loaded = await loadFlow('flow-1');
      const loadedWithMeta = loaded as Flow & { _meta?: FlowMeta };
      expect(loadedWithMeta._meta).toBeUndefined();
    });
  });

  describe('listFlows', () => {
    it('空存储返回空数组', async () => {
      const list = await listFlows();
      expect(list).toEqual([]);
    });

    it('返回流程元数据列表', async () => {
      await saveFlow(createTestFlow('flow-1', '流程1'));
      await saveFlow(createTestFlow('flow-2', '流程2'));

      const list = await listFlows();

      expect(list).toHaveLength(2);
      expect(list.find((m) => m.uuid === 'flow-1')?.name).toBe('流程1');
      expect(list.find((m) => m.uuid === 'flow-2')?.name).toBe('流程2');
    });

    it('元数据包含必需字段', async () => {
      await saveFlow(createTestFlow('flow-1', '测试流程'));

      const list = await listFlows();
      const meta = list[0];

      expect(meta).toHaveProperty('uuid');
      expect(meta).toHaveProperty('name');
      expect(meta).toHaveProperty('filePath');
      expect(meta).toHaveProperty('updatedAt');
      expect(meta).toHaveProperty('folderPath');
      expect(meta.filePath).toContain('mock://flows/');
    });
  });

  describe('deleteFlow', () => {
    it('可以删除已保存的流程', async () => {
      await saveFlow(createTestFlow('flow-1', '测试流程'));
      expect(getMockFlowCount()).toBe(1);

      await deleteFlow('flow-1');

      expect(getMockFlowCount()).toBe(0);
      await expect(loadFlow('flow-1')).rejects.toThrow('Flow not found');
    });

    it('删除不存在的流程抛出错误', async () => {
      await expect(deleteFlow('non-existent')).rejects.toThrow('Flow not found: non-existent');
    });

    it('删除后不影响其他流程', async () => {
      await saveFlow(createTestFlow('flow-1', '流程1'));
      await saveFlow(createTestFlow('flow-2', '流程2'));

      await deleteFlow('flow-1');

      expect(getMockFlowCount()).toBe(1);
      const loaded = await loadFlow('flow-2');
      expect(loaded.uuid).toBe('flow-2');
    });
  });

  describe('copyFlow', () => {
    it('复制已存在的流程', async () => {
      await saveFlow(createTestFlow('flow-1', '原始流程'));

      const copy = await copyFlow('flow-1');

      expect(copy.uuid).toContain('flow-1-copy');
      expect(copy.name).toContain('原始流程 (副本)');
      expect(getMockFlowCount()).toBe(2);
    });

    it('复制不存在的流程抛出错误', async () => {
      await expect(copyFlow('non-existent')).rejects.toThrow('Flow not found: non-existent');
    });
  });

  describe('refreshFlows', () => {
    it('刷新列表与 listFlows 返回相同结果', async () => {
      await saveFlow(createTestFlow('flow-1', '流程1'));

      const listResult = await listFlows();
      const refreshResult = await refreshFlows();

      expect(refreshResult).toEqual(listResult);
    });
  });

  describe('文件夹操作', () => {
    it('listFolders 空文件夹时返回空数组', async () => {
      const folders = await listFolders();
      expect(folders).toEqual([]);
    });

    it('createFolder 可以创建文件夹', async () => {
      const folder = await createFolder('my-flows');
      expect(folder.name).toBe('my-flows');
      expect(folder.path).toBe('my-flows');
    });

    it('createFolder 支持嵌套路径', async () => {
      const folder = await createFolder('parent/sub');
      expect(folder.name).toBe('sub');
      expect(folder.path).toBe('parent/sub');
    });

    it('deleteFolder 可以删除文件夹', async () => {
      await createFolder('my-flows');
      await deleteFolder('my-flows');

      const folders = await listFolders();
      expect(folders).toHaveLength(0);
    });

    it('renameFolder 可以重命名文件夹', async () => {
      await createFolder('old-name');
      const result = await renameFolder('old-name', 'new-name');

      expect(result.name).toBe('new-name');
      expect(result.path).toBe('new-name');

      const folders = await listFolders();
      expect(folders).toHaveLength(1);
      expect(folders[0].name).toBe('new-name');
    });

    it('moveFlowToFolder 更新流程所属文件夹', async () => {
      const flow = createTestFlow('flow-1', '测试流程');
      await saveFlow(flow);

      await moveFlowToFolder('flow-1', 'my-folder');

      const list = await listFlows();
      const meta = list.find((m) => m.uuid === 'flow-1');
      expect(meta?.folderPath).toBe('my-folder');
    });

    it('moveFlowToFolder 对不存在的流程不做处理', async () => {
      await expect(moveFlowToFolder('non-existent', 'folder')).resolves.not.toThrow();
    });

    it('moveFolder 移动文件夹到目标路径下', async () => {
      await createFolder('source/sub');
      await createFolder('source/sub/deep');

      await moveFolder('source', 'dest');

      const folders = await listFolders();
      // 原始路径不再存在
      expect(folders.find((f) => f.path === 'source')).toBeUndefined();
      // 文件夹被移动到 dest/source 下
      expect(folders.find((f) => f.path === 'dest/source')).toBeDefined();
      expect(folders.find((f) => f.path === 'dest/source/sub')).toBeDefined();
      expect(folders.find((f) => f.path === 'dest/source/sub/deep')).toBeDefined();
    });
  });

  describe('clearMockFlows', () => {
    it('清空所有流程和文件夹', async () => {
      await saveFlow(createTestFlow('flow-1', '流程1'));
      await createFolder('test-folder');
      expect(getMockFlowCount()).toBe(1);

      clearMockFlows();

      expect(getMockFlowCount()).toBe(0);
      const folders = await listFolders();
      expect(folders).toHaveLength(0);
    });
  });

  describe('集成场景', () => {
    it('保存→列出→加载→删除完整流程', async () => {
      const flow = createTestFlow('flow-1', '完整流程');
      await saveFlow(flow);

      let list = await listFlows();
      expect(list).toHaveLength(1);
      expect(list[0].uuid).toBe('flow-1');

      const loaded = await loadFlow('flow-1');
      expect(loaded.name).toBe('完整流程');

      await deleteFlow('flow-1');
      list = await listFlows();
      expect(list).toHaveLength(0);
    });

    it('文件夹和流程的混合操作', async () => {
      await createFolder('api-flows');
      await saveFlow(createTestFlow('flow-1', 'API 流程'));
      await moveFlowToFolder('flow-1', 'api-flows');

      const folders = await listFolders();
      expect(folders).toHaveLength(1);
      expect(folders[0].name).toBe('api-flows');

      const flows = await listFlows();
      const flowMeta = flows.find((m) => m.uuid === 'flow-1');
      expect(flowMeta?.folderPath).toBe('api-flows');
    });
  });
});
