import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DatasourceItemDetailDialog } from '@/components/dialogs/DatasourceItemDetailDialog';
import type * as DatasourceServiceModule from '@/services/tauri/datasource';
import { useFlowStore, useDatasourceStore } from '@/stores';
import type { DataItem, DatasourceDescription } from '@/types/datasource';

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

    // 切换到概览 Tab
    const overviewTab = screen.getByRole('tab', { name: /概览/ });
    fireEvent.pointerDown(overviewTab);
    fireEvent.click(overviewTab);

    // 切换到模板预览 Tab
    const templateTab = screen.getByRole('tab', { name: /节点预览/ });
    fireEvent.pointerDown(templateTab);
    fireEvent.click(templateTab);
    await waitFor(() => {
      expect(screen.getByText('添加到画布')).toBeDefined();
    });
  });

  it('adds converted node template to canvas on button click', async () => {
    const onOpenChange = vi.fn();
    render(
      <DatasourceItemDetailDialog
        open={true}
        onOpenChange={onOpenChange}
        datasource={mockDatasource}
        item={mockItem}
      />,
    );

    fireEvent.click(screen.getByText('节点预览'));
    await waitFor(() => {
      expect(screen.getByText('添加到画布')).toBeDefined();
    });

    fireEvent.click(screen.getByText('添加到画布'));

    const nodes = useFlowStore.getState().flow?.nodes ?? [];
    expect(nodes.length).toBe(1);
    expect(nodes[0].name).toBe('获取用户信息');
    expect(nodes[0].type).toBe('http');
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
