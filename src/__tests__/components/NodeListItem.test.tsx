/**
 * NodeListItem 组件测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodeListItem } from '@/components/execution/NodeListItem';
import type { NodeExecutionReport } from '@/types';

describe('NodeListItem', () => {
  const mockReport: NodeExecutionReport = {
    node_uuid: 'node-001',
    node_type: 'http',
    node_name: '获取用户数据',
    start_time: Date.now() - 500,
    end_time: Date.now(),
    duration_ms: 500,
    status: 'success',
    input_data: { url: 'https://api.example.com/users' },
    output_data: { status: 200, data: [] },
  };

  const failedReport: NodeExecutionReport = {
    node_uuid: 'node-002',
    node_type: 'http',
    node_name: '失败的请求',
    start_time: Date.now() - 1000,
    end_time: Date.now(),
    duration_ms: 1000,
    status: 'failed',
    error: {
      error_kind: 'timeout',
      message: '请求超时',
    },
  };

  it('正确渲染成功的节点', () => {
    const onClick = vi.fn();
    render(<NodeListItem report={mockReport} isSelected={false} onClick={onClick} />);

    // 检查节点名称
    expect(screen.getByText('获取用户数据')).toBeDefined();

    // 检查类型标签
    expect(screen.getByText('HTTP 请求')).toBeDefined();

    // 检查执行时间
    expect(screen.getByText('500ms')).toBeDefined();
  });

  it('正确渲染失败的节点', () => {
    const onClick = vi.fn();
    render(<NodeListItem report={failedReport} isSelected={false} onClick={onClick} />);

    // 检查节点名称
    expect(screen.getByText('失败的请求')).toBeDefined();

    // 检查执行时间
    expect(screen.getByText('1000ms')).toBeDefined();
  });

  it('点击时调用 onClick 回调', () => {
    const onClick = vi.fn();
    render(<NodeListItem report={mockReport} isSelected={false} onClick={onClick} />);

    const button = screen.getByRole('button');
    fireEvent.click(button);

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('选中状态时有高亮样式', () => {
    const onClick = vi.fn();
    const { container } = render(
      <NodeListItem report={mockReport} isSelected={true} onClick={onClick} />
    );

    const button = container.querySelector('button');
    expect(button?.className).toContain('bg-accent');
  });

  it('非选中状态时有 hover 样式', () => {
    const onClick = vi.fn();
    const { container } = render(
      <NodeListItem report={mockReport} isSelected={false} onClick={onClick} />
    );

    const button = container.querySelector('button');
    expect(button?.className).toContain('hover:bg-muted/50');
    expect(button?.className).not.toContain('bg-accent');
  });

  it('正确显示各节点类型标签', () => {
    const nodeTypes: Array<{ type: string; label: string }> = [
      { type: 'entry', label: '入口' },
      { type: 'http', label: 'HTTP 请求' },
      { type: 'conditional', label: '条件分支' },
      { type: 'end', label: '终止' },
    ];

    nodeTypes.forEach(({ type, label }) => {
      const report: NodeExecutionReport = {
        ...mockReport,
        node_uuid: `node-${type}`,
        node_type: type,
        node_name: `${label}节点`,
      };

      const onClick = vi.fn();
      render(<NodeListItem report={report} isSelected={false} onClick={onClick} />);

      expect(screen.getByText(label)).toBeDefined();
    });
  });

  it('未知节点类型显示原始类型名', () => {
    const unknownReport: NodeExecutionReport = {
      ...mockReport,
      node_type: 'unknown_type',
    };

    const onClick = vi.fn();
    render(<NodeListItem report={unknownReport} isSelected={false} onClick={onClick} />);

    expect(screen.getByText('unknown_type')).toBeDefined();
  });
});
