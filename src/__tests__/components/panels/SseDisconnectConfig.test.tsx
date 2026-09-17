/**
 * SseDisconnectConfig 组件测试
 * - 渲染连接 ID 字段
 * - 编辑并保存（卸载时触发 onSave）
 * - 未检测到 SSE Connect 节点时手动输入
 * - 检测到 SSE Connect 节点时下拉选择并保存
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SseDisconnectConfig } from '@/components/panels/SseDisconnectConfig';
import type { SseDisconnectConfig as SseDisconnectConfigType } from '@/types';
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

/** SSE Connect 节点（用于 flow mock） */
const sseConnectNode = {
  uuid: 'sse-1',
  type: 'sse:connect' as const,
  name: 'SSE Server',
  position: { x: 0, y: 0 },
  data: { type: 'sse:connect' as const, url: 'https://example.com/events' },
};

const mockFlow = {
  uuid: 'test-flow',
  name: 'Test Flow',
  nodes: [sseConnectNode],
  connections: [],
};

/** 创建基础 SseDisconnectConfig 数据 */
function createBaseConfig(
  overrides: Partial<SseDisconnectConfigType> = {},
): SseDisconnectConfigType {
  return {
    type: 'sse:disconnect',
    connection_id: 'sse-conn-1',
    ...overrides,
  };
}

describe('SseDisconnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useFlowStore as unknown as Mock).mockReset();
  });

  describe('基础渲染', () => {
    it('渲染 SSE 关闭连接节点标题', () => {
      render(<SseDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('SSE 关闭连接节点')).toBeDefined();
    });

    it('渲染连接 ID 的当前值', () => {
      render(
        <SseDisconnectConfig data={createBaseConfig({ connection_id: 'sse-conn-9' })} onSave={mockOnSave} />,
      );

      expect((document.getElementById('connection_id') as HTMLInputElement)?.value).toBe('sse-conn-9');
    });
  });

  describe('编辑与保存', () => {
    it('修改连接 ID 后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <SseDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const input = document.getElementById('connection_id') as HTMLInputElement;
      await user.clear(input);
      await user.type(input, 'sse-conn-2');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ connection_id: 'sse-conn-2' }),
        );
      });
    });
  });

  describe('面板特有行为', () => {
    it('未检测到 SSE Connect 节点时显示提示', () => {
      render(<SseDisconnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('未检测到 SSE Connect 节点，请手动输入连接 ID')).toBeDefined();
    });

    it('检测到 SSE Connect 节点时通过下拉选择连接并保存', async () => {
      (useFlowStore as unknown as Mock).mockImplementation(
        (selector: (state: { flow: typeof mockFlow }) => unknown) => selector({ flow: mockFlow }),
      );

      render(
        <SseDisconnectConfig data={createBaseConfig({ connection_id: '' })} onSave={mockOnSave} />,
      );

      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getByText('SSE Server'));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ connection_id: 'sse-1' }),
        );
      });
    });
  });
});
