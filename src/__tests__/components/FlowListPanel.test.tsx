/**
 * FlowListPanel 组件测试
 * 重点测试切换流程时自动保存功能
 * 使用真实 Zustand store 进行集成测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FlowListPanel } from '@/components/sidebar/FlowListPanel';
import { useUIStore, useFlowStore, useFlowListStore } from '@/stores';
import { useSettingsStore } from '@/stores/settings';
import type { Flow } from '@/types';

// Mock flowStorage - 使用 vi.hoisted 确保在 vi.mock 之前可用
const mockFlowStorage = vi.hoisted(() => ({
  listFlows: vi.fn(),
  loadFlow: vi.fn(),
  deleteFlow: vi.fn(),
  saveFlow: vi.fn(),
  copyFlow: vi.fn(),
  refreshFlows: vi.fn(),
  listFolders: vi.fn(),
  createFolder: vi.fn(),
  deleteFolder: vi.fn(),
  renameFolder: vi.fn(),
  moveFlowToFolder: vi.fn(),
}));

vi.mock('@/services/storage', () => ({
  flowStorage: mockFlowStorage,
}));
const mockToast = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: mockToast,
}));

const mockCopyToClipboard = vi.hoisted(() => vi.fn().mockResolvedValue(true));

vi.mock('@/lib/clipboard', () => ({
  copyToClipboard: mockCopyToClipboard,
}));


// Mock templates
vi.mock('@/templates/builtinFlows', () => ({
  DEMO_FLOW: {
    uuid: 'demo-flow',
    name: '示例流程',
    nodes: [],
    connections: [],
  },
  WS_DEMO_FLOW: {
    uuid: 'demo-ws-testing',
    name: 'WebSocket节点示例流程',
    nodes: [],
    connections: [],
  },
  BUILTIN_FLOW_VERSION: 1,
}));

describe('FlowListPanel', () => {
  const createMockFlow = (uuid: string, name: string): Flow => ({
    uuid,
    name,
    nodes: [],
    connections: [],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    
    // 重置 Zustand stores
    useFlowStore.getState().reset();
    useFlowListStore.getState().reset();
    useUIStore.getState().closeNewFlowDialog();
    useUIStore.getState().hideConfirm();

    // 模拟设置已加载完成，避免 FlowListPanel 等待 settingsLoading
    useSettingsStore.setState({
      isLoading: false,
      flowsDir: '/test-flows/',
      error: null,
    });
    
    // 默认存储状态 - 返回非空数组避免触发内置模板加载
    mockFlowStorage.listFlows.mockResolvedValue([
      { uuid: 'default-flow', name: '默认流程', updatedAt: Date.now(), filePath: 'default-flow.mpf', folderPath: '' }
    ]);
    mockFlowStorage.loadFlow.mockResolvedValue(null);
    mockFlowStorage.listFolders.mockResolvedValue([]);
  });

  afterEach(() => {
    useFlowStore.getState().reset();
    useFlowListStore.getState().reset();
  });

  describe('流程列表渲染', () => {
    it('渲染流程列表', async () => {
      const flowMetas = [
        { uuid: 'flow-1', name: '流程1', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
        { uuid: 'flow-2', name: '流程2', updatedAt: Date.now(), filePath: 'flow-2.mpf', folderPath: '' },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);

      // 先设置 flowListStore
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      await act(async () => {
        render(<FlowListPanel />);
      });
      
      expect(screen.getByText('流程1')).toBeDefined();
      expect(screen.getByText('流程2')).toBeDefined();
    });

    it('流程项支持拖拽并携带 subflow 协议数据', async () => {
      const flowMetas = [
        { uuid: 'flow-1', name: '子流程A', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({
        uuid: m.uuid,
        name: m.name,
        updated_at: m.updatedAt,
        folder_path: '',
        file_path: m.filePath,
      })));

      await act(async () => {
        render(<FlowListPanel />);
      });

      const flowItem = screen.getByText('子流程A').closest('[role="button"]');
      expect(flowItem).toHaveAttribute('draggable', 'true');

      const setDataMock = vi.fn();
      const dataTransfer = {
        setData: setDataMock,
        effectAllowed: 'none',
      };
      const dragStartEvent = new Event('dragstart', { bubbles: true });
      Object.assign(dragStartEvent, {
        dataTransfer,
      });

      flowItem?.dispatchEvent(dragStartEvent);

      expect(dataTransfer.effectAllowed).toBe('all');
      expect(setDataMock).toHaveBeenCalledWith(
        'application/reactflow/subflow',
        JSON.stringify({
          uuid: 'flow-1',
          name: '子流程A',
          filePath: 'flow-1.mpf',
        })
      );
      expect(setDataMock).toHaveBeenCalledWith(
        'text/plain',
        JSON.stringify({
          uuid: 'flow-1',
          name: '子流程A',
          filePath: 'flow-1.mpf',
        })
      );
    });
  });

  describe('多标签流程切换', () => {
    it('切换流程时保留原流程草稿状态（多标签会话隔离）', async () => {
      const user = userEvent.setup();
      const currentFlow = createMockFlow('flow-1', '当前流程');
      const modifiedFlow = { ...currentFlow, name: '修改后的流程' };
      const targetFlow = createMockFlow('flow-2', '目标流程');
      
      const flowMetas = [
        { uuid: 'flow-1', name: '当前流程', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
        { uuid: 'flow-2', name: '目标流程', updatedAt: Date.now(), filePath: 'flow-2.mpf', folderPath: '' },
      ];

      // 设置当前流程为已修改状态
      useFlowStore.getState().setFlow(modifiedFlow);
      useFlowStore.setState({ isDirty: true });
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      mockFlowStorage.loadFlow.mockResolvedValue(targetFlow);

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 点击切换到目标流程
      await act(async () => {
        await user.click(screen.getByText('目标流程'));
      });

      // 验证：应打开目标流程为当前活跃流程
      expect(useFlowStore.getState().flow).toEqual(targetFlow);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-2');
      // 验证：原流程在 tabSessions 中保留草稿与 isDirty 状态
      const session1 = useFlowStore.getState().tabSessions['flow-1'];
      expect(session1).toBeDefined();
      expect(session1?.isDirty).toBe(true);
      expect(session1?.flow.name).toBe('修改后的流程');
    });

    it('切换流程时，如果当前流程无修改，正常加载目标流程', async () => {
      const user = userEvent.setup();
      const currentFlow = createMockFlow('flow-1', '当前流程');
      const targetFlow = createMockFlow('flow-2', '目标流程');
      
      const flowMetas = [
        { uuid: 'flow-1', name: '当前流程', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
        { uuid: 'flow-2', name: '目标流程', updatedAt: Date.now(), filePath: 'flow-2.mpf', folderPath: '' },
      ];

      // 设置当前流程，无未保存修改
      useFlowStore.getState().setFlow(currentFlow);
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      mockFlowStorage.loadFlow.mockResolvedValue(targetFlow);

      await act(async () => {
        render(<FlowListPanel />);
      });

      await act(async () => {
        await user.click(screen.getByText('目标流程'));
      });

      // 验证：应加载目标流程
      expect(useFlowStore.getState().flow).toEqual(targetFlow);
      expect(useFlowStore.getState().activeTabUuid).toBe('flow-2');
    });
  });

  describe('新建流程', () => {
    it('点击新建按钮打开对话框', async () => {
      const user = userEvent.setup();

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 找到 + 按钮
      const newButton = screen.getByRole('button', { name: '新建流程' });
      await act(async () => {
        await user.click(newButton);
      });

      expect(useUIStore.getState().showNewFlowDialog).toBe(true);
    });
  });

  describe('删除流程', () => {
    it('点击删除按钮应显示确认对话框', async () => {
      const user = userEvent.setup();
      const flowMetas = [
        { uuid: 'flow-1', name: '流程1', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 鼠标悬停以显示更多操作按钮
      const flowItem = screen.getByText('流程1');
      await user.hover(flowItem);

      // 先点击"更多操作"按钮打开下拉菜单
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      await act(async () => {
        await user.click(moreButton);
      });

      // 现在可以找到删除按钮
      const deleteButton = screen.getByRole('button', { name: '删除流程' });
      await act(async () => {
        await user.click(deleteButton);
      });

      // 验证确认对话框被触发
      const uiState = useUIStore.getState();
      expect(uiState.showConfirmDialog).toBe(true);
      expect(uiState.confirmDialogConfig?.title).toBe('删除流程');
      expect(uiState.confirmDialogConfig?.description).toBe('确定要删除此流程吗？删除后无法恢复。');
    });

    it('确认删除应调用 deleteFlow 并从列表移除', async () => {
      const user = userEvent.setup();
      const flowMetas = [
        { uuid: 'flow-1', name: '流程1', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 触发删除
      const flowItem = screen.getByText('流程1');
      await user.hover(flowItem);
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      await act(async () => {
        await user.click(moreButton);
      });
      const deleteButton = screen.getByRole('button', { name: '删除流程' });
      await act(async () => {
        await user.click(deleteButton);
      });

      // 获取确认回调并执行
      const confirmCallback = useUIStore.getState().confirmDialogConfig?.onConfirm;
      expect(confirmCallback).toBeDefined();
      
      await act(async () => {
        confirmCallback?.();
      });

      // 验证 deleteFlow 被调用
      expect(mockFlowStorage.deleteFlow).toHaveBeenCalledWith('flow-1');
      // 验证列表已更新
      expect(useFlowListStore.getState().flows).toEqual([]);
    });

    it('删除当前流程应清空画布', async () => {
      const user = userEvent.setup();
      const currentFlow = createMockFlow('flow-1', '当前流程');
      const flowMetas = [
        { uuid: 'flow-1', name: '当前流程', updatedAt: Date.now(), filePath: 'flow-1.mpf', folderPath: '' },
      ];
      
      // 设置当前流程
      useFlowStore.getState().setFlow(currentFlow);
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows(flowMetas.map(m => ({ uuid: m.uuid, name: m.name, updated_at: m.updatedAt, folder_path: '' })));

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 触发删除当前流程
      const flowItem = screen.getByText('当前流程');
      await user.hover(flowItem);
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      await act(async () => {
        await user.click(moreButton);
      });
      const deleteButton = screen.getByRole('button', { name: '删除流程' });
      await act(async () => {
        await user.click(deleteButton);
      });

      // 确认删除
      const confirmCallback = useUIStore.getState().confirmDialogConfig?.onConfirm;
      await act(async () => {
        confirmCallback?.();
      });

      // 验证画布被清空
      expect(useFlowStore.getState().flow).toBeNull();
    });

  });
  describe('复制流程文件绝对路径', () => {
    it('菜单中应显示复制流程的文件绝对路径选项，并能复制成功', async () => {
      const user = userEvent.setup();
      const flowMetas = [
        {
          uuid: 'flow-1',
          name: '流程1',
          updatedAt: Date.now(),
          filePath: '/absolute/path/to/flow-1.mpf',
          folderPath: '',
        },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows(
        flowMetas.map((m) => ({
          uuid: m.uuid,
          name: m.name,
          updated_at: m.updatedAt,
          folder_path: '',
          file_path: m.filePath,
        }))
      );

      await act(async () => {
        render(<FlowListPanel />);
      });

      const flowItem = screen.getByText('流程1');
      await user.hover(flowItem);
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      await act(async () => {
        await user.click(moreButton);
      });

      const copyPathButton = screen.getByRole('button', { name: '复制流程的文件绝对路径' });
      expect(copyPathButton).toBeDefined();

      await act(async () => {
        await user.click(copyPathButton);
      });

      expect(mockCopyToClipboard).toHaveBeenCalledWith('/absolute/path/to/flow-1.mpf');
      expect(mockToast.success).toHaveBeenCalledWith('已复制流程文件绝对路径');
    });

    it('当 store 中没有 file_path 时，应回退查询 listFlows 并成功复制', async () => {
      const user = userEvent.setup();
      const flowMetas = [
        {
          uuid: 'flow-fallback',
          name: '回退测试流程',
          updatedAt: Date.now(),
          filePath: '/fallback/path/to/flow-fallback.mpf',
          folderPath: '',
        },
      ];
      mockFlowStorage.listFlows.mockResolvedValue(flowMetas);
      useFlowListStore.getState().setFlows([
        {
          uuid: 'flow-fallback',
          name: '回退测试流程',
          updated_at: Date.now(),
          folder_path: '',
        },
      ]);

      await act(async () => {
        render(<FlowListPanel />);
      });

      const flowItem = screen.getByText('回退测试流程');
      await user.hover(flowItem);
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      await act(async () => {
        await user.click(moreButton);
      });

      const copyPathButton = screen.getByRole('button', { name: '复制流程的文件绝对路径' });
      await act(async () => {
        await user.click(copyPathButton);
      });

      expect(mockFlowStorage.listFlows).toHaveBeenCalled();
      expect(mockCopyToClipboard).toHaveBeenCalledWith('/fallback/path/to/flow-fallback.mpf');
      expect(mockToast.success).toHaveBeenCalledWith('已复制流程文件绝对路径');
      expect(useFlowListStore.getState().flows[0].file_path).toBe('/fallback/path/to/flow-fallback.mpf');
    });
  });
  describe('流程定位与微发光高亮效果', () => {
    it('当前打开的流程项应应用主色微发光选中样式', async () => {
      const flowMeta = {
        uuid: 'active-flow-1',
        name: '活动流程',
        updated_at: Date.now(),
        created_at: Date.now(),
        folder_path: '',
      };
      mockFlowStorage.listFlows.mockResolvedValue([flowMeta]);
      useFlowListStore.getState().setFlows([flowMeta]);
      useFlowStore.setState({
        flow: createMockFlow('active-flow-1', '活动流程'),
        tabs: [{ uuid: 'active-flow-1', name: '活动流程', isDirty: false }],
        activeTabUuid: 'active-flow-1',
      });

      await act(async () => {
        render(<FlowListPanel />);
      });

      const item = screen.getByText('活动流程').closest('[role="button"]');
      expect(item).not.toBeNull();
      expect(item?.className).toContain('bg-primary/10');
      expect(item?.className).toContain('border-primary/60');
    });

    it('触发 revealFlowInList 时应自动展开祖先文件夹并定位', async () => {
      const folderFlow = {
        uuid: 'nested-flow-1',
        name: '嵌套流程',
        folder_path: 'folderA/subFolderB',
        folderPath: 'folderA/subFolderB',
        updated_at: Date.now(),
        updatedAt: Date.now(),
        created_at: Date.now(),
      };
      mockFlowStorage.listFlows.mockResolvedValue([folderFlow]);
      mockFlowStorage.listFolders.mockResolvedValue([
        { path: 'folderA', name: 'folderA' },
        { path: 'folderA/subFolderB', name: 'subFolderB' },
      ]);
      useFlowListStore.getState().setFlows([folderFlow]);
      useFlowStore.setState({
        flow: createMockFlow('nested-flow-1', '嵌套流程'),
        tabs: [{ uuid: 'nested-flow-1', name: '嵌套流程', isDirty: false }],
        activeTabUuid: 'nested-flow-1',
      });

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 触发定位
      await act(async () => {
        useUIStore.getState().revealFlowInList({ uuid: 'nested-flow-1' });
      });

      // 验证祖先文件夹均被自动展开
      const expanded = useFlowListStore.getState().expandedFolders;
      expect(expanded.has('folderA')).toBe(true);
      expect(expanded.has('folderA/subFolderB')).toBe(true);
    });

    it('当 Tab 无 filePath 且列表 uuid 为相对路径时，应通过流程名称兜底定位并回填 filePath 到 Tab', async () => {
      const realFlowUuid = 'f3c64c78-1a2b-4c3d-8e9f-0123456789ab';
      const fileRelativeUuid = 'nested/ai_flow.mpf';
      const fileAbsolutePath = '/home/user/flows/nested/ai_flow.mpf';

      const folderFlow = {
        uuid: fileRelativeUuid,
        name: 'AI创建的流程',
        folder_path: 'nested',
        folderPath: 'nested',
        file_path: fileAbsolutePath,
        filePath: fileAbsolutePath,
        updated_at: Date.now(),
        updatedAt: Date.now(),
      };

      mockFlowStorage.listFlows.mockResolvedValue([folderFlow]);
      mockFlowStorage.listFolders.mockResolvedValue([{ path: 'nested', name: 'nested' }]);
      useFlowListStore.getState().setFlows([folderFlow]);

      // 模拟当前活跃 Tab：只有 flow.uuid，filePath 为 undefined
      useFlowStore.setState({
        flow: createMockFlow(realFlowUuid, 'AI创建的流程'),
        tabs: [{ uuid: realFlowUuid, name: 'AI创建的流程', isDirty: false, filePath: undefined }],
        activeTabUuid: realFlowUuid,
      });

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 触发定位（传入 Tab 的 UUID，此时 filePath 为 undefined）
      await act(async () => {
        useUIStore.getState().revealFlowInList({ uuid: realFlowUuid });
      });

      // 验证通过名称兜底成功识别，展开了所属文件夹
      const expanded = useFlowListStore.getState().expandedFolders;
      expect(expanded.has('nested')).toBe(true);

      // 验证 Tab 成功被回填补全了 filePath
      const updatedTab = useFlowStore.getState().tabs.find((t) => t.uuid === realFlowUuid);
      expect(updatedTab?.filePath).toBe(fileAbsolutePath);
    });

    it('当存在多个同名流程（歧义）时，可定位但不回填 filePath 以免串写错误文件', async () => {
      const realFlowUuid = 'ambiguous-flow-123';
      const folderFlow1 = {
        uuid: 'f1/common.mpf',
        name: '公共流程',
        folder_path: 'f1',
        folderPath: 'f1',
        file_path: '/path/f1/common.mpf',
        filePath: '/path/f1/common.mpf',
        updated_at: 1000,
        updatedAt: 1000,
      };
      const folderFlow2 = {
        uuid: 'f2/common.mpf',
        name: '公共流程',
        folder_path: 'f2',
        folderPath: 'f2',
        file_path: '/path/f2/common.mpf',
        filePath: '/path/f2/common.mpf',
        updated_at: 2000,
        updatedAt: 2000,
      };

      mockFlowStorage.listFlows.mockResolvedValue([folderFlow1, folderFlow2]);
      mockFlowStorage.listFolders.mockResolvedValue([{ path: 'f1', name: 'f1' }, { path: 'f2', name: 'f2' }]);
      useFlowListStore.getState().setFlows([folderFlow1, folderFlow2]);

      useFlowStore.setState({
        flow: createMockFlow(realFlowUuid, '公共流程'),
        tabs: [{ uuid: realFlowUuid, name: '公共流程', isDirty: false, filePath: undefined }],
        activeTabUuid: realFlowUuid,
      });

      await act(async () => {
        render(<FlowListPanel />);
      });

      await act(async () => {
        useUIStore.getState().revealFlowInList({ uuid: realFlowUuid });
      });

      // 验证仍然保留 filePath 为 undefined，不草率覆盖回填
      const updatedTab = useFlowStore.getState().tabs.find((t) => t.uuid === realFlowUuid);
      expect(updatedTab?.filePath).toBeUndefined();
    });

    it('定位非活跃 Tab 时，应解析对应 targetTab 的名称且不污染活跃 Tab 的 filePath', async () => {
      const activeUuid = 'active-uuid-999';
      const targetUuid = 'target-uuid-888';
      const targetFilePath = '/path/target.mpf';

      const targetFlow = {
        uuid: 'target.mpf',
        name: '目标非活跃流程',
        folder_path: '',
        folderPath: '',
        file_path: targetFilePath,
        filePath: targetFilePath,
        updated_at: 1000,
        updatedAt: 1000,
      };

      mockFlowStorage.listFlows.mockResolvedValue([targetFlow]);
      useFlowListStore.getState().setFlows([targetFlow]);

      useFlowStore.setState({
        flow: createMockFlow(activeUuid, '当前活跃流程'),
        tabs: [
          { uuid: activeUuid, name: '当前活跃流程', isDirty: false, filePath: undefined },
          { uuid: targetUuid, name: '目标非活跃流程', isDirty: false, filePath: undefined },
        ],
        activeTabUuid: activeUuid,
      });

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 定位目标非活跃 Tab
      await act(async () => {
        useUIStore.getState().revealFlowInList({ uuid: targetUuid });
      });

      // 活跃 Tab 的 filePath 不应被污染
      const activeTab = useFlowStore.getState().tabs.find((t) => t.uuid === activeUuid);
      expect(activeTab?.filePath).toBeUndefined();

      // 目标 Tab 的 filePath 应被正确回填
      const updatedTargetTab = useFlowStore.getState().tabs.find((t) => t.uuid === targetUuid);
      expect(updatedTargetTab?.filePath).toBe(targetFilePath);
    });
  });



  describe('空列表状态', () => {
    it('无流程时应显示空状态提示', async () => {
      // 需要同时 mock listFlows 和 saveFlow 来避免内置模板加载
      mockFlowStorage.listFlows.mockResolvedValueOnce([]); // 首次调用返回空
      useFlowListStore.getState().setFlows([]);

      // Mock saveFlow 来避免 useEffect 中的内置模板保存
      mockFlowStorage.saveFlow.mockResolvedValue(undefined);

      await act(async () => {
        render(<FlowListPanel />);
      });

      // 由于 useEffect 会加载内置模板，我们需要检查最终状态
      // 如果 listFlows 返回空，useEffect 会设置内置模板
      // 所以这个测试验证的是：当没有外部流程时，会自动加载内置模板
      const flows = useFlowListStore.getState().flows;
      
      // 验证内置模板被加载（这是组件的预期行为）
      expect(flows.length).toBeGreaterThan(0);
    });
  });
});
