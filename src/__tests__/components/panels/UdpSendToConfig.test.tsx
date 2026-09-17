/**
 * UdpSendToConfig 组件测试
 * - 渲染表单字段：目标地址、目标端口、数据输入
 * - 数据保存 (onSave 回调)：卸载保存
 * - 连接选择：有/无 udp:bind 节点时的渲染差异
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { UdpSendToConfig } from '@/components/panels/UdpSendToConfig';
import type { UdpSendToConfig as UdpSendToConfigType } from '@/types';
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
 * 创建基础 UdpSendToConfig 数据
 */
function createBaseConfig(
  overrides: Partial<UdpSendToConfigType> = {},
): UdpSendToConfigType {
  return {
    type: 'udp:send_to',
    connection_id: 'udp-bind-001',
    data: 'ping',
    data_encoding: 'text',
    target_addr: '127.0.0.1',
    target_port: 9000,
    send_timeout_ms: 30000,
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

describe('UdpSendToConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 UDP 发送节点标题', () => {
      setupStoreMock(false);
      render(<UdpSendToConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('UDP 发送节点')).toBeDefined();
    });

    it('渲染目标地址、目标端口与数据输入的当前值', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ target_addr: '10.0.0.8', target_port: 12345, data: 'hello' });
      render(<UdpSendToConfig data={config} onSave={mockOnSave} />);

      const addrInput = document.getElementById('target_addr') as HTMLInputElement;
      expect(addrInput?.value).toBe('10.0.0.8');

      const portInput = document.getElementById('target_port') as HTMLInputElement;
      expect(portInput?.value).toBe('12345');

      const dataInput = document.getElementById('data') as HTMLTextAreaElement;
      expect(dataInput?.value).toBe('hello');
    });
  });

  describe('数据保存', () => {
    it('修改数据内容后卸载时保存新值', async () => {
      setupStoreMock(false);
      const user = userEvent.setup();
      const { unmount } = render(
        <UdpSendToConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const dataInput = document.getElementById('data') as HTMLTextAreaElement;
      await user.clear(dataInput);
      await user.type(dataInput, 'pong');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ data: 'pong', target_addr: '127.0.0.1', target_port: 9000 }),
        );
      });
    });
  });

  describe('连接选择', () => {
    it('有 udp:bind 节点时显示 Select 选择器', () => {
      setupStoreMock(true);
      const config = createBaseConfig();
      render(<UdpSendToConfig data={config} onSave={mockOnSave} />);

      const selectTrigger = document.getElementById('connection_id');
      expect(selectTrigger).toBeDefined();
      // Select 模式下不显示手动输入提示
      expect(screen.queryByText(/未检测到 UDP Bind 节点/)).toBeNull();
    });

    it('无 udp:bind 节点时显示 Input 与提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<UdpSendToConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
      expect(screen.getByText(/未检测到 UDP Bind 节点/)).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
