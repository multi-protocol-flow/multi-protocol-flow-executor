/**
 * TcpReceiveConfig 组件测试
 * - 渲染表单字段：连接 ID、接收超时
 * - 数据保存 (onSave 回调)：卸载保存
 * - 连接选择：有/无 tcp_connect 节点时的渲染差异
 */
import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TcpReceiveConfig } from '@/components/panels/TcpReceiveConfig';
import type { TcpReceiveConfig as TcpReceiveConfigType } from '@/types';
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
 * 创建基础 TcpReceiveConfig 数据
 */
function createBaseConfig(
  overrides: Partial<TcpReceiveConfigType> = {},
): TcpReceiveConfigType {
  return {
    type: 'tcp_receive',
    connection_id: 'tcp-conn-001',
    receive_timeout_ms: 5000,
    framing_mode: '',
    framing_config: {},
    ...overrides,
  };
}

/**
 * 配置 store mock，模拟包含 tcp_connect 节点的 flow
 */
function setupStoreMock(withTcpConnect: boolean = true) {
  const nodes: Array<{
    uuid: string;
    type: string;
    name: string;
    position: { x: number; y: number };
    data: Record<string, unknown>;
  }> = [];

  if (withTcpConnect) {
    nodes.push({
      uuid: 'tcp-conn-001',
      type: 'tcp_connect',
      name: 'TCP 连接 1',
      position: { x: 200, y: 0 },
      data: {
        type: 'tcp_connect',
        host: '127.0.0.1',
        port: 8080,
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

describe('TcpReceiveConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 TCP 接收节点标题', () => {
      setupStoreMock(false);
      render(<TcpReceiveConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('TCP 接收节点')).toBeDefined();
    });

    it('渲染连接 ID 与接收超时的当前值', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ receive_timeout_ms: 10000 });
      render(<TcpReceiveConfig data={config} onSave={mockOnSave} />);

      const connInput = document.getElementById('connection_id') as HTMLInputElement;
      expect(connInput?.value).toBe('tcp-conn-001');

      const timeoutInput = document.getElementById('receive_timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('10000');
    });
  });

  describe('数据保存', () => {
    it('修改接收超时后卸载时保存新值', async () => {
      setupStoreMock(false);
      const user = userEvent.setup();
      const { unmount } = render(
        <TcpReceiveConfig data={createBaseConfig()} onSave={mockOnSave} />,
      );

      const timeoutInput = document.getElementById('receive_timeout_ms') as HTMLInputElement;
      await user.clear(timeoutInput);
      await user.type(timeoutInput, '15000');

      unmount();

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ receive_timeout_ms: 15000, connection_id: 'tcp-conn-001' }),
        );
      });
    });
  });

  describe('连接选择', () => {
    it('有 tcp_connect 节点时显示 Select 选择器', () => {
      setupStoreMock(true);
      const config = createBaseConfig();
      render(<TcpReceiveConfig data={config} onSave={mockOnSave} />);

      const selectTrigger = document.getElementById('connection_id');
      expect(selectTrigger).toBeDefined();
      // Select 模式下不显示手动输入提示
      expect(screen.queryByText(/未检测到 TCP Connect 节点/)).toBeNull();
    });

    it('无 tcp_connect 节点时显示 Input 与提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<TcpReceiveConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
      expect(screen.getByText(/未检测到 TCP Connect 节点/)).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
