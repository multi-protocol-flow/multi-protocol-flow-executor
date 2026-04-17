/**
 * RawJsonViewer 组件测试
 * - JSON 树渲染测试（对象/数组）
 * - 纯文本渲染测试（字符串）
 * - 原始值渲染测试（null/undefined/数字/布尔）
 * - JSON 字符串自动解析测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';

// Mock react-json-view-lite — 在 mock 的 pre 上标记 data-testid
// RawJsonViewer 组件不再在 wrapper div 上添加 data-testid，所以不会重复
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('RawJsonViewer', () => {
  describe('JSON 树渲染（对象/数组）', () => {
    it('渲染对象数据', () => {
      const data = { name: 'test', value: 123 };
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('test');
      expect(jsonView.textContent).toContain('123');
    });

    it('渲染数组数据', () => {
      const data = [1, 2, 3];
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('1');
      expect(jsonView.textContent).toContain('2');
      expect(jsonView.textContent).toContain('3');
    });

    it('渲染嵌套对象', () => {
      const data = {
        user: {
          name: 'John',
          address: {
            city: 'Beijing'
          }
        }
      };
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('John');
      expect(jsonView.textContent).toContain('Beijing');
    });

    it('渲染空对象', () => {
      const data = {};
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
    });

    it('渲染空数组', () => {
      const data: unknown[] = [];
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
    });

    it('渲染包含特殊字符的字符串（对象内）', () => {
      const data = { text: '包含"引号"和\n换行' };
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
    });

    it('渲染复杂嵌套结构', () => {
      const data = {
        items: [
          { id: 1, name: 'Item 1' },
          { id: 2, name: 'Item 2' }
        ],
        meta: {
          total: 2,
          page: 1
        }
      };
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('Item 1');
      expect(jsonView.textContent).toContain('Item 2');
      expect(jsonView.textContent).toContain('total');
    });
  });

  describe('原始值渲染', () => {
    it('渲染 null 值', () => {
      render(<RawJsonViewer data={null} />);

      const el = screen.getByTestId('primitive-value');
      expect(el.textContent).toBe('null');
    });

    it('渲染 undefined 值', () => {
      render(<RawJsonViewer data={undefined} />);

      const el = screen.getByTestId('primitive-value');
      expect(el.textContent).toBe('undefined');
    });

    it('渲染数字', () => {
      render(<RawJsonViewer data={42} />);

      const el = screen.getByTestId('primitive-value');
      expect(el.textContent).toBe('42');
    });

    it('渲染布尔值', () => {
      render(<RawJsonViewer data={true} />);

      const el = screen.getByTestId('primitive-value');
      expect(el.textContent).toBe('true');
    });
  });

  describe('字符串渲染', () => {
    it('纯文本字符串显示为纯文本', () => {
      render(<RawJsonViewer data="hello world" />);

      const el = screen.getByTestId('plain-text-view');
      expect(el.textContent).toBe('hello world');
    });

    it('JSON 字符串自动解析为 JSON 树', () => {
      const jsonString = '{"name":"test","value":123}';
      render(<RawJsonViewer data={jsonString} />);

      // JSON.parse 成功 → 对象 → 走 json-view 分支
      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('test');
      expect(jsonView.textContent).toContain('123');
    });

    it('JSON 数组字符串自动解析为 JSON 树', () => {
      const jsonString = '[1,2,3]';
      render(<RawJsonViewer data={jsonString} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('1');
      expect(jsonView.textContent).toContain('2');
      expect(jsonView.textContent).toContain('3');
    });

    it('JSON 字符串解析为原始值时显示纯文本', () => {
      // JSON.parse("42") → 42 (number, not object) → 纯文本
      render(<RawJsonViewer data="42" />);

      const el = screen.getByTestId('plain-text-view');
      expect(el.textContent).toBe('42');
    });

    it('无效 JSON 字符串显示为纯文本', () => {
      render(<RawJsonViewer data="not valid json {" />);

      const el = screen.getByTestId('plain-text-view');
      expect(el.textContent).toBe('not valid json {');
    });

    it('XML 字符串显示为纯文本', () => {
      const xml = '<root><item>test</item></root>';
      render(<RawJsonViewer data={xml} />);

      const el = screen.getByTestId('plain-text-view');
      expect(el.textContent).toBe(xml);
    });
  });
});
