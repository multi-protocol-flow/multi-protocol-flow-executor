/**
 * StreamInspectorPanel 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { StreamInspectorPanel } from '@/components/execution/StreamInspectorPanel';
import { useWsStreamStore } from '@/stores/wsStream';
import type { WsConnectionStream } from '@/types/ws_stream';

describe('StreamInspectorPanel', () => {
  beforeEach(() => {
    useWsStreamStore.getState().reset();
    vi.clearAllMocks();
  });

  const mockStreams: WsConnectionStream[] = [
    {
      connection_id: 'conn-1',
      url: 'wss://echo.websocket.org',
      current_owner_node_uuid: 'node-1',
      segments: [
        {
          node_uuid: 'node-1',
          node_name: 'WS Connect',
          node_type: 'ws:connect',
          started_at: 1000,
          ended_at: null,
          messages: [
            {
              direction: 'sent',
              timestamp: 1050,
              is_binary: false,
              size: 17,
              content: {
                type: 'text',
                content: '{"action":"init"}',
              },
            },
            {
              direction: 'received',
              timestamp: 1100,
              is_binary: false,
              size: 32,
              content: {
                type: 'text',
                content: '{"status":200,"message":"ready"}',
              },
            },
          ],
          max_messages: 100,
          dropped_message_count: 0,
        },
      ],
      max_messages_per_segment: 100,
      max_total_messages: 1000,
      is_closed: false,
    },
  ];

  it('正确渲染连接和双向消息瀑布流', () => {
    render(<StreamInspectorPanel streams={mockStreams} live={true} />);

    // 应展示连接选择与本地化状态徽标（测试 locale 为 zh-CN）
    expect(screen.getByRole('combobox')).toHaveValue('conn-1');
    expect(screen.getByText('已连接')).toBeInTheDocument();

    // 应该渲染方向标签
    expect(screen.getByText('OUT')).toBeInTheDocument();
    expect(screen.getByText('IN')).toBeInTheDocument();

    // 应该渲染消息体内容
    expect(screen.getByText('{"action":"init"}')).toBeInTheDocument();
    expect(screen.getByText('{"status":200,"message":"ready"}')).toBeInTheDocument();
  });

  it('点击 Ping 按钮调用 sendLiveFrame', async () => {
    const sendSpy = vi.fn().mockResolvedValue(undefined);
    useWsStreamStore.setState({ sendLiveFrame: sendSpy });

    render(<StreamInspectorPanel streams={mockStreams} live={true} />);

    const pingBtn = screen.getByRole('button', { name: /Ping/i });
    fireEvent.click(pingBtn);

    expect(sendSpy).toHaveBeenCalledWith(
      'conn-1',
      expect.stringContaining('"ping"'),
      false
    );
  });

  it('输入数据并点击发送调用 sendLiveFrame', async () => {
    const sendSpy = vi.fn().mockResolvedValue(undefined);
    useWsStreamStore.setState({ sendLiveFrame: sendSpy });

    render(<StreamInspectorPanel streams={mockStreams} live={true} />);

    const input = screen.getByPlaceholderText(/输入要发送的数据帧内容/i);
    fireEvent.change(input, { target: { value: '{"custom":"test"}' } });

    const sendBtn = screen.getByRole('button', { name: /发送帧/i });
    fireEvent.click(sendBtn);

    expect(sendSpy).toHaveBeenCalledWith(
      'conn-1',
      '{"custom":"test"}',
      false
    );
  });
});
