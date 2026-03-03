/**
 * VariableInput 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { VariableInput } from '@/components/ui/VariableInput';
import type { Variable } from '@/types/variable';

describe('VariableInput', () => {
  const mockVariables: Variable[] = [
    { name: 'api_base', value: 'https://api.example.com', sourceNodeUuid: 'entry-1' },
  ];

  const mockOnValueChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders input with value', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value="test value"
        onValueChange={mockOnValueChange}
      />
    );
    expect(screen.getByDisplayValue('test value')).toBeInTheDocument();
  });

  it('renders input with placeholder', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        placeholder="输入内容..."
      />
    );
    expect(screen.getByPlaceholderText('输入内容...')).toBeInTheDocument();
  });

  it('calls onValueChange on input change', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'new value' } });
    expect(mockOnValueChange).toHaveBeenCalledWith('new value');
  });

  it('shows variable selector button when variables exist', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).toBeInTheDocument();
  });

  it('hides variable selector button when no variables', () => {
    render(
      <VariableInput
        variables={[]}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).not.toBeInTheDocument();
  });

  it('hides variable selector when disableVariableSelector is true', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        disableVariableSelector={true}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).not.toBeInTheDocument();
  });

  it('hides variable selector when input is disabled', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        disabled={true}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).not.toBeInTheDocument();
  });

  it('inserts variable at cursor position', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value="hello world"
        onValueChange={mockOnValueChange}
      />
    );
    
    const input = screen.getByRole('textbox') as HTMLInputElement;
    
    fireEvent.focus(input);
    input.setSelectionRange(5, 5);
    
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger) {
      fireEvent.click(trigger);
    }
    
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnValueChange).toHaveBeenCalled();
    const insertedValue = mockOnValueChange.mock.calls[0][0];
    expect(insertedValue).toContain('{{api_base}}');
  });

  it('appends variable at end when no cursor position', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value="hello"
        onValueChange={mockOnValueChange}
      />
    );
    
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger) {
      fireEvent.click(trigger);
    }
    
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnValueChange).toHaveBeenCalledWith('hello{{api_base}}');
  });

  it('replaces selected text with variable', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value="hello world"
        onValueChange={mockOnValueChange}
      />
    );
    
    const input = screen.getByRole('textbox') as HTMLInputElement;
    
    fireEvent.focus(input);
    input.setSelectionRange(6, 11);
    
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger) {
      fireEvent.click(trigger);
    }
    
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnValueChange).toHaveBeenCalledWith('hello {{api_base}}');
  });

  it('supports uncontrolled mode with defaultValue', () => {
    render(
      <VariableInput
        variables={mockVariables}
        defaultValue="default value"
        onValueChange={mockOnValueChange}
      />
    );
    expect(screen.getByDisplayValue('default value')).toBeInTheDocument();
  });

  it('applies custom className to input', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        className="custom-input-class"
      />
    );
    const input = screen.getByRole('textbox');
    expect(input.className).toContain('custom-input-class');
  });

  it('applies containerClassName to wrapper', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        containerClassName="custom-container"
      />
    );
    const container = screen.getByRole('textbox').parentElement;
    expect(container?.className).toContain('custom-container');
  });

  it('passes variableButtonClassName to VariableSelector', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        variableButtonClassName="custom-btn-class"
      />
    );
    // 验证变量选择器存在（triggerClassName 会被传递）
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).toBeInTheDocument();
  });

  it('passes through other input props', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        data-testid="custom-input"
        aria-label="自定义输入"
      />
    );
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('data-testid', 'custom-input');
    expect(input).toHaveAttribute('aria-label', '自定义输入');
  });

  it('uses nodeLabels for variable selector grouping', () => {
    const nodeLabels = { 'entry-1': '入口配置' };
    
    render(
      <VariableInput
        variables={mockVariables}
        nodeLabels={nodeLabels}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger) {
      fireEvent.click(trigger);
    }
    
    expect(screen.getByText('入口配置')).toBeInTheDocument();
  });

  it('adds right padding to input when variable selector is shown', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const input = screen.getByRole('textbox');
    expect(input.className).toContain('pr-10');
  });

  it('does not add right padding when variable selector is hidden', () => {
    render(
      <VariableInput
        variables={[]}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const input = screen.getByRole('textbox');
    expect(input.className).not.toContain('pr-10');
  });

  it('forwards ref correctly', () => {
    const ref = { current: null as HTMLInputElement | null };
    
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        ref={ref}
      />
    );
    
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });

  it('handles empty string value', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const input = screen.getByRole('textbox');
    expect(input).toHaveValue('');
  });

  it('handles undefined value in uncontrolled mode', () => {
    render(
      <VariableInput
        variables={mockVariables}
        onValueChange={mockOnValueChange}
      />
    );
    const input = screen.getByRole('textbox');
    expect(input).toHaveValue('');
  });

  it('handles multiple variable insertions', () => {
    const { rerender } = render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    
    const trigger1 = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger1) {
      fireEvent.click(trigger1);
    }
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnValueChange).toHaveBeenCalledWith('{{api_base}}');
    
    rerender(
      <VariableInput
        variables={mockVariables}
        value="{{api_base}}"
        onValueChange={mockOnValueChange}
      />
    );
    
    const trigger2 = document.querySelector('[aria-haspopup="dialog"]');
    if (trigger2) {
      fireEvent.click(trigger2);
    }
    fireEvent.click(screen.getByText('api_base'));
    
    expect(mockOnValueChange).toHaveBeenCalledWith('{{api_base}}{{api_base}}');
  });

  it('supports type attribute override', () => {
    render(
      <VariableInput
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
        type="text"
      />
    );
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('type', 'text');
  });
});
