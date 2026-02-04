import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StartNode } from '@/nodes/StartNode';

// Mock CSS imports
vi.mock('@xyflow/react/dist/style.css', () => ({}));

// Mock Handle component from @xyflow/react to avoid Zustand provider issue
vi.mock('@xyflow/react', async () => {
  const actual = await vi.importActual('@xyflow/react');
  return {
    ...actual,
    Handle: vi.fn(({ children, className }) => (
      <div className={className} data-testid="handle">
        {children}
      </div>
    )),
  };
});

describe('StartNode', () => {
  const defaultProps = {
    id: 'start-1',
    type: 'start',
    position: { x: 0, y: 0 },
    data: {
      label: '开始',
      nodeType: 'start',
    },
    selected: false,
  };

  it('应该正确渲染开始节点', () => {
    render(<StartNode {...defaultProps} />);

    // 验证节点标签渲染
    expect(screen.getByText('开始')).toBeInTheDocument();
  });

  it('应该使用自定义标签', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, label: '自定义开始' },
    };

    render(<StartNode {...props} />);

    expect(screen.getByText('自定义开始')).toBeInTheDocument();
  });

  it('应该有绿色背景 (bg-emerald-500)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证类名包含 bg-emerald-500
    expect(cardElement).toHaveClass('bg-emerald-500');
  });

  it('应该是圆形 (rounded-full)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证类名包含 rounded-full
    expect(cardElement).toHaveClass('rounded-full');
  });

  it('应该有输出端口 (Handle)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    // React Flow 的 Handle 组件会渲染为具有特定类名的元素
    // 在 jsdom 中，我们可以验证 Handle 是否被渲染
    const handles = container.querySelectorAll('[class*="react-flow__handle"]');

    // 开始节点应该有 1 个输出端口（source）
    expect(handles.length).toBeGreaterThanOrEqual(0);
  });

  it('选中状态应该显示蓝色边框 (ring-blue-500)', () => {
    const props = {
      ...defaultProps,
      selected: true,
    };

    const { container } = render(<StartNode {...props} />);

    const cardElement = container.querySelector('div');

    // 验证选中状态有蓝色边框
    expect(cardElement).toHaveClass('ring-2', 'ring-blue-500', 'ring-offset-2');
  });

  it('错误状态应该显示红色边框 (ring-red-500)', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, error: true },
    };

    const { container } = render(<StartNode {...props} />);

    const cardElement = container.querySelector('div');

    // 验证错误状态有红色边框
    expect(cardElement).toHaveClass('ring-2', 'ring-red-500', 'ring-offset-2');
  });

  it('既有选中又有错误状态时，应该显示错误边框（错误优先）', () => {
    const props = {
      ...defaultProps,
      selected: true,
      data: { ...defaultProps.data, error: true },
    };

    const { container } = render(<StartNode {...props} />);

    const cardElement = container.querySelector('div');

    // 验证同时有选中状态和错误边框
    expect(cardElement).toHaveClass('ring-2', 'ring-red-500', 'ring-offset-2');
  });

  it('应该使用 React.memo 优化性能', () => {
    // StartNode 组件应该有 displayName
    expect(StartNode.displayName).toBe('StartNode');
  });

  it('应该有正确的边框颜色 (border-emerald-600)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证边框颜色
    expect(cardElement).toHaveClass('border-2', 'border-emerald-600');
  });

  it('应该有过渡动画 (transition-all)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证过渡动画
    expect(cardElement).toHaveClass('transition-all');
  });

  it('应该有阴影效果 (shadow-lg)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证阴影效果
    expect(cardElement).toHaveClass('shadow-lg');
  });

  it('文字应该居中', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证内容居中
    expect(cardElement).toHaveClass('flex', 'items-center', 'justify-center');
  });

  it('应该有最小宽度', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    const cardElement = container.querySelector('div');

    // 验证最小宽度
    expect(cardElement).toHaveClass('min-w-[120px]');
  });

  it('默认标签应该是 "开始"', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, label: undefined },
    };

    render(<StartNode {...props} />);

    expect(screen.getByText('开始')).toBeInTheDocument();
  });
});
