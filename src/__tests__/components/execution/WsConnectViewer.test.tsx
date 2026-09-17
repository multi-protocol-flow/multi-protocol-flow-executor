/**
 * WsConnectViewer 组件测试
 * - WS 连接展示测试
 * - 连接状态显示测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WsConnectViewer } from '@/components/execution/viewers/WsConnectViewer';
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

describe('WsConnectViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-ws-connect-001',
    node_type: 'ws:connect',
    node_name: 'WebSocket 连接',
    start_time: Date.now() - 100,
    end_time: Date.now(),
    duration_ms: 100,
    status: 'success',
    output_data: outputData,
  });

  describe('连接状态显示', () => {
    it('显示成功连接状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport({ success: true });
      render(<WsConnectViewer report={report} />);

      // 连接状态在折叠面板内，需要展开
      const buttons = screen.getAllByRole('button');
      const statusButton = buttons.find(btn => btn.textContent?.includes('连接状态'));
      if (statusButton && statusButton.getAttribute('aria-expanded') === 'false') {
        await user.click(statusButton);
      }

      expect(screen.getByText('已连接')).toBeDefined();
    });

    it('显示失败连接状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport({ success: false });
      render(<WsConnectViewer report={report} />);

      // 展开连接状态面板
      const buttons = screen.getAllByRole('button');
      const statusButton = buttons.find(btn => btn.textContent?.includes(getLL().execution.viewers.ws_connect_status()));
      if (statusButton && statusButton.getAttribute('aria-expanded') === 'false') {
        await user.click(statusButton);
      }

      expect(screen.getByText(getLL().execution.viewers.disconnected())).toBeDefined();
    });

    it('根据 report.status 判断状态（无 success 字段时）', async () => {
      const user = userEvent.setup();
      const successReport = createMockReport({});
      successReport.status = 'success';
      const { rerender } = render(<WsConnectViewer report={successReport} />);

      // 展开连接状态面板
      const buttons = screen.getAllByRole('button');
      const statusButton = buttons.find(btn => btn.textContent?.includes(getLL().execution.viewers.ws_connect_status()));
      if (statusButton && statusButton.getAttribute('aria-expanded') === 'false') {
        await user.click(statusButton);
      }

      expect(screen.getByText(getLL().execution.viewers.connected())).toBeDefined();

      const failedReport = createMockReport({});
      failedReport.status = 'failed';
      rerender(<WsConnectViewer report={failedReport} />);

      expect(screen.getByText(getLL().execution.viewers.disconnected())).toBeDefined();
    });
  });

  describe('连接信息显示', () => {
    it('显示 WebSocket URL', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsConnectViewer report={report} />);

      // URL 在连接配置面板中，默认展开
      expect(screen.getByText('wss://api.example.com/ws')).toBeDefined();
    });

    it('显示连接 ID', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        connection_id: 'conn-12345'
      });
      render(<WsConnectViewer report={report} />);

      // 连接 ID 在连接状态面板中
      const buttons = screen.getAllByRole('button');
      const statusButton = buttons.find(btn => btn.textContent?.includes('连接状态'));
      if (statusButton && statusButton.getAttribute('aria-expanded') === 'false') {
        await user.click(statusButton);
      }

      expect(screen.getByText('conn-12345')).toBeDefined();
    });

    it('无 URL 时显示占位符', () => {
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      // 检查"未配置 URL"提示
      expect(screen.getByText('未配置 URL')).toBeDefined();
    });
  });

  describe('耗时分解显示', () => {
    it('显示耗时分解面板', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        timing: { connect_ms: 50 }
      });
      render(<WsConnectViewer report={report} />);

      // 耗时分解面板存在
      expect(screen.getByText('耗时分解')).toBeDefined();
      
      // 展开查看详情
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton && timingButton.getAttribute('aria-expanded') === 'false') {
        await user.click(timingButton);
      }

      expect(screen.getByText('连接耗时')).toBeDefined();
      expect(screen.getByText('50ms')).toBeDefined();
    });

    it('显示采集消息耗时', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        timing: { collect_response_ms: 200 }
      });
      render(<WsConnectViewer report={report} />);

      // 展开查看详情
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton && timingButton.getAttribute('aria-expanded') === 'false') {
        await user.click(timingButton);
      }

      expect(screen.getByText('采集消息')).toBeDefined();
      expect(screen.getByText('200ms')).toBeDefined();
    });

    it('显示所有耗时信息', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        timing: { connect_ms: 30, collect_response_ms: 150 }
      });
      render(<WsConnectViewer report={report} />);

      // 展开查看详情
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton && timingButton.getAttribute('aria-expanded') === 'false') {
        await user.click(timingButton);
      }

      expect(screen.getByText('30ms')).toBeDefined();
      expect(screen.getByText('150ms')).toBeDefined();
    });

    it('无耗时数据时不显示耗时分解', () => {
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      expect(screen.queryByText('耗时分解')).toBeNull();
    });
  });

  describe('首条消息显示', () => {
    it('显示消息类型', () => {
      const report = createMockReport({
        message: {
          type: 'text'
        }
      });
      render(<WsConnectViewer report={report} />);

      // 首条消息面板存在
      expect(screen.getByText('首条消息')).toBeDefined();
      
      // 类型 Badge 在面板标题栏
      expect(screen.getByText('text')).toBeDefined();
    });

    it('显示字符串消息数据', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        message: {
          type: 'text',
          data: 'Hello WebSocket'
        }
      });
      render(<WsConnectViewer report={report} />);

      // 展开首条消息面板
      const buttons = screen.getAllByRole('button');
      const msgButton = buttons.find(btn => btn.textContent?.includes('首条消息'));
      if (msgButton && msgButton.getAttribute('aria-expanded') === 'false') {
        await user.click(msgButton);
      }

      expect(screen.getByText('Hello WebSocket')).toBeDefined();
    });

    it('显示对象消息数据（JSON 格式）', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        message: {
          type: 'text',
          data: { event: 'connected', id: 1 }
        }
      });
      render(<WsConnectViewer report={report} />);

      // 展开首条消息面板
      const buttons = screen.getAllByRole('button');
      const msgButton = buttons.find(btn => btn.textContent?.includes('首条消息'));
      if (msgButton && msgButton.getAttribute('aria-expanded') === 'false') {
        await user.click(msgButton);
      }

      // JSON 数据会通过 RawJsonViewer 渲染
      expect(screen.getByTestId('json-view')).toBeDefined();
    });

    it('无消息时显示提示', async () => {
      const user = userEvent.setup();
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      // 展开首条消息面板
      const buttons = screen.getAllByRole('button');
      const msgButton = buttons.find(btn => btn.textContent?.includes('首条消息'));
      if (msgButton) {
        await user.click(msgButton);
      }

      expect(screen.getByText('无首条消息')).toBeDefined();
    });
  });

  describe('完整数据展示', () => {
    it('显示完整的连接信息', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        success: true,
        url: 'wss://example.com/socket',
        connection_id: 'conn-abc-123',
        timing: { connect_ms: 45, collect_response_ms: 120 },
        message: {
          type: 'text',
          data: '{"status":"ok"}'
        }
      });
      render(<WsConnectViewer report={report} />);

      // URL（在连接配置面板，默认展开）
      expect(screen.getByText('wss://example.com/socket')).toBeDefined();

      // 展开连接状态面板
      let buttons = screen.getAllByRole('button');
      let statusButton = buttons.find(btn => btn.textContent?.includes('连接状态'));
      if (statusButton && statusButton.getAttribute('aria-expanded') === 'false') {
        await user.click(statusButton);
      }

      // 状态和连接 ID
      expect(screen.getByText('已连接')).toBeDefined();
      expect(screen.getByText('conn-abc-123')).toBeDefined();

      // 展开耗时分解面板
      buttons = screen.getAllByRole('button');
      let timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton && timingButton.getAttribute('aria-expanded') === 'false') {
        await user.click(timingButton);
      }

      // 耗时
      expect(screen.getByText('45ms')).toBeDefined();
      expect(screen.getByText('120ms')).toBeDefined();

      // 消息类型在标题栏
      expect(screen.getByText('text')).toBeDefined();
    });
  });
});
