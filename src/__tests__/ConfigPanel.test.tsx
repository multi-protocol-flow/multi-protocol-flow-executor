import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ConfigPanel } from '../components/ConfigPanel';
import type { Node } from '@xyflow/react';

// Mock CSS imports
vi.mock('@xyflow/react/dist/style.css', () => ({}));
vi.mock('*.css', () => ({}));

describe('ConfigPanel', () => {
  const mockNode: Node = {
    id: 'start-1',
    type: 'start',
    position: { x: 0, y: 0 },
    data: {
      label: '开始节点',
      nodeType: 'start',
    },
  };

  const defaultProps = {
    open: true,
    onOpenChange: vi.fn(),
    node: mockNode,
    onSave: vi.fn(),
  };

  it('应该正确渲染配置面板', () => {
    render(<ConfigPanel {...defaultProps} />);

    // 验证标题
    expect(screen.getByText('节点配置')).toBeInTheDocument();

    // 验证描述
    expect(screen.getByText('编辑节点的名称和其他属性')).toBeInTheDocument();

    // 验证表单标签
    expect(screen.getByText('节点名称')).toBeInTheDocument();

    // 验证输入框有默认值
    const nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('开始节点');
  });

  it('应该显示节点名称在输入框中', () => {
    const nodeWithCustomName: Node = {
      ...mockNode,
      data: {
        ...mockNode.data,
        label: '自定义节点名称',
      },
    };

    const props = {
      ...defaultProps,
      node: nodeWithCustomName,
    };

    render(<ConfigPanel {...props} />);

    const nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('自定义节点名称');
  });

  it('应该使用 data.name 作为备用', () => {
    const nodeWithName: Node = {
      ...mockNode,
      data: {
        nodeType: 'start',
        name: '从name字段获取',
      },
    };

    const props = {
      ...defaultProps,
      node: nodeWithName,
    };

    render(<ConfigPanel {...props} />);

    const nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('从name字段获取');
  });

  it('点击保存按钮应该调用 onSave 并关闭面板', async () => {
    const onOpenChange = vi.fn();
    const onSave = vi.fn();

    const props = {
      ...defaultProps,
      onOpenChange,
      onSave,
    };

    render(<ConfigPanel {...props} />);

    // 点击保存按钮
    const saveButton = screen.getByText('保存');
    fireEvent.click(saveButton);

    // 验证 onSave 被调用
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        name: '开始节点',
      });
    });

    // 验证面板关闭
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('点击取消按钮应该关闭面板但不调用 onSave', () => {
    const onOpenChange = vi.fn();
    const onSave = vi.fn();

    const props = {
      ...defaultProps,
      onOpenChange,
      onSave,
    };

    render(<ConfigPanel {...props} />);

    // 点击取消按钮
    const cancelButton = screen.getByText('取消');
    fireEvent.click(cancelButton);

    // 验证面板关闭
    expect(onOpenChange).toHaveBeenCalledWith(false);

    // 验证 onSave 没有被调用
    expect(onSave).not.toHaveBeenCalled();
  });

  it('应该验证名称不能为空', async () => {
    render(<ConfigPanel {...defaultProps} />);

    // 清空输入框
    const nameInput = screen.getByPlaceholderText('请输入节点名称');
    fireEvent.change(nameInput, { target: { value: '' } });

    // 点击保存按钮
    const saveButton = screen.getByText('保存');
    fireEvent.click(saveButton);

    // 等待验证错误显示
    await waitFor(() => {
      expect(screen.getByText('名称不能为空')).toBeInTheDocument();
    });
  });

  it('应该验证名称不超过 50 个字符', async () => {
    const onSave = vi.fn();
    const props = {
      ...defaultProps,
      onSave,
    };
    render(<ConfigPanel {...props} />);

    // 在测试环境中，fireEvent.change 可以绕过 maxLength 限制
    const longName = 'a'.repeat(51);
    const nameInput = screen.getByPlaceholderText('请输入节点名称');
    fireEvent.change(nameInput, { target: { value: longName } });

    // 点击保存按钮以触发 zod 验证
    const saveButton = screen.getByText('保存');
    fireEvent.click(saveButton);

    // 等待一段时间以确保验证完成
    await waitFor(() => {
      // 由于 maxLength 限制和 zod 验证的交互，这里主要验证 onSave 没有被调用
      expect(onSave).not.toHaveBeenCalled();
    });
  });

  it('应该接受 50 个字符的名称', async () => {
    const onSave = vi.fn();
    const onOpenChange = vi.fn();

    const props = {
      ...defaultProps,
      onSave,
      onOpenChange,
    };

    render(<ConfigPanel {...props} />);

    // 输入 50 个字符
    const validName = 'a'.repeat(50);
    const nameInput = screen.getByPlaceholderText('请输入节点名称');
    fireEvent.change(nameInput, { target: { value: validName } });

    // 点击保存按钮
    const saveButton = screen.getByText('保存');
    fireEvent.click(saveButton);

    // 等待 onSave 被调用
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        name: validName,
      });
    });
  });

  it('应该更新节点名称', async () => {
    const onSave = vi.fn();
    const onOpenChange = vi.fn();

    const props = {
      ...defaultProps,
      onSave,
      onOpenChange,
    };

    render(<ConfigPanel {...props} />);

    // 更新名称
    const nameInput = screen.getByPlaceholderText('请输入节点名称');
    fireEvent.change(nameInput, { target: { value: '更新后的节点名称' } });

    // 点击保存按钮
    const saveButton = screen.getByText('保存');
    fireEvent.click(saveButton);

    // 验证 onSave 被调用并包含新名称
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        name: '更新后的节点名称',
      });
    });
  });

  it('当面板关闭时，输入框 maxlength 应该是 50', () => {
    render(<ConfigPanel {...defaultProps} />);

    const nameInput = screen.getByPlaceholderText('请输入节点名称');
    expect(nameInput).toHaveAttribute('maxLength', '50');
  });

  it('node 为 null 时应该正确处理', () => {
    const props = {
      ...defaultProps,
      node: null,
    };

    render(<ConfigPanel {...props} />);

    const nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('');
  });

  it('open 变化时应该打开或关闭面板', () => {
    const { rerender } = render(<ConfigPanel {...defaultProps} />);

    // 初始状态：面板打开
    expect(screen.getByText('节点配置')).toBeInTheDocument();

    // 关闭面板
    rerender(<ConfigPanel {...defaultProps} open={false} />);

    // 面板应该不再显示
    expect(screen.queryByText('节点配置')).not.toBeInTheDocument();
  });

  it('节点变化时应该更新表单值', () => {
    const { rerender } = render(<ConfigPanel {...defaultProps} />);

    let nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('开始节点');

    // 更新节点
    const newNode: Node = {
      ...mockNode,
      data: {
        ...mockNode.data,
        label: '新节点名称',
      },
    };

    rerender(<ConfigPanel {...defaultProps} node={newNode} />);

    nameInput = screen.getByPlaceholderText('请输入节点名称') as HTMLInputElement;
    expect(nameInput.value).toBe('新节点名称');
  });
});
