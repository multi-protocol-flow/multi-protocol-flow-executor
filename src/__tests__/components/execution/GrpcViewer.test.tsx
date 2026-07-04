/**
 * GrpcViewer 组件测试
 * - gRPC Connect/Call/Close 三种节点的执行结果展示
 * - 流式模式 (Unary/Server/Client/Bidi) Badge 显示
 * - 流式统计、响应时间线、Trailers 显示
 * - 错误响应显示
 * - 旧数据格式向后兼容
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GrpcViewer } from '@/components/execution/viewers/GrpcViewer';
import { getLL } from '@/i18n/i18n-lazy';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

// Helper: 创建 mock report
function createMockReport(
  nodeType: string,
  outputData?: Record<string, unknown>,
  overrides?: Partial<NodeExecutionReport>,
): NodeExecutionReport {
  return {
    node_uuid: `node-${nodeType}-001`,
    node_type: nodeType,
    node_name: `gRPC ${nodeType}`,
    start_time: Date.now() - 200,
    end_time: Date.now(),
    duration_ms: 200,
    status: 'success',
    output_data: outputData,
    ...overrides,
  };
}

// Helper: 展开折叠面板
async function expandSection(user: ReturnType<typeof userEvent.setup>, title: string) {
  const buttons = screen.getAllByRole('button');
  const btn = buttons.find(b => b.textContent?.includes(title));
  if (btn && btn.getAttribute('aria-expanded') === 'false') {
    await user.click(btn);
  }
}

// 更精确的展开面板：匹配标题文本但不包含排除文本
async function expandSectionExact(user: ReturnType<typeof userEvent.setup>, include: string, exclude?: string) {
  const buttons = screen.getAllByRole('button');
  const btn = buttons.find(b => {
    const text = b.textContent ?? '';
    return text.includes(include) && (!exclude || !text.includes(exclude));
  });
  if (btn && btn.getAttribute('aria-expanded') === 'false') {
    await user.click(btn);
  }
}

describe('GrpcViewer', () => {
  describe('gRPC Connect 节点', () => {
    it('显示连接配置 URL', () => {
      const report = createMockReport('grpc_connect', {
        success: true,
        url: 'http://localhost:50051',
        request: { url: 'http://localhost:50051', use_tls: false, enable_reflection: true },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('http://localhost:50051')).toBeDefined();
    });

    it('显示已连接状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_connect', {
        success: true,
        services: [],
      });
      render(<GrpcViewer report={report} />);

      await expandSection(user, '连接状态');
      expect(screen.getByText('已连接')).toBeDefined();
    });

    it('显示连接失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_connect', {
        success: false,
        error: '连接被拒绝',
      });
      render(<GrpcViewer report={report} />);

      await expandSection(user, '连接状态');
      expect(screen.getByText(getLL().execution.viewers.disconnected())).toBeDefined();
      // 错误信息出现在两个地方（span + p），使用 getAllByText
      const errorElements = screen.getAllByText('连接被拒绝');
      expect(errorElements.length).toBeGreaterThanOrEqual(1);
    });

    it('显示服务发现列表', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_connect', {
        success: true,
        services: [
          {
            service_name: 'test.Greeter',
            methods: [
              { method_name: 'SayHello', input_type: 'HelloRequest', output_type: 'HelloReply', is_server_streaming: false },
              { method_name: 'StreamGreetings', input_type: 'HelloRequest', output_type: 'HelloReply', is_server_streaming: true },
            ],
          },
        ],
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('可用服务')).toBeDefined();
      await expandSection(user, '可用服务');
      expect(screen.getByText('test.Greeter')).toBeDefined();
      expect(screen.getByText('SayHello')).toBeDefined();
      expect(screen.getByText('StreamGreetings')).toBeDefined();
    });
  });

  describe('gRPC Call - Unary 模式', () => {
    it('显示 Unary Badge（绿色）', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'unary',
        service_name: 'test.Greeter',
        method_name: 'SayHello',
        data: { message: 'Hello' },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('Unary')).toBeDefined();
      // Badge 包含服务方法名（可能出现在调用配置 + 响应状态两处）
      const serviceMethodMatches = screen.getAllByText(/test\.Greeter\/SayHello/);
      expect(serviceMethodMatches.length).toBeGreaterThanOrEqual(1);
    });

    it('显示单一响应数据', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'unary',
        service_name: 'test.Greeter',
        method_name: 'SayHello',
        data: { message: 'Hello, World!' },
      });
      render(<GrpcViewer report={report} />);

      // 展开响应面板（section_response = '响应'，排除"响应状态"）
      await expandSectionExact(user, '响应', '响应状态');
      expect(screen.getByTestId('json-view')).toBeDefined();
    });
  });

  describe('gRPC Call - Server Streaming 模式', () => {
    it('显示 Server Stream Badge（琥珀色）', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'server_streaming',
        service_name: 'test.Greeter',
        method_name: 'StreamGreetings',
        sent_count: 1,
        received_count: 3,
        responses: [
          { index: 1, data: { message: 'Hello 1' }, received_at_ms: 10 },
          { index: 2, data: { message: 'Hello 2' }, received_at_ms: 20 },
          { index: 3, data: { message: 'Hello 3' }, received_at_ms: 30 },
        ],
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('服务端流式')).toBeDefined();
    });

    it('显示流式统计（sent_count / received_count）', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'server_streaming',
        service_name: 'test.Greeter',
        method_name: 'StreamGreetings',
        sent_count: 1,
        received_count: 3,
        responses: [
          { index: 1, data: { msg: 'a' }, received_at_ms: 10 },
        ],
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('流式统计')).toBeDefined();
      // 统计面板默认展开，检查数值
      expect(screen.getByText('发送消息数')).toBeDefined();
      expect(screen.getByText('接收消息数')).toBeDefined();
    });

    it('显示响应消息时间线', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'server_streaming',
        service_name: 'test.Greeter',
        method_name: 'StreamGreetings',
        sent_count: 1,
        received_count: 2,
        responses: [
          { index: 1, data: { message: 'resp1' }, received_at_ms: 10 },
          { index: 2, data: { message: 'resp2' }, received_at_ms: 25 },
        ],
      });
      render(<GrpcViewer report={report} />);

      // 响应消息面板标题包含数量（格式: "响应消息 (2)"）
      expect(screen.getByText('响应消息 (2)')).toBeDefined();

      await expandSection(user, '响应消息');
      // 检查时间线显示（格式: +Nms）
      expect(screen.getByText('+10ms')).toBeDefined();
      expect(screen.getByText('+25ms')).toBeDefined();
    });
  });

  describe('gRPC Call - Client Streaming 模式', () => {
    it('显示 Client Stream Badge（蓝色）', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'client_streaming',
        service_name: 'test.Greeter',
        method_name: 'SendGreetings',
        sent_count: 5,
        received_count: 1,
        data: { summary: 'processed 5 messages' },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('客户端流式')).toBeDefined();
    });

    it('显示单一响应数据', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'client_streaming',
        service_name: 'test.Greeter',
        method_name: 'SendGreetings',
        sent_count: 5,
        received_count: 1,
        data: { summary: 'processed' },
      });
      render(<GrpcViewer report={report} />);

      await expandSectionExact(user, '响应', '响应状态');
      expect(screen.getByTestId('json-view')).toBeDefined();
    });
  });

  describe('gRPC Call - Bidi Streaming 模式', () => {
    it('显示 Bidi Stream Badge（紫色）', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'bidi_streaming',
        service_name: 'test.Chat',
        method_name: 'ChatStream',
        sent_count: 3,
        received_count: 3,
        responses: [
          { index: 1, data: { text: 'hi' }, received_at_ms: 10 },
          { index: 2, data: { text: 'hello' }, received_at_ms: 20 },
          { index: 3, data: { text: 'bye' }, received_at_ms: 30 },
        ],
        trailers: { 'grpc-status': '0' },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('双向流式')).toBeDefined();
    });

    it('显示流式统计和响应时间线', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'bidi_streaming',
        service_name: 'test.Chat',
        method_name: 'ChatStream',
        sent_count: 3,
        received_count: 3,
        responses: [
          { index: 1, data: { text: 'hi' }, received_at_ms: 10 },
          { index: 2, data: { text: 'hello' }, received_at_ms: 20 },
        ],
      });
      render(<GrpcViewer report={report} />);

      // 流式统计
      expect(screen.getByText('流式统计')).toBeDefined();

      // 响应消息
      expect(screen.getByText('响应消息 (2)')).toBeDefined();
      await expandSection(user, '响应消息');
      expect(screen.getByText('+10ms')).toBeDefined();
    });

    it('显示 Trailers', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: true,
        pattern: 'bidi_streaming',
        service_name: 'test.Chat',
        method_name: 'ChatStream',
        sent_count: 1,
        received_count: 1,
        responses: [{ index: 1, data: {}, received_at_ms: 5 }],
        trailers: { 'grpc-status': '0', 'grpc-message': 'OK' },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('Trailers')).toBeDefined();
      await expandSection(user, 'Trailers');
      expect(screen.getByText('grpc-message:')).toBeDefined();
      // "OK" 可能出现在多处（状态 Badge + Trailers 值），使用 getAllByText
      const okElements = screen.getAllByText('OK');
      expect(okElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('gRPC Call - 错误响应', () => {
    it('显示错误信息', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_call', {
        success: false,
        pattern: 'unary',
        service_name: 'test.Greeter',
        method_name: 'SayHello',
        error: 'gRPC status UNAVAILABLE: connection refused',
      }, { status: 'failed' });
      render(<GrpcViewer report={report} />);

      await expandSection(user, '响应状态');
      // 错误信息出现在两个地方（span + p），使用 getAllByText
      const errorElements = screen.getAllByText('gRPC status UNAVAILABLE: connection refused');
      expect(errorElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('gRPC Call - 旧数据格式向后兼容', () => {
    it('无 pattern 字段时根据 isStreamingData 判断为 Unary', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        service_name: 'test.Greeter',
        method_name: 'SayHello',
        // 旧格式没有 pattern，data 为对象（非数组）→ Unary
        data: { message: 'Hello' },
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('Unary')).toBeDefined();
    });

    it('无 pattern 字段且 data 为数组时判断为 Server Stream', () => {
      const report = createMockReport('grpc_call', {
        success: true,
        service_name: 'test.Greeter',
        method_name: 'StreamGreetings',
        // 旧格式：data 为数组 → Streaming
        data: [{ message: 'Hello 1' }, { message: 'Hello 2' }],
      });
      render(<GrpcViewer report={report} />);

      expect(screen.getByText('服务端流式')).toBeDefined();
    });
  });

  describe('gRPC Close 节点', () => {
    it('显示已关闭状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_close', {
        success: true,
        connection_id: 'conn-123',
      });
      render(<GrpcViewer report={report} />);

      await expandSection(user, '关闭状态');
      expect(screen.getByText('已关闭')).toBeDefined();
    });

    it('显示关闭失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('grpc_close', {
        success: false,
        error: '连接不存在',
      }, { status: 'failed' });
      render(<GrpcViewer report={report} />);

      await expandSection(user, '关闭状态');
      expect(screen.getByText('关闭失败')).toBeDefined();
    });
  });

  describe('未知节点类型', () => {
    it('显示未知类型提示', () => {
      const report = createMockReport('grpc_unknown', {});
      render(<GrpcViewer report={report} />);

      expect(screen.getByText(/未知 gRPC 节点类型/)).toBeDefined();
    });
  });
});

vi.restoreAllMocks();
