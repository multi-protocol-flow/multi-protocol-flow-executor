/**
 * GenericViewer 组件测试
 * - 通用展示测试
 * - 节点信息显示测试
 * - 变量变更展示测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { GenericViewer } from '@/components/execution/viewers/GenericViewer';
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

describe('GenericViewer', () => {
  const createMockReport = (
    overrides?: Partial<NodeExecutionReport>
  ): NodeExecutionReport => ({
    node_uuid: 'node-generic-001',
    node_type: 'entry',
    node_name: '入口节点',
    start_time: Date.now() - 50,
    end_time: Date.now(),
    duration_ms: 50,
    status: 'success',
    input_data: null,
    output_data: { message: '开始执行' },
    ...overrides,
  });

  describe('节点信息显示', () => {
    it('显示节点名称', () => {
      const report = createMockReport({ node_name: '测试节点' });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('测试节点')).toBeDefined();
    });

    it('显示节点类型', () => {
      const report = createMockReport({ node_type: 'entry' });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('entry')).toBeDefined();
    });

    it('显示节点信息标题', () => {
      const report = createMockReport();
      render(<GenericViewer report={report} />);

      expect(screen.getByText('节点信息')).toBeDefined();
      expect(screen.getByText('名称')).toBeDefined();
      expect(screen.getByText('类型')).toBeDefined();
    });
  });

  describe('输入数据显示', () => {
    it('显示输入数据', () => {
      const report = createMockReport({
        input_data: { url: 'https://api.example.com' }
      });
      render(<GenericViewer report={report} />);

      // 输入数据面板默认展开，检查内容
      expect(screen.getByText(/"url": "https:\/\/api.example.com"/)).toBeDefined();
    });

    it('无输入数据时显示提示', () => {
      const report = createMockReport({ input_data: null });
      render(<GenericViewer report={report} />);

      // 输入数据面板默认展开
      expect(screen.getByText('无输入数据')).toBeDefined();
    });

    it('输入数据为 undefined 时显示提示', () => {
      const report = createMockReport({ input_data: undefined });
      render(<GenericViewer report={report} />);

      // 输入数据面板默认展开
      expect(screen.getByText('无输入数据')).toBeDefined();
    });
  });

  describe('输出数据显示', () => {
    it('显示输出数据', () => {
      const report = createMockReport({
        output_data: { result: 'success', data: { id: 1 } }
      });
      render(<GenericViewer report={report} />);

      // 输出数据面板默认展开，检查内容
      expect(screen.getByText(/"result": "success"/)).toBeDefined();
    });

    it('无输出数据时显示提示', () => {
      const report = createMockReport({ output_data: null });
      render(<GenericViewer report={report} />);

      // 输出数据面板默认展开
      expect(screen.getByText('无输出数据')).toBeDefined();
    });
  });

  describe('折叠面板', () => {
    it('所有面板默认展开', () => {
      const report = createMockReport();
      render(<GenericViewer report={report} />);

      // 所有面板标题应该可见
      expect(screen.getByText('节点信息')).toBeDefined();
      expect(screen.getByText('输入数据')).toBeDefined();
      expect(screen.getByText('输出数据')).toBeDefined();
    });

    it('可以折叠和展开面板', () => {
      const report = createMockReport({ node_name: 'TestNode' });
      render(<GenericViewer report={report} />);

      // 节点名称应该可见
      expect(screen.getByText('TestNode')).toBeDefined();

      // 点击折叠节点信息
      fireEvent.click(screen.getByRole('button', { name: /节点信息/ }));
      expect(screen.queryByText('TestNode')).toBeNull();

      // 再次点击展开
      fireEvent.click(screen.getByRole('button', { name: /节点信息/ }));
      expect(screen.getByText('TestNode')).toBeDefined();
    });
  });

  describe('不同节点类型', () => {
    it('Entry 节点正常显示', () => {
      const report = createMockReport({
        node_type: 'entry',
        node_name: '入口',
        output_data: { initial_variables: { api_key: 'xxx' } }
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('入口')).toBeDefined();
      expect(screen.getByText('entry')).toBeDefined();
    });

    it('End 节点正常显示', () => {
      const report = createMockReport({
        node_type: 'end',
        node_name: '结束',
        input_data: { final_result: 'done' }
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('结束')).toBeDefined();
      expect(screen.getByText('end')).toBeDefined();
    });

    it('未知节点类型正常显示', () => {
      const report = createMockReport({
        node_type: 'custom_type' as 'entry',
        node_name: '自定义节点'
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('自定义节点')).toBeDefined();
      expect(screen.getByText('custom_type')).toBeDefined();
    });
  });

  describe('变量变更展示', () => {
    it('无变量变更时不显示变量变更面板', () => {
      const report = createMockReport();
      render(<GenericViewer report={report} />);

      expect(screen.queryByRole('button', { name: /变量变更/ })).toBeNull();
    });

    it('有变量变更时显示变量变更面板', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 }
        ]
      });
      render(<GenericViewer report={report} />);

      // 简化格式：显示 "变量变更" + 变更数量
      expect(screen.getByRole('button', { name: new RegExp(getLL().execution.viewers.variable_changes()) })).toBeDefined();
    });

    it('显示变量变更数量', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 },
          { key: 'body', old_value: null, new_value: { data: 'test' } }
        ]
      });
      render(<GenericViewer report={report} />);

      // 新组件显示变更统计
      expect(screen.getByText(/变量变更/)).toBeDefined();
    });
  });

  describe('old → new 对比显示', () => {
    it('显示变量名', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 }
        ]
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('status')).toBeDefined();
    });

    it('显示旧值（删除线）', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 }
        ]
      });
      render(<GenericViewer report={report} />);

      // 查找旧值 - 新组件使用 text-slate-500 line-through
      const oldCode = screen.getByText('404');
      expect(oldCode).toBeDefined();
      expect(oldCode.className).toContain('line-through');
    });

    it('显示新值', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 }
        ]
      });
      render(<GenericViewer report={report} />);

      // 查找新值 - 新组件使用 text-slate-700
      const newCode = screen.getByText('200');
      expect(newCode).toBeDefined();
    });

    it('新创建的变量显示正确', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'newVar', old_value: undefined, new_value: 'test' }
        ]
      });
      render(<GenericViewer report={report} />);

      // 新变量应该显示
      const newCode = screen.getByText('test');
      expect(newCode).toBeDefined();
    });

    it('新创建的变量不显示旧值', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'newVar', old_value: undefined, new_value: 'test' }
        ]
      });
      render(<GenericViewer report={report} />);

      // 新变量应该显示 "新增" 标记
      expect(screen.getByText('新增')).toBeDefined();
    });

    it('显示删除标记和添加标记', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 }
        ]
      });
      render(<GenericViewer report={report} />);

      // 应该显示"更新"标记（VariableChangesCard 使用中文标记）
      expect(screen.getByText('更新')).toBeDefined();
    });
  });

  describe('复杂值展示', () => {
    it('展示对象类型的值', () => {
      const report = createMockReport({
        variable_changes: [
          { 
            key: 'body', 
            old_value: { status: 'old' }, 
            new_value: { status: 'new' }
          }
        ]
      });
      render(<GenericViewer report={report} />);

      // 对象使用 RawJsonViewer 组件（mock 后显示 JSON）
      // 查找 JSON 内容
      const jsonViews = screen.getAllByTestId('json-view');
      expect(jsonViews.length).toBeGreaterThan(0);
    });

    it('展示变量值为 null 的情况（new_value 为 null）', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'value', old_value: 'old', new_value: null }
        ]
      });
      render(<GenericViewer report={report} />);

      // 简化格式：变量名 → 当前值 → 旧值
      // 当 new_value 为 null 时，当前值显示为 "null"
      expect(screen.getByText('value')).toBeDefined();
      expect(screen.getByText('null')).toBeDefined();
      // 显示旧值
      expect(screen.getByText('old')).toBeDefined();
    });

    it('展示字符串值', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'message', old_value: 'hello', new_value: 'world' }
        ]
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('hello')).toBeDefined();
      expect(screen.getByText('world')).toBeDefined();
    });

    it('展示数字值', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'count', old_value: 10, new_value: 20 }
        ]
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('10')).toBeDefined();
      expect(screen.getByText('20')).toBeDefined();
    });
  });

  describe('多变量变更', () => {
    it('展示多个变量变更', () => {
      const report = createMockReport({
        variable_changes: [
          { key: 'status', old_value: 404, new_value: 200 },
          { key: 'body', old_value: null, new_value: { data: 'test' } },
          { key: 'count', old_value: 1, new_value: 2 }
        ]
      });
      render(<GenericViewer report={report} />);

      expect(screen.getByText('status')).toBeDefined();
      expect(screen.getByText('body')).toBeDefined();
      expect(screen.getByText('count')).toBeDefined();
    });
  });
});
