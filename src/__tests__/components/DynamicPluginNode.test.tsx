/**
 * DynamicPluginNode 组件测试 — mcp:connect 发现快照数量徽章
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ReactFlowProvider, type NodeProps } from '@xyflow/react';
import { DynamicPluginNode } from '@/components/canvas/nodes';
import { useFlowStore } from '@/stores';

function TestWrapper({ children }: { children: React.ReactNode }) {
  return <ReactFlowProvider>{children}</ReactFlowProvider>;
}

/** DynamicPluginNode 是 React Flow NodeProps 组件；测试只提供它用到的字段 */
function renderNode(props: { type?: string; data?: Record<string, unknown> } = {}) {
  const { type = 'mcp:connect', data = {} } = props;
  const nodeProps = {
    id: 'node-1',
    type,
    data,
    selected: false,
  } as unknown as NodeProps;
  return render(
    <TestWrapper>
      <DynamicPluginNode {...nodeProps} />
    </TestWrapper>
  );
}

describe('DynamicPluginNode — mcp:connect 发现徽章', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  it('平铺 data 中的 discovered_* 数量渲染为徽章', () => {
    renderNode({
      data: {
        type: 'mcp:connect',
        transport: 'streamable_http',
        url: 'https://example.com/mcp',
        discovered_tools: [{ name: 'echo' }, { name: 'add' }],
        discovered_resources: [{ uri: 'config://app' }],
        discovered_prompts: [{ name: 'greet' }],
      },
    });

    expect(screen.getByTestId('mcp-tools-count').textContent).toBe('2');
    expect(screen.getByTestId('mcp-resources-count').textContent).toBe('1');
    expect(screen.getByTestId('mcp-prompts-count').textContent).toBe('1');
  });

  it('嵌套 data.config 中的 discovered_* 数量渲染为徽章', () => {
    renderNode({
      data: {
        config: {
          type: 'mcp:connect',
          discovered_tools: [{ name: 'echo' }],
          discovered_resources: [],
          discovered_prompts: [],
        },
      },
    });

    expect(screen.getByTestId('mcp-tools-count').textContent).toBe('1');
    expect(screen.queryByTestId('mcp-resources-count')).toBeNull();
    expect(screen.queryByTestId('mcp-prompts-count')).toBeNull();
  });

  it('发现为空时不渲染徽章', () => {
    renderNode({
      data: {
        type: 'mcp:connect',
        discovered_tools: [],
        discovered_resources: [],
        discovered_prompts: [],
      },
    });

    expect(screen.queryByTestId('mcp-tools-count')).toBeNull();
    expect(screen.queryByTestId('mcp-resources-count')).toBeNull();
    expect(screen.queryByTestId('mcp-prompts-count')).toBeNull();
  });

  it('无 discovered_* 字段时不渲染徽章', () => {
    renderNode({
      data: { type: 'mcp:connect', transport: 'stdio', command: 'npx' },
    });

    expect(screen.queryByTestId('mcp-tools-count')).toBeNull();
    expect(screen.queryByTestId('mcp-resources-count')).toBeNull();
    expect(screen.queryByTestId('mcp-prompts-count')).toBeNull();
  });

  it('非 mcp:connect 节点即使带 discovered_* 也不渲染徽章', () => {
    renderNode({
      type: 'redis:connect',
      data: {
        type: 'redis:connect',
        discovered_tools: [{ name: 'echo' }],
      },
    });

    expect(screen.queryByTestId('mcp-tools-count')).toBeNull();
    expect(screen.queryByTestId('mcp-resources-count')).toBeNull();
    expect(screen.queryByTestId('mcp-prompts-count')).toBeNull();
  });
});

describe('DynamicPluginNode — grpc:connect 发现徽章', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  it('平铺 data 中的 discovered_services 数量渲染为徽章', () => {
    renderNode({
      type: 'grpc:connect',
      data: {
        type: 'grpc:connect',
        url: 'http://localhost:50051',
        discovered_services: [
          {
            service_name: 'helloworld.Greeter',
            methods: [
              { name: 'SayHello', is_server_streaming: false, is_client_streaming: false },
              { name: 'SayHelloStream', is_server_streaming: true, is_client_streaming: false },
            ],
          },
          {
            service_name: 'calculator.Calculator',
            methods: [
              { name: 'Add', is_server_streaming: false, is_client_streaming: false },
            ],
          },
        ],
      },
    });

    expect(screen.getByTestId('grpc-services-count').textContent).toBe('2');
    expect(screen.getByTestId('grpc-methods-count').textContent).toBe('3');
    expect(screen.getByTestId('grpc-streaming-count').textContent).toBe('1');
  });

  it('嵌套 config 中的 discovered_services 数量渲染为徽章', () => {
    renderNode({
      type: 'grpc:connect',
      data: {
        config: {
          type: 'grpc:connect',
          discovered_services: [
            {
              service_name: 'test.Service',
              methods: [
                { name: 'Unary', is_server_streaming: false, is_client_streaming: false },
              ],
            },
          ],
        },
      },
    });

    expect(screen.getByTestId('grpc-services-count').textContent).toBe('1');
    expect(screen.getByTestId('grpc-methods-count').textContent).toBe('1');
    expect(screen.queryByTestId('grpc-streaming-count')).toBeNull();
  });

  it('发现服务为空时不渲染徽章', () => {
    renderNode({
      type: 'grpc:connect',
      data: {
        type: 'grpc:connect',
        discovered_services: [],
      },
    });

    expect(screen.queryByTestId('grpc-services-count')).toBeNull();
    expect(screen.queryByTestId('grpc-methods-count')).toBeNull();
    expect(screen.queryByTestId('grpc-streaming-count')).toBeNull();
  });
});
