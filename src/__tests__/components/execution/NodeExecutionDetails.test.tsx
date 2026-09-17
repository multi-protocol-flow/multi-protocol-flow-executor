/**
 * NodeExecutionDetails 组件测试
 * - Tab 切换测试
 * - 展开/折叠测试
 * - 状态显示测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodeExecutionDetails } from '@/components/execution/NodeExecutionDetails';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('NodeExecutionDetails', () => {
  const createMockReport = (overrides?: Partial<NodeExecutionReport>): NodeExecutionReport => ({
    node_uuid: 'node-001',
    node_type: 'http:request',
    node_name: '测试节点',
    start_time: Date.now() - 100,
    end_time: Date.now(),
    duration_ms: 100,
    status: 'success',
    input_data: { url: 'https://api.example.com' },
    output_data: { status: 200, data: { id: 1 } },
    ...overrides,
  });

  describe('平铺详情展示', () => {
    it('默认直接展示 Header 与 Tab 切换栏，无需额外展开', () => {
      const report = createMockReport();
      render(<NodeExecutionDetails report={report} />);

      // 节点名与 Tab 按钮均直接可见
      expect(screen.getByText('测试节点')).toBeDefined();
      expect(screen.getByText('结构化')).toBeDefined();
      expect(screen.getByText('原始 JSON')).toBeDefined();
    });
  });

  describe('Tab 切换功能', () => {
    it('默认选中"结构化"Tab', () => {
      const report = createMockReport();
      render(<NodeExecutionDetails report={report} />);


      // 检查结构化 tab 的激活状态
      const structuredTab = screen.getByRole('button', { name: '结构化' });
      expect(structuredTab.className).toContain('bg-background');
    });

    it('切换到"原始 JSON"Tab', () => {
      const report = createMockReport();
      render(<NodeExecutionDetails report={report} />);


      // 点击原始 JSON tab
      fireEvent.click(screen.getByRole('button', { name: '原始 JSON' }));

      // 检查原始 JSON tab 的激活状态
      const rawTab = screen.getByRole('button', { name: '原始 JSON' });
      expect(rawTab.className).toContain('bg-background');
      
      // 结构化 tab 应该不再是激活状态
      const structuredTab = screen.getByRole('button', { name: '结构化' });
      expect(structuredTab.className).toContain('text-muted-foreground');
    });

    it('HTTP 节点结构化视图显示 HTTP 特定内容', () => {
      const report = createMockReport({
        node_type: 'http:request',
        input_data: { url: 'https://test.com' },
        output_data: { status: 200, request: { url: 'https://test.com' } },
      });
      render(<NodeExecutionDetails report={report} />);


      // HTTP 节点使用 HttpNodeViewer，显示请求配置和响应状态
      expect(screen.getByText('请求配置')).toBeDefined();
      expect(screen.getByText('响应状态')).toBeDefined();
    });

    it('Entry 节点结构化视图显示通用内容', () => {
      const report = createMockReport({
        node_type: 'entry',
        input_data: null,
        output_data: { message: '开始执行' },
      });
      render(<NodeExecutionDetails report={report} />);


      // Entry 节点使用 GenericViewer，显示输入输出数据
      expect(screen.getByText('输入数据')).toBeDefined();
      expect(screen.getByText('输出数据')).toBeDefined();
    });

    it('原始 JSON 视图显示完整报告', () => {
      const report = createMockReport();
      render(<NodeExecutionDetails report={report} />);

      
      // 切换到原始 JSON
      fireEvent.click(screen.getByRole('button', { name: '原始 JSON' }));

      // 检查 JSON 视图
      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
      expect(jsonView.textContent).toContain('测试节点');
    });
  });

  describe('状态显示', () => {
    it('成功状态显示绿色图标', () => {
      const report = createMockReport({ status: 'success' });
      const { container } = render(<NodeExecutionDetails report={report} />);

      // 检查成功状态的边框颜色
      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper.className).toContain('border-emerald-200');
    });

    it('失败状态显示红色图标', () => {
      const report = createMockReport({ 
        status: 'failed',
        error: { error_kind: 'execution', message: '执行失败' }
      });
      const { container } = render(<NodeExecutionDetails report={report} />);

      // 检查失败状态的边框颜色
      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper.className).toContain('border-red-200');
    });

    it('显示执行耗时', () => {
      const report = createMockReport({ duration_ms: 250 });
      render(<NodeExecutionDetails report={report} />);

      expect(screen.getByText('250ms')).toBeDefined();
    });
  });

  describe('错误信息显示', () => {
    it('失败状态显示红色边框和错误图标', () => {
      const report = createMockReport({
        status: 'failed',
        error: {
          error_kind: 'execution',
          message: '请求超时',
          stack_trace: 'Error at line 1...',
        },
      });
      const { container } = render(<NodeExecutionDetails report={report} />);

      // 检查失败状态的边框颜色
      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper.className).toContain('border-red-200');
    });

    it('原始 JSON 视图中可以看到完整错误信息', () => {
      const report = createMockReport({
        status: 'failed',
        error: {
          error_kind: 'execution',
          message: '请求超时',
          stack_trace: 'Error at line 1...',
        },
      });
      render(<NodeExecutionDetails report={report} />);

      
      // 切换到原始 JSON
      fireEvent.click(screen.getByRole('button', { name: '原始 JSON' }));

      // 原始 JSON 视图中包含完整的报告数据（包括 error）
      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('请求超时');
    });
  });

  describe('输出端口显示', () => {
    it('Conditional 节点显示评估结果', () => {
      const report = createMockReport({ 
        node_type: 'conditional',
        used_port_id: 'true',
        output_data: { 
          expression: 'status == 200',
          evaluated_value: 200,
          result: true 
        }
      });
      render(<NodeExecutionDetails report={report} />);


      // Conditional 节点使用 ConditionalViewer
      expect(screen.getByText('条件表达式')).toBeDefined();
      expect(screen.getByText('评估结果')).toBeDefined();
    });

    it('没有输出端口时不显示特定内容', () => {
      const report = createMockReport({ used_port_id: undefined });
      render(<NodeExecutionDetails report={report} />);


      // HTTP 节点默认显示请求配置和响应状态
      expect(screen.getByText('请求配置')).toBeDefined();
    });
  });

  describe('节点类型标签', () => {
    it('显示 HTTP 节点类型标签', () => {
      const report = createMockReport({ node_type: 'http:request' });
      render(<NodeExecutionDetails report={report} />);

      expect(screen.getByText('HTTP 请求')).toBeDefined();
    });

    it('显示 Entry 节点类型标签', () => {
      const report = createMockReport({ node_type: 'entry' });
      render(<NodeExecutionDetails report={report} />);

      expect(screen.getByText('入口')).toBeDefined();
    });

    it('显示 Conditional 节点类型标签', () => {
      const report = createMockReport({ node_type: 'conditional' });
      render(<NodeExecutionDetails report={report} />);

      expect(screen.getByText('条件判断')).toBeDefined();
    });

    it('显示未知节点类型的原始值', () => {
      const report = createMockReport({ node_type: 'unknown_type' as 'http:request' });
      render(<NodeExecutionDetails report={report} />);
      expect(screen.getAllByText('unknown_type').length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('紧凑容器排版', () => {
    it('在较窄宽度或容器下，Header 两行分层清晰渲染且节点名具备 truncate 保护', () => {
      const report = createMockReport({
        node_name: '这是一个非常非常长的节点名称用于验证窄容器下的溢出保护和截断行为',
      });
      render(
        <div style={{ width: '320px' }}>
          <NodeExecutionDetails report={report} />
        </div>
      );

      // 第一行：标题与标签正常渲染
      const nodeNameEl = screen.getByText(/这是一个非常非常长/);
      expect(nodeNameEl).toBeDefined();
      expect(nodeNameEl.className).toContain('truncate');
      expect(screen.getByText('HTTP 请求')).toBeDefined();
      expect(screen.getByText('100ms')).toBeDefined();

      // 第二行：操作栏与 Tab 正常渲染
      expect(screen.getByText('结构化')).toBeDefined();
      expect(screen.getByText('原始 JSON')).toBeDefined();
      expect(screen.getByRole('button', { name: /复制节点完整报告/ })).toBeDefined();
    });
  });
});
