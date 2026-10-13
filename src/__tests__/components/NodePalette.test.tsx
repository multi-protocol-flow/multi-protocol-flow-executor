import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodePalette } from '@/components/canvas/NodePalette';
import { useUIStore, useFlowStore } from '@/stores';
import { buildCatalogGroups, convertToCatalogItem } from '@/lib/NodeRegistry';
import type { NodeMetadata } from '@/types/nodeRegistry';

describe('NodeRegistry Catalog Grouping', () => {
  const sampleMetadata: NodeMetadata[] = [
    {
      type_id: 'http:request',
      display_name: 'HTTP 请求',
      category: 'http',
      icon: 'globe',
      color: '#3B82F6',
      ports: [{ id: 'flow_in', label: 'In', position: 'top' }],
      default_config: {},
    },
    {
      type_id: 'conditional',
      display_name: '条件判断',
      category: 'logic',
      icon: 'git-branch',
      color: '#F97316',
      ports: [{ id: 'flow_in', label: 'In', position: 'top' }],
      default_config: {},
    },
    {
      type_id: 'redis:connect',
      display_name: 'Redis 连接',
      category: 'database',
      icon: 'database',
      color: '#DC2626',
      ports: [{ id: 'flow_in', label: 'In', position: 'top' }],
      default_config: {},
    },
    {
      type_id: 'redis:command',
      display_name: 'Redis 命令',
      category: 'database',
      icon: 'terminal',
      color: '#DC2626',
      ports: [{ id: 'flow_in', label: 'In', position: 'top' }],
      default_config: {},
    },
  ];

  it('正确构建插件分组与核心分类', () => {
    const groups = buildCatalogGroups(sampleMetadata, ['http:request']);
    expect(groups.length).toBeGreaterThan(0);

    const allGroup = groups.find((g) => g.id === 'all');
    expect(allGroup).toBeDefined();
    expect(allGroup?.count).toBe(4);

    const recentGroup = groups.find((g) => g.id === 'recent');
    expect(recentGroup).toBeDefined();
    expect(recentGroup?.count).toBe(1);

    const redisGroup = groups.find((g) => g.id === 'plugin:redis');
    expect(redisGroup).toBeDefined();
    expect(redisGroup?.count).toBe(2);
    expect(redisGroup?.name).toContain('Redis');
  });

  it('内置节点的命名空间前缀不会被误判为插件', () => {
    // 内置节点使用 `proto:op` 命名空间（http:ingress），与插件 id 形状相同，
    // 判定必须依据内置目录而非 `:` 字符。
    const builtin = convertToCatalogItem({
      type_id: 'http:ingress',
      display_name: 'HTTP Ingress',
      category: 'network',
      icon: 'log-in',
      color: '#6366F1',
      ports: [{ id: 'out', label: 'Out', position: 'right' }],
      default_config: {},
    });
    expect(builtin.isPlugin).toBe(false);
    expect(builtin.pluginId).toBeUndefined();
    expect(builtin.pluginName).toBeUndefined();

    const plugin = convertToCatalogItem({
      type_id: 'grpc:connect',
      display_name: 'gRPC Connect',
      category: 'grpc',
      icon: 'plug',
      color: '#6366F1',
      ports: [{ id: 'out', label: 'Out', position: 'right' }],
      default_config: {},
    });
    expect(plugin.isPlugin).toBe(true);
    expect(plugin.pluginName).toBe('Grpc');
  });
});

describe('NodePalette Component', () => {
  beforeEach(() => {
    localStorage.setItem('locale', 'zh-CN');
    useUIStore.setState({
      isNodePaletteOpen: true,
      nodePalettePosition: { x: 100, y: 100 },
    });
    useFlowStore.setState({
      flow: {
        uuid: 'test-flow',
        name: 'Test Flow',
        description: '',
        nodes: [],
        connections: [],
        initial_variables: {},
      },
    });
  });
  it('中文模式下应渲染中文搜索框和分组', async () => {
    localStorage.setItem('locale', 'zh-CN');
    render(<NodePalette />);
    expect(screen.getByPlaceholderText(/搜索节点/)).toBeDefined();
    expect(screen.getByText('全部节点')).toBeDefined();
    expect(screen.getByText('核心逻辑')).toBeDefined();
  });

  it('英文模式下应渲染英文搜索框和分组', async () => {
    localStorage.setItem('locale', 'en-US');
    render(<NodePalette />);
    expect(screen.getByPlaceholderText(/Search nodes/)).toBeDefined();
    expect(screen.getByText('All Nodes')).toBeDefined();
    expect(screen.getByText('Core Logic')).toBeDefined();
  });

  it('输入搜索词能过滤并展示算子', async () => {
    localStorage.setItem('locale', 'zh-CN');
    render(<NodePalette />);
    const searchInput = screen.getByPlaceholderText(/搜索节点/);
    fireEvent.change(searchInput, { target: { value: 'http' } });

    expect(screen.getAllByText('HTTP 请求').length).toBeGreaterThan(0);
  });

  it('当流程缺少 entry 节点时，节点选择器中包含入口节点且可点击添加', async () => {
    localStorage.setItem('locale', 'zh-CN');
    useFlowStore.setState({
      flow: {
        uuid: 'flow-no-entry',
        name: 'No Entry Flow',
        nodes: [],
        connections: [],
        initial_variables: {},
      },
    });

    render(<NodePalette />);
    const searchInput = screen.getByPlaceholderText(/搜索节点/);
    fireEvent.change(searchInput, { target: { value: 'entry' } });
    const entryItems = screen.getAllByText('入口');
    expect(entryItems.length).toBeGreaterThan(0);
    fireEvent.click(entryItems[0]);
    expect(useFlowStore.getState().pendingAddNodeType).toBe('entry');
  });

  it('当流程已有 entry 节点时，入口节点显示已存在标识且无法重复添加', async () => {
    localStorage.setItem('locale', 'zh-CN');
    useFlowStore.setState({
      flow: {
        uuid: 'flow-with-entry',
        name: 'With Entry Flow',
        nodes: [
          { uuid: 'entry-1', type: 'entry', name: '入口', position: { x: 0, y: 0 }, data: { type: 'entry' } },
        ],
        connections: [],
        initial_variables: {},
      },
      pendingAddNodeType: null,
    });

    render(<NodePalette />);
    const searchInput = screen.getByPlaceholderText(/搜索节点/);
    fireEvent.change(searchInput, { target: { value: 'entry' } });

    expect(screen.getByText('已存在')).toBeInTheDocument();
    const entryItems = screen.getAllByText('入口');
    fireEvent.click(entryItems[0]);
    // 重复点击不会将 entry 设置到 pendingAddNodeType
    expect(useFlowStore.getState().pendingAddNodeType).toBeNull();
  });
});
