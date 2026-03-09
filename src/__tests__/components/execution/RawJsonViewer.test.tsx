/**
 * RawJsonViewer 组件测试
 * - JSON 渲染测试
 * - 数据类型处理测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

describe('RawJsonViewer', () => {
  describe('JSON 渲染', () => {
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

    it('渲染 null 值', () => {
      const data = null;
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
    });

    it('渲染 undefined 值', () => {
      const data = undefined;
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView).toBeDefined();
    });

    it('渲染字符串', () => {
      const data = 'hello world';
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('hello world');
    });

    it('渲染数字', () => {
      const data = 42;
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('42');
    });

    it('渲染布尔值', () => {
      const data = true;
      render(<RawJsonViewer data={data} />);

      const jsonView = screen.getByTestId('json-view');
      expect(jsonView.textContent).toContain('true');
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

    it('渲染包含特殊字符的字符串', () => {
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
});
