/**
 * TcpConnectConfig 组件测试
 * - 渲染表单字段：主机地址、端口、连接超时
 * - TLS 开关联动（跳过证书验证 / CA 证书显隐）
 * - 数据保存 (onSave 回调)：卸载保存、开关即时保存
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fireEvent } from '@testing-library/react';
import { TcpConnectConfig } from '@/components/panels/TcpConnectConfig';
import type { TcpConnectConfig as TcpConnectConfigType } from '@/types';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

/**
 * 创建基础 TcpConnectConfig 数据
 */
function createBaseConfig(
  overrides: Partial<TcpConnectConfigType> = {},
): TcpConnectConfigType {
  return {
    type: 'tcp:connect',
    host: '127.0.0.1',
    port: 8080,
    use_tls: false,
    tls_skip_verify: false,
    connect_timeout_ms: 30000,
    ...overrides,
  };
}

describe('TcpConnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 TCP 连接节点标题', () => {
      render(<TcpConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('TCP 连接节点')).toBeDefined();
    });

    it('渲染主机地址、端口与连接超时的当前值', () => {
      render(<TcpConnectConfig data={createBaseConfig()} onSave={mockOnSave} />);

      const hostInput = document.getElementById('host') as HTMLInputElement;
      expect(hostInput?.value).toBe('127.0.0.1');

      const portInput = document.getElementById('port') as HTMLInputElement;
      expect(portInput?.value).toBe('8080');

      const timeoutInput = document.getElementById('connect_timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('30000');
    });

    it('未启用 TLS 时不显示 CA 证书输入框', () => {
      const config = createBaseConfig({ use_tls: false });
      render(<TcpConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByText('自定义 CA 证书 (PEM)')).toBeNull();
    });
  });

  describe('数据保存', () => {
    it('修改主机地址后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <TcpConnectConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const hostInput = document.getElementById('host') as HTMLInputElement;
      await user.clear(hostInput);
      await user.type(hostInput, 'example.com');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ host: 'example.com', port: 8080 }),
        );
      });
    });

    it('切换启用 TLS 后立即保存 use_tls 为 true', async () => {
      render(<TcpConnectConfig data={createBaseConfig({ use_tls: false })} onSave={mockOnSave} />);

      fireEvent.click(document.getElementById('use_tls') as HTMLInputElement);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ use_tls: true }),
        );
      });
    });
  });

  describe('TLS 配置', () => {
    it('启用 TLS 后显示跳过证书验证开关与 CA 证书输入框', () => {
      const config = createBaseConfig({ use_tls: true });
      render(<TcpConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('跳过证书验证')).toBeDefined();
      expect(screen.getByText('自定义 CA 证书 (PEM)')).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
