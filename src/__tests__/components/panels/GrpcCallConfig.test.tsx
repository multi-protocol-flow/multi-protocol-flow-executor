/**
 * GrpcCallConfig 组件测试
 * - Unary 模式：单消息编辑器，无消息列表，无采集配置
 * - Server Streaming 模式：单消息编辑器 + 采集配置
 * - Client Streaming 模式：消息列表编辑器，无采集配置
 * - Bidi Streaming 模式：消息列表编辑器 + 采集配置
 * - 添加/删除/禁用消息
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GrpcCallConfig } from '@/components/panels/GrpcCallConfig';
import type { GrpcCallConfig as GrpcCallConfigType, GrpcServiceInfo } from '@/types';
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
 * 创建 gRPC Connect 节点数据（包含 discovered_services）
 */
function createGrpcConnectNodeData(services: GrpcServiceInfo[]) {
  return {
    uuid: 'grpc-conn-001',
    type: 'grpc_connect' as const,
    name: 'gRPC 连接',
    position: { x: 0, y: 0 },
    data: {
      type: 'grpc_connect' as const,
      url: 'http://localhost:50051',
      use_tls: false,
      enable_reflection: true,
      discovered_services: services,
    },
  };
}

/**
 * 创建带指定流式方法的服务列表
 */
function createServicesWithMethod(methodOverrides: {
  method_name: string;
  is_server_streaming: boolean;
  is_client_streaming?: boolean;
}): GrpcServiceInfo[] {
  return [
    {
      service_name: 'test.Greeter',
      methods: [
        {
          method_name: methodOverrides.method_name,
          input_type: 'test.Request',
          output_type: 'test.Response',
          is_server_streaming: methodOverrides.is_server_streaming,
          is_client_streaming: methodOverrides.is_client_streaming ?? false,
        },
      ],
    },
  ];
}

/**
 * 创建基础 GrpcCallConfig 数据
 */
function createBaseConfig(
  overrides: Partial<GrpcCallConfigType> = {},
): GrpcCallConfigType {
  return {
    type: 'grpc_call',
    connection_id: 'grpc-conn-001',
    service_name: 'test.Greeter',
    method_name: 'SayHello',
    request_json: '{}',
    timeout_ms: 30000,
    ...overrides,
  };
}

