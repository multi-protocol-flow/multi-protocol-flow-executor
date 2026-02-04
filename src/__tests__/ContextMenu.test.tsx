import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ContextMenu } from '../components/ContextMenu';

// Mock CSS imports
vi.mock('@xyflow/react/dist/style.css', () => ({}));
vi.mock('*.css', () => ({}));

describe('ContextMenu', () => {
  const defaultProps = {
    children: <button>触发菜单</button>,
    onRename: vi.fn(),
    onCopy: vi.fn(),
    onViewInfo: vi.fn(),
  };

  it('应该正确渲染菜单组件', () => {
    render(<ContextMenu {...defaultProps} />);

    // 验证触发元素存在
    expect(screen.getByText('触发菜单')).toBeInTheDocument();
  });

  it('应该显示所有菜单项', async () => {
    const user = userEvent.setup();
    render(<ContextMenu {...defaultProps} />);

    // 点击触发元素以打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 等待菜单项渲染（使用 waitFor 等待 Portal 渲染）
    await waitFor(() => {
      expect(screen.getByText('重命名')).toBeInTheDocument();
      expect(screen.getByText('复制')).toBeInTheDocument();
      expect(screen.getByText('查看信息')).toBeInTheDocument();
    });
  });

  it('应该在点击重命名时调用 onRename 回调', async () => {
    const user = userEvent.setup();
    render(<ContextMenu {...defaultProps} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 等待菜单项渲染并点击
    await waitFor(async () => {
      const renameItem = screen.getByText('重命名');
      await user.click(renameItem);
    });

    // 验证回调被调用
    expect(defaultProps.onRename).toHaveBeenCalledTimes(1);
  });

  it('应该在点击复制时调用 onCopy 回调', async () => {
    const user = userEvent.setup();
    render(<ContextMenu {...defaultProps} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 等待菜单项渲染并点击
    await waitFor(async () => {
      const copyItem = screen.getByText('复制');
      await user.click(copyItem);
    });

    // 验证回调被调用
    expect(defaultProps.onCopy).toHaveBeenCalledTimes(1);
  });

  it('应该在点击查看信息时调用 onViewInfo 回调', async () => {
    const user = userEvent.setup();
    render(<ContextMenu {...defaultProps} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 等待菜单项渲染并点击
    await waitFor(async () => {
      const viewInfoItem = screen.getByText('查看信息');
      await user.click(viewInfoItem);
    });

    // 验证回调被调用
    expect(defaultProps.onViewInfo).toHaveBeenCalledTimes(1);
  });

  it('应该在未提供 onRename 回调时不显示重命名菜单项', async () => {
    const user = userEvent.setup();
    const propsWithoutRename = {
      ...defaultProps,
      onRename: undefined,
    };

    render(<ContextMenu {...propsWithoutRename} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 验证重命名菜单项不存在
    await waitFor(() => {
      expect(screen.queryByText('重命名')).not.toBeInTheDocument();

      // 验证其他菜单项仍然存在
      expect(screen.getByText('复制')).toBeInTheDocument();
      expect(screen.getByText('查看信息')).toBeInTheDocument();
    });
  });

  it('应该在未提供 onCopy 回调时不显示复制菜单项', async () => {
    const user = userEvent.setup();
    const propsWithoutCopy = {
      ...defaultProps,
      onCopy: undefined,
    };

    render(<ContextMenu {...propsWithoutCopy} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 验证复制菜单项不存在
    await waitFor(() => {
      expect(screen.queryByText('复制')).not.toBeInTheDocument();

      // 验证其他菜单项仍然存在
      expect(screen.getByText('重命名')).toBeInTheDocument();
      expect(screen.getByText('查看信息')).toBeInTheDocument();
    });
  });

  it('应该在未提供 onViewInfo 回调时不显示查看信息菜单项', async () => {
    const user = userEvent.setup();
    const propsWithoutViewInfo = {
      ...defaultProps,
      onViewInfo: undefined,
    };

    render(<ContextMenu {...propsWithoutViewInfo} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 验证查看信息菜单项不存在
    await waitFor(() => {
      expect(screen.queryByText('查看信息')).not.toBeInTheDocument();

      // 验证其他菜单项仍然存在
      expect(screen.getByText('重命名')).toBeInTheDocument();
      expect(screen.getByText('复制')).toBeInTheDocument();
    });
  });

  it('应该支持自定义 className', () => {
    render(<ContextMenu {...defaultProps} className="custom-class" />);

    // 验证触发元素存在
    const trigger = screen.getByText('触发菜单');
    expect(trigger).toBeInTheDocument();
  });

  it('应该正确处理所有回调都未提供的情况', async () => {
    const user = userEvent.setup();
    const propsWithNoCallbacks = {
      children: <button>触发菜单</button>,
      onRename: undefined,
      onCopy: undefined,
      onViewInfo: undefined,
    };

    render(<ContextMenu {...propsWithNoCallbacks} />);

    // 打开菜单
    const trigger = screen.getByText('触发菜单');
    await user.click(trigger);

    // 验证所有菜单项都不存在
    await waitFor(() => {
      expect(screen.queryByText('重命名')).not.toBeInTheDocument();
      expect(screen.queryByText('复制')).not.toBeInTheDocument();
      expect(screen.queryByText('查看信息')).not.toBeInTheDocument();
    });
  });
});
