/**
 * HttpNodeViewer 组件测试
 * - 状态码着色测试
 * - HTTP 方法颜色测试
 * - 折叠面板测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HttpNodeViewer } from '@/components/execution/viewers/HttpNodeViewer';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('HttpNodeViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>,
    resolvedConfig?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-http-001',
    node_type: 'http:request',
    node_name: 'HTTP 请求节点',
    start_time: Date.now() - 500,
    end_time: Date.now(),
    duration_ms: 500,
    status: 'success',
    output_data: outputData,
    resolved_config: resolvedConfig,
  });

  describe('状态码着色', () => {
    it('2xx 状态码显示绿色', () => {
      const report = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={report} />);

      // 响应状态面板默认展开，直接查找包含状态码的 Badge
      const badge = screen.getByText('200');
      expect(badge.className).toContain('text-emerald-600');
      expect(badge.className).toContain('bg-emerald-50');
    });

    it('4xx 状态码显示黄色', () => {
      const report = createMockReport({ status: 404 });
      render(<HttpNodeViewer report={report} />);

      // 响应状态面板默认展开
      const badge = screen.getByText('404');
      expect(badge.className).toContain('text-amber-600');
      expect(badge.className).toContain('bg-amber-50');
    });

    it('5xx 状态码显示红色', () => {
      const report = createMockReport({ status: 500 });
      render(<HttpNodeViewer report={report} />);

      // 响应状态面板默认展开
      const badge = screen.getByText('500');
      expect(badge.className).toContain('text-red-600');
      expect(badge.className).toContain('bg-red-50');
    });

    it('其他状态码显示灰色', () => {
      const report = createMockReport({ status: 301 });
      render(<HttpNodeViewer report={report} />);

      // 响应状态面板默认展开
      const badge = screen.getByText('301');
      expect(badge.className).toContain('text-muted-foreground');
    });

    it('显示状态码描述文字', () => {
      const successReport = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={successReport} />);
      // 响应状态面板默认展开
      expect(screen.getByText('请求成功')).toBeDefined();
    });
  });

  describe('HTTP 方法颜色', () => {
    it('GET 方法显示蓝色', () => {
      const report = createMockReport({}, { method: 'GET' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('GET');
      expect(methodBadge.className).toContain('text-blue-600');
    });

    it('POST 方法显示绿色', () => {
      const report = createMockReport({}, { method: 'POST' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('POST');
      expect(methodBadge.className).toContain('text-emerald-600');
    });

    it('PUT 方法显示黄色', () => {
      const report = createMockReport({}, { method: 'PUT' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('PUT');
      expect(methodBadge.className).toContain('text-amber-600');
    });

    it('PATCH 方法显示紫色', () => {
      const report = createMockReport({}, { method: 'PATCH' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('PATCH');
      expect(methodBadge.className).toContain('text-purple-600');
    });

    it('DELETE 方法显示红色', () => {
      const report = createMockReport({}, { method: 'DELETE' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('DELETE');
      expect(methodBadge.className).toContain('text-red-600');
    });

    it('未知方法显示灰色', () => {
      const report = createMockReport({}, { method: 'CUSTOM' });
      render(<HttpNodeViewer report={report} />);

      const methodBadge = screen.getByText('CUSTOM');
      expect(methodBadge.className).toContain('text-muted-foreground');
    });

    it('方法名大小写不敏感（颜色正确）', () => {
      const report = createMockReport({}, { method: 'get' });
      render(<HttpNodeViewer report={report} />);

      // 方法名显示为原始值 'get'，但颜色应该匹配 GET 的蓝色
      const methodBadge = screen.getByText('get');
      expect(methodBadge.className).toContain('text-blue-600');
    });
  });

  describe('折叠面板', () => {
    it('请求配置面板默认展开', () => {
      const report = createMockReport({}, { url: 'https://api.example.com' });
      render(<HttpNodeViewer report={report} />);

      // 请求配置应该可见
      expect(screen.getByText('https://api.example.com')).toBeDefined();
    });

    it('响应状态面板默认展开', () => {
      const report = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={report} />);

      // 响应状态应该可见
      expect(screen.getByText('200')).toBeDefined();
    });

    it('响应头面板默认折叠', () => {
      const report = createMockReport({ 
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
      render(<HttpNodeViewer report={report} />);

      // 响应头内容应该不可见（需要点击展开）
      expect(screen.queryByText('content-type:')).toBeNull();
    });

    it('点击可以展开/折叠面板', () => {
      const report = createMockReport({ 
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
      render(<HttpNodeViewer report={report} />);

      // 展开响应头
      fireEvent.click(screen.getByRole('button', { name: /响应头/ }));
      expect(screen.getByText('content-type:')).toBeDefined();

      // 折叠响应头
      fireEvent.click(screen.getByRole('button', { name: /响应头/ }));
      expect(screen.queryByText('content-type:')).toBeNull();
    });
  });

  describe('请求配置显示', () => {
    it('显示 URL', () => {
      const report = createMockReport({}, { url: 'https://api.example.com/users' });
      render(<HttpNodeViewer report={report} />);

      expect(screen.getByText('https://api.example.com/users')).toBeDefined();
    });

    it('显示请求头', () => {
      const report = createMockReport({}, { 
        headers: { 'authorization': 'Bearer token' }
      });
      render(<HttpNodeViewer report={report} />);

      expect(screen.getByText('请求头')).toBeDefined();
      expect(screen.getByText(/authorization:/)).toBeDefined();
    });

    it('显示请求体', () => {
      const report = createMockReport({}, { 
        body: { name: 'test' },
        body_type: 'body'
      });
      render(<HttpNodeViewer report={report} />);

      expect(screen.getByText(/请求体/)).toBeDefined();
    });

    it('未配置 URL 时显示提示', () => {
      const report = createMockReport({}, { url: '' });
      render(<HttpNodeViewer report={report} />);

      expect(screen.getByText('未配置 URL')).toBeDefined();
    });
  });

  describe('耗时分解显示', () => {
    it('显示耗时分解信息', () => {
      const report = createMockReport({
        status: 200,
        timing: {
          dns_ms: 10,
          connect_ms: 20,
          tls_ms: 30,
          ttfb_ms: 50,
          download_ms: 15,
        }
      });
      render(<HttpNodeViewer report={report} />);

      // 展开耗时分解面板
      fireEvent.click(screen.getByRole('button', { name: /耗时分解/ }));

      expect(screen.getByText('DNS 解析')).toBeDefined();
      expect(screen.getByText('10ms')).toBeDefined();
      expect(screen.getByText('TCP 连接')).toBeDefined();
      expect(screen.getByText('20ms')).toBeDefined();
      expect(screen.getByText('TLS 握手')).toBeDefined();
      expect(screen.getByText('30ms')).toBeDefined();
      expect(screen.getByText('首字节时间')).toBeDefined();
      expect(screen.getByText('50ms')).toBeDefined();
      expect(screen.getByText('下载耗时')).toBeDefined();
      expect(screen.getByText('15ms')).toBeDefined();
    });

    it('没有耗时数据时不显示耗时分解面板', () => {
      const report = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={report} />);

      expect(screen.queryByRole('button', { name: /耗时分解/ })).toBeNull();
    });
  });

  describe('响应体显示', () => {
    it('默认直接显示响应体，无需手动展开', () => {
      const report = createMockReport({
        status: 200,
        body: { data: 'test' }
      });
      render(<HttpNodeViewer report={report} />);

      // 检查 JSON 视图直接可见
      expect(screen.getByTestId('json-view')).toBeDefined();
    });
    it('没有响应体时不显示响应体面板', () => {
      const report = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={report} />);

      expect(screen.queryByRole('button', { name: /响应体/ })).toBeNull();
    });
  });

  describe('空数据处理', () => {
    it('无响应状态时显示提示', () => {
      const report = createMockReport({});
      render(<HttpNodeViewer report={report} />);

      // 响应状态面板默认展开
      expect(screen.getByText('无响应状态')).toBeDefined();
    });

    it('无响应头时显示提示', () => {
      const report = createMockReport({ status: 200 });
      render(<HttpNodeViewer report={report} />);

      fireEvent.click(screen.getByRole('button', { name: /响应头/ }));

      expect(screen.getByText('无数据')).toBeDefined();
    });
  });
});
