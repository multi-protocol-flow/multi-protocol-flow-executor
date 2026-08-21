import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DatasourceItemDetailDialog } from '@/components/dialogs/DatasourceItemDetailDialog';
import { useFlowStore, useDatasourceStore } from '@/stores';
import { switchLocale } from '@/i18n';
import type { DataItem, DatasourceDescription } from '@/types/datasource';
import type * as DatasourceServiceModule from '@/services/tauri/datasource';
vi.mock('@/services/tauri/datasource', async (importOriginal) => {
  const actual = await importOriginal<typeof DatasourceServiceModule>();
  return {
    ...actual,
    saveConnections: vi.fn().mockResolvedValue(undefined),
    datasourceGetItem: vi.fn().mockResolvedValue({
      id: 'proj_1:api_1001',
      name: '获取用户信息',
      protocol: 'http',
      method: 'GET',
      path: '/v3/api/user/info',
      summary: '获取当前用户信息详情',
      extra: [{ label: '分组', value: '用户模块' }, { label: '状态', value: '已发布' }],
      raw: {
        api_name: '获取用户信息',
        api_uri: '/v3/api/user/info',
        api_request_type: 1,
        response_data: { user_id: 123, username: 'alice' },
      },
    }),
    datasourceGetTemplates: vi.fn().mockResolvedValue([
      {
        type_id: 'http',
        name: '获取用户信息',
        data: {
          type: 'http',
          url: '/v3/api/user/info',
          method: 'get',
          headers: {},
          body_type: 'none',
        },
        source_ref: { datasource: 'eolink', item_id: 'proj_1:api_1001' },
      },
    ]),
  };
});

const mockDatasource: DatasourceDescription = {
  id: 'eolink',
  display_name: 'Eolink',
  auth: { type: 'api_key' },
  resources: [
    { id: 'project', label: '项目', children: 'api', leaf: false },
    { id: 'api', label: '接口', children: null, leaf: true },
  ],
};

const mockItem: DataItem = {
  id: 'proj_1:api_1001',
  name: '获取用户信息',
  protocol: 'http',
  method: 'GET',
  path: '/v3/api/user/info',
  summary: '获取当前用户信息详情',
  extra: [{ label: '分组', value: '用户模块' }],
  raw: {
    api_name: '获取用户信息',
    api_uri: '/v3/api/user/info',
  },
};

describe('DatasourceItemDetailDialog', () => {
  beforeEach(async () => {
    useDatasourceStore.getState().reset();
    await useDatasourceStore.getState().saveConnection({
      id: 'conn_1',
      datasource_id: 'eolink',
      display_name: 'Eolink',
      config: { api_key: 'test_key' },
      created_at: new Date().toISOString(),
    });

    useFlowStore.getState().reset();
    useFlowStore.getState().setFlow({
      uuid: 'flow_1',
      name: 'Test Flow',
      nodes: [],
      connections: [],
    });
    switchLocale('zh-CN');
  });

  it('renders dialog with raw data tabs and overview', async () => {
    render(
      <DatasourceItemDetailDialog
        open={true}
        onOpenChange={vi.fn()}
        datasource={mockDatasource}
        item={mockItem}
      />,
    );

    expect(screen.getAllByText('获取用户信息').length).toBeGreaterThan(0);
    expect(screen.getAllByText('/v3/api/user/info').length).toBeGreaterThan(0);
    expect(screen.getByText('Eolink')).toBeDefined();

    // 复制按钮
    expect(screen.getByText('复制 JSON')).toBeDefined();

    const user = userEvent.setup();
    const overviewTab = screen.getByRole('tab', { name: /概览/ });
    await user.click(overviewTab);
    await waitFor(() => {
      expect(screen.getByText('条目 ID')).toBeDefined();
      expect(screen.getByText('接口名称')).toBeDefined();
      expect(screen.getByText('请求方法')).toBeDefined();
      expect(screen.getByText('请求路径')).toBeDefined();
      expect(screen.getByText('所属数据源')).toBeDefined();
      expect(screen.getByText('简介描述')).toBeDefined();
      expect(screen.getByText('平台属性 (Extra)')).toBeDefined();
    });

    const templateTab = screen.getByRole('tab', { name: /节点预览/ });
    await user.click(templateTab);
    await waitFor(() => {
      expect(screen.getByText('添加到画布')).toBeDefined();
    });
  });

  it('adds converted node template to canvas on button click', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <DatasourceItemDetailDialog
        open={true}
        onOpenChange={onOpenChange}
        datasource={mockDatasource}
        item={mockItem}
      />,
    );

    await user.click(screen.getByText('节点预览'));
    await waitFor(() => {
      expect(screen.getByText('添加到画布')).toBeDefined();
    });

    await user.click(screen.getByText('添加到画布'));

    const nodes = useFlowStore.getState().flow?.nodes ?? [];
    expect(nodes.length).toBe(1);
    expect(nodes[0].name).toBe('获取用户信息');
    expect(nodes[0].type).toBe('http');
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('renders localized content in en-US correctly', async () => {
    switchLocale('en-US');
    const user = userEvent.setup();
    render(
      <DatasourceItemDetailDialog
        open={true}
        onOpenChange={vi.fn()}
        datasource={mockDatasource}
        item={mockItem}
      />,
    );

    expect(screen.getByText('Copy JSON')).toBeDefined();

    // Overview tab in en-US
    const overviewTab = screen.getByRole('tab', { name: /Overview/ });
    await user.click(overviewTab);
    await waitFor(() => {
      expect(screen.getByText('Item ID')).toBeDefined();
      expect(screen.getByText('API Name')).toBeDefined();
      expect(screen.getByText('Method')).toBeDefined();
      expect(screen.getByText('Path')).toBeDefined();
      expect(screen.getByText('Data Source')).toBeDefined();
      expect(screen.getByText('Description')).toBeDefined();
      expect(screen.getByText('Extra Properties')).toBeDefined();
    });

    // Template tab in en-US
    const templateTab = screen.getByRole('tab', { name: /Node Preview/ });
    await user.click(templateTab);
    await waitFor(() => {
      expect(screen.getByText('Canvas Node Template (NodeTemplate):')).toBeDefined();
      expect(screen.getByText('Add to Canvas')).toBeDefined();
    });
  });
});
