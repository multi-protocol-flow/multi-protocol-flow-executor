import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { SubFlowConfig } from '@/components/panels/SubFlowConfig';
import { useFlowListStore, useFlowStore } from '@/stores';
import type { SubFlowConfig as SubFlowConfigType } from '@/types';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

const ROOT_FLOW = {
  uuid: 'flow-root',
  name: '根目录流程',
  updated_at: 1,
  folder_path: '',
};

const NESTED_FLOW = {
  uuid: 'flow-nested',
  name: '登录流程',
  updated_at: 2,
  folder_path: '认证/登录',
};

const SIBLING_FLOW = {
  uuid: 'flow-sibling',
  name: '登出流程',
  updated_at: 3,
  folder_path: '认证',
};

beforeEach(() => {
  useFlowStore.getState().reset();
  useFlowListStore.getState().reset();
  useFlowListStore.getState().setFlows([ROOT_FLOW, NESTED_FLOW, SIBLING_FLOW]);
});

describe('SubFlowConfig', () => {
  it('路径模式下渲染路径输入与浏览按钮', () => {
    const data: SubFlowConfigType = {
      type: 'sub_flow',
      target_flow_path: './subflows/login.json',
    };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);

    expect(screen.getByDisplayValue('./subflows/login.json')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /浏览/ })).toBeInTheDocument();
  });

  it('切换到已保存流程模式时清空路径，保证二选一', () => {
    const data: SubFlowConfigType = {
      type: 'sub_flow',
      target_flow_path: './subflows/login.json',
    };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);
    fireEvent.click(screen.getByRole('button', { name: '选择已保存流程' }));

    // 空串表达"该引用未使用"：顶层键无法删除（store 浅合并会填回旧值），空串是唯一可靠表达
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({ target_flow_path: '' })
    );
  });

  it('流程模式按文件夹分组展示，选择嵌套流程后调用 onSave', async () => {
    const data: SubFlowConfigType = { type: 'sub_flow' };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);

    // 打开树形下拉：根目录流程直接可见，嵌套流程初始折叠
    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));
    expect(screen.getByText('根目录流程')).toBeInTheDocument();
    expect(screen.queryByText('登录流程')).not.toBeInTheDocument();

    // 展开一级文件夹「认证」
    await act(async () => {
      fireEvent.click(screen.getByText('认证'));
    });

    expect(screen.getByText('登出流程')).toBeInTheDocument();
    expect(screen.queryByText('登录流程')).not.toBeInTheDocument();

    // 展开二级文件夹「登录」后选择目标流程
    await act(async () => {
      fireEvent.click(screen.getByText('登录'));
    });
    await act(async () => {
      fireEvent.click(screen.getByText('登录流程'));
    });

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({ target_flow_id: 'flow-nested', target_flow_path: '' })
    );
  });

  it('搜索时命中深层流程并自动展开其所在文件夹', async () => {
    const data: SubFlowConfigType = { type: 'sub_flow' };
    render(<SubFlowConfig data={data} onSave={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));
    await act(async () => {
      fireEvent.change(screen.getByPlaceholderText(/搜索流程名称/), {
        target: { value: '登录流程' },
      });
    });

    expect(screen.getByText('登录流程')).toBeInTheDocument();
    expect(screen.queryByText('根目录流程')).not.toBeInTheDocument();
    // 深层文件夹链在搜索态自动展开：二级文件夹「登录」可见
    expect(screen.getByText('登录')).toBeInTheDocument();
  });

  it('选择已保存流程后清除既有文件路径引用', async () => {
    const data: SubFlowConfigType = {
      type: 'sub_flow',
      target_flow_path: './legacy.json',
    };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);
    fireEvent.click(screen.getByRole('button', { name: '选择已保存流程' }));
    fireEvent.click(screen.getByRole('button', { name: /请选择目标流程/ }));
    await act(async () => {
      fireEvent.click(screen.getByText('根目录流程'));
    });

    expect(onSave).toHaveBeenLastCalledWith(
      expect.objectContaining({ target_flow_id: 'flow-root', target_flow_path: '' })
    );
  });

  it('支持添加与删除输入变量映射', () => {
    const data: SubFlowConfigType = { type: 'sub_flow' };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);
    fireEvent.click(screen.getByRole('button', { name: /添加输入变量/ }));

    expect(onSave).toHaveBeenLastCalledWith(
      expect.objectContaining({ variable_mappings: [{ source: '', target: '' }] })
    );
  });

  it('浏览按钮选择文件后写回绝对路径（并清除已保存流程引用）', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(open).mockResolvedValue('/abs/subflows/auth.mpf');

    const data: SubFlowConfigType = { type: 'sub_flow', target_flow_id: 'flow-root' };
    const onSave = vi.fn();

    render(<SubFlowConfig data={data} onSave={onSave} />);
    fireEvent.click(screen.getByRole('button', { name: '指定流程文件' }));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /浏览/ }));
    });

    expect(onSave).toHaveBeenLastCalledWith(
      expect.objectContaining({
        target_flow_path: '/abs/subflows/auth.mpf',
        target_flow_id: '',
      })
    );
  });
});
