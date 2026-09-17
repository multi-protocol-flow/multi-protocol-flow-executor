/**
 * GraphqlSubscribeConfig 组件测试
 * - 渲染表单字段：连接 ID、订阅、变量、操作名称、最大事件数、超时
 * - 编辑并保存（卸载时触发 onSave）
 * - 变量 JSON 校验（无效报错 / 有效保存解析结果）
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { GraphqlSubscribeConfig } from '@/components/panels/GraphqlSubscribeConfig';
import type { GraphqlSubscribeConfig as GraphqlSubscribeConfigType } from '@/types';

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

/** 创建基础 GraphqlSubscribeConfig 数据 */
function createBaseConfig(
  overrides: Partial<GraphqlSubscribeConfigType> = {},
): GraphqlSubscribeConfigType {
  return {
    type: 'graphql:subscribe',
    connection_id: 'gql-conn-1',
    subscription: 'subscription OnMessage { messageAdded { id text } }',
    ...overrides,
  };
}

describe('GraphqlSubscribeConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 GraphQL 订阅节点标题', () => {
      render(<GraphqlSubscribeConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('GraphQL 订阅节点')).toBeDefined();
    });

    it('渲染订阅、变量、操作名称、最大事件数与超时的当前值', () => {
      const config = createBaseConfig({
        variables: { roomId: 'room-123' },
        operation_name: 'OnMessage',
        max_events: 5,
        timeout_ms: 8000,
      });
      render(<GraphqlSubscribeConfig data={config} onSave={mockOnSave} />);

      expect((document.getElementById('subscription') as HTMLTextAreaElement)?.value).toBe('subscription OnMessage { messageAdded { id text } }');
      expect((document.getElementById('variables') as HTMLTextAreaElement)?.value).toBe('{\n  "roomId": "room-123"\n}');
      expect((document.getElementById('operation_name') as HTMLInputElement)?.value).toBe('OnMessage');
      expect((document.getElementById('max_events') as HTMLInputElement)?.value).toBe('5');
      expect((document.getElementById('timeout_ms') as HTMLInputElement)?.value).toBe('8000');
    });
  });

  describe('编辑与保存', () => {
    it('修改订阅语句后卸载时保存新值', async () => {
      const { unmount } = render(
        <GraphqlSubscribeConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const subscriptionInput = document.getElementById('subscription') as HTMLTextAreaElement;
      // 订阅文本含花括号，userEvent.type 会将其解析为修饰键，改用 fireEvent.change
      fireEvent.change(subscriptionInput, {
        target: { value: 'subscription OnNewPost { postAdded { id title } }' },
      });

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ subscription: 'subscription OnNewPost { postAdded { id title } }' }),
        );
      });
    });
  });

  describe('变量校验', () => {
    it('变量为无效 JSON 时失焦显示错误', async () => {
      render(<GraphqlSubscribeConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const variablesInput = document.getElementById('variables') as HTMLTextAreaElement;
      fireEvent.change(variablesInput, { target: { value: '{"roomId": ' } });
      fireEvent.blur(variablesInput);

      expect(screen.getByText('JSON 格式无效')).toBeDefined();
    });

    it('变量为有效 JSON 时失焦保存解析后的对象', async () => {
      render(<GraphqlSubscribeConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const variablesInput = document.getElementById('variables') as HTMLTextAreaElement;
      fireEvent.change(variablesInput, { target: { value: '{"roomId": "room-9"}' } });
      fireEvent.blur(variablesInput);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ variables: { roomId: 'room-9' } }),
        );
      });
    });
  });
});
