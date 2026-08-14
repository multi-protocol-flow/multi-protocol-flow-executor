/**
 * ProxyConfig 组件测试
 * - 渲染代理模式与代理地址/用户名/密码当前值
 * - 编辑代理地址 + 卸载保存 (onSave)
 * - 切换代理模式（清除模式清空代理字段）
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProxyConfig } from '@/components/panels/ProxyConfig';
import type { ProxyConfig as ProxyConfigType } from '@/types/node';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

/**
 * 创建基础 ProxyConfig 数据
 */
function createBaseConfig(overrides: Partial<ProxyConfigType> = {}): ProxyConfigType {
  return {
    type: 'proxy',
    mode: 'clear',
    ...overrides,
  };
}

describe('ProxyConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('清除模式下不显示代理字段', () => {
      render(<ProxyConfig data={createBaseConfig({ mode: 'clear' })} onSave={mockOnSave} />);

      const modeSelect = document.getElementById('proxy-mode') as HTMLSelectElement;
      expect(modeSelect?.value).toBe('clear');
      expect(screen.queryByLabelText('代理地址')).toBeNull();
      expect(screen.queryByLabelText('用户名')).toBeNull();
      expect(screen.queryByLabelText('密码')).toBeNull();
    });

    it('http 模式下渲染代理地址、用户名、密码当前值', () => {
      render(
        <ProxyConfig
          data={createBaseConfig({
            mode: 'http',
            proxy_url: 'http://proxy:8080',
            username: 'user1',
            password: 'pass1',
          })}
          onSave={mockOnSave}
        />,
      );

      expect(screen.getByLabelText('代理地址')).toHaveValue('http://proxy:8080');
      expect(screen.getByLabelText('用户名')).toHaveValue('user1');
      expect(screen.getByLabelText('密码')).toHaveValue('pass1');
    });
  });

  describe('编辑与保存', () => {
    it('修改代理地址后卸载保存', async () => {
      const user = userEvent.setup();
      const { unmount } = render(
        <ProxyConfig data={createBaseConfig({ mode: 'http', proxy_url: 'http://proxy:8080' })} onSave={mockOnSave} />,
      );

      const urlInput = document.getElementById('proxy-url') as HTMLInputElement;
      await user.clear(urlInput);
      await user.type(urlInput, 'http://new-proxy:9090');
      unmount();

      expect(mockOnSave).toHaveBeenCalledTimes(1);
      expect(mockOnSave).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'proxy', mode: 'http', proxy_url: 'http://new-proxy:9090' }),
      );
    });

    it('切换到清除模式后延迟保存并清空代理字段', async () => {
      const user = userEvent.setup();
      render(
        <ProxyConfig
          data={createBaseConfig({ mode: 'http', proxy_url: 'http://proxy:8080', username: 'u', password: 'p' })}
          onSave={mockOnSave}
        />,
      );

      await user.selectOptions(document.getElementById('proxy-mode') as HTMLSelectElement, 'clear');

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ mode: 'clear', proxy_url: undefined }),
        );
      });
      // 清除模式下代理字段隐藏
      expect(screen.queryByLabelText('代理地址')).toBeNull();
    });

    it('切换到 http 模式后显示代理字段并保存', async () => {
      const user = userEvent.setup();
      render(<ProxyConfig data={createBaseConfig({ mode: 'clear' })} onSave={mockOnSave} />);

      await user.selectOptions(document.getElementById('proxy-mode') as HTMLSelectElement, 'http');

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(expect.objectContaining({ mode: 'http' }));
      });
      expect(screen.getByLabelText('代理地址')).toBeDefined();
      expect(screen.getByLabelText('用户名')).toBeDefined();
    });
  });
});
