/**
 * GraphqlConnectConfig 组件测试
 * - 渲染表单字段：HTTP 端点、WS 端点、请求头、超时、连接 ID
 * - 编辑并保存（卸载时触发 onSave）
 * - Schema 探索（mock Tauri IPC）：成功保存 discovered_schema
 * - 未配置端点时探索报错
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GraphqlConnectConfig } from '@/components/panels/GraphqlConnectConfig';
import type { GraphqlConnectConfig as GraphqlConnectConfigType } from '@/types';
import { introspectSchema } from '@/services/tauri/graphql';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock Schema 内省服务（避免真实 Tauri IPC 调用）
vi.mock('@/services/tauri/graphql', () => ({
  introspectSchema: vi.fn(),
}));

/** 创建基础 GraphqlConnectConfig 数据 */
function createBaseConfig(
  overrides: Partial<GraphqlConnectConfigType> = {},
): GraphqlConnectConfigType {
  return {
    type: 'graphql_connect',
    endpoint: 'https://api.example.com/graphql',
    ...overrides,
  };
}

describe('GraphqlConnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 GraphQL 连接节点标题', () => {
      render(<GraphqlConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('GraphQL 连接节点')).toBeDefined();
    });

    it('渲染端点、WS 端点、超时、连接 ID 与请求头的当前值', () => {
      const config = createBaseConfig({
        endpoint: 'https://api.example.com/graphql',
        ws_endpoint: 'wss://api.example.com/graphql',
        connection_timeout_ms: 5000,
        connection_id: 'gql-conn-1',
        headers: { Authorization: 'Bearer token123' },
      });
      render(<GraphqlConnectConfig data={config} onSave={mockOnSave} />);

      expect((document.getElementById('endpoint') as HTMLInputElement)?.value).toBe('https://api.example.com/graphql');
      expect((document.getElementById('ws_endpoint') as HTMLInputElement)?.value).toBe('wss://api.example.com/graphql');
      expect((document.getElementById('connection_timeout_ms') as HTMLInputElement)?.value).toBe('5000');
      expect((document.getElementById('connection_id') as HTMLInputElement)?.value).toBe('gql-conn-1');
      expect(screen.getByDisplayValue('Authorization')).toBeDefined();
      expect(screen.getByDisplayValue('Bearer token123')).toBeDefined();
    });
  });

  describe('编辑与保存', () => {
    it('修改端点后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <GraphqlConnectConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const endpointInput = document.getElementById('endpoint') as HTMLInputElement;
      await user.clear(endpointInput);
      await user.type(endpointInput, 'https://new.example.com/graphql');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ endpoint: 'https://new.example.com/graphql' }),
        );
      });
    });
  });

  describe('Schema 探索', () => {
    it('探索成功后保存 discovered_schema 并显示成功消息', async () => {
      const user = userEvent.setup();
      vi.mocked(introspectSchema).mockResolvedValue({
        success: true,
        schema: { Query: {}, Mutation: {} },
        error: null,
        query_templates: [
          { type_name: 'User', operation: 'query', template: 'query { user { id } }' },
        ],
      });
      render(<GraphqlConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      await user.click(screen.getByText('获取 Schema'));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({
            discovered_schema: { Query: {}, Mutation: {} },
            discovered_queries: [
              { type_name: 'User', operation: 'query', template: 'query { user { id } }' },
            ],
          }),
        );
      });
      expect(screen.getByText('Schema 探索成功（2 个类型）')).toBeDefined();
    });

    it('未配置端点时探索显示错误且不保存', async () => {
      const user = userEvent.setup();
      render(
        <GraphqlConnectConfig data={createBaseConfig({ endpoint: '' })} onSave={mockOnSave} />,
      );

      await user.click(screen.getByText('获取 Schema'));

      expect(screen.getByText('请先配置 GraphQL 端点 URL')).toBeDefined();
      expect(mockOnSave).not.toHaveBeenCalled();
    });
  });
});
