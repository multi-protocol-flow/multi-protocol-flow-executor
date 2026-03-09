/**
 * WsConnectViewer 组件测试
 * - WS 连接展示测试
 * - 连接状态显示测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { WsConnectViewer } from '@/components/execution/viewers/WsConnectViewer';
import type { NodeExecutionReport } from '@/types';

describe('WsConnectViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-ws-connect-001',
    node_type: 'ws_connect',
    node_name: 'WebSocket 连接',
    start_time: Date.now() - 100,
    end_time: Date.now(),
    duration_ms: 100,
    status: 'success',
    output_data: outputData,
  });

  describe('连接状态显示', () => {
    it('显示成功连接状态', () => {
      const report = createMockReport({ success: true });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('已连接')).toBeDefined();
    });

    it('显示失败连接状态', () => {
      const report = createMockReport({ success: false });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('连接失败')).toBeDefined();
    });

    it('根据 report.status 判断状态（无 success 字段时）', () => {
      const successReport = createMockReport({});
      successReport.status = 'success';
      const { rerender } = render(<WsConnectViewer report={successReport} />);

      expect(screen.getByText('已连接')).toBeDefined();

      const failedReport = createMockReport({});
      failedReport.status = 'failed';
      rerender(<WsConnectViewer report={failedReport} />);

      expect(screen.getByText('连接失败')).toBeDefined();
    });
  });

  describe('连接信息显示', () => {
    it('显示 WebSocket URL', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('wss://api.example.com/ws')).toBeDefined();
    });

    it('显示连接 ID', () => {
      const report = createMockReport({
        connection_id: 'conn-12345'
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('conn-12345')).toBeDefined();
    });

    it('无 URL 时不显示 URL 行', () => {
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      expect(screen.queryByText('URL:')).toBeNull();
    });

    it('无连接 ID 时不显示连接 ID 行', () => {
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      expect(screen.queryByText('连接 ID:')).toBeNull();
    });
  });

  describe('耗时分解显示', () => {
    it('显示连接耗时', () => {
      const report = createMockReport({
        connect_duration_ms: 50
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('连接耗时:')).toBeDefined();
      expect(screen.getByText('50ms')).toBeDefined();
    });

    it('显示等待消息耗时', () => {
      const report = createMockReport({
        wait_message_duration_ms: 200
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('等待消息耗时:')).toBeDefined();
      expect(screen.getByText('200ms')).toBeDefined();
    });

    it('显示所有耗时信息', () => {
      const report = createMockReport({
        connect_duration_ms: 30,
        wait_message_duration_ms: 150
      });
      render(<WsConnectViewer report={report} />);

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

      expect(screen.getByText('类型:')).toBeDefined();
      expect(screen.getByText('text')).toBeDefined();
    });

    it('显示字符串消息数据', () => {
      const report = createMockReport({
        message: {
          type: 'text',
          data: 'Hello WebSocket'
        }
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('Hello WebSocket')).toBeDefined();
    });

    it('显示对象消息数据（JSON 格式）', () => {
      const report = createMockReport({
        message: {
          type: 'text',
          data: { event: 'connected', id: 1 }
        }
      });
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText(/connected/)).toBeDefined();
    });

    it('无消息时显示提示', () => {
      const report = createMockReport({});
      render(<WsConnectViewer report={report} />);

      expect(screen.getByText('无首条消息')).toBeDefined();
    });
  });

  describe('完整数据展示', () => {
    it('显示完整的连接信息', () => {
      const report = createMockReport({
        success: true,
        url: 'wss://example.com/socket',
        connection_id: 'conn-abc-123',
        connect_duration_ms: 45,
        wait_message_duration_ms: 120,
        message: {
          type: 'text',
          data: '{"status":"ok"}'
        }
      });
      render(<WsConnectViewer report={report} />);

      // 状态
      expect(screen.getByText('已连接')).toBeDefined();
      // URL
      expect(screen.getByText('wss://example.com/socket')).toBeDefined();
      // 连接 ID
      expect(screen.getByText('conn-abc-123')).toBeDefined();
      // 耗时
      expect(screen.getByText('45ms')).toBeDefined();
      expect(screen.getByText('120ms')).toBeDefined();
      // 消息
      expect(screen.getByText('text')).toBeDefined();
    });
  });
});
