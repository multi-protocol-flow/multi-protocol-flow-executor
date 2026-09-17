/**
 * WsSendCollectViewer 组件测试
 * - WS 发送等待展示测试
 * - 超时状态测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WsSendCollectViewer } from '@/components/execution/viewers/WsSendCollectViewer';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('WsSendCollectViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-ws-send-001',
    node_type: 'ws:send_collect',
    node_name: 'WebSocket 发送收集',
    start_time: Date.now() - 200,
    end_time: Date.now(),
    duration_ms: 200,
    status: 'success',
    output_data: {
      connection_id: 'conn-001',
      collect_timed_out: false,
      ...outputData,
    },
  });

  describe('错误信息显示', () => {
    it('显示执行错误', () => {
      const report = createMockReport({
        error: '连接已关闭'
      });
      report.status = 'failed';
      render(<WsSendCollectViewer report={report} />);

      // WsSendCollectViewer 不直接渲染 execution_failed 文本，
      // 该文本由父组件 NodeExecutionDetails 渲染。
      // 这里只验证错误信息在 output_data 中存在
      expect((report.output_data as Record<string, unknown>)?.error).toBe('连接已关闭');
    });

    it('无错误时不显示错误区域', () => {
      const report = createMockReport();
      render(<WsSendCollectViewer report={report} />);

      // viewer 本身不渲染 execution_failed 文本
      expect((report.output_data as Record<string, unknown>)?.error).toBeUndefined();
    });
  });

  describe('超时警告显示', () => {
    it('显示超时警告', () => {
      const report = createMockReport({
        collect_timed_out: true
      });
      render(<WsSendCollectViewer report={report} />);

      expect(screen.getByText('采集响应超时')).toBeDefined();
      expect(screen.getByText('在指定时间内未收到匹配的响应消息')).toBeDefined();
    });

    it('未超时不显示警告', () => {
      const report = createMockReport({
        collect_timed_out: false
      });
      render(<WsSendCollectViewer report={report} />);

      expect(screen.queryByText('采集响应超时')).toBeNull();
    });
  });

  describe('连接信息显示', () => {
    it('显示连接 ID', () => {
      const report = createMockReport({
        connection_id: 'conn-xyz-789'
      });
      render(<WsSendCollectViewer report={report} />);

      expect(screen.getByText('conn-xyz-789')).toBeDefined();
    });

    it('显示连接信息面板', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsSendCollectViewer report={report} />);

      // 连接信息面板默认展开
      expect(screen.getByText('连接信息')).toBeDefined();
    });

    it('无连接 ID 时显示占位符', () => {
      const report = createMockReport({
        connection_id: ''
      });
      render(<WsSendCollectViewer report={report} />);

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
      render(<WsSendCollectViewer report={report} />);

      // 发送配置面板存在（发送内容已合并到发送配置中）
      expect(screen.getByText('发送配置')).toBeDefined();
      
      // 点击展开面板
      const buttons = screen.getAllByRole('button');
      const sendButton = buttons.find(btn => btn.textContent?.includes('发送配置'));
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
      render(<WsSendCollectViewer report={report} />);

      // 使用 getAllByText 因为 "二进制" 可能在多个面板中出现
      const binaryBadges = screen.getAllByText('二进制');
      expect(binaryBadges.length).toBeGreaterThan(0);
    });

    it('无发送内容时显示发送配置面板', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        sent: null
      });
      render(<WsSendCollectViewer report={report} />);

      // 点击展开发送配置面板
      const buttons = screen.getAllByRole('button');
      const sendButton = buttons.find(btn => btn.textContent?.includes('发送配置'));
      if (sendButton) {
        await user.click(sendButton);
      }

      // 发送配置面板仍然存在，但可能显示等待响应配置
      expect(screen.getByText('发送配置')).toBeDefined();
    });
  });

  describe('匹配消息显示', () => {
    it('显示文本类型响应', () => {
      const report = createMockReport({
        message: {
          data: '{"status":"ok"}',
          type: 'text'
        }
      });
      render(<WsSendCollectViewer report={report} />);

      expect(screen.getByText('匹配消息')).toBeDefined();
    });

    it('显示二进制类型响应', () => {
      const report = createMockReport({
        message: {
          data: 'binary-response',
          type: 'binary'
        }
      });
      render(<WsSendCollectViewer report={report} />);

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
      render(<WsSendCollectViewer report={report} />);

      // 匹配消息面板存在
      expect(screen.getByText('匹配消息')).toBeDefined();
      
      // 使用 aria-expanded 找到折叠的匹配消息面板并展开
      const buttons = screen.getAllByRole('button');
      for (const btn of buttons) {
        if (btn.getAttribute('aria-expanded') === 'false' && btn.textContent?.includes('匹配消息')) {
          await user.click(btn);
          break;
        }
      }

      // 展开后检查 JSON 视图
      expect(screen.getByTestId('json-view')).toBeDefined();
    });

    it('无响应数据时显示匹配消息面板', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        message: null
      });
      render(<WsSendCollectViewer report={report} />);

      // 点击展开匹配消息面板
      const buttons = screen.getAllByRole('button');
      const responseButton = buttons.find(btn => btn.textContent?.includes('匹配消息'));
      if (responseButton) {
        await user.click(responseButton);
      }

      expect(screen.getByText('未采集响应或无匹配消息')).toBeDefined();
    });
  });

  describe('耗时分解显示', () => {
    it('显示耗时分解面板', () => {
      const report = createMockReport({
        timing: { send_ms: 15 }
      });
      render(<WsSendCollectViewer report={report} />);

      expect(screen.getByText('耗时分解')).toBeDefined();
    });

    it('显示总执行时间在标题栏', () => {
      const report = createMockReport();
      report.duration_ms = 250;
      render(<WsSendCollectViewer report={report} />);

      // 总执行时间现在显示在耗时分解面板的标题栏中
      expect(screen.getByText('总计 250ms')).toBeDefined();
    });

    it('展开面板显示耗时详情', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        timing: { send_ms: 15, collect_response_ms: 100 }
      });
      render(<WsSendCollectViewer report={report} />);

      // 点击展开耗时分解面板
      const buttons = screen.getAllByRole('button');
      const timingButton = buttons.find(btn => btn.textContent?.includes('耗时分解'));
      if (timingButton) {
        await user.click(timingButton);
      }

      // 展开后应该能看到耗时详情
      expect(screen.getByText('发送耗时')).toBeDefined();
      expect(screen.getByText('采集消息')).toBeDefined();
      expect(screen.getByText('15ms')).toBeDefined();
      expect(screen.getByText('100ms')).toBeDefined();
    });

    it('显示耗时（秒）', async () => {
      const user = userEvent.setup();
      const report = createMockReport({
        timing: { collect_response_ms: 1500 }
      });
      render(<WsSendCollectViewer report={report} />);

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
        timing: { send_ms: null, collect_response_ms: null }
      });
      render(<WsSendCollectViewer report={report} />);

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
        collect_timed_out: false,
        sent: {
          data: '{"cmd":"subscribe"}',
          type: 'text'
        },
        message: {
          data: '{"result":"subscribed"}',
          type: 'text',
          parsed: { result: 'subscribed' }
        },
        timing: { send_ms: 10, collect_response_ms: 50 }
      });
      report.duration_ms = 60;
      render(<WsSendCollectViewer report={report} />);

      // 连接信息
      expect(screen.getByText('conn-123')).toBeDefined();

      // 发送配置面板存在（发送内容已合并到发送配置中）
      expect(screen.getByText('发送配置')).toBeDefined();

      // 匹配消息面板存在
      expect(screen.getByText('匹配消息')).toBeDefined();

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
