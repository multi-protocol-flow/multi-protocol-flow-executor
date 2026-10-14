import { describe, it, expect, beforeEach, vi } from 'vitest';
import { flowFileToolHandlers } from '@/services/ai/tools/flowFileTools';
import { flowStorage, type FlowMetadata } from '@/services/storage/flowStorage';
import { useFlowStore } from '@/stores/flow';
import { useFlowListStore } from '@/stores/flowList';
import type { Flow } from '@/types';

vi.mock('@/services/storage/flowStorage', () => ({
  flowStorage: {
    listFlows: vi.fn(),
    loadFlow: vi.fn(),
    saveFlow: vi.fn(),
    moveFlowToFolder: vi.fn(),
  },
}));

describe('flowFileTools', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useFlowStore.setState({
      flow: null,
      tabs: [],
      activeTabUuid: null,
      tabSessions: {},
    });
    useFlowListStore.getState().reset();
  });

  describe('create_flow', () => {
    it('创建流程时自动保存并为打开的 Tab 绑定 filePath', async () => {
      const mockList: FlowMetadata[] = [
        {
          uuid: '登录流程.mpf',
          name: '登录流程',
          filePath: '/workspace/flows/登录流程.mpf',
          updatedAt: 1000,
          folderPath: '',
        },
      ];

      vi.mocked(flowStorage.saveFlow).mockResolvedValue(undefined);
      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);

      const result = await flowFileToolHandlers.create_flow({
        name: '登录流程',
        description: '测试登录流程',
      });

      expect(result.success).toBe(true);
      expect(result.name).toBe('登录流程');
      expect(result.file_path).toBe('/workspace/flows/登录流程.mpf');
      expect(flowStorage.saveFlow).toHaveBeenCalledTimes(1);

      // 验证 Tab 打开且绑定了文件绝对路径
      const state = useFlowStore.getState();
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0].name).toBe('登录流程');
      expect(state.tabs[0].filePath).toBe('/workspace/flows/登录流程.mpf');
      expect(state.activeTabUuid).toBe(state.tabs[0].uuid);
    });

    it('指定 folder_path 时应调用 moveFlowToFolder 并绑定正确目录的文件路径', async () => {
      const mockList: FlowMetadata[] = [
        {
          uuid: 'auth/授权流程.mpf',
          name: '授权流程',
          filePath: '/workspace/flows/auth/授权流程.mpf',
          updatedAt: 1000,
          folderPath: 'auth',
        },
      ];

      vi.mocked(flowStorage.saveFlow).mockResolvedValue(undefined);
      vi.mocked(flowStorage.moveFlowToFolder).mockResolvedValue(undefined);
      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);

      const result = await flowFileToolHandlers.create_flow({
        name: '授权流程',
        folder_path: 'auth',
      });

      expect(result.success).toBe(true);
      expect(result.folder_path).toBe('auth');
      expect(result.file_path).toBe('/workspace/flows/auth/授权流程.mpf');
      expect(flowStorage.moveFlowToFolder).toHaveBeenCalledWith(result.uuid, 'auth');

      // 验证 Tab 上的路径正确绑定
      const tab = useFlowStore.getState().tabs[0];
      expect(tab.filePath).toBe('/workspace/flows/auth/授权流程.mpf');
    });
  });

  describe('open_flow', () => {
    it('按名称查找流程并为打开的 Tab 绑定 filePath', async () => {
      const mockLoadedFlow: Flow = {
        uuid: 'loaded-flow-uuid',
        name: '订单查询',
        initial_variables: {},
        nodes: [],
        connections: [],
      };

      const mockList: FlowMetadata[] = [
        {
          uuid: 'order/订单查询.mpf',
          name: '订单查询',
          filePath: '/workspace/flows/order/订单查询.mpf',
          updatedAt: 2000,
          folderPath: 'order',
        },
      ];

      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);
      vi.mocked(flowStorage.loadFlow).mockResolvedValue(mockLoadedFlow);

      const result = await flowFileToolHandlers.open_flow({
        name: '订单查询',
      });

      expect(result.success).toBe(true);
      expect(result.uuid).toBe('loaded-flow-uuid');
      expect(result.file_path).toBe('/workspace/flows/order/订单查询.mpf');
      expect(flowStorage.loadFlow).toHaveBeenCalledWith('order/订单查询.mpf');

      const tab = useFlowStore.getState().tabs[0];
      expect(tab.filePath).toBe('/workspace/flows/order/订单查询.mpf');
    });

    it('当传入内部 Flow UUID 时，通过内部扫描加载并回退解析出 filePath', async () => {
      const internalUuid = 'a1b2c3d4-e5f6-7890-abcd-ef1234567890';
      const mockLoadedFlow: Flow = {
        uuid: internalUuid,
        name: '内部UUID流程',
        initial_variables: {},
        nodes: [],
        connections: [],
      };

      const mockList: FlowMetadata[] = [
        {
          uuid: 'nested/内部UUID流程.mpf', // 列表中 uuid 为相对路径
          name: '内部UUID流程',
          filePath: '/workspace/flows/nested/内部UUID流程.mpf',
          updatedAt: 2500,
          folderPath: 'nested',
        },
      ];

      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);
      vi.mocked(flowStorage.loadFlow).mockResolvedValue(mockLoadedFlow);

      const result = await flowFileToolHandlers.open_flow({
        uuid: internalUuid,
      });

      expect(result.success).toBe(true);
      expect(result.uuid).toBe(internalUuid);
      expect(result.file_path).toBe('/workspace/flows/nested/内部UUID流程.mpf');
      expect(flowStorage.loadFlow).toHaveBeenCalledWith(internalUuid);

      const tab = useFlowStore.getState().tabs[0];
      expect(tab.filePath).toBe('/workspace/flows/nested/内部UUID流程.mpf');
    });

    it('找不到流程时抛出异常', async () => {
      vi.mocked(flowStorage.listFlows).mockResolvedValue([]);

      await expect(
        flowFileToolHandlers.open_flow({ name: '不存在的流程' }),
      ).rejects.toThrow('Flow UUID or valid name must be provided');
    });
  });

  describe('save_current_flow', () => {
    it('保存活跃流程并将文件绝对路径回填到 Tab', async () => {
      const testFlow: Flow = {
        uuid: 'flow-to-save',
        name: '未绑路径流程',
        initial_variables: {},
        nodes: [],
        connections: [],
      };

      // 初始 Tab 没有 filePath
      useFlowStore.getState().openTab(testFlow, undefined);

      const mockList: FlowMetadata[] = [
        {
          uuid: '未绑路径流程.mpf',
          name: '未绑路径流程',
          filePath: '/workspace/flows/未绑路径流程.mpf',
          updatedAt: 3000,
          folderPath: '',
        },
      ];

      vi.mocked(flowStorage.saveFlow).mockResolvedValue(undefined);
      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);

      const result = await flowFileToolHandlers.save_current_flow();

      expect(result.success).toBe(true);
      expect(result.file_path).toBe('/workspace/flows/未绑路径流程.mpf');

      // 验证 Tab 成功回填了 filePath
      const tab = useFlowStore.getState().tabs[0];
      expect(tab.filePath).toBe('/workspace/flows/未绑路径流程.mpf');
    });

    it('当前无活跃流程时抛出异常', async () => {
      await expect(flowFileToolHandlers.save_current_flow()).rejects.toThrow(
        'No active flow to save.',
      );
    });
  });

  describe('list_flows', () => {
    it('返回流程列表及其元数据', async () => {
      const mockList: FlowMetadata[] = [
        {
          uuid: 'a.mpf',
          name: 'Flow A',
          filePath: '/flows/a.mpf',
          updatedAt: 100,
          folderPath: '',
        },
      ];

      vi.mocked(flowStorage.listFlows).mockResolvedValue(mockList);

      const result = await flowFileToolHandlers.list_flows();
      expect(result.success).toBe(true);
      expect(result.count).toBe(1);
      expect(result.flows[0].name).toBe('Flow A');
      expect(result.flows[0].filePath).toBe('/flows/a.mpf');
    });
  });
});
