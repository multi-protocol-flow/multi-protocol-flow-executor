/**
 * FlowTabBar 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FlowTabBar } from '@/components/canvas/FlowTabBar';
import {
  useFlowStore,
  useUIStore,
  useExecutionStore,
  useDebugStore,
} from '@/stores';
import { toast } from 'sonner';
import { flowStorage } from '@/services/storage';
import type { Flow } from '@/types';

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    info: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
  },
}));

const mockCopyToClipboard = vi.fn().mockResolvedValue(true);
vi.mock('@/lib/clipboard', () => ({
  copyToClipboard: (...args: unknown[]) => mockCopyToClipboard(...args),
}));

// Mock flowStorage
vi.mock('@/services/storage', () => ({
  flowStorage: {
    saveFlow: vi.fn().mockResolvedValue(undefined),
    loadFlow: vi.fn(),
    listFlows: vi.fn().mockResolvedValue([]),
  },
}));

describe('FlowTabBar', () => {
  const flow1: Flow = {
    uuid: 'flow-1',
    name: 'Flow One',
    nodes: [],
    connections: [],
  };

  const flow2: Flow = {
    uuid: 'flow-2',
    name: 'Flow Two',
    nodes: [],
    connections: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    useFlowStore.getState().reset();
    useUIStore.getState().resetPanels();
    useExecutionStore.getState().reset();
    useDebugStore.getState().reset();
  });

  it('没有标签时仅渲染新建流程按钮', () => {
    render(<FlowTabBar />);
    expect(screen.queryByRole('tab')).toBeNull();
    expect(screen.getByTitle('新建流程')).toBeDefined();
  });

  it('正确渲染多个标签及活跃状态', () => {
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    render(<FlowTabBar />);

    const tabs = screen.getAllByRole('tab');
    expect(tabs).toHaveLength(2);
    expect(screen.getByText('Flow One')).toBeDefined();
    expect(screen.getByText('Flow Two')).toBeDefined();

    // Flow Two 是最后打开的，应为活跃状态
    expect(tabs[1]).toHaveAttribute('aria-selected', 'true');
    expect(tabs[0]).toHaveAttribute('aria-selected', 'false');
  });

  it('修改流程后显示未保存黄色指示点', () => {
    useFlowStore.getState().openTab(flow1);
    expect(useFlowStore.getState().isDirty).toBe(false);

    const { rerender } = render(<FlowTabBar />);
    expect(screen.queryByTitle('未保存')).toBeNull();

    // 修改流程产生 dirty
    useFlowStore.getState().updateFlowName('Flow One Renamed');
    rerender(<FlowTabBar />);

    expect(screen.getByTitle('未保存')).toBeDefined();
  });

  it('点击非活跃标签切换活跃流程', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    render(<FlowTabBar />);

    // 当前活跃为 flow2，点击 flow1 切换
    await user.click(screen.getByText('Flow One'));

    expect(useFlowStore.getState().activeTabUuid).toBe('flow-1');
    expect(useFlowStore.getState().flow?.uuid).toBe('flow-1');
  });

  it('策略 A：流程执行中禁止切换标签并弹出提示', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    // 模拟执行中
    useExecutionStore.setState({ isExecuting: true });

    render(<FlowTabBar />);

    await user.click(screen.getByText('Flow One'));

    expect(toast.info).toHaveBeenCalledWith('执行中不可切换流程');
    // 活跃标签未改变
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-2');
  });

  it('策略 A：流程调试中禁止切换标签并弹出提示', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    // 模拟调试中
    useDebugStore.setState({ status: 'running' });

    render(<FlowTabBar />);

    await user.click(screen.getByText('Flow One'));

    expect(toast.info).toHaveBeenCalledWith('调试中不可切换流程');
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-2');
  });

  it('关闭未修改标签直接关闭', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    render(<FlowTabBar />);

    // 点击 flow2 的关闭按钮
    const closeButtons = screen.getAllByLabelText('关闭标签');
    expect(closeButtons).toHaveLength(2);

    await user.click(closeButtons[1]);

    expect(useFlowStore.getState().tabs).toHaveLength(1);
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-1');
  });

  it('关闭未保存的标签触发确认弹窗，点击放弃修改并关闭', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().updateFlowName('Dirty Flow');

    render(<FlowTabBar />);

    const closeButton = screen.getByLabelText('关闭标签');
    await user.click(closeButton);

    // 应弹出未保存确认弹窗
    expect(screen.getByText('未保存的修改')).toBeDefined();

    // 点击放弃修改并关闭
    const discardButton = screen.getByText('放弃修改并关闭');
    await user.click(discardButton);

    expect(useFlowStore.getState().tabs).toHaveLength(0);
  });

  it('关闭未保存标签点击保存并关闭，调用 saveFlow 并关闭', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().updateFlowName('Dirty Flow To Save');

    render(<FlowTabBar />);

    const closeButton = screen.getByLabelText('关闭标签');
    await user.click(closeButton);

    const saveAndCloseButton = screen.getByText('保存并关闭');
    await user.click(saveAndCloseButton);

    await waitFor(() => {
      expect(flowStorage.saveFlow).toHaveBeenCalled();
      expect(useFlowStore.getState().tabs).toHaveLength(0);
    });
  });

  it('双击标签打开流程属性配置面板', async () => {
    useFlowStore.getState().openTab(flow1);

    render(<FlowTabBar />);

    const tab = screen.getByRole('tab');
    fireEvent.doubleClick(tab);

    expect(useUIStore.getState().isConfigPanelOpen).toBe(true);
    expect(useFlowStore.getState().selectedNodeId).toBeNull();
  });

  it('鼠标中键点击直接关闭未修改标签', () => {
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    render(<FlowTabBar />);

    const tabs = screen.getAllByRole('tab');
    // button 1 = 鼠标中键
    fireEvent(tabs[1], new MouseEvent('auxclick', { button: 1, bubbles: true }));

    expect(useFlowStore.getState().tabs).toHaveLength(1);
    expect(useFlowStore.getState().activeTabUuid).toBe('flow-1');
  });

  it('默认单行滚动模式下使用 scrollbar-mini 与横向滚动容器', () => {
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);

    render(<FlowTabBar />);

    const tab = screen.getByText('Flow One');
    const container = tab.closest('[role="tab"]')?.parentElement;
    expect(container).toBeDefined();
    expect(container?.className).toContain('overflow-x-auto');
    expect(container?.className).toContain('scrollbar-mini');
  });

  it('切换为多行平铺模式后容器使用 flex-wrap 并且无横向滚动', () => {
    useFlowStore.getState().openTab(flow1);
    useFlowStore.getState().openTab(flow2);
    useUIStore.getState().setFlowTabDisplayMode('wrap');

    render(<FlowTabBar />);

    const tab = screen.getByText('Flow One');
    const container = tab.closest('[role="tab"]')?.parentElement;
    expect(container).toBeDefined();
    expect(container?.className).toContain('flex-wrap');
    expect(container?.className).not.toContain('overflow-x-auto');
  });

  it('可以通过标签栏右侧模式切换按钮切换显示模式', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useUIStore.getState().setFlowTabDisplayMode('scroll');

    render(<FlowTabBar />);

    const modeButton = screen.getByLabelText('标签显示模式');
    await user.click(modeButton);

    // 弹出菜单中包含单行滚动与多行平铺选项
    const wrapOption = screen.getByText('多行平铺');
    await user.click(wrapOption);

    expect(useUIStore.getState().flowTabDisplayMode).toBe('wrap');
  });

  it('可以通过右键上下文菜单切换显示模式', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1);
    useUIStore.getState().setFlowTabDisplayMode('scroll');

    render(<FlowTabBar />);

    const tab = screen.getByRole('tab');
    fireEvent.contextMenu(tab);

    const wrapMenuBtn = screen.getByText('多行平铺');
    await user.click(wrapMenuBtn);

    expect(useUIStore.getState().flowTabDisplayMode).toBe('wrap');
  });

  it('标签右键菜单中包含复制流程的文件绝对路径选项，并能复制成功', async () => {
    const user = userEvent.setup();
    useFlowStore.getState().openTab(flow1, '/path/to/my-flow.mpf');

    render(<FlowTabBar />);

    const tab = screen.getByRole('tab');
    fireEvent.contextMenu(tab);

    const copyPathBtn = screen.getByText('复制流程的文件绝对路径');
    expect(copyPathBtn).toBeInTheDocument();
    await user.click(copyPathBtn);

    expect(mockCopyToClipboard).toHaveBeenCalledWith('/path/to/my-flow.mpf');
    expect(toast.success).toHaveBeenCalledWith('已复制流程文件绝对路径');
  });
});
