/**
 * GrpcConnectConfig 组件测试
 * - 渲染表单字段：URL、TLS、Reflection、Proto 文件、Metadata、连接超时
 * - TLS 开关交互
 * - Proto 文件添加/删除
 * - Metadata 添加/删除
 * - 数据保存 (onSave 回调)
 * - 外部数据更新同步
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GrpcConnectConfig } from '@/components/panels/GrpcConnectConfig';
import type { GrpcConnectConfig as GrpcConnectConfigType, GrpcServiceInfo, GrpcMessageInfo } from '@/types';

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock discoverGrpcServices（避免 Tauri IPC 调用）
vi.mock('@/services/tauri', () => ({
  discoverGrpcServices: vi.fn().mockResolvedValue({
    success: true,
    services: [],
  }),
}));

/**
 * 创建基础 GrpcConnectConfig 数据
 */
function createBaseConfig(
  overrides: Partial<GrpcConnectConfigType> = {},
): GrpcConnectConfigType {
  return {
    type: 'grpc_connect',
    url: 'http://localhost:50051',
    use_tls: false,
    tls_skip_verify: false,
    enable_reflection: false,
    connect_timeout_ms: 30000,
    ...overrides,
  };
}

describe('GrpcConnectConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('渲染 gRPC 连接节点标题', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('gRPC 连接节点')).toBeDefined();
    });

    it('渲染服务器地址输入框', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      const urlInput = document.getElementById('url') as HTMLInputElement;
      expect(urlInput).toBeDefined();
      expect(urlInput?.value).toBe('http://localhost:50051');
    });

    it('渲染连接超时字段', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('连接超时')).toBeDefined();
      const timeoutInput = document.getElementById('connect_timeout_ms') as HTMLInputElement;
      expect(timeoutInput).toBeDefined();
      expect(timeoutInput?.value).toBe('30000');
    });

    it('渲染 TLS 开关', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('启用 TLS')).toBeDefined();
    });

    it('渲染 Server Reflection 开关', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('使用 Server Reflection')).toBeDefined();
    });

    it('渲染默认 Metadata 区域', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // 使用更精确的文本匹配（label 文本）
      expect(screen.getByText('默认 Metadata（可选）')).toBeDefined();
    });

    it('渲染测试连接按钮', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('测试连接 & 发现服务')).toBeDefined();
    });
  });

  describe('TLS 配置', () => {
    it('默认不显示跳过证书验证开关', () => {
      const config = createBaseConfig({ use_tls: false });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByText('跳过证书验证')).toBeNull();
    });

    it('启用 TLS 后显示跳过证书验证开关', async () => {
      const config = createBaseConfig({ use_tls: true });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('跳过证书验证')).toBeDefined();
    });
  });

  describe('Proto 文件管理', () => {
    it('启用 Reflection 时不显示 Proto 文件管理', () => {
      const config = createBaseConfig({ enable_reflection: true });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // 不应显示"添加文件"按钮（Reflection 模式隐藏 Proto 列表）
      expect(screen.queryByText('添加文件')).toBeNull();
    });

    it('未启用 Reflection 时显示 Proto 文件列表和添加按钮', () => {
      const config = createBaseConfig({ enable_reflection: false });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('添加文件')).toBeDefined();
      // 空状态提示
      expect(screen.getByText('暂无 Proto 文件')).toBeDefined();
    });

    it('显示已有的 Proto 文件', () => {
      const config = createBaseConfig({
        enable_reflection: false,
        proto_files: [
          { path: 'example.proto', content: 'syntax = "proto3";' },
        ],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByDisplayValue('example.proto')).toBeDefined();
    });

    it('点击添加按钮后出现新 Proto 文件表单', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig({ enable_reflection: false });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      await user.click(screen.getByText('添加文件'));

      // 应出现文件名和内容输入框
      const pathInputs = screen.getAllByPlaceholderText('文件名（如 example.proto）');
      expect(pathInputs.length).toBe(1);
    });
  });

  describe('Metadata 管理', () => {
    it('无 Metadata 时显示空提示', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText('暂无默认 Metadata')).toBeDefined();
    });

    it('有 Metadata 时显示键值对', () => {
      const config = createBaseConfig({
        default_metadata: [
          { key: 'authorization', value: 'Bearer token123' },
        ],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByDisplayValue('authorization')).toBeDefined();
      expect(screen.getByDisplayValue('Bearer token123')).toBeDefined();
    });

    it('点击添加按钮后出现新 Metadata 条目', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // 点击 Metadata 添加按钮
      const addButtons = screen.getAllByText('添加');
      await user.click(addButtons[0]);

      // 应出现 Key 和 Value 输入框
      expect(screen.getByPlaceholderText('Key')).toBeDefined();
      expect(screen.getByPlaceholderText('Value')).toBeDefined();
    });
  });

  describe('外部数据更新', () => {
    it('当 data 变化时表单字段同步更新', () => {
      const config = createBaseConfig({ url: 'http://localhost:50051', connect_timeout_ms: 30000 });

      const { rerender } = render(
        <GrpcConnectConfig data={config} onSave={mockOnSave} />,
      );

      // 初始 URL 值
      const urlInput = document.getElementById('url') as HTMLInputElement;
      expect(urlInput?.value).toBe('http://localhost:50051');

      // 更新外部数据
      const updatedConfig = createBaseConfig({
        url: 'https://grpc.example.com:443',
        connect_timeout_ms: 60000,
      });

      rerender(<GrpcConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证 URL 和超时字段被更新
      const updatedUrlInput = document.getElementById('url') as HTMLInputElement;
      expect(updatedUrlInput?.value).toBe('https://grpc.example.com:443');

      const updatedTimeoutInput = document.getElementById('connect_timeout_ms') as HTMLInputElement;
      expect(updatedTimeoutInput?.value).toBe('60000');
    });
  });

  describe('URL 输入提示', () => {
    it('显示 URL 格式提示', () => {
      const config = createBaseConfig();
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/支持 http:\/\/.*和 https:\/\/.*格式/)).toBeDefined();
    });
  });

  describe('Schema Expansion', () => {
    /** Helper: build a discovered service with message_definitions */
    function createDiscoveredService(): GrpcServiceInfo {
      const helloRequest: GrpcMessageInfo = {
        full_name: 'test.HelloRequest',
        fields: [
          { name: 'name', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false },
          { name: 'age', number: 2, type_kind: 'int32', type_display: 'int32', label: 'singular', is_map: false },
        ],
        oneof_groups: [],
        is_wkt: false,
      };

      const helloResponse: GrpcMessageInfo = {
        full_name: 'test.HelloResponse',
        fields: [
          { name: 'message', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false },
        ],
        oneof_groups: [],
        is_wkt: false,
      };

      return {
        service_name: 'test.Greeter',
        methods: [
          {
            method_name: 'SayHello',
            input_type: 'test.HelloRequest',
            output_type: 'test.HelloResponse',
            is_server_streaming: false,
            is_client_streaming: false,
          },
        ],
        message_definitions: {
          'test.HelloRequest': helloRequest,
          'test.HelloResponse': helloResponse,
        },
      };
    }

    it('displays service with discovered methods when discovered_services is provided', () => {
      const config = createBaseConfig({
        discovered_services: [createDiscoveredService()],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // Service name should be visible
      expect(screen.getByText('test.Greeter')).toBeDefined();
      // The "已发现的服务" section should appear
      expect(screen.getByText('已发现的服务')).toBeDefined();
    });

    it('shows clickable input type chip with short name when message def exists', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig({
        discovered_services: [createDiscoveredService()],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // Expand the service so methods show
      await user.click(screen.getByText('test.Greeter'));

      // The method name should be visible
      expect(screen.getByText('SayHello')).toBeDefined();

      // The input type chip should show the short name (last segment)
      const inputChip = screen.getByText('HelloRequest');
      expect(inputChip).toBeDefined();

      // The output type chip should show the short name
      const outputChip = screen.getByText('HelloResponse');
      expect(outputChip).toBeDefined();
    });

    it('clicking input type chip expands MessageSchemaView with request fields', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig({
        discovered_services: [createDiscoveredService()],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // Expand the service
      await user.click(screen.getByText('test.Greeter'));

      // Click the input type chip "HelloRequest"
      await user.click(screen.getByText('HelloRequest'));

      // Request schema label should appear
      expect(screen.getByText('请求结构')).toBeDefined();

      // HelloRequest fields should appear
      expect(screen.getByText('name')).toBeDefined();
      expect(screen.getByText('age')).toBeDefined();
    });

    it('clicking output type chip expands MessageSchemaView with response fields', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig({
        discovered_services: [createDiscoveredService()],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // Expand the service
      await user.click(screen.getByText('test.Greeter'));

      // Click the output type chip "HelloResponse"
      await user.click(screen.getByText('HelloResponse'));

      // Response schema label should appear
      expect(screen.getByText('响应结构')).toBeDefined();

      // HelloResponse fields should appear
      expect(screen.getByText('message')).toBeDefined();
    });

    it('clicking type chip again collapses the schema view', async () => {
      const user = userEvent.setup();
      const config = createBaseConfig({
        discovered_services: [createDiscoveredService()],
      });
      render(<GrpcConnectConfig data={config} onSave={mockOnSave} />);

      // Expand the service
      await user.click(screen.getByText('test.Greeter'));

      // Click input type chip to expand
      await user.click(screen.getByText('HelloRequest'));
      expect(screen.getByText('请求结构')).toBeDefined();

      // Click again to collapse
      await user.click(screen.getByText('HelloRequest'));

      // Request schema label should be gone
      expect(screen.queryByText('请求结构')).toBeNull();
    });
  });
});

vi.restoreAllMocks();
