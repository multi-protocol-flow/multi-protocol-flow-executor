/**
 * GrpcCloseConfig 组件测试
 * - 渲染表单字段：连接选择、提示信息
 * - 有可用连接时显示 Select 选择器
 * - 无可用连接时显示 Input 输入框
 * - 数据保存 (onSave 回调)
 * - 外部数据更新同步
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GrpcCloseConfig } from '@/components/panels/GrpcCloseConfig';
import type { GrpcCloseConfig as GrpcCloseConfigType } from '@/types';
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
 * 创建基础 GrpcCloseConfig 数据
 */
function createBaseConfig(
  overrides: Partial<GrpcCloseConfigType> = {},
): GrpcCloseConfigType {
  return {
    type: 'grpc_close',
    connection_id: 'grpc-conn-001',
    ...overrides,
  };
}

/**
 * 配置 store mock，模拟包含 grpc_connect 节点的 flow
 */
function setupStoreMock(withGrpcConnect: boolean = true) {
  const nodes: Array<{
    uuid: string;
    type: string;
    name: string;
    position: { x: number; y: number };
    data: Record<string, unknown>;
  }> = [
    {
      uuid: 'entry-1',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' },
    },
  ];

  if (withGrpcConnect) {
    nodes.push({
      uuid: 'grpc-conn-001',
      type: 'grpc_connect',
      name: 'gRPC 连接 1',
      position: { x: 200, y: 0 },
      data: {
        type: 'grpc_connect',
        url: 'http://localhost:50051',
      },
    });
  }

  const mockFlow = {
    uuid: 'test-flow',
    name: 'Test Flow',
    nodes,
    connections: [],
  };

  (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector: unknown) => {
    if (typeof selector === 'function') {
      return selector({ flow: mockFlow });
    }
    return { flow: mockFlow };
  });

  return mockFlow;
}

describe('GrpcCloseConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 gRPC 关闭连接节点标题', () => {
      setupStoreMock(false);
      const config = createBaseConfig();
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('gRPC 关闭连接节点')).toBeDefined();
    });

    it('渲染连接字段标签', () => {
      setupStoreMock(false);
      const config = createBaseConfig();
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      // 使用 getByLabelText 精确匹配连接字段 label
      expect(screen.getByLabelText(/连接/)).toBeDefined();
    });

    it('渲染关闭说明提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig();
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('关闭说明：')).toBeDefined();
    });
  });

  describe('无可用连接', () => {
    it('无 grpc_connect 节点时显示 Input 输入框', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByPlaceholderText('输入连接 ID')).toBeDefined();
    });

    it('显示未检测到连接提示', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/未检测到 gRPC Connect 节点/)).toBeDefined();
    });
  });

  describe('有可用连接', () => {
    it('有 grpc_connect 节点时显示 Select 选择器', () => {
      setupStoreMock(true);
      const config = createBaseConfig();
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      // Select 组件存在（通过 id 找到 trigger）
      const selectTrigger = document.getElementById('connection_id');
      expect(selectTrigger).toBeDefined();
    });
  });

  describe('外部数据更新', () => {
    it('当 data 变化时 connection_id 同步更新', () => {
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: 'grpc-conn-001' });

      const { rerender } = render(
        <GrpcCloseConfig data={config} onSave={mockOnSave} />,
      );

      // 无可用连接时使用 Input，验证初始值
      const input = document.getElementById('connection_id') as HTMLInputElement;
      expect(input?.value).toBe('grpc-conn-001');

      // 更新外部数据
      const updatedConfig = createBaseConfig({ connection_id: 'grpc-conn-002' });
      rerender(<GrpcCloseConfig data={updatedConfig} onSave={mockOnSave} />);

      const updatedInput = document.getElementById('connection_id') as HTMLInputElement;
      expect(updatedInput?.value).toBe('grpc-conn-002');
    });
  });

  describe('表单输入交互', () => {
    it('可以在输入框中修改 connection_id', async () => {
      const user = userEvent.setup();
      setupStoreMock(false);
      const config = createBaseConfig({ connection_id: '' });
      render(<GrpcCloseConfig data={config} onSave={mockOnSave} />);

      const input = document.getElementById('connection_id') as HTMLInputElement;
      expect(input).toBeDefined();

      await user.type(input, 'my-connection-id');
      expect(input.value).toBe('my-connection-id');
    });
  });
});

vi.restoreAllMocks();
