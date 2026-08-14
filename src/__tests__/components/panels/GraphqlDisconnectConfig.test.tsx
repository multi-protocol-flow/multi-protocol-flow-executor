/**
 * GraphqlDisconnectConfig 组件测试
 * - 渲染连接 ID 字段
 * - 编辑并保存（卸载时触发 onSave）
 * - 未检测到 GraphQL Connect 节点提示
 * - 检测到 GraphQL Connect 节点时下拉选择并保存
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GraphqlDisconnectConfig } from '@/components/panels/GraphqlDisconnectConfig';
import type { GraphqlDisconnectConfig as GraphqlDisconnectConfigType } from '@/types';
import { useFlowStore } from '@/stores';

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

// jsdom 未实现 scrollIntoView，Radix Select 打开时会调用（兼容测试环境）
Object.defineProperty(window.Element.prototype, 'scrollIntoView', {
  writable: true,
  value: vi.fn(),
});

/** GraphQL Connect 节点（用于 flow mock） */
const graphqlConnectNode = {
  uuid: 'gql-1',
  type: 'graphql_connect' as const,
  name: 'GraphQL API',
  position: { x: 0, y: 0 },
  data: { type: 'graphql_connect' as const, endpoint: 'https://api.example.com/graphql' },
};

const mockFlow = {
  uuid: 'test-flow',
  name: 'Test Flow',
  nodes: [graphqlConnectNode],
  connections: [],
};

/** 创建基础 GraphqlDisconnectConfig 数据 */
function createBaseConfig(
  overrides: Partial<GraphqlDisconnectConfigType> = {},
): GraphqlDisconnectConfigType {
  return {
    type: 'graphql_disconnect',
    connection_id: 'gql-conn-1',
    ...overrides,
  };
}

describe('GraphqlDisconnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useFlowStore as unknown as Mock).mockReset();
  });

  describe('基础渲染', () => {
    it('渲染 GraphQL 关闭连接节点标题', () => {
      render(<GraphqlDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('GraphQL 关闭连接节点')).toBeDefined();
    });

    it('渲染连接 ID 的当前值', () => {
      render(
        <GraphqlDisconnectConfig data={createBaseConfig({ connection_id: 'gql-conn-9' })} onSave={mockOnSave} />,
      );

      expect((document.getElementById('connection_id') as HTMLInputElement)?.value).toBe('gql-conn-9');
    });
  });

  describe('编辑与保存', () => {
    it('修改连接 ID 后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <GraphqlDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />,
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
      render(<GraphqlDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('未检测到 GraphQL Connect 节点，请手动输入连接 ID')).toBeDefined();
    });

    it('检测到 GraphQL Connect 节点时通过下拉选择连接并保存', async () => {
      (useFlowStore as unknown as Mock).mockImplementation(
        (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow }),
      );

      render(
        <GraphqlDisconnectConfig data={createBaseConfig({ connection_id: '' })} onSave={mockOnSave} />,
      );

      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getByText('GraphQL API'));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ connection_id: 'gql-1' }),
        );
      });
    });
  });
});
