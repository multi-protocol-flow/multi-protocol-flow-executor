/**
 * WsCloseViewer 组件测试
 * - WS 关闭展示测试
 * - 关闭码颜色测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { WsCloseViewer } from '@/components/execution/viewers/WsCloseViewer';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('WsCloseViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-ws-close-001',
    node_type: 'ws:close',
    node_name: 'WebSocket 关闭',
    start_time: Date.now() - 50,
    end_time: Date.now(),
    duration_ms: 50,
    status: 'success',
    output_data: outputData,
  });

  describe('连接信息显示', () => {
    it('显示连接 URL', () => {
      const report = createMockReport({
        url: 'wss://api.example.com/ws'
      });
      render(<WsCloseViewer report={report} />);

      expect(screen.getByText('wss://api.example.com/ws')).toBeDefined();
    });

    it('无 URL 时显示提示', () => {
      const report = createMockReport({});
      render(<WsCloseViewer report={report} />);

      // 连接信息面板默认展开
      expect(screen.getByText('无连接 URL')).toBeDefined();
    });
  });

  describe('关闭状态显示', () => {
    it('显示成功关闭状态', () => {
      const report = createMockReport({
        success: true
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('成功')).toBeDefined();
    });

    it('显示失败关闭状态', () => {
      const report = createMockReport({
        success: false
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('失败')).toBeDefined();
    });

    it('无执行状态时显示提示', () => {
      const report = createMockReport({});
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('无执行状态')).toBeDefined();
    });
  });

  describe('关闭码颜色', () => {
    it('1000 正常关闭显示绿色', () => {
      const report = createMockReport({
        close_code: 1000
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      const badge = screen.getByText('1000');
      expect(badge.className).toContain('text-emerald-600');
      expect(badge.className).toContain('bg-emerald-50');
    });

    it('1001 端点离开显示黄色', () => {
      const report = createMockReport({
        close_code: 1001
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      const badge = screen.getByText('1001');
      expect(badge.className).toContain('text-amber-600');
      expect(badge.className).toContain('bg-amber-50');
    });

    it('1006 连接异常关闭显示黄色', () => {
      const report = createMockReport({
        close_code: 1006
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      const badge = screen.getByText('1006');
      expect(badge.className).toContain('text-amber-600');
    });

    it('未知关闭码显示灰色', () => {
      const report = createMockReport({
        close_code: 9999
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      const badge = screen.getByText('9999');
      expect(badge.className).toContain('text-muted-foreground');
    });
  });

  describe('关闭码描述', () => {
    it('显示 1000 正常关闭描述', () => {
      const report = createMockReport({
        close_code: 1000
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('正常关闭')).toBeDefined();
    });

    it('显示 1001 端点离开描述', () => {
      const report = createMockReport({
        close_code: 1001
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('端点离开')).toBeDefined();
    });

    it('显示 1006 连接异常关闭描述', () => {
      const report = createMockReport({
        close_code: 1006
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('连接异常关闭')).toBeDefined();
    });

    it('显示 1002 协议错误描述', () => {
      const report = createMockReport({
        close_code: 1002
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('协议错误')).toBeDefined();
    });

    it('显示 1015 TLS 握手失败描述', () => {
      const report = createMockReport({
        close_code: 1015
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('TLS 握手失败')).toBeDefined();
    });

    it('未知关闭码显示未知状态码', () => {
      const report = createMockReport({
        close_code: 9999
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('未知状态码')).toBeDefined();
    });
  });

  describe('关闭原因显示', () => {
    it('显示关闭原因', () => {
      const report = createMockReport({
        close_reason: '用户主动断开'
      });
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.getByText('用户主动断开')).toBeDefined();
    });

    it('无关闭原因时不显示', () => {
      const report = createMockReport({});
      render(<WsCloseViewer report={report} />);

      // 关闭状态面板默认展开
      expect(screen.queryByText('关闭原因')).toBeNull();
    });
  });

  describe('折叠面板', () => {
    it('连接信息面板默认展开', () => {
      const report = createMockReport({
        url: 'wss://test.com'
      });
      render(<WsCloseViewer report={report} />);

      // URL 应该直接可见
      expect(screen.getByText('wss://test.com')).toBeDefined();
    });

    it('关闭状态面板默认展开', () => {
      const report = createMockReport({
        success: true,
        close_code: 1000
      });
      render(<WsCloseViewer report={report} />);

      // 关闭码应该直接可见
      expect(screen.getByText('1000')).toBeDefined();
    });

    it('可以折叠和展开面板', () => {
      const report = createMockReport({
        url: 'wss://test.com'
      });
      render(<WsCloseViewer report={report} />);

      // 默认展开
      expect(screen.getByText('wss://test.com')).toBeDefined();

      // 点击折叠
      fireEvent.click(screen.getByRole('button', { name: /连接信息/ }));
      expect(screen.queryByText('wss://test.com')).toBeNull();

      // 再次点击展开
      fireEvent.click(screen.getByRole('button', { name: /连接信息/ }));
      expect(screen.getByText('wss://test.com')).toBeDefined();
    });
  });

  describe('完整数据展示', () => {
    it('显示完整的关闭信息', () => {
      const report = createMockReport({
        success: true,
        url: 'wss://api.example.com/socket',
        close_code: 1000,
        close_reason: '正常关闭'
      });
      render(<WsCloseViewer report={report} />);

      // URL
      expect(screen.getByText('wss://api.example.com/socket')).toBeDefined();
      // 成功状态
      expect(screen.getByText('成功')).toBeDefined();
      // 关闭码
      expect(screen.getByText('1000')).toBeDefined();
      // "正常关闭" 出现两次（关闭码描述 + 关闭原因）
      const normalCloseTexts = screen.getAllByText('正常关闭');
      expect(normalCloseTexts.length).toBeGreaterThanOrEqual(1);
    });
  });
});
