/**
 * EntryConfig 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EntryConfig } from '@/components/panels/EntryConfig';
import type { EntryConfig as EntryConfigType } from '@/types';

describe('EntryConfig - 样式测试', () => {
  const mockOnSave = vi.fn();
  const defaultData: EntryConfigType = {
    type: 'entry',
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('应该渲染入口节点说明卡片', () => {
    render(<EntryConfig data={defaultData} onSave={mockOnSave} />);

    // 验证说明卡片标题存在
    expect(screen.getByText('入口节点')).toBeDefined();
  });

  it('说明卡片内容应该左对齐（没有图标缩进）', () => {
    const dataWithVariables: EntryConfigType = {
      type: 'entry',
      initial_variables: [
        { name: 'api_base', value: 'https://api.example.com', description: 'API基础URL' }
      ]
    };

    render(<EntryConfig data={dataWithVariables} onSave={mockOnSave} />);

    // 验证说明卡片存在
    const card = screen.getByText('入口节点').closest('div');
    expect(card).toBeDefined();
    
    // 验证标题和图标在同一个容器中（左对齐布局）
    const titleElement = screen.getByText('入口节点');
    expect(titleElement.className).toContain('flex items-center gap-2');
  });

  it('初始变量删除按钮应该使用紫色样式', () => {
    const dataWithVariables: EntryConfigType = {
      type: 'entry',
      initial_variables: [
        { name: 'api_base', value: 'https://api.example.com', description: 'API基础URL' }
      ]
    };

    render(<EntryConfig data={dataWithVariables} onSave={mockOnSave} />);

    // 查找删除按钮（通过 Trash2 图标的父按钮）
    const deleteButtons = document.querySelectorAll('button');
    let deleteButton: HTMLButtonElement | null = null;
    
    for (const btn of deleteButtons) {
      const hasTrashIcon = btn.querySelector('.lucide-trash-2');
      if (hasTrashIcon) {
        deleteButton = btn as HTMLButtonElement;
        break;
      }
    }

    expect(deleteButton).toBeDefined();
    
    // 验证删除按钮包含紫色 hover 样式
    expect(deleteButton?.className).toContain('hover:text-violet-500');
  });

  it('应该显示初始变量列表', () => {
    const dataWithVariables: EntryConfigType = {
      type: 'entry',
      initial_variables: [
        { name: 'api_base', value: 'https://api.example.com', description: 'API基础URL' }
      ]
    };

    render(<EntryConfig data={dataWithVariables} onSave={mockOnSave} />);

    // 验证变量名显示
    expect(screen.getByDisplayValue('api_base')).toBeDefined();
    // 验证变量值显示
    expect(screen.getByDisplayValue('https://api.example.com')).toBeDefined();
    // 验证变量描述显示
    expect(screen.getByDisplayValue('API基础URL')).toBeDefined();
  });

  it('应该显示添加变量按钮', () => {
    render(<EntryConfig data={defaultData} onSave={mockOnSave} />);

    // 验证添加变量按钮存在（使用 role 和 name 定位）
    const addButton = screen.getByRole('button', { name: /添加变量/i });
    expect(addButton).toBeDefined();
  });

  it('变量名提示不应该包含图标', () => {
    const dataWithVariables: EntryConfigType = {
      type: 'entry',
      initial_variables: [
        { name: 'api_base', value: 'https://api.example.com', description: 'API基础URL' }
      ]
    };

    render(<EntryConfig data={dataWithVariables} onSave={mockOnSave} />);

    // 验证提示文字存在
    const tipText = screen.getByText(/变量名只能包含字母、数字和下划线/);
    expect(tipText).toBeDefined();

    // 验证提示容器不包含 Info 图标（通过检查父容器没有 lucide-info 类）
    const tipContainer = tipText.closest('div');
    expect(tipContainer).toBeDefined();
    const infoIcon = tipContainer?.querySelector('.lucide-info');
    expect(infoIcon).toBeNull();
  });


});
