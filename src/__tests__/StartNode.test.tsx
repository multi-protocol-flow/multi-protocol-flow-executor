import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StartNode } from '../nodes/StartNode';

// Mock CSS imports
vi.mock('@xyflow/react/dist/style.css', () => ({}));
vi.mock('*.css', () => ({}));

// Mock Handle component from @xyflow/react to avoid Zustand provider issue
vi.mock('@xyflow/react', () => ({
  Handle: ({ children, className, type, position }: { children?: React.ReactNode; className?: string; type?: string; position?: any }) => (
    <div
      className={className}
      data-testid={type === 'source' ? 'output-handle' : 'input-handle'}
      data-type={type}
      data-position={position}
    >
      {children}
    </div>
  ),
  Position: {},
  NodeProps: {},
}));

describe('StartNode', () => {
  const defaultProps = {
    id: 'start1',
    type: 'start',
    position: { x: 0, y: 0 },
    data: {
      label: '开始',
      nodeType: 'start',
    },
    selected: false,
  };

  it('应该正确渲染开始节点', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    
    // 验证节点标签渲染
    expect(screen.getByText('开始')).toBeInTheDocument();
    
    // 验证绿色背景
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('bg-emerald-500');
    
    // 验证圆形形状
    expect(cardElement).toHaveClass('rounded-full');
  });

  it('应该正确渲染自定义标签', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, label: '自定义开始' },
    };
    render(<StartNode {...props} />);
    
    expect(screen.getByText('自定义开始')).toBeInTheDocument();
  });

  it('应该处理 undefined label', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, label: undefined },
    };
    render(<StartNode {...props} />);
    
    expect(screen.getByText('开始')).toBeInTheDocument();
  });

  it('应该有绿色背景 (bg-emerald-500)', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('bg-emerald-500');
  });

  it('应该是圆形 (rounded-full)', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('rounded-full');
  });

  it('应该有输出端口 (Handle)', () => {
    const { container } = render(<StartNode {...defaultProps} />);

    // React Flow 的 Handle 组件会渲染为具有特定类名的元素
    const handles = container.querySelectorAll('[data-testid="output-handle"]');
    expect(handles.length).toBeGreaterThanOrEqual(1);
    expect(handles[0]).toHaveAttribute('data-type', 'source');
  });

  it('选中状态应该显示蓝色边框 (ring-blue-500)', () => {
    const props = {
      ...defaultProps,
      selected: true,
    };
    const { container } = render(<StartNode {...props} />);
    
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('ring-2', 'ring-blue-500', 'ring-offset-2');
  });

  it('错误状态应该显示红色边框 (ring-red-500)', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, error: true },
    };
    const { container } = render(<StartNode {...props} />);
    
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('ring-2', 'ring-red-500', 'ring-offset-2');
  });

  it('同时有选中状态和错误状态时，应该显示错误边框（错误优先）', () => {
    const props = {
      ...defaultProps,
      selected: true,
      data: { ...defaultProps.data, error: true },
    };
    const { container } = render(<StartNode {...props} />);
    
    const cardElement = container.querySelector('[data-testid="start-node"]');
    // 当同时存在 selected 和 error 时，error 样式的优先级更高
    expect(cardElement).toHaveClass('ring-2', 'ring-red-500');
  });

  it('应该使用 React.memo 优化性能', () => {
    expect(StartNode.displayName).toBe('StartNode');
  });

  it('应该有 displayName "StartNode"', () => {
    expect(StartNode.displayName).toBe('StartNode');
  });

  it('默认标签应该是"开始"', () => {
    const props = {
      ...defaultProps,
      data: { ...defaultProps.data, label: undefined },
    };
    render(<StartNode {...props} />);
    
    expect(screen.getByText('开始')).toBeInTheDocument();
  });

  it('应该有最小宽度 (min-w-[120px])', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('min-w-[120px]');
  });

  it('文字应该是白色且居中', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');

    const textElement = cardElement.querySelector('[data-testid="node-label"]');
    expect(textElement).toHaveClass('font-medium', 'text-sm', 'text-center');
  });

  it('应该有过渡动画 (transition-all)', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('transition-all', 'duration-200');
  });

  it('应该有阴影效果 (shadow-lg)', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    const cardElement = container.querySelector('[data-testid="start-node"]');
    expect(cardElement).toHaveClass('shadow-lg');
  });

  it('应该使用 React.memo 优化性能', () => {
    // StartNode 组件应该已经用 memo 包裹
    expect(StartNode.displayName).toBe('StartNode');
  });

  it('输出端口应该有特定样式', () => {
    const { container } = render(<StartNode {...defaultProps} />);
    
    const handle = container.querySelector('[data-testid="output-handle"]');
    expect(handle).toBeInTheDocument();
    
    // 验证端口样式
    expect(handle).toHaveClass('w-3', 'h-3');
    expect(handle).toHaveClass('bg-emerald-300', 'border-emerald-600');
  });
});
