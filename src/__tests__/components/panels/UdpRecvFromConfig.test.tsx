/**
 * UdpRecvFromConfig 组件测试
 * - 渲染表单字段：连接 ID、接收超时、最大接收大小
 * - 数据保存 (onSave 回调)：卸载保存
 * - 连接选择：有/无 udp:bind 节点时的渲染差异
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { UdpRecvFromConfig } from '@/components/panels/UdpRecvFromConfig';
import type { UdpRecvFromConfig as UdpRecvFromConfigType } from '@/types';
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
 * 创建基础 UdpRecvFromConfig 数据
 */
function createBaseConfig(
  overrides: Partial<UdpRecvFromConfigType> = {},
): UdpRecvFromConfigType {
  return {
    type: 'udp:recv_from',
    connection_id: 'udp-bind-001',
    receive_timeout_ms: 5000,
    max_size: 65536,
    ...overrides,
  };
}

/**
 * 配置 store mock，模拟包含 udp:bind 节点的 flow
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
      type: 'udp:bind',
      name: 'UDP 绑定 1',
      position: { x: 200, y: 0 },
      data: {
        type: 'udp:bind',
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

describe('UdpRecvFromConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 UDP 接收节点标题', () => {
      setupStoreMock(false);
      render(<UdpRecvFromConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('UDP 接收节点')).toBeDefined();
    });

    it('渲染连接 ID、接收超时与最大接收大小的当前值', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ receive_timeout_ms: 10000, max_size: 4096 });
      render(<UdpRecvFromConfig data={config} onSave={mockOnSave} />);

      const connInput = document.getElementById('connection_id') as HTMLInputElement;
      expect(connInput?.value).toBe('udp-bind-001');

      const timeoutInput = document.getElementById('receive_timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('10000');

      const maxSizeInput = document.getElementById('max_size') as HTMLInputElement;
      expect(maxSizeInput?.value).toBe('4096');
    });
  });

  describe('数据保存', () => {
    it('修改最大接收大小后卸载时保存新值', async () => {
      setupStoreMock(false);
      const user = userEvent.setup();
      const { unmount } = render(
        <UdpRecvFromConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const maxSizeInput = document.getElementById('max_size') as HTMLInputElement;
      await user.clear(maxSizeInput);
      await user.type(maxSizeInput, '8192');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ max_size: 8192, connection_id: 'udp-bind-001' }),
        );
      });
    });
  });

  describe('连接选择', () => {
    it('有 udp:bind 节点时显示 Select 选择器', () => {
      setupStoreMock(true);
      const config = createBaseConfig();
      render(<UdpRecvFromConfig data={config} onSave={mockOnSave} />);

      const selectTrigger = document.getElementById('connection_id');
      expect(selectTrigger).toBeDefined();
      // Select 模式下不显示手动输入提示
      expect(screen.queryByText(/未检测到 UDP Bind 节点/)).toBeNull();
    });

    it('无 udp:bind 节点时显示 Input 与提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<UdpRecvFromConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
      expect(screen.getByText(/未检测到 UDP Bind 节点/)).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
