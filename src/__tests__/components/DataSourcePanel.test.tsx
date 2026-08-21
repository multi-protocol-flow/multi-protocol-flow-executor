import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DataSourcePanel } from '@/components/sidebar/DataSourcePanel';
import { useDatasourceStore } from '@/stores/datasource';
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

import { datasourceBrowse } from '@/services/tauri/datasource';

describe('DataSourcePanel', () => {
  beforeEach(() => {
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
});
