/**
 * GraphqlQueryConfig 组件测试
 * - 渲染表单字段：连接 ID、查询、变量、操作名称、超时
 * - 编辑并保存（卸载时触发 onSave）
 * - 变量 JSON 校验（无效报错 / 有效保存解析结果）
 * - 文件上传行添加
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GraphqlQueryConfig } from '@/components/panels/GraphqlQueryConfig';
import type { GraphqlQueryConfig as GraphqlQueryConfigType } from '@/types';

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

/** 创建基础 GraphqlQueryConfig 数据 */
function createBaseConfig(
  overrides: Partial<GraphqlQueryConfigType> = {},
): GraphqlQueryConfigType {
  return {
    type: 'graphql:query',
    connection_id: 'gql-conn-1',
    query: 'query GetUser { user { id name } }',
    ...overrides,
  };
}

describe('GraphqlQueryConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 GraphQL 查询节点标题', () => {
      render(<GraphqlQueryConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('GraphQL 查询节点')).toBeDefined();
    });

    it('渲染查询、变量、操作名称与超时的当前值', () => {
      const config = createBaseConfig({
        variables: { limit: 10 },
        operation_name: 'GetUser',
        timeout_ms: 8000,
      });
      render(<GraphqlQueryConfig data={config} onSave={mockOnSave} />);

      expect((document.getElementById('query') as HTMLTextAreaElement)?.value).toBe('query GetUser { user { id name } }');
      expect((document.getElementById('variables') as HTMLTextAreaElement)?.value).toBe('{\n  "limit": 10\n}');
      expect((document.getElementById('operation_name') as HTMLInputElement)?.value).toBe('GetUser');
      expect((document.getElementById('timeout_ms') as HTMLInputElement)?.value).toBe('8000');
    });
  });

  describe('编辑与保存', () => {
    it('修改查询语句后卸载时保存新值', async () => {
      const { unmount } = render(
        <GraphqlQueryConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const queryInput = document.getElementById('query') as HTMLTextAreaElement;
      // 查询文本含花括号，userEvent.type 会将其解析为修饰键，改用 fireEvent.change
      fireEvent.change(queryInput, { target: { value: 'query NewQuery { posts { id } }' } });

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ query: 'query NewQuery { posts { id } }' }),
        );
      });
    });
  });

  describe('变量校验', () => {
    it('变量为无效 JSON 时失焦显示错误', async () => {
      render(<GraphqlQueryConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const variablesInput = document.getElementById('variables') as HTMLTextAreaElement;
      fireEvent.change(variablesInput, { target: { value: '{"limit": ' } });
      fireEvent.blur(variablesInput);

      expect(screen.getByText('JSON 格式无效')).toBeDefined();
    });

    it('变量为有效 JSON 时失焦保存解析后的对象', async () => {
      render(<GraphqlQueryConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const variablesInput = document.getElementById('variables') as HTMLTextAreaElement;
      fireEvent.change(variablesInput, { target: { value: '{"limit": 5}' } });
      fireEvent.blur(variablesInput);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ variables: { limit: 5 } }),
        );
      });
    });
  });

  describe('文件上传', () => {
    it('点击添加后出现文件上传输入行', async () => {
      const user = userEvent.setup();
      render(<GraphqlQueryConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('暂无文件上传')).toBeDefined();

      await user.click(screen.getByText('添加'));

      expect(screen.getByPlaceholderText('字段名（如 file0）')).toBeDefined();
      expect(screen.getByPlaceholderText('文件路径')).toBeDefined();
    });
  });
});
