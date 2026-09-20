import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { LeftSidebarPanel } from '@/components/sidebar/LeftSidebarPanel';
import { useUIStore, useDebugStore, useDatasourceStore, useFlowStore } from '@/stores';
import { switchLocale } from '@/i18n';
import type { Flow } from '@/types';

// Mock flowStorage
vi.mock('@/services/storage', () => ({
  flowStorage: {
    listFlows: vi.fn().mockResolvedValue([]),
    loadFlow: vi.fn(),
    deleteFlow: vi.fn(),
    saveFlow: vi.fn(),
    copyFlow: vi.fn(),
    refreshFlows: vi.fn(),
    listFolders: vi.fn().mockResolvedValue([]),
    createFolder: vi.fn(),
    deleteFolder: vi.fn(),
    renameFolder: vi.fn(),
    moveFlowToFolder: vi.fn(),
  },
}));

vi.mock('@/services/tauri/datasource', () => ({
  listDatasources: vi.fn().mockResolvedValue([]),
  loadConnections: vi.fn().mockResolvedValue([]),
  saveConnections: vi.fn().mockResolvedValue(undefined),
  datasourceBrowse: vi.fn().mockResolvedValue([]),
  datasourceValidateConfig: vi.fn().mockResolvedValue({ valid: true }),
}));

describe('LeftSidebarPanel', () => {
  beforeEach(() => {
    switchLocale('zh-CN');
    useUIStore.getState().resetPanels();
    useDebugStore.getState().reset();
    useDatasourceStore.getState().reset();
    useFlowStore.setState({ flow: null });
  });

  it('renders data-source panel collapsed by default', async () => {
    render(<LeftSidebarPanel />);

    const datasourcePanelHeader = screen.getByText('数据源');
    expect(datasourcePanelHeader).toBeInTheDocument();

    const datasourceHeaderButton = datasourcePanelHeader.closest('[role="button"]');
    expect(datasourceHeaderButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('无已安装的数据源插件')).not.toBeInTheDocument();

    // 点击展开
    fireEvent.click(datasourcePanelHeader);
    expect(datasourceHeaderButton).toHaveAttribute('aria-expanded', 'true');
    await waitFor(() => {
      expect(screen.getByText('无已安装的数据源插件')).toBeInTheDocument();
    });
  });

  it('places collapsed data-source panel at bottom with mt-auto', () => {
    render(<LeftSidebarPanel />);

    const datasourcePanelHeader = screen.getByText('数据源');
    const panelContainer = datasourcePanelHeader.closest('.flex-col');
    expect(panelContainer).toHaveClass('mt-auto');
  });

  it('keeps flow list collapsed when closing data-source while flow list was collapsed, docking data-source to bottom', async () => {
    render(<LeftSidebarPanel />);

    const flowListHeader = screen.getByText('流程列表');
    const datasourceHeader = screen.getByText('数据源');

    // 展开数据源
    fireEvent.click(datasourceHeader);
    expect(datasourceHeader.closest('[role="button"]')).toHaveAttribute('aria-expanded', 'true');

    // 折叠流程列表（数据源置顶）
    fireEvent.click(flowListHeader);
    expect(useUIStore.getState().isFlowListCollapsed).toBe(true);

    // 关闭数据源：流程列表应保持关闭状态（不自动打开），数据源置底 (mt-auto)
    fireEvent.click(datasourceHeader);
    expect(datasourceHeader.closest('[role="button"]')).toHaveAttribute('aria-expanded', 'false');
    expect(useUIStore.getState().isFlowListCollapsed).toBe(true);
    expect(datasourceHeader.closest('.flex-col')).toHaveClass('mt-auto');
  });

  it('renders top resize handle when both panels are expanded', async () => {
    render(<LeftSidebarPanel />);

    const datasourceHeader = screen.getByText('数据源');
    const datasourceButton = datasourceHeader.closest('[role="button"]');
    // 默认折叠：数据源自身不产生 resize handle（其余展开面板的 handle 数量保持不变）
    expect(datasourceButton).toHaveAttribute('aria-expanded', 'false');
    const separatorsBefore = screen.queryAllByRole('separator').length;

    // 展开数据源（此时流程列表也展开）
    fireEvent.click(datasourceHeader);
    expect(datasourceButton).toHaveAttribute('aria-expanded', 'true');
    expect(screen.queryAllByRole('separator')).toHaveLength(separatorsBefore + 1);
    await waitFor(() => {
      expect(screen.getByText('无已安装的数据源插件')).toBeInTheDocument();
    });
  });

  it('环境管理面板支持折叠与拖拽调高，且标题栏不含新增环境按钮', async () => {
    render(<LeftSidebarPanel />);

    const envHeader = screen.getByText('环境管理');
    const envButton = envHeader.closest('[role="button"]');
    expect(envButton).toHaveAttribute('aria-expanded', 'true');

    // 展开态提供 resize handle
    expect(screen.queryAllByRole('separator').length).toBeGreaterThan(0);

    // 标题栏只提供「管理环境」入口，不再内联新增环境按钮
    expect(screen.getByLabelText('管理环境')).toBeInTheDocument();
    expect(screen.queryByTitle('新建环境')).not.toBeInTheDocument();

    // 折叠后 handle 数量减少
    const separatorsExpanded = screen.queryAllByRole('separator').length;
    fireEvent.click(envHeader);
    expect(envButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryAllByRole('separator')).toHaveLength(separatorsExpanded - 1);
  });

  it('当有打开的流程时渲染流程节点大纲面板并展示节点列表', async () => {
    const sampleFlow: Flow = {
      uuid: 'flow-sidebar-test',
      name: '侧边栏测试流程',
      nodes: [
        {
          uuid: 'n-entry',
          name: '开始节点',
          type: 'entry',
          position: { x: 0, y: 0 },
          data: { type: 'entry' },
        },
        {
          uuid: 'n-http',
          name: '查询用户信息',
          type: 'http:request',
          position: { x: 50, y: 50 },
          data: { type: 'http:request', url: 'https://api.example.com', method: 'get' },
        },
      ],
      connections: [],
    };

    useFlowStore.setState({ flow: sampleFlow });
    render(<LeftSidebarPanel />);

    // 显示流程节点面板标题及数量徽章
    const outlineHeader = screen.getByText('流程节点');
    expect(outlineHeader).toBeInTheDocument();
    expect(screen.getByText('(2)')).toBeInTheDocument();

    // 显示节点列表项
    expect(screen.getByText('开始节点')).toBeInTheDocument();
    expect(screen.getByText('查询用户信息')).toBeInTheDocument();

    // 点击折叠节点大纲
    fireEvent.click(outlineHeader);
    const outlineButton = outlineHeader.closest('[role="button"]');
    expect(outlineButton).toHaveAttribute('aria-expanded', 'false');
  });
});
