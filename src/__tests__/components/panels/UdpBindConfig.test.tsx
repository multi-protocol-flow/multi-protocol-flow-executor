/**
 * UdpBindConfig 组件测试
 * - 渲染表单字段：本地地址、本地端口、地址复用开关
 * - 数据保存 (onSave 回调)：卸载保存、广播开关即时保存
 * - 多播配置折叠展开
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fireEvent } from '@testing-library/react';
import { UdpBindConfig } from '@/components/panels/UdpBindConfig';
import type { UdpBindConfig as UdpBindConfigType } from '@/types';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

/**
 * 创建基础 UdpBindConfig 数据
 */
function createBaseConfig(
  overrides: Partial<UdpBindConfigType> = {},
): UdpBindConfigType {
  return {
    type: 'udp_bind',
    local_addr: '0.0.0.0',
    local_port: 9000,
    reuse_addr: true,
    broadcast: false,
    ...overrides,
  };
}

describe('UdpBindConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 UDP 绑定节点标题', () => {
      render(<UdpBindConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('UDP 绑定节点')).toBeDefined();
    });

    it('渲染本地地址与本地端口的当前值', () => {
      const config = createBaseConfig({ local_addr: '192.168.1.100', local_port: 9999 });
      render(<UdpBindConfig data={config} onSave={mockOnSave} />);

      const addrInput = document.getElementById('local_addr') as HTMLInputElement;
      expect(addrInput?.value).toBe('192.168.1.100');

      const portInput = document.getElementById('local_port') as HTMLInputElement;
      expect(portInput?.value).toBe('9999');
    });

    it('地址复用开关默认开启', () => {
      const config = createBaseConfig({ reuse_addr: true });
      render(<UdpBindConfig data={config} onSave={mockOnSave} />);

      const reuseSwitch = document.getElementById('reuse_addr') as HTMLInputElement;
      expect(reuseSwitch?.checked).toBe(true);
    });
  });

  describe('数据保存', () => {
    it('修改本地地址后卸载时保存新值', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <UdpBindConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const addrInput = document.getElementById('local_addr') as HTMLInputElement;
      await user.clear(addrInput);
      await user.type(addrInput, '10.0.0.5');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ local_addr: '10.0.0.5', local_port: 9000 }),
        );
      });
    });

    it('切换广播模式后立即保存 broadcast 为 true', async () => {
      render(<UdpBindConfig data={createBaseConfig({ broadcast: false })} onSave={mockOnSave} />);

      fireEvent.click(document.getElementById('broadcast') as HTMLInputElement);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ broadcast: true }),
        );
      });
    });
  });

  describe('多播配置', () => {
    it('默认折叠，点击展开后显示多播组地址输入框', async () => {
      const user = userEvent.setup();
      render(<UdpBindConfig data={createBaseConfig()} onSave={mockOnSave} />);

      // 默认折叠：多播组地址输入框不可见
      expect(document.getElementById('multicast_group')).toBeNull();

      await user.click(screen.getByText('多播配置'));

      const groupInput = document.getElementById('multicast_group') as HTMLInputElement;
      expect(groupInput).toBeDefined();
      expect(screen.getByText('多播组地址')).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
