/**
 * WsSendWaitViewer 组件测试
 * - WS 发送等待展示测试
 * - 超时状态测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { WsSendWaitViewer } from '@/components/execution/viewers/WsSendWaitViewer';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('WsSendWaitViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-ws-send-001',
    node_type: 'ws_send_wait',
    node_name: 'WebSocket 发送等待',
    start_time: Date.now() - 200,
    end_time: Date.now(),
    duration_ms: 200,
    status: 'success',
    output_data: {
      connection_id: 'conn-001',
      wait_timed_out: false,
      ...outputData,
    },
  });

  describe('错误信息显示', () => {
    it('显示执行错误', () => {
      const report = createMockReport({
        error: '连接已关闭'
      });
      report.status = 'failed';
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('执行错误')).toBeDefined();
      expect(screen.getByText('连接已关闭')).toBeDefined();
    });

    it('无错误时不显示错误区域', () => {
      const report = createMockReport();
      render(<WsSendWaitViewer report={report} />);

      expect(screen.queryByText('执行错误')).toBeNull();
    });
  });

  describe('超时警告显示', () => {
    it('显示超时警告', () => {
      const report = createMockReport({
        wait_timed_out: true
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('等待响应超时')).toBeDefined();
      expect(screen.getByText('在指定时间内未收到匹配的响应消息')).toBeDefined();
    });

    it('未超时不显示警告', () => {
      const report = createMockReport({
        wait_timed_out: false
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.queryByText('等待响应超时')).toBeNull();
    });
  });

  describe('连接信息显示', () => {
    it('显示连接 ID', () => {
      const report = createMockReport({
        connection_id: 'conn-xyz-789'
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('conn-xyz-789')).toBeDefined();
    });

    it('显示连接 URL', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('wss://api.example.com/ws')).toBeDefined();
    });

    it('无连接 ID 时显示占位符', () => {
      const report = createMockReport({
        connection_id: ''
      });
      render(<WsSendWaitViewer report={report} />);

      // 使用 getAllByText 因为有多个 '-' 占位符
      const dashes = screen.getAllByText('-');
      expect(dashes.length).toBeGreaterThan(0);
    });
  });

  describe('发送内容显示', () => {
    it('显示文本类型发送内容', () => {
      const report = createMockReport({
        sent: {
          data: '{"action":"ping"}',
          type: 'text'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('发送内容')).toBeDefined();
      expect(screen.getByText('文本')).toBeDefined();
      expect(screen.getByText(/ping/)).toBeDefined();
    });

    it('显示二进制类型发送内容', () => {
      const report = createMockReport({
        sent: {
          data: 'binary-data-here',
          type: 'binary'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('二进制')).toBeDefined();
    });

    it('无发送内容时不显示发送区域', () => {
      const report = createMockReport({
        sent: null
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.queryByText('发送内容')).toBeNull();
    });
  });

  describe('响应内容显示', () => {
    it('显示文本类型响应', () => {
      const report = createMockReport({
        message: {
          data: '{"status":"ok"}',
          type: 'text'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('响应内容')).toBeDefined();
      expect(screen.getByText('文本')).toBeDefined();
    });

    it('显示二进制类型响应', () => {
      const report = createMockReport({
        message: {
          data: 'binary-response',
          type: 'binary'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('二进制')).toBeDefined();
    });

    it('显示解析后的 JSON', () => {
      const report = createMockReport({
        message: {
          data: '{"result":"success"}',
          type: 'text',
          parsed: { result: 'success' }
        }
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('解析后的 JSON:')).toBeDefined();
      expect(screen.getByTestId('json-view')).toBeDefined();
    });

    it('无响应数据时显示提示', () => {
      const report = createMockReport({
        message: null
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('未等待响应或无响应数据')).toBeDefined();
    });
  });

  describe('耗时分解显示', () => {
    it('显示发送耗时（毫秒）', () => {
      const report = createMockReport({
        send_duration_ms: 15
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('发送耗时')).toBeDefined();
      expect(screen.getByText('15ms')).toBeDefined();
    });

    it('显示等待耗时（毫秒）', () => {
      const report = createMockReport({
        wait_response_duration_ms: 100
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('等待耗时')).toBeDefined();
      expect(screen.getByText('100ms')).toBeDefined();
    });

    it('显示耗时（秒）', () => {
      const report = createMockReport({
        wait_response_duration_ms: 1500
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('1.50s')).toBeDefined();
    });

    it('显示总执行时间', () => {
      const report = createMockReport();
      report.duration_ms = 250;
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('总执行时间')).toBeDefined();
      expect(screen.getByText('250ms')).toBeDefined();
    });

    it('无耗时数据时显示占位符', () => {
      const report = createMockReport({
        send_duration_ms: null,
        wait_response_duration_ms: null
      });
      render(<WsSendWaitViewer report={report} />);

      // 应该显示两个 '-' 占位符
      const dashes = screen.getAllByText('-');
      expect(dashes.length).toBe(2);
    });
  });

  describe('完整数据展示', () => {
    it('显示完整的发送等待信息', () => {
      const report = createMockReport({
        connection_id: 'conn-123',
        url: 'wss://test.com/ws',
        wait_timed_out: false,
        sent: {
          data: '{"cmd":"subscribe"}',
          type: 'text'
        },
        message: {
          data: '{"result":"subscribed"}',
          type: 'text',
          parsed: { result: 'subscribed' }
        },
        send_duration_ms: 10,
        wait_response_duration_ms: 50
      });
      report.duration_ms = 60;
      render(<WsSendWaitViewer report={report} />);

      // 连接信息
      expect(screen.getByText('conn-123')).toBeDefined();
      expect(screen.getByText('wss://test.com/ws')).toBeDefined();
      // 发送内容
      expect(screen.getByText('发送内容')).toBeDefined();
      // 响应内容
      expect(screen.getByText('响应内容')).toBeDefined();
      // 耗时
      expect(screen.getByText('10ms')).toBeDefined();
      expect(screen.getByText('50ms')).toBeDefined();
      expect(screen.getByText('60ms')).toBeDefined();
    });
  });
});
