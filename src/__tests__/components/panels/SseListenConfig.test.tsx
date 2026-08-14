/**
 * SseListenConfig 组件测试
 * - 渲染表单字段：连接 ID、事件过滤、最大事件数、超时、开关
 * - 编辑并保存（卸载时触发 onSave）
 * - 包含事件数据开关即时保存
 * - 未检测到 SSE Connect 节点提示
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SseListenConfig } from '@/components/panels/SseListenConfig';
import type { SseListenConfig as SseListenConfigType } from '@/types';

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

/** 创建基础 SseListenConfig 数据 */
function createBaseConfig(
  overrides: Partial<SseListenConfigType> = {},
): SseListenConfigType {
  return {
    type: 'sse_listen',
    connection_id: 'sse-conn-1',
    ...overrides,
  };
}

describe('SseListenConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 SSE 监听节点标题', () => {
      render(<SseListenConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('SSE 监听节点')).toBeDefined();
    });

    it('渲染连接 ID、事件过滤、最大事件数、超时与开关的当前值', () => {
      const config = createBaseConfig({
        connection_id: 'sse-conn-9',
        event_filter: 'message,update',
        max_events: 10,
        timeout_ms: 5000,
        include_data: true,
        decode_json: true,
      });
      render(<SseListenConfig data={config} onSave={mockOnSave} />);

      expect((document.getElementById('connection_id') as HTMLInputElement)?.value).toBe('sse-conn-9');
      expect((document.getElementById('event_filter') as HTMLInputElement)?.value).toBe('message,update');
      expect((document.getElementById('max_events') as HTMLInputElement)?.value).toBe('10');
      expect((document.getElementById('timeout_ms') as HTMLInputElement)?.value).toBe('5000');

      const switches = screen.getAllByRole('checkbox') as HTMLInputElement[];
      expect(switches[0]).toBeChecked(); // 包含事件数据
      expect(switches[1]).toBeChecked(); // 自动解码 JSON
    });
  });

  describe('编辑与保存', () => {
    it('修改事件过滤后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <SseListenConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const filterInput = document.getElementById('event_filter') as HTMLInputElement;
      await user.clear(filterInput);
      await user.type(filterInput, 'ping');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ event_filter: 'ping' }),
        );
      });
    });

    it('切换"包含事件数据"开关后立即保存', async () => {
      render(<SseListenConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const switches = screen.getAllByRole('checkbox') as HTMLInputElement[];
      fireEvent.click(switches[0]); // 关闭包含事件数据

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ include_data: false }),
        );
      });
    });
  });

  describe('面板特有行为', () => {
    it('未检测到 SSE Connect 节点时显示提示与手动输入框', () => {
      render(<SseListenConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('未检测到 SSE Connect 节点，请手动输入连接 ID')).toBeDefined();
      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
    });
  });
});
