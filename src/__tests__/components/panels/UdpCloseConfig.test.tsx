/**
 * UdpCloseConfig 组件测试
 * - 渲染表单字段：连接 ID
 * - 数据保存 (onSave 回调)：卸载保存
 * - 连接选择：有/无 udp_bind 节点时的渲染差异
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { UdpCloseConfig } from '@/components/panels/UdpCloseConfig';
import type { UdpCloseConfig as UdpCloseConfigType } from '@/types';
import { useFlowStore } from '@/stores';

// Mock useFlowStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

/**
 * 创建基础 UdpCloseConfig 数据
 */
function createBaseConfig(
  overrides: Partial<UdpCloseConfigType> = {},
): UdpCloseConfigType {
  return {
    type: 'udp_close',
    connection_id: 'udp-bind-001',
    ...overrides,
  };
}

/**
 * 配置 store mock，模拟包含 udp_bind 节点的 flow
 */
function setupStoreMock(withUdpBind: boolean = true) {
  const nodes: Array<{
    uuid: string;
    type: string;
    name: string;
    position: { x: number; y: number };
    data: Record<string, unknown>;
  }> = [];

  if (withUdpBind) {
    nodes.push({
      uuid: 'udp-bind-001',
      type: 'udp_bind',
      name: 'UDP 绑定 1',
      position: { x: 200, y: 0 },
      data: {
        type: 'udp_bind',
        local_addr: '0.0.0.0',
        local_port: 9000,
      },
    });
  }

  const mockFlow = {
    uuid: 'test-flow',
    name: 'Test Flow',
    nodes,
    connections: [],
  };

  (useFlowStore as unknown as Mock).mockImplementation((selector: unknown) => {
    if (typeof selector === 'function') {
      return selector({ flow: mockFlow });
    }
    return { flow: mockFlow };
  });

  return mockFlow;
}

describe('UdpCloseConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 UDP 关闭节点标题', () => {
      setupStoreMock(false);
      render(<UdpCloseConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('UDP 关闭节点')).toBeDefined();
    });

    it('渲染连接 ID 的当前值', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: 'udp-bind-002' });
      render(<UdpCloseConfig data={config} onSave={mockOnSave} />);

      const connInput = document.getElementById('connection_id') as HTMLInputElement;
      expect(connInput?.value).toBe('udp-bind-002');
    });
  });

  describe('数据保存', () => {
    it('修改连接 ID 后卸载时保存新值', async () => {
      setupStoreMock(false);
      const user = userEvent.setup();
      const { unmount } = render(
        <UdpCloseConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const connInput = document.getElementById('connection_id') as HTMLInputElement;
      await user.clear(connInput);
      await user.type(connInput, 'udp-bind-099');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ connection_id: 'udp-bind-099' }),
        );
      });
    });
  });

  describe('连接选择', () => {
    it('有 udp_bind 节点时显示 Select 选择器', () => {
      setupStoreMock(true);
      const config = createBaseConfig();
      render(<UdpCloseConfig data={config} onSave={mockOnSave} />);

      const selectTrigger = document.getElementById('connection_id');
      expect(selectTrigger).toBeDefined();
      // Select 模式下不显示手动输入提示
      expect(screen.queryByText(/未检测到 UDP Bind 节点/)).toBeNull();
    });

    it('无 udp_bind 节点时显示 Input 与提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<UdpCloseConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
      expect(screen.getByText(/未检测到 UDP Bind 节点/)).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
