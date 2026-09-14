/**
 * RawJsonViewer 组件测试
 * - JSON 树渲染测试（对象/数组）
 * - 纯文本渲染测试（字符串）
 * - 原始值渲染测试（null/undefined/数字/布尔）
 * - JSON 字符串自动解析测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';
import { JSON_VIEWER_THRESHOLDS } from '@/components/execution/viewers/jsonViewerUtils';

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

  describe('多视图切换与大体积防护', () => {
    it('可以在 Tree、JSON 和 Raw 视图之间切换', () => {
      const data = { hello: 'world', count: 123 };
      render(<RawJsonViewer data={data} />);

      // 默认应该展示 Tree 视图
      expect(screen.getByTestId('json-view')).toBeInTheDocument();

      // 点击 JSON 按钮切换至 CodeMirror JSON 视图
      const jsonTabBtn = screen.getByRole('button', { name: 'JSON 代码' });
      fireEvent.click(jsonTabBtn);
      expect(screen.getByTestId('codemirror-json-viewer')).toBeInTheDocument();

      // 点击 Raw 按钮切换至 Raw 视图
      const rawTabBtn = screen.getByRole('button', { name: '原始文本' });
      fireEvent.click(rawTabBtn);
      expect(screen.getByTestId('codemirror-raw-viewer')).toBeInTheDocument();

      // 点击 Tree 按钮切回 Tree 视图
      const treeTabBtn = screen.getByRole('button', { name: '树视图' });
      fireEvent.click(treeTabBtn);
      expect(screen.getByTestId('json-view')).toBeInTheDocument();
    });

    it('紧凑模式 variant="compact" 不渲染顶部工具栏', () => {
      const data = { hello: 'world' };
      render(<RawJsonViewer data={data} variant="compact" />);

      // 应该直接渲染 json-view，且不应该存在视图切换按钮
      expect(screen.getByTestId('json-view')).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: '树视图' })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: 'JSON 代码' })).not.toBeInTheDocument();
    });

    it('超大体积（>= 5MB）且激活 Tree 时展示屏障警示，确认后才渲染树', () => {
      // 构造超过 5MB 的大字符串
      const bigString = 'x'.repeat(JSON_VIEWER_THRESHOLDS.EXTREME + 10);
      const bigObj = { data: bigString };
      render(<RawJsonViewer data={bigObj} defaultView="tree" />);

      // 应该不直接渲染 json-view，而是渲染警示和确认按钮
      expect(screen.queryByTestId('json-view')).not.toBeInTheDocument();
      const forceBtn = screen.getByRole('button', { name: '仍要渲染树视图' });
      expect(forceBtn).toBeInTheDocument();

      // 点击确认后渲染
      fireEvent.click(forceBtn);
      expect(screen.getByTestId('json-view')).toBeInTheDocument();
    });

    it('Tree 视图支持全部折叠和全部展开切换', () => {
      const data = { a: { b: { c: 1 } } };
      render(<RawJsonViewer data={data} />);

      // 初始为小数据，默认展开，切换按钮显示为“全部展开”或“全部折叠”
      const toggleBtn = screen.getByRole('button', { name: /全部(折叠|展开)/ });
      expect(toggleBtn).toBeInTheDocument();

      // 点击切换
      fireEvent.click(toggleBtn);
      expect(screen.getByRole('button', { name: /全部(折叠|展开)/ })).toBeInTheDocument();
    });

    it('同一组件实例在原始值、纯文本与复杂 JSON 对象之间切换时不报 Hooks 错误', () => {
      // 初始为 null
      const { rerender } = render(<RawJsonViewer data={null} />);
      expect(screen.getByTestId('primitive-value').textContent).toBe('null');

      // 切换到数字
      rerender(<RawJsonViewer data={123} />);
      expect(screen.getByTestId('primitive-value').textContent).toBe('123');

      // 切换到普通字符串
      rerender(<RawJsonViewer data="hello text" />);
      expect(screen.getByTestId('plain-text-view').textContent).toBe('hello text');

      // 切换到复杂 JSON 对象
      rerender(<RawJsonViewer data={{ foo: 'bar', list: [1, 2] }} />);
      expect(screen.getByTestId('json-view')).toBeInTheDocument();

      // 切回 undefined
      rerender(<RawJsonViewer data={undefined} />);
      expect(screen.getByTestId('primitive-value').textContent).toBe('undefined');

      // 切回复杂 JSON
      rerender(<RawJsonViewer data={{ active: true }} />);
      expect(screen.getByTestId('json-view')).toBeInTheDocument();
    });
  });
});
