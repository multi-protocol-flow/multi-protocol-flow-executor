import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FlowTreeSelect } from '@/components/ui/FlowTreeSelect';

const ITEMS = [
  { uuid: 'f-root', name: '根目录流程', folderPath: '' },
  { uuid: 'f-auth', name: '登录流程', folderPath: '认证' },
  { uuid: 'f-auth-2', name: '登出流程', folderPath: '认证' },
  { uuid: 'f-deep', name: '令牌刷新', folderPath: '认证/令牌' },
];

describe('FlowTreeSelect', () => {
  it('未选择时展示占位文本，选择后展示流程名与文件夹路径', () => {
    const { rerender } = render(
      <FlowTreeSelect value="" onValueChange={vi.fn()} items={ITEMS} placeholder="请选择目标流程" />
    );
    expect(screen.getByRole('button', { name: /请选择目标流程/ })).toBeInTheDocument();

    rerender(
      <FlowTreeSelect value="f-deep" onValueChange={vi.fn()} items={ITEMS} placeholder="请选择目标流程" />
    );
    expect(screen.getByRole('button', { name: /令牌刷新/ })).toBeInTheDocument();
    expect(screen.getByText('认证/令牌')).toBeInTheDocument();
  });

  it('文件夹优先于根目录流程，与侧边栏流程列表排序一致', () => {
    render(
      <FlowTreeSelect value="" onValueChange={vi.fn()} items={ITEMS} placeholder="请选择目标流程" />
    );
    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));

    const folderRow = screen.getByText('认证');
    const rootFlowRow = screen.getByText('根目录流程');
    // 文件夹行在根目录流程之前（DOCUMENT_POSITION_FOLLOWING = 4）
    expect(folderRow.compareDocumentPosition(rootFlowRow) & Node.DOCUMENT_POSITION_FOLLOWING).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
  });

  it('按层级展开文件夹后可选中最深层流程', async () => {
    const onValueChange = vi.fn();
    render(
      <FlowTreeSelect value="" onValueChange={onValueChange} items={ITEMS} placeholder="请选择目标流程" />
    );

    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));
    expect(screen.getByText('根目录流程')).toBeInTheDocument();
    expect(screen.queryByText('登录流程')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('认证'));
    expect(screen.getByText('登录流程')).toBeInTheDocument();
    expect(screen.queryByText('令牌刷新')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('令牌'));
    fireEvent.click(screen.getByText('令牌刷新'));

    expect(onValueChange).toHaveBeenCalledWith('f-deep');
    // 选择后下拉关闭
    expect(screen.queryByPlaceholderText(/搜索流程名称/)).not.toBeInTheDocument();
  });

  it('清空按钮回调空值且不会顺带展开下拉', () => {
    const onValueChange = vi.fn();
    render(
      <FlowTreeSelect
        value="f-root"
        onValueChange={onValueChange}
        items={ITEMS}
        placeholder="请选择目标流程"
        clearLabel="清除"
      />
    );

    fireEvent.click(screen.getByTitle('清除'));

    expect(onValueChange).toHaveBeenCalledWith('');
    expect(screen.queryByPlaceholderText(/搜索流程名称/)).not.toBeInTheDocument();
  });

  it('列表为空时展示空状态文本', () => {
    render(
      <FlowTreeSelect
        value=""
        onValueChange={vi.fn()}
        items={[]}
        placeholder="请选择目标流程"
        emptyText="暂无已保存的流程"
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));
    expect(screen.getByText('暂无已保存的流程')).toBeInTheDocument();
  });
});
