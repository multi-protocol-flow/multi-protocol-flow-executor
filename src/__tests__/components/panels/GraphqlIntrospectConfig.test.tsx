/**
 * GraphqlIntrospectConfig 组件测试
 * - 渲染表单字段：连接 ID、超时
 * - 编辑并保存（卸载时触发 onSave）
 * - 未检测到 GraphQL Connect 节点提示
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GraphqlIntrospectConfig } from '@/components/panels/GraphqlIntrospectConfig';
import type { GraphqlIntrospectConfig as GraphqlIntrospectConfigType } from '@/types';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock useFlowStore（默认无 flow，走手动输入连接 ID 路径）
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

/** 创建基础 GraphqlIntrospectConfig 数据 */
function createBaseConfig(
  overrides: Partial<GraphqlIntrospectConfigType> = {},
): GraphqlIntrospectConfigType {
  return {
    type: 'graphql_introspect',
    connection_id: 'gql-conn-1',
    ...overrides,
  };
}

describe('GraphqlIntrospectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 GraphQL 内省节点标题', () => {
      render(<GraphqlIntrospectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('GraphQL 内省节点')).toBeDefined();
    });

    it('渲染连接 ID 与超时的当前值', () => {
      const config = createBaseConfig({
        connection_id: 'gql-conn-9',
        timeout_ms: 8000,
      });
      render(<GraphqlIntrospectConfig data={config} onSave={mockOnSave} />);

      expect((document.getElementById('connection_id') as HTMLInputElement)?.value).toBe('gql-conn-9');
      expect((document.getElementById('timeout_ms') as HTMLInputElement)?.value).toBe('8000');
    });
  });

  describe('编辑与保存', () => {
    it('修改连接 ID 后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <GraphqlIntrospectConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const input = document.getElementById('connection_id') as HTMLInputElement;
      await user.clear(input);
      await user.type(input, 'gql-conn-2');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ connection_id: 'gql-conn-2' }),
        );
      });
    });
  });

  describe('面板特有行为', () => {
    it('未检测到 GraphQL Connect 节点时显示提示', () => {
      render(<GraphqlIntrospectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('未检测到 GraphQL Connect 节点，请手动输入连接 ID')).toBeDefined();
    });
  });
});
