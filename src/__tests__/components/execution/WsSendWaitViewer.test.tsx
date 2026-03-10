/**
 * WsSendWaitViewer 组件测试
 * - WS 发送等待展示测试
 * - 超时状态测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

    it('显示连接信息面板', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsSendWaitViewer report={report} />);

      // 连接信息面板默认展开
      expect(screen.getByText('连接信息')).toBeDefined();
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
    it('显示文本类型发送内容', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        sent: {
          data: '{"action":"ping"}',
          type: 'text'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      // 发送内容面板存在（默认折叠）
      expect(screen.getByText('发送内容')).toBeDefined();
      
      // 点击展开面板
      const buttons = screen.getAllByRole('button');
      const sendButton = buttons.find(btn => btn.textContent?.includes('发送内容'));
      if (sendButton) {
        await user.click(sendButton);
      }

      // 展开后检查发送的数据
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

      // 使用 getAllByText 因为 "二进制" 可能在多个面板中出现
      const binaryBadges = screen.getAllByText('二进制');
      expect(binaryBadges.length).toBeGreaterThan(0);
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
    });

    it('显示二进制类型响应', () => {
      const report = createMockReport({
        message: {
          data: 'binary-response',
          type: 'binary'
        }
      });
      render(<WsSendWaitViewer report={report} />);

      const binaryBadges = screen.getAllByText('二进制');
      expect(binaryBadges.length).toBeGreaterThan(0);
    });

    it('显示解析后的 JSON', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        message: {
          data: '{"result":"success"}',
          type: 'text',
          parsed: { result: 'success' }
        }
      });
      render(<WsSendWaitViewer report={report} />);

      // 响应内容面板存在
      expect(screen.getByText('响应内容')).toBeDefined();
      
      // 使用 aria-expanded 找到折叠的响应内容面板并展开
      const buttons = screen.getAllByRole('button');
      for (const btn of buttons) {
        if (btn.getAttribute('aria-expanded') === 'false' && btn.textContent?.includes('响应内容')) {
          await user.click(btn);
          break;
        }
      }

      // 展开后检查 JSON 视图
      expect(screen.getByTestId('json-view')).toBeDefined();
    });

    it('无响应数据时显示响应内容面板', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        message: null
      });
      render(<WsSendWaitViewer report={report} />);

      // 点击展开响应内容面板
      const buttons = screen.getAllByRole('button');
      const responseButton = buttons.find(btn => btn.textContent?.includes('响应内容'));
      if (responseButton) {
        await user.click(responseButton);
      }

      expect(screen.getByText('未等待响应或无响应数据')).toBeDefined();
    });
  });

  describe('耗时分解显示', () => {
    it('显示耗时分解面板', () => {
      const report = createMockReport({
        send_duration_ms: 15
      });
      render(<WsSendWaitViewer report={report} />);

      expect(screen.getByText('耗时分解')).toBeDefined();
    });

    it('显示总执行时间在标题栏', () => {
      const report = createMockReport();
      report.duration_ms = 250;
      render(<WsSendWaitViewer report={report} />);

      // 总执行时间现在显示在耗时分解面板的标题栏中
      expect(screen.getByText('总计 250ms')).toBeDefined();
    });

    it('展开面板显示耗时详情', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        send_duration_ms: 15,
        wait_response_duration_ms: 100
      });
      render(<WsSendWaitViewer report={report} />);

      // 点击展开耗时分解面板
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton) {
        await user.click(timingButton);
      }

      // 展开后应该能看到耗时详情
      expect(screen.getByText('发送耗时')).toBeDefined();
      expect(screen.getByText('等待耗时')).toBeDefined();
      expect(screen.getByText('15ms')).toBeDefined();
      expect(screen.getByText('100ms')).toBeDefined();
    });

    it('显示耗时（秒）', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        wait_response_duration_ms: 1500
      });
      render(<WsSendWaitViewer report={report} />);

      // 点击展开耗时分解面板
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton) {
        await user.click(timingButton);
      }

      expect(screen.getByText('1.50s')).toBeDefined();
    });

    it('无耗时数据时显示占位符', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        send_duration_ms: null,
        wait_response_duration_ms: null
      });
      render(<WsSendWaitViewer report={report} />);

      // 点击展开耗时分解面板
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton) {
        await user.click(timingButton);
      }

      // 应该显示两个 '-' 占位符
      const dashes = screen.getAllByText('-');
      expect(dashes.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('完整数据展示', () => {
    it('显示完整的发送等待信息', async () => {
      const user = userEvent.setup();
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

      // 发送内容面板存在
      expect(screen.getByText('发送内容')).toBeDefined();

      // 响应内容面板存在
      expect(screen.getByText('响应内容')).toBeDefined();

      // 总执行时间在标题栏
      expect(screen.getByText('总计 60ms')).toBeDefined();

      // 展开耗时面板查看详情
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton) {
        await user.click(timingButton);
      }

      expect(screen.getByText('10ms')).toBeDefined();
      expect(screen.getByText('50ms')).toBeDefined();
    });
  });
});
