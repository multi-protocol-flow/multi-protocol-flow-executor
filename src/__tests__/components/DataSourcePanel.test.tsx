import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DataSourcePanel } from '@/components/sidebar/DataSourcePanel';
import { useDatasourceStore } from '@/stores/datasource';
import { switchLocale } from '@/i18n';
import type { DataItem, DatasourceDescription, DatasourceConnection } from '@/types/datasource';
const { mockDatasource, mockConnection, mockItem } = vi.hoisted(() => {
  const mockDatasource: DatasourceDescription = {
    id: 'eolink',
    display_name: 'Eolink',
    auth: { type: 'api_key' },
    resources: [
      { id: 'api', label: '接口列表', children: null, leaf: true },
    ],
  };

  const mockConnection: DatasourceConnection = {
    id: 'conn-1',
    datasource_id: 'eolink',
    display_name: 'Eolink',
    config: { api_key: 'secret' },
    created_at: '2026-08-05T00:00:00Z',
  };

  const mockItem: DataItem = {
    id: 'proj_1:api_1001',
    name: '获取用户信息',
    protocol: 'http',
    method: 'GET',
    path: '/v3/api/user/info',
    summary: '获取当前用户信息详情',
    extra: [{ label: '分组', value: '用户模块' }],
    raw: { group_name: '用户模块' },
  };

  return { mockDatasource, mockConnection, mockItem };
});

vi.mock('@/services/tauri/datasource', () => ({
  listDatasources: vi.fn().mockResolvedValue([mockDatasource]),
  datasourceBrowse: vi.fn().mockResolvedValue([mockItem]),
  datasourceGetItem: vi.fn(),
  datasourceGetTemplates: vi.fn(),
  datasourceValidateConfig: vi.fn(),
  loadConnections: vi.fn().mockResolvedValue([mockConnection]),
  saveConnections: vi.fn(),
}));

import { listDatasources, datasourceBrowse, loadConnections } from '@/services/tauri/datasource';

