/**
 * WsMessageStreamPanel 组件测试
 * 测试 WebSocket 消息流面板的渲染和交互
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WsMessageStreamPanel } from '@/components/debug/WsMessageStreamPanel';
import type { WsConnectionStream, WsMessage, WsStreamSegment } from '@/types/ws_stream';

// Mock useDebugStore
vi.mock('@/stores/debug', () => ({
  useDebugStore: vi.fn(),
}));

import { useDebugStore } from '@/stores/debug';

// ============================================================================
// 测试数据工厂
// ============================================================================

function createMockMessage(overrides: Partial<WsMessage> = {}): WsMessage {
  return {
    timestamp: Date.now(),
    direction: 'received',
    content: { type: 'text', content: 'test message' },
    is_binary: false,
    size: 100,
    ...overrides,
  };
}

function createMockSegment(overrides: Partial<WsStreamSegment> = {}): WsStreamSegment {
  return {
    node_uuid: 'node-1',
    node_name: 'Test Node',
    node_type: 'ws_connect',
    started_at: Date.now(),
    ended_at: null,
    messages: [createMockMessage()],
    max_messages: 100,
    dropped_message_count: 0,
    ...overrides,
  };
}

function createMockStream(overrides: Partial<WsConnectionStream> = {}): WsConnectionStream {
  return {
    connection_id: 'conn-1',
    url: 'wss://example.com/ws',
    current_owner_node_uuid: 'node-1',
    segments: [createMockSegment()],
    max_messages_per_segment: 100,
    max_total_messages: 1000,
    is_closed: false,
    ...overrides,
  };
}

// ============================================================================
// 测试套件
// ============================================================================

describe('WsMessageStreamPanel', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mockStore = (state: any) => {
    vi.mocked(useDebugStore).mockImplementation((selector: any) => {
      return selector(state);
    });
  };

  const defaultState = {
    wsStreams: new Map<string, WsConnectionStream>(),
    activeConnectionId: null,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockStore(defaultState);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('空状态', () => {
    it('当无连接时应该显示空状态', () => {
      mockStore({ wsStreams: new Map(), activeConnectionId: null });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText('暂无 WebSocket 连接')).toBeDefined();
      expect(screen.getByText('调试时 WebSocket 消息将在此显示')).toBeDefined();
    });
  });

  describe('连接信息显示', () => {
    it('应该显示连接 URL', () => {
      const stream = createMockStream({ url: 'wss://test.example.com/socket' });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText(/wss:\/\/test.example.com\/socket/)).toBeDefined();
    });

    it('应该显示连接状态为已连接', () => {
      const stream = createMockStream({ is_closed: false });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText('已连接')).toBeDefined();
    });

    it('应该显示连接状态为已关闭', () => {
      const stream = createMockStream({ is_closed: true });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText('已关闭')).toBeDefined();
    });

    it('应该显示连接 ID（截断）', () => {
      const stream = createMockStream({ connection_id: 'very-long-connection-id-12345' });
      const wsStreams = new Map([['very-long-connection-id-12345', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'very-long-connection-id-12345' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText(/ID: very-lon\.\.\./)).toBeDefined();
    });
  });

  describe('消息统计', () => {
    it('应该显示发送和接收数量', () => {
      const segment = createMockSegment({
        messages: [
          createMockMessage({ direction: 'sent' }),
          createMockMessage({ direction: 'sent' }),
          createMockMessage({ direction: 'received' }),
        ],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 2 sent, 1 received
      expect(screen.getByText('2')).toBeDefined();
      expect(screen.getByText('1')).toBeDefined();
    });

    it('应该显示丢弃消息数', () => {
      const segment = createMockSegment({
        messages: [],
        dropped_message_count: 5,
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText(/丢弃: 5/)).toBeDefined();
    });
  });

  describe('消息列表', () => {
    it('应该显示段头部', () => {
      const segment = createMockSegment({
        node_uuid: 'node-123',
        node_name: 'WebSocket连接',
        messages: [],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText('WebSocket连接')).toBeDefined();
    });

    it('点击段头部应该展开/折叠消息', async () => {
      const user = userEvent.setup();
      const message = createMockMessage({
        content: { type: 'text', content: 'Hello WebSocket' },
      });
      const segment = createMockSegment({
        node_uuid: 'node-123',
        node_name: 'Test Segment',
        messages: [message],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 初始状态，段是折叠的，消息不可见
      expect(screen.queryByText('Hello WebSocket')).toBeNull();

      // 点击段头部展开
      await user.click(screen.getByText('Test Segment'));

      // 消息应该可见
      expect(screen.getByText(/Hello WebSocket/)).toBeDefined();
    });

    it('应该显示消息方向图标', async () => {
      const user = userEvent.setup();
      const sentMessage = createMockMessage({ direction: 'sent' });
      const receivedMessage = createMockMessage({ direction: 'received' });
      const segment = createMockSegment({
        node_uuid: 'node-123',
        messages: [sentMessage, receivedMessage],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 展开段 - 点击段头部 div
      await user.click(screen.getByText('Test Node'));

      // 应该看到发送和接收标签
      expect(screen.getByText('发送')).toBeDefined();
      expect(screen.getByText('接收')).toBeDefined();
    });
  });

  describe('二进制消息处理', () => {
    it('应该显示 [Binary] 前缀', async () => {
      const user = userEvent.setup();
      const binaryMessage = createMockMessage({
        is_binary: true,
        content: { type: 'binary', content: [72, 101, 108, 108, 111] }, // "Hello" in bytes
      });
      const segment = createMockSegment({
        node_uuid: 'node-123',
        messages: [binaryMessage],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 展开段 - 点击段头部
      await user.click(screen.getByText('Test Node'));

      // 应该看到 [Binary] 前缀
      expect(screen.getByText(/\[Binary\]/)).toBeDefined();
    });
  });

  describe('自动滚动切换', () => {
    it('应该显示自动滚动按钮', () => {
      const stream = createMockStream();
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      expect(screen.getByText('自动滚动')).toBeDefined();
    });

    it('点击应该切换到手动模式', async () => {
      const user = userEvent.setup();
      const stream = createMockStream();
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 点击切换按钮
      await user.click(screen.getByText('自动滚动'));

      // 应该显示手动模式
      expect(screen.getByText('手动')).toBeDefined();
    });
  });

  describe('多连接选择器', () => {
    it('应该显示多个连接的选项卡', () => {
      const stream1 = createMockStream({ connection_id: 'conn-1', url: 'wss://example1.com/ws' });
      const stream2 = createMockStream({ connection_id: 'conn-2', url: 'wss://example2.com/ws' });
      const wsStreams = new Map([
        ['conn-1', stream1],
        ['conn-2', stream2],
      ]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 应该显示连接数量
      expect(screen.getByText('2 个连接')).toBeDefined();
    });

    it('点击选项卡应该切换连接', async () => {
      const user = userEvent.setup();
      const mockSetActiveConnectionId = vi.fn();

      const stream1 = createMockStream({ connection_id: 'conn-1', url: 'wss://example1.com/ws' });
      const stream2 = createMockStream({ connection_id: 'conn-2', url: 'wss://example2.com/ws' });
      const wsStreams = new Map([
        ['conn-1', stream1],
        ['conn-2', stream2],
      ]);

      // Mock setState
      vi.mocked(useDebugStore).mockImplementation((selector: any) => {
        return selector({
          wsStreams,
          activeConnectionId: 'conn-1',
        });
      });

      // Mock setState directly
      const originalSetState = useDebugStore.setState;
      useDebugStore.setState = mockSetActiveConnectionId;

      render(<WsMessageStreamPanel />);

      // 找到第二个连接按钮（通过 URL 截断名）
      const buttons = screen.getAllByRole('button');
      const conn2Button = buttons.find(btn => btn.textContent?.includes('example2.com'));
      
      if (conn2Button) {
        await user.click(conn2Button);
        expect(mockSetActiveConnectionId).toHaveBeenCalled();
      }

      // 恢复
      useDebugStore.setState = originalSetState;
    });
  });

  describe('消息详情对话框', () => {
    it('点击消息应该打开详情对话框', async () => {
      const user = userEvent.setup();
      const message = createMockMessage({
        content: { type: 'text', content: 'Detailed message content here' },
      });
      const segment = createMockSegment({
        node_uuid: 'node-123',
        messages: [message],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 展开段 - 点击段头部
      await user.click(screen.getByText('Test Node'));

      // 点击消息
      const messageElement = screen.getByText(/Detailed message content/);
      await user.click(messageElement);

      // 应该显示详情对话框
      expect(screen.getByText('接收的消息')).toBeDefined();
    });

    it('详情对话框应该显示消息大小', async () => {
      const user = userEvent.setup();
      const message = createMockMessage({
        size: 1024,
        content: { type: 'text', content: 'SizeTestMessage' },
      });
      const segment = createMockSegment({
        node_uuid: 'node-123',
        messages: [message],
      });
      const stream = createMockStream({ segments: [segment] });
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 展开段并点击消息
      await user.click(screen.getByText('Test Node'));
      await user.click(screen.getByText(/SizeTestMessage/));

      // 应该显示大小 (列表和对话框中都有)
      expect(screen.getByText(/大小:/)).toBeDefined();
      expect(screen.getAllByText(/1\.0KB/).length).toBeGreaterThan(0);
    });
  });

  describe('组件结构', () => {
    it('应该显示连接数量标签', () => {
      const stream = createMockStream();
      const wsStreams = new Map([['conn-1', stream]]);
      mockStore({ wsStreams, activeConnectionId: 'conn-1' });

      render(<WsMessageStreamPanel />);

      // 连接数量标签在组件内部显示（标题由 CollapsiblePanel 提供）
      expect(screen.getByText('1 个连接')).toBeDefined();
    });
  });
});
