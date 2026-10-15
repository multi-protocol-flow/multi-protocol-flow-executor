/**
 * IngressDebugWaitingToolbar 组件单元测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { IngressDebugWaitingToolbar } from '@/components/canvas/IngressDebugWaitingToolbar';
import { copyToClipboard } from '@/lib/clipboard';
import type { Flow } from '@/types/flow';

vi.mock('@/lib/clipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue(true),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('IngressDebugWaitingToolbar', () => {
  const mockOnStop = vi.fn();

  const mockFlow: Flow = {
    uuid: 'flow-ingress-test',
    name: 'Ingress Test Flow',
    nodes: [
      {
        uuid: 'ingress-1',
        type: 'http:ingress',
        name: 'HTTP Ingress',
        position: { x: 0, y: 0 },
        data: {
          type: 'http:ingress',
          port: 9527,
          path_pattern: '/webhook/stripe',
          methods: ['POST', 'GET'],
        },
      },
    ],
    connections: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('渲染 Ingress 调试监听标题与目标地址', () => {
    render(
      <IngressDebugWaitingToolbar
        port={9527}
        flow={mockFlow}
        onStop={mockOnStop}
      />
    );

    expect(screen.getByText('Ingress 调试监听中')).toBeInTheDocument();
    expect(screen.getByText('http://127.0.0.1:9527/webhook/stripe')).toBeInTheDocument();
    expect(screen.getByText('等待外部请求进入以触发断点调试...')).toBeInTheDocument();
  });

  it('点击复制 URL 按钮触发复制并提示成功', async () => {
    render(
      <IngressDebugWaitingToolbar
        port={9527}
        flow={mockFlow}
        onStop={mockOnStop}
      />
    );

    const copyBtn = screen.getByTitle('复制请求地址');
    await act(async () => {
      fireEvent.click(copyBtn);
    });

    expect(copyToClipboard).toHaveBeenCalledWith('http://127.0.0.1:9527/webhook/stripe');
  });

  it('点击复制 cURL 按钮触发复制测试命令', async () => {
    render(
      <IngressDebugWaitingToolbar
        port={9527}
        flow={mockFlow}
        onStop={mockOnStop}
      />
    );

    const curlBtn = screen.getByTitle('复制 cURL 测试命令');
    await act(async () => {
      fireEvent.click(curlBtn);
    });

    expect(copyToClipboard).toHaveBeenCalledWith(
      expect.stringContaining('curl -X POST http://127.0.0.1:9527/webhook/stripe')
    );
  });

  it('点击停止按钮调用 onStop 回调', () => {
    render(
      <IngressDebugWaitingToolbar
        port={9527}
        flow={mockFlow}
        onStop={mockOnStop}
      />
    );

    const stopBtn = screen.getByTitle('停止调试监听');
    fireEvent.click(stopBtn);

    expect(mockOnStop).toHaveBeenCalledTimes(1);
  });
});