describe('DataSourcePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    switchLocale('zh-CN');
    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue([mockItem]);
    useDatasourceStore.getState().reset();
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': [mockItem],
      },
    });
  });

  it('renders concise item row with type and name, with url/extra in hover title', async () => {
    render(<DataSourcePanel />);

    // Wait for group folder to appear and expand it
    await waitFor(() => {
      expect(screen.getByText('用户模块')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('用户模块'));

    // Wait for item and method badge to be rendered inside the group
    await waitFor(() => {
      expect(screen.getByText('获取用户信息')).toBeInTheDocument();
      expect(screen.getByText('GET')).toBeInTheDocument();
      expect(screen.queryByText('/v3/api/user/info')).not.toBeInTheDocument();
      expect(screen.queryByText('分组: 用户模块')).not.toBeInTheDocument();

      const itemElement = screen.getByText('获取用户信息').closest('[title]');
      expect(itemElement).toBeInTheDocument();
      const title = itemElement?.getAttribute('title') ?? '';
      expect(title).toContain('[GET] 获取用户信息');
      expect(title).toContain('GET /v3/api/user/info');
      expect(title).toContain('获取当前用户信息详情');
      expect(title).toContain('分组: 用户模块');
    });
  });

  it('renders ungrouped items concisely', async () => {
    const ungroupedItem: DataItem = {
      id: 'proj_1:api_1002',
      name: '创建订单',
      protocol: 'http',
      method: 'POST',
      path: '/v3/api/orders',
      summary: '创建新订单接口',
      extra: [],
    };

    vi.mocked(datasourceBrowse).mockResolvedValue([ungroupedItem]);
    useDatasourceStore.setState({
      browseCache: {
        'eolink/api/': [ungroupedItem],
      },
    });

    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('创建订单')).toBeInTheDocument();
      expect(screen.getByText('POST')).toBeInTheDocument();
      expect(screen.queryByText('/v3/api/orders')).not.toBeInTheDocument();

      const itemElement = screen.getByText('创建订单').closest('[title]');
      expect(itemElement).toBeInTheDocument();
      const title = itemElement?.getAttribute('title') ?? '';
      expect(title).toContain('[POST] 创建订单');
      expect(title).toContain('POST /v3/api/orders');
      expect(title).toContain('创建新订单接口');
    });
  });

  it('renders default group in en-US when items have named groups and empty groups', async () => {
    switchLocale('en-US');
    const itemsWithEmptyGroup: DataItem[] = [
      mockItem, // has group '用户模块'
      {
        id: 'proj_1:api_1003',
        name: '未分类接口',
        protocol: 'http',
        method: 'GET',
        path: '/v3/api/misc',
        summary: '杂项接口',
        extra: [],
      },
    ];

    vi.mocked(datasourceBrowse).mockResolvedValue(itemsWithEmptyGroup);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': itemsWithEmptyGroup,
      },
    });

    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('Default Group')).toBeInTheDocument();
      expect(screen.getByText('用户模块')).toBeInTheDocument();
    });
  });

  it('renders multi-level nested groups recursively', async () => {
    switchLocale('zh-CN');
    const nestedItems: DataItem[] = [
      {
        id: 'proj_1:api_2001',
        name: 'OAuth登录',
        protocol: 'http',
        method: 'POST',
        path: '/oauth/token',
        summary: 'OAuth 登录接口',
        extra: [{ label: '分组', value: '用户中心/OAuth认证/Token管理' }],
      },
      {
        id: 'proj_1:api_2002',
        name: '查看个人资料',
        protocol: 'http',
        method: 'GET',
        path: '/users/info',
        summary: '用户资料',
        extra: [{ label: '分组', value: '用户中心/个人资料' }],
      },
    ];
    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue(nestedItems);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': nestedItems,
      },
    });
    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('已连接')).toBeInTheDocument();
      expect(screen.getByText('用户中心')).toBeInTheDocument();
      expect(screen.getByText('用户中心').parentElement).toHaveTextContent('(2)');
    });

    expect(screen.queryByText('OAuth认证')).not.toBeInTheDocument();
    expect(screen.queryByText('个人资料')).not.toBeInTheDocument();

    // Expand Level 1: "用户中心"
    fireEvent.click(screen.getByText('用户中心'));

    // Level 2 subfolders become visible
    await waitFor(() => {
      expect(screen.getByText('OAuth认证')).toBeInTheDocument();
      expect(screen.getByText('个人资料')).toBeInTheDocument();
    });

    // Expand "OAuth认证" -> Level 3 "Token管理" appears
    fireEvent.click(screen.getByText('OAuth认证'));
    await waitFor(() => {
      expect(screen.getByText('Token管理')).toBeInTheDocument();
    });

    // Expand "Token管理" -> "OAuth登录" item appears
    fireEvent.click(screen.getByText('Token管理'));
    await waitFor(() => {
      expect(screen.getByText('OAuth登录')).toBeInTheDocument();
      expect(screen.getByText('POST')).toBeInTheDocument();
    });

    // Expand "个人资料" -> "查看个人资料" item appears
    fireEvent.click(screen.getByText('个人资料'));
    await waitFor(() => {
      expect(screen.getByText('查看个人资料')).toBeInTheDocument();
      expect(screen.getByText('GET')).toBeInTheDocument();
    });
  });

  it('supports alternative path separators like > and \\\\ in group paths', async () => {
    switchLocale('zh-CN');
    const sepItems: DataItem[] = [
      {
        id: 'proj_1:api_3001',
        name: '订单支付',
        protocol: 'http',
        method: 'POST',
        path: '/pay',
        extra: [{ label: '分组', value: '商城系统 > 支付模块 > 微信支付' }],
      },
      {
        id: 'proj_1:api_3002',
        name: '订单退款',
        protocol: 'http',
        method: 'POST',
        extra: [{ label: '分组', value: '商城系统\\售后模块\\退款' }],
      },
    ];
    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue(sepItems);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': sepItems,
      },
    });

    render(<DataSourcePanel />);
    await waitFor(() => {
      expect(screen.getByText('已连接')).toBeInTheDocument();
      expect(screen.getByText('商城系统')).toBeInTheDocument();
      expect(screen.getByText('商城系统').parentElement).toHaveTextContent('(2)');
    });
    fireEvent.click(screen.getByText('商城系统'));

    await waitFor(() => {
      expect(screen.getByText('支付模块')).toBeInTheDocument();
      expect(screen.getByText('售后模块')).toBeInTheDocument();
    });
  });

  it('resolves numeric group_path ID chains ("1906207,1906234,1906268") into readable nested hierarchy', async () => {
    switchLocale('zh-CN');
    const eolinkIdItems: DataItem[] = [
      {
        id: 'proj_1:api_4001',
        name: '系统设置接口',
        protocol: 'http',
        method: 'GET',
        path: '/sys/setting',
        extra: [],
        raw: {
          group_id: 1906207,
          group_name: '系统管理',
        },
      },
      {
        id: 'proj_1:api_4002',
        name: '用户列表接口',
        protocol: 'http',
        method: 'GET',
        path: '/users',
        extra: [],
        raw: {
          group_id: 1906234,
          group_name: '用户管理',
          group_path: '1906207,1906234',
        },
      },
      {
        id: 'proj_1:api_4003',
        name: '获取租户列表',
        protocol: 'http',
        method: 'GET',
        path: '/tenants',
        extra: [],
        raw: {
          group_id: 1906268,
          group_name: '租户管理（系统管理员）',
          group_path: '1906207,1906234,1906268',
        },
      },
    ];

    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue(eolinkIdItems);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': eolinkIdItems,
      },
    });

    render(<DataSourcePanel />);

    // Level 1: "系统管理" folder appears, not raw ID "1906207"
    await waitFor(() => {
      expect(screen.getByText('系统管理')).toBeInTheDocument();
      expect(screen.queryByText('1906207')).not.toBeInTheDocument();
      expect(screen.queryByText('1906207,1906234,1906268')).not.toBeInTheDocument();
    });

    // Expand Level 1: "系统管理" -> Level 2 "用户管理" & "系统设置接口" appear
    fireEvent.click(screen.getByText('系统管理'));
    await waitFor(() => {
      expect(screen.getByText('用户管理')).toBeInTheDocument();
      expect(screen.getByText('系统设置接口')).toBeInTheDocument();
    });

    // Expand Level 2: "用户管理" -> Level 3 "租户管理（系统管理员）" & "用户列表接口" appear
    fireEvent.click(screen.getByText('用户管理'));
    await waitFor(() => {
      expect(screen.getByText('租户管理（系统管理员）')).toBeInTheDocument();
      expect(screen.getByText('用户列表接口')).toBeInTheDocument();
    });

    // Expand Level 3: "租户管理（系统管理员）" -> "获取租户列表" appears
    fireEvent.click(screen.getByText('租户管理（系统管理员）'));
    await waitFor(() => {
      expect(screen.getByText('获取租户列表')).toBeInTheDocument();
    });
  });

  it('falls back to group_name when numeric group_path IDs cannot be resolved, never showing raw numbers', async () => {
    switchLocale('zh-CN');
    const unresolvedItems: DataItem[] = [
      {
        id: 'proj_1:api_5001',
        name: '未关联父组接口',
        protocol: 'http',
        method: 'GET',
        path: '/unknown',
        extra: [{ label: '分组', value: '9999991,9999992,9999993' }],
        raw: {
          group_id: 9999993,
          group_name: '特殊功能模块',
          group_path: '9999991,9999992,9999993',
        },
      },
    ];

    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue(unresolvedItems);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': unresolvedItems,
      },
    });

    render(<DataSourcePanel />);

    // Displays group_name "特殊功能模块", NEVER raw number "9999991" or "9999991,9999992,9999993"
    await waitFor(() => {
      expect(screen.getByText('特殊功能模块')).toBeInTheDocument();
      expect(screen.queryByText('9999991')).not.toBeInTheDocument();
      expect(screen.queryByText('9999991,9999992,9999993')).not.toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('特殊功能模块'));
    await waitFor(() => {
      expect(screen.getByText('未关联父组接口')).toBeInTheDocument();
    });
  });

  it('supports searching items by name, path, and directory with auto-expansion', async () => {
    switchLocale('zh-CN');
    const searchTestItems: DataItem[] = [
      {
        id: 'api_1',
        name: '获取用户信息',
        protocol: 'http',
        method: 'GET',
        path: '/v1/users/profile',
        summary: '查询用户个人信息',
        extra: [{ label: '分组', value: '用户模块' }],
        raw: { group_name: '用户模块' },
      },
      {
        id: 'api_2',
        name: '提交订单',
        protocol: 'http',
        method: 'POST',
        path: '/v1/orders/create',
        summary: '创建新订单',
        extra: [{ label: '分组', value: '交易系统' }],
        raw: { group_name: '交易系统' },
      },
    ];

    vi.mocked(listDatasources).mockResolvedValue([mockDatasource]);
    vi.mocked(loadConnections).mockResolvedValue([mockConnection]);
    vi.mocked(datasourceBrowse).mockResolvedValue(searchTestItems);
    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': searchTestItems,
      },
    });

    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('用户模块')).toBeInTheDocument();
      expect(screen.getByText('交易系统')).toBeInTheDocument();
    });

    const searchInput = screen.getByTestId('datasource-search-input');
    expect(searchInput).toBeInTheDocument();

    // 1. 按名称检索：输入“用户”，匹配“获取用户信息”，自动展开“用户模块”，过滤掉“交易系统”
    fireEvent.change(searchInput, { target: { value: '用户' } });
    await waitFor(() => {
      expect(screen.getByText('获取用户信息')).toBeInTheDocument();
      expect(screen.queryByText('交易系统')).not.toBeInTheDocument();
    });

    // 2. 按路径检索：输入“/orders/create”，匹配“提交订单”，自动展开“交易系统”
    fireEvent.change(searchInput, { target: { value: '/orders/create' } });
    await waitFor(() => {
      expect(screen.getByText('提交订单')).toBeInTheDocument();
      expect(screen.queryByText('获取用户信息')).not.toBeInTheDocument();
    });

    // 3. 按目录检索：输入“交易”，整个“交易系统”目录及其条目展示
    fireEvent.change(searchInput, { target: { value: '交易' } });
    await waitFor(() => {
      expect(screen.getByText('交易系统')).toBeInTheDocument();
      expect(screen.getByText('提交订单')).toBeInTheDocument();
      expect(screen.queryByText('用户模块')).not.toBeInTheDocument();
    });

    // 4. 清空检索：点击清空按钮，恢复所有目录
    const clearBtn = screen.getByTestId('datasource-search-clear');
    fireEvent.click(clearBtn);
    await waitFor(() => {
      expect(screen.getByText('用户模块')).toBeInTheDocument();
      expect(screen.getByText('交易系统')).toBeInTheDocument();
    });
  });

  it('shows no_search_results when search matches nothing', async () => {
    switchLocale('zh-CN');
    const searchTestItems: DataItem[] = [
      {
        id: 'api_1',
        name: '获取用户信息',
        protocol: 'http',
        method: 'GET',
        path: '/v1/users',
        summary: '查询用户',
        extra: [{ label: '分组', value: '用户' }],
        raw: { group_name: '用户' },
      },
    ];

    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [mockConnection],
      browseCache: {
        'eolink/api/': searchTestItems,
      },
    });

    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('用户')).toBeInTheDocument();
    });

    const searchInput = screen.getByTestId('datasource-search-input');
    fireEvent.change(searchInput, { target: { value: 'non_existent_keyword_xyz' } });

    await waitFor(() => {
      expect(screen.getByText('未找到匹配的数据项')).toBeInTheDocument();
      expect(screen.queryByText('获取用户信息')).not.toBeInTheDocument();
    });
  });

  it('renders multiple connections under the same datasource plugin with independent titles', async () => {
    const connSpace1: DatasourceConnection = {
      id: 'conn-space-1',
      datasource_id: 'eolink',
      name: '支付空间',
      display_name: 'Eolink',
      config: { space_id: '101' },
      created_at: '2026-08-05T00:00:00Z',
    };
    const connSpace2: DatasourceConnection = {
      id: 'conn-space-2',
      datasource_id: 'eolink',
      name: '用户空间',
      display_name: 'Eolink',
      config: { space_id: '202' },
      created_at: '2026-08-06T00:00:00Z',
    };

    useDatasourceStore.setState({
      descriptions: [mockDatasource],
      connections: [connSpace1, connSpace2],
      browseCache: {
        'conn-space-1/api/': [mockItem],
      },
    });

    render(<DataSourcePanel />);

    await waitFor(() => {
      expect(screen.getByText('2 个连接')).toBeInTheDocument();
      expect(screen.getByText('支付空间')).toBeInTheDocument();
      expect(screen.getByText('用户空间')).toBeInTheDocument();
      expect(screen.getByText('用户模块')).toBeInTheDocument();
    });
  });
});
