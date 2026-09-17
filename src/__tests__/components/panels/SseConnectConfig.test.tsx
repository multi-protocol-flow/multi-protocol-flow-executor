/**
 * SseConnectConfig 组件测试
 * - 渲染表单字段：URL、请求头、Last-Event-ID、超时、TLS 模式
 * - 编辑并保存（卸载时触发 onSave）
 * - 自动重连开关即时保存
 * - TLS 模式选择即时保存
 * - 测试连接按钮（mock Tauri IPC）
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SseConnectConfig } from '@/components/panels/SseConnectConfig';
import type { SseConnectConfig as SseConnectConfigType } from '@/types';
import { testSseConnection } from '@/services/tauri/sse';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock SSE 测试连接服务（避免真实 Tauri IPC 调用）
vi.mock('@/services/tauri/sse', () => ({
  testSseConnection: vi.fn(),
}));

// jsdom 未实现 scrollIntoView，Radix Select 打开时会调用（兼容测试环境）
Object.defineProperty(window.Element.prototype, 'scrollIntoView', {
  writable: true,
  value: vi.fn(),
});

/** 创建基础 SseConnectConfig 数据 */
function createBaseConfig(
  overrides: Partial<SseConnectConfigType> = {},
): SseConnectConfigType {
  return {
    type: 'sse:connect',
    url: 'https://example.com/events',
    ...overrides,
  };
}

describe('SseConnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 SSE 连接节点标题', () => {
      render(<SseConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('SSE 连接节点')).toBeDefined();
    });

    it('渲染 URL、Last-Event-ID 与连接超时的当前值', () => {
      const config = createBaseConfig({
        url: 'https://stream.example.com/events',
        last_event_id: 'evt-42',
        timeout_ms: 15000,
      });
      render(<SseConnectConfig data={config} onSave={mockOnSave} />);

      const urlInput = document.getElementById('url') as HTMLInputElement;
      expect(urlInput?.value).toBe('https://stream.example.com/events');

      const lastEventIdInput = document.getElementById('last_event_id') as HTMLInputElement;
      expect(lastEventIdInput?.value).toBe('evt-42');

      const timeoutInput = document.getElementById('timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('15000');
    });

    it('渲染已有请求头键值对', () => {
      const config = createBaseConfig({
        headers: { Authorization: 'Bearer token123' },
      });
      render(<SseConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByDisplayValue('Authorization')).toBeDefined();
      expect(screen.getByDisplayValue('Bearer token123')).toBeDefined();
    });
  });

  describe('编辑与保存', () => {
    it('修改 URL 后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <SseConnectConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const urlInput = document.getElementById('url') as HTMLInputElement;
      await user.clear(urlInput);
      await user.type(urlInput, 'https://new.example.com/events');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ url: 'https://new.example.com/events' }),
        );
      });
    });

    it('切换自动重连开关后立即保存并显示重连选项', async () => {
      render(<SseConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const autoReconnectSwitch = screen.getByRole('checkbox') as HTMLInputElement;
      expect(autoReconnectSwitch).not.toBeChecked();

      fireEvent.click(autoReconnectSwitch);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ auto_reconnect: true }),
        );
      });
      // 重连选项区域出现
      expect(document.getElementById('reconnect_interval_ms')).toBeDefined();
      expect(document.getElementById('max_reconnect_attempts')).toBeDefined();
    });

    it('展开 TLS 区域并选择模式后立即保存', async () => {
      const user = userEvent.setup();
      render(<SseConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      // 展开 TLS 区域
      await user.click(screen.getAllByText('TLS 模式')[0]);

      // 打开下拉并选择"标准 TLS"（[1] 为可见的 item 文本，[0] 是隐藏的原生 option）
      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getAllByText('标准 TLS')[1]);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ tls_mode: 'standard' }),
        );
      });
    });
  });

  describe('测试连接', () => {
    it('点击测试连接按钮后显示成功结果', async () => {
      const user = userEvent.setup();
      vi.mocked(testSseConnection).mockResolvedValue(true);
      render(<SseConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      await user.click(screen.getByText('测试连接'));

      await waitFor(() => {
        expect(screen.getByText('连接成功：SSE 端点 URL 有效')).toBeDefined();
      });
      expect(testSseConnection).toHaveBeenCalledWith({ url: 'https://example.com/events' });
    });
  });
});
