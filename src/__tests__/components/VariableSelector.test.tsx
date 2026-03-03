/**
 * VariableSelector 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { VariableSelector } from '@/components/ui/VariableSelector';
import type { Variable } from '@/types/variable';

describe('VariableSelector', () => {
  const mockVariables: Variable[] = [
    { name: 'api_base', value: 'https://api.example.com', sourceNodeUuid: 'entry-1' },
    { name: 'user_status', value: '{{status}}', sourceNodeUuid: 'http-1' },
  ];

  const mockNodeLabels = {
    'entry-1': '入口节点',
    'http-1': 'HTTP请求',
  };

  const mockOnSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders trigger button', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    const trigger = screen.getByRole('button');
    expect(trigger).toBeInTheDocument();
  });

  it('renders custom trigger children when provided', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
        triggerChildren={<span>自定义触发器</span>}
      />
    );
    expect(screen.getByText('自定义触发器')).toBeInTheDocument();
  });

  it('shows empty state when no variables', () => {
    render(
      <VariableSelector
        variables={[]}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('暂无变量')).toBeInTheDocument();
  });

  it('groups variables by source node', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('入口节点')).toBeInTheDocument();
    expect(screen.getByText('HTTP请求')).toBeInTheDocument();
  });

  it('displays node ID when no label provided', () => {
    const variables: Variable[] = [
      { name: 'test_var', value: 'test', sourceNodeUuid: 'abcdefgh12345678' },
    ];
    
    render(
      <VariableSelector
        variables={variables}
        nodeLabels={{}}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    // 应该显示截取的 UUID，格式为 "节点 xxx..."
    expect(screen.getByText(/节点 abcdefgh\.\.\./)).toBeInTheDocument();
  });

  it('displays variable names in list', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('displays variable syntax hint {{name}}', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('{{api_base}}')).toBeInTheDocument();
    expect(screen.getByText('{{user_status}}')).toBeInTheDocument();
  });

  it('displays variable description when available', () => {
    const variablesWithDesc: Variable[] = [
      { name: 'api_key', value: 'secret', sourceNodeUuid: 'entry-1', description: 'API密钥' },
    ];
    
    render(
      <VariableSelector
        variables={variablesWithDesc}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('API密钥')).toBeInTheDocument();
  });

  it('calls onSelect when variable is clicked', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnSelect).toHaveBeenCalledWith('api_base');
    expect(mockOnSelect).toHaveBeenCalledTimes(1);
  });

  it('filters variables by search query', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    fireEvent.change(searchInput, { target: { value: 'api' } });
    
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.queryByText('user_status')).not.toBeInTheDocument();
  });

  it('filters by description in search', () => {
    const variablesWithDesc: Variable[] = [
      { name: 'var1', value: 'v1', sourceNodeUuid: 'n1', description: '用户名称' },
      { name: 'var2', value: 'v2', sourceNodeUuid: 'n2', description: '用户年龄' },
    ];
    
    render(
      <VariableSelector
        variables={variablesWithDesc}
        nodeLabels={{ n1: '节点1', n2: '节点2' }}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    fireEvent.change(searchInput, { target: { value: '名称' } });
    
    expect(screen.getByText('var1')).toBeInTheDocument();
    expect(screen.queryByText('var2')).not.toBeInTheDocument();
  });

  it('shows empty state when search has no results', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    fireEvent.change(searchInput, { target: { value: 'nonexistent' } });
    
    expect(screen.getByText('暂无变量')).toBeInTheDocument();
  });

  it('clears search input when clear button clicked', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    fireEvent.change(searchInput, { target: { value: 'api' } });
    
    // 点击清除按钮 (X)
    const clearButtons = document.querySelectorAll('button');
    for (const btn of clearButtons) {
      const hasXIcon = btn.querySelector('.lucide-x');
      if (hasXIcon) {
        fireEvent.click(btn);
        break;
      }
    }
    
    expect(searchInput).toHaveValue('');
  });

  it('closes popover after selecting variable', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByPlaceholderText('搜索变量...')).toBeInTheDocument();
    
    fireEvent.click(screen.getByText('api_base'));
    
    expect(screen.queryByPlaceholderText('搜索变量...')).not.toBeInTheDocument();
  });

  it('supports controlled open state', () => {
    const onOpenChange = vi.fn();
    
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
        open={true}
        onOpenChange={onOpenChange}
      />
    );
    
    expect(screen.getByPlaceholderText('搜索变量...')).toBeInTheDocument();
  });

  it('displays custom placeholder text', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
        placeholder="查找变量..."
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByPlaceholderText('查找变量...')).toBeInTheDocument();
  });

  it('displays custom empty text', () => {
    render(
      <VariableSelector
        variables={[]}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
        emptyText="没有可用变量"
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText('没有可用变量')).toBeInTheDocument();
  });

  it('applies custom trigger class name', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
        triggerClassName="custom-trigger-class"
      />
    );
    
    const button = screen.getByRole('button');
    expect(button.className).toContain('custom-trigger-class');
  });

  it('shows bottom hint text', () => {
    render(
      <VariableSelector
        variables={mockVariables}
        nodeLabels={mockNodeLabels}
        onSelect={mockOnSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(screen.getByText(/点击变量名插入/)).toBeInTheDocument();
  });
});