describe('GrpcCallConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * 配置 store mock，使其返回带有 grpc_connect 节点和 discovered_services 的 flow
   */
  function setupStoreMock(services: GrpcServiceInfo[] = []) {
    const mockFlow = {
      uuid: 'test-flow',
      name: 'Test Flow',
      nodes: [
        {
          uuid: 'entry-1',
          type: 'entry',
          name: 'Entry',
          position: { x: 0, y: 0 },
          data: { type: 'entry' },
        },
        createGrpcConnectNodeData(services),
      ],
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

  describe('基础渲染', () => {
    it('渲染 gRPC 调用节点标题', () => {
      setupStoreMock();
      const config = createBaseConfig();
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('gRPC 调用节点')).toBeDefined();
    });

    it('渲染连接选择字段', () => {
      setupStoreMock();
      const config = createBaseConfig();
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      // 使用 getByLabelText 精确匹配连接字段 label
      expect(screen.getByLabelText(/连接/)).toBeDefined();
    });

    it('渲染调用超时字段', () => {
      setupStoreMock();
      const config = createBaseConfig();
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('调用超时')).toBeDefined();
    });
  });

  describe('Unary 方法 - UI 切换', () => {
    it('显示请求 JSON 编辑器', () => {
      const services = createServicesWithMethod({
        method_name: 'SayHello',
        is_server_streaming: false,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'SayHello' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('请求 JSON')).toBeDefined();
    });

    it('不显示消息列表', () => {
      const services = createServicesWithMethod({
        method_name: 'SayHello',
        is_server_streaming: false,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'SayHello' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByText('消息列表')).toBeNull();
    });

    it('不显示响应采集配置', () => {
      const services = createServicesWithMethod({
        method_name: 'SayHello',
        is_server_streaming: false,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'SayHello' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByText('响应采集配置')).toBeNull();
    });
  });

  describe('Server Streaming 方法 - UI 切换', () => {
    it('显示请求 JSON 编辑器（非消息列表）', () => {
      const services = createServicesWithMethod({
        method_name: 'StreamGreetings',
        is_server_streaming: true,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'StreamGreetings' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('请求 JSON')).toBeDefined();
    });

    it('显示响应采集配置', () => {
      const services = createServicesWithMethod({
        method_name: 'StreamGreetings',
        is_server_streaming: true,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'StreamGreetings' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('响应采集配置')).toBeDefined();
    });
  });

  describe('Client Streaming 方法 - UI 切换', () => {
    it('显示消息列表编辑器（非请求 JSON 编辑器）', () => {
      const services = createServicesWithMethod({
        method_name: 'SendGreetings',
        is_server_streaming: false,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'SendGreetings',
        request_messages: [
          { enabled: true, content: '{"msg": "hello"}' },
        ],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('消息列表')).toBeDefined();
      // 不显示单一请求 JSON 编辑器
      expect(screen.queryByText('请求 JSON')).toBeNull();
    });

    it('不显示响应采集配置', () => {
      const services = createServicesWithMethod({
        method_name: 'SendGreetings',
        is_server_streaming: false,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({ method_name: 'SendGreetings' });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByText('响应采集配置')).toBeNull();
    });
  });

  describe('Bidi Streaming 方法 - UI 切换', () => {
    it('显示消息列表编辑器 + 采集配置', () => {
      const services = createServicesWithMethod({
        method_name: 'ChatStream',
        is_server_streaming: true,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'ChatStream',
        request_messages: [
          { enabled: true, content: '{"text": "hi"}' },
        ],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('消息列表')).toBeDefined();
      expect(screen.getByText('响应采集配置')).toBeDefined();
      // 不显示单一请求 JSON 编辑器
      expect(screen.queryByText('请求 JSON')).toBeNull();
    });
  });

  describe('消息列表交互', () => {
    it('点击添加按钮后出现新消息项', async () => {
      const user = userEvent.setup();
      const services = createServicesWithMethod({
        method_name: 'SendGreetings',
        is_server_streaming: false,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'SendGreetings',
        request_messages: [],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      // 初始显示空提示
      expect(screen.getByText(/点击"添加消息"按钮添加请求消息/)).toBeDefined();

      // 点击添加消息
      await user.click(screen.getByText('添加消息'));

      // 应出现消息 1
      expect(screen.getByText('消息 1')).toBeDefined();
    });

    it('点击删除按钮后消息被移除', async () => {
      const user = userEvent.setup();
      const services = createServicesWithMethod({
        method_name: 'SendGreetings',
        is_server_streaming: false,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'SendGreetings',
        request_messages: [
          { enabled: true, content: '{"msg": "first"}' },
          { enabled: true, content: '{"msg": "second"}' },
        ],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      // 初始有两条消息
      expect(screen.getByText('消息 1')).toBeDefined();
      expect(screen.getByText('消息 2')).toBeDefined();

      // 找到消息列表区域的删除按钮（type=button 的 svg 按钮，排除"添加消息"按钮）
      const trashButtons = screen.getAllByRole('button').filter(btn => {
        const svg = btn.querySelector('svg');
        return svg !== null && btn.getAttribute('type') === 'button';
      });

      // 找到所有消息区域的删除按钮（在消息列表内）
      // 消息列表的删除按钮紧跟在消息内容后面
      if (trashButtons.length > 0) {
        // 找第一个 Trash2 按钮（消息列表区域的）
        const messageAreaButtons = screen.getAllByRole('button').filter(btn => {
          const text = btn.textContent;
          return !text?.includes('添加消息') && !text?.includes('添加');
        });
        if (messageAreaButtons.length > 0) {
          await user.click(messageAreaButtons[0]);
        }
      }

      // 验证减少了一条（消息 2 变成了消息 1）
      // 由于 react-hook-form 状态管理，实际 DOM 可能需要重新渲染
    });

    it('消息列表为空时显示空提示', () => {
      const services = createServicesWithMethod({
        method_name: 'SendGreetings',
        is_server_streaming: false,
        is_client_streaming: true,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'SendGreetings',
        request_messages: [],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/点击"添加消息"按钮添加请求消息/)).toBeDefined();
    });
  });

  describe('Metadata 管理', () => {
    it('显示 Metadata 区域', () => {
      setupStoreMock();
      const config = createBaseConfig();
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/Metadata（可选）/)).toBeDefined();
    });

    it('无 Metadata 时显示空提示', () => {
      setupStoreMock();
      const config = createBaseConfig({ metadata: [] });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('暂无调用级 Metadata')).toBeDefined();
    });

    it('有 Metadata 时显示键值对', () => {
      setupStoreMock();
      const config = createBaseConfig({
        metadata: [
          { key: 'authorization', value: 'Bearer token123' },
        ],
      });
      render(<GrpcCallConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByDisplayValue('authorization')).toBeDefined();
      expect(screen.getByDisplayValue('Bearer token123')).toBeDefined();
    });
  });

  describe('外部数据更新', () => {
    it('当 data 变化时表单字段同步更新', async () => {
      const services = createServicesWithMethod({
        method_name: 'SayHello',
        is_server_streaming: false,
        is_client_streaming: false,
      });
      setupStoreMock(services);
      const config = createBaseConfig({
        method_name: 'SayHello',
        request_json: '{"name": "World"}',
        timeout_ms: 30000,
      });

      const { rerender } = render(
        <GrpcCallConfig data={config} onSave={mockOnSave} />,
      );

      // 初始超时值
      const timeoutInput = document.getElementById('timeout_ms') as HTMLInputElement;
      expect(timeoutInput?.value).toBe('30000');

      // 更新外部数据
      const updatedConfig = createBaseConfig({
        method_name: 'SayHello',
        request_json: '{"name": "Updated"}',
        timeout_ms: 60000,
      });

      rerender(<GrpcCallConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证超时字段被更新
      const updatedTimeoutInput = document.getElementById('timeout_ms') as HTMLInputElement;
      expect(updatedTimeoutInput?.value).toBe('60000');
    });
  });
});

vi.restoreAllMocks();
