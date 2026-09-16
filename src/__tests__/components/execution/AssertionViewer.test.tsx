/**
 * AssertionViewer 组件测试
 * - 空数据提示
 * - 断言卡片与详情展示
 * - 自定义描述与国际化回退（断言 #N / Assertion #N）
 * - 失败断言的索引匹配（避免 JSON 反序列化后引用不一致导致的 #0 缺陷）
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { AssertionViewer } from '@/components/execution/viewers/AssertionViewer';
import type { NodeExecutionReport } from '@/types';
import type { AssertionOutput } from '@/types/node';
import enUS from '@/i18n/en-US';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('AssertionViewer', () => {
  const createMockReport = (
    outputData?: AssertionOutput,
    resolvedConfig?: Record<string, unknown>
  ): NodeExecutionReport => ({
    node_uuid: 'node-assert-001',
    node_type: 'assertion',
    node_name: '断言节点',
    start_time: Date.now() - 100,
    end_time: Date.now(),
    duration_ms: 100,
    status: 'success',
    output_data: outputData as unknown as Record<string, unknown>,
    resolved_config: resolvedConfig,
  });

  it('无 output_data 时显示空数据提示', () => {
    const report = createMockReport(undefined);
    render(<AssertionViewer report={report} />);
    expect(screen.getByText('无断言执行数据')).toBeDefined();
  });

  it('当断言有自定义描述时，优先展示自定义描述', () => {
    const output: AssertionOutput = {
      passed: true,
      total: 1,
      passed_count: 1,
      failed_count: 0,
      results: [
        {
          id: 'assert_1',
          description: '状态码必须为 200',
          passed: true,
          target: '$.status',
          operator: 'eq',
          expected: 200,
          actual: 200,
        },
      ],
      failed_assertions: [],
      input: {},
    };

    const report = createMockReport(output);
    render(<AssertionViewer report={report} />);

// 无失败时全部断言列表默认已直接展开

    expect(screen.getByText('状态码必须为 200')).toBeDefined();
  });

  it('当断言无自定义描述时，展示国际化的 断言 #N 默认标签', () => {
    const output: AssertionOutput = {
      passed: true,
      total: 2,
      passed_count: 2,
      failed_count: 0,
      results: [
        {
          id: 'assert_1',
          passed: true,
          target: '$.status',
          operator: 'eq',
          expected: 200,
          actual: 200,
        },
        {
          id: 'assert_2',
          passed: true,
          target: '$.body.code',
          operator: 'eq',
          expected: 0,
          actual: 0,
        },
      ],
      failed_assertions: [],
      input: {},
    };

    const report = createMockReport(output);
    render(<AssertionViewer report={report} />);

// 无失败时全部断言列表默认已直接展开

    expect(screen.getByText('断言 #1')).toBeDefined();
    expect(screen.getByText('断言 #2')).toBeDefined();
  });

  it('失败断言即便经过 JSON 反序列化成不同对象实例，也能正确匹配索引而不是显示 断言 #0', () => {
    // 模拟来自后端的独立对象（反序列化后 results 与 failed_assertions 是深拷贝，引用不同）
    const failedItemInResults = {
      id: 'assert_2',
      passed: false,
      target: '$.data.status',
      operator: 'eq',
      expected: 'active',
      actual: 'pending',
      error: 'Expected active but got pending',
    };

    const failedItemInFailures = {
      ...failedItemInResults,
    };

    // 确认两个对象引用不同
    expect(failedItemInResults).not.toBe(failedItemInFailures);

    const output: AssertionOutput = {
      passed: false,
      total: 3,
      passed_count: 2,
      failed_count: 1,
      results: [
        {
          id: 'assert_1',
          passed: true,
          target: '$.status',
          operator: 'eq',
          expected: 200,
          actual: 200,
        },
        failedItemInResults,
        {
          id: 'assert_3',
          passed: true,
          target: '$.message',
          operator: 'eq',
          expected: 'success',
          actual: 'success',
        },
      ],
      failed_assertions: [failedItemInFailures],
      input: {},
    };

    const report = createMockReport(output);
    render(<AssertionViewer report={report} />);

    // 失败卡片默认展开，第二条断言失败应显示为 "断言 #2"，绝不能是 "断言 #0"
    expect(screen.queryByText('断言 #0')).toBeNull();
    expect(screen.getByText('断言 #2')).toBeDefined();
  });

  it('英文语言包中包含正确的 assertion_item 格式', () => {
    const enExecution = enUS.execution as { viewers: { assertion_item: string } };
    expect(enExecution.viewers.assertion_item).toBe('Assertion #{index}');
  });
});
