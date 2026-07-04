/**
 * 存储服务测试
 * 测试 flowStorage 对象 API (async IPC-based)
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  flowStorage,
} from '@/services/storage';
import { invoke } from '@tauri-apps/api/core';
import type { Flow } from '@/types';

// Mock Tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock settings
vi.mock('@/services/tauri/settings', () => ({
  getAppDataDir: vi.fn().mockResolvedValue('/test/appdata'),
}));

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
  const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    // 设置 Tauri 环境标志：使 isTauriEnv() 返回 true，路由到 invoke() 路径
    (window as any).__TAURI__ = true;
  });

  afterEach(() => {
    delete (window as any).__TAURI__;
  });

  describe('saveFlow', () => {
    it('调用 IPC 保存流程（无 _meta）', async () => {
      const flow = createTestFlow('test-1', '测试流程');
      mockInvoke.mockResolvedValue(undefined);

      await flowStorage.saveFlow(flow);

      expect(invoke).toHaveBeenCalledWith('save_flow_file', {
        flow,
        flowsDir: '/test/appdata/flows/',
        meta: null,
      });
    });

    it('调用 IPC 保存流程（带 _meta）', async () => {
      const flow = { ...createTestFlow('test-1', '测试流程'), _meta: { version: 5, isBuiltin: true } };
      mockInvoke.mockResolvedValue(undefined);

      await flowStorage.saveFlow(flow);

      expect(invoke).toHaveBeenCalledWith('save_flow_file', {
        flow: createTestFlow('test-1', '测试流程'),
        flowsDir: '/test/appdata/flows/',
        meta: { version: 5, isBuiltin: true },
      });
    });
  });

  describe('loadFlow', () => {
    it('调用 IPC 加载流程（无 _meta）', async () => {
      const flow = createTestFlow('test-1', '测试流程');
      mockInvoke.mockResolvedValue({ _version: 1, flow });

      const loaded = await flowStorage.loadFlow('test-1');

      expect(invoke).toHaveBeenCalledWith('load_flow_file', {
        uuid: 'test-1',
        flowsDir: '/test/appdata/flows/',
      });
      expect(loaded.uuid).toBe('test-1');
      expect(loaded.name).toBe('测试流程');
    });

    it('调用 IPC 加载流程（带 _meta）', async () => {
      const flow = createTestFlow('test-1', '测试流程');
      const _meta = { version: 5, isBuiltin: true };
      mockInvoke.mockResolvedValue({ _version: 1, flow, _meta });

      const loaded = await flowStorage.loadFlow('test-1') as typeof flow & { _meta?: typeof _meta };

      expect(invoke).toHaveBeenCalledWith('load_flow_file', {
        uuid: 'test-1',
        flowsDir: '/test/appdata/flows/',
      });
      expect(loaded.uuid).toBe('test-1');
      expect(loaded._meta).toEqual(_meta);
    });
  });

  describe('listFlows', () => {
    it('调用 IPC 列出流程', async () => {
      const flowMetas = [
        { uuid: 'test-1', name: '流程1', filePath: 'test-1.mpf', updatedAt: Date.now() },
      ];
      mockInvoke.mockResolvedValue(flowMetas);

      const list = await flowStorage.listFlows();

      expect(invoke).toHaveBeenCalledWith('list_flow_files', {
        flowsDir: '/test/appdata/flows/',
      });
      expect(list).toEqual(flowMetas);
      expect(list).toHaveLength(1);
      expect(list[0].name).toBe('流程1');
    });

    it('空结果返回空数组', async () => {
      mockInvoke.mockResolvedValue([]);

      const list = await flowStorage.listFlows();
      expect(list).toEqual([]);
    });
  });

  describe('deleteFlow', () => {
    it('调用 IPC 删除流程', async () => {
      mockInvoke.mockResolvedValue(undefined);

      await flowStorage.deleteFlow('test-1');

      expect(invoke).toHaveBeenCalledWith('delete_flow_file', {
        uuid: 'test-1',
        flowsDir: '/test/appdata/flows/',
      });
    });
  });

  describe('copyFlow', () => {
    it('调用 IPC 复制流程', async () => {
      const copiedFlow = createTestFlow('new-uuid', '流程1 (副本)');
      mockInvoke.mockResolvedValue(copiedFlow);

      const result = await flowStorage.copyFlow('test-1');

      expect(invoke).toHaveBeenCalledWith('copy_flow_file', {
        uuid: 'test-1',
        flowsDir: '/test/appdata/flows/',
      });
      expect(result).toEqual(copiedFlow);
    });
  });

  describe('refreshFlows', () => {
    it('调用 IPC 刷新流程列表', async () => {
      const flowMetas = [
        { uuid: 'test-1', name: '流程1', filePath: 'test-1.mpf', updatedAt: Date.now() },
      ];
      mockInvoke.mockResolvedValue(flowMetas);

      const list = await flowStorage.refreshFlows();

      expect(invoke).toHaveBeenCalledWith('list_flow_files', {
        flowsDir: '/test/appdata/flows/',
      });
      expect(list).toEqual(flowMetas);
    });
  });
});
