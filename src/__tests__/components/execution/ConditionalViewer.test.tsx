/**
 * ConditionalViewer 组件测试
 * - 条件结果展示测试
 * - 折叠面板测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConditionalViewer } from '@/components/execution/viewers/ConditionalViewer';
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

describe('ConditionalViewer', () => {
  const createMockReport = (
    outputData?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-cond-001',
    node_type: 'conditional',
    node_name: '条件分支节点',
    start_time: Date.now() - 100,
    end_time: Date.now(),
    duration_ms: 100,
    status: 'success',
    output_data: outputData,
  });

  describe('条件表达式显示', () => {
    it('显示条件表达式', () => {
      const report = createMockReport({
        expression: 'status == 200'
      });
      render(<ConditionalViewer report={report} />);

      expect(screen.getByText('status == 200')).toBeDefined();
    });

    it('未配置条件表达式时显示提示', () => {
      const report = createMockReport({});
      render(<ConditionalViewer report={report} />);

      expect(screen.getByText('未配置条件表达式')).toBeDefined();
    });
  });

  describe('评估结果显示', () => {
    it('显示 true 结果', () => {
      const report = createMockReport({
        result: true
      });
      render(<ConditionalViewer report={report} />);

      // 评估结果面板默认展开，直接检查内容
      expect(screen.getByText('true')).toBeDefined();
      expect(screen.getByText('条件成立')).toBeDefined();
    });

    it('显示 false 结果', () => {
      const report = createMockReport({
        result: false
      });
      render(<ConditionalViewer report={report} />);

      // 评估结果面板默认展开
      expect(screen.getByText('false')).toBeDefined();
      expect(screen.getByText('条件不成立')).toBeDefined();
    });

    it('无评估结果时显示提示', () => {
      const report = createMockReport({});
      render(<ConditionalViewer report={report} />);

      // 评估结果面板默认展开
      expect(screen.getByText('无评估结果')).toBeDefined();
    });
  });

  describe('评估值显示', () => {
    it('显示字段值', () => {
      const report = createMockReport({
        field_value: '200'
      });
      render(<ConditionalViewer report={report} />);

      // 评估值面板默认展开
      expect(screen.getByText('Field Value')).toBeDefined();
    });

    it('显示评估后的值', () => {
      const report = createMockReport({
        evaluated_value: 200
      });
      render(<ConditionalViewer report={report} />);

      // 评估值面板默认展开
      expect(screen.getByText('Evaluated Value')).toBeDefined();
    });

    it('未设置值时显示提示', () => {
      const report = createMockReport({});
      render(<ConditionalViewer report={report} />);

      // 评估值面板默认展开
      // 有两个"无数据"提示（字段值和评估值）
      const unsetTexts = screen.getAllByText(getLL().execution.viewers.no_data());
      expect(unsetTexts.length).toBe(2);
    });

    it('显示对象类型的值', () => {
      const report = createMockReport({
        field_value: { nested: 'value' }
      });
      render(<ConditionalViewer report={report} />);

      // 评估值面板默认展开
      // 对象会被 JSON.stringify
      expect(screen.getByText(/nested/)).toBeDefined();
    });
  });

  describe('错误状态显示', () => {
    it('显示评估失败状态', () => {
      const report = createMockReport({
        success: false,
        error: '表达式语法错误'
      });
      // 组件检查的是 report.status 而非 output_data.success
      report.status = 'failed';
      render(<ConditionalViewer report={report} />);

      // 评估结果面板默认展开，显示 evaluation_failed i18n 文本
      expect(screen.getByText('评估失败')).toBeDefined();
    });

    it('显示错误图标', () => {
      const report = createMockReport({
        success: false,
        error: '未知错误'
      });
      render(<ConditionalViewer report={report} />);

      // 评估结果面板应该使用错误图标
      const resultButton = screen.getByRole('button', { name: /评估结果/ });
      expect(resultButton).toBeDefined();
    });
  });

  describe('折叠面板', () => {
    it('条件表达式面板默认展开', () => {
      const report = createMockReport({ expression: 'test' });
      render(<ConditionalViewer report={report} />);

      // 条件表达式应该直接可见
      expect(screen.getByText('test')).toBeDefined();
    });

    it('评估值面板默认展开', () => {
      const report = createMockReport({ field_value: 'test' });
      render(<ConditionalViewer report={report} />);

      // 字段值标题应该可见
      expect(screen.getByText('Field Value')).toBeDefined();
    });

    it('评估结果面板默认展开', () => {
      const report = createMockReport({ result: true });
      render(<ConditionalViewer report={report} />);

      // 评估结果应该可见
      expect(screen.getByText('true')).toBeDefined();
    });

    it('可以折叠和展开面板', () => {
      const report = createMockReport({ expression: 'test' });
      render(<ConditionalViewer report={report} />);

      // 默认展开，表达式可见
      expect(screen.getByText('test')).toBeDefined();

      // 点击折叠
      fireEvent.click(screen.getByRole('button', { name: /条件表达式/ }));
      expect(screen.queryByText('test')).toBeNull();

      // 再次点击展开
      fireEvent.click(screen.getByRole('button', { name: /条件表达式/ }));
      expect(screen.getByText('test')).toBeDefined();
    });
  });
});
