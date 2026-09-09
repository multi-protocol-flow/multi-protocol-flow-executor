/**
 * VariableTextarea 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { VariableTextarea } from '@/components/ui/VariableTextarea';
import type { Variable } from '@/types/variable';

describe('VariableTextarea', () => {
  const mockVariables: Variable[] = [
    { name: 'auth_token', value: 'secret-token-123', sourceNodeUuid: 'node-1' },
    { name: 'user_id', value: 'u-42', sourceNodeUuid: 'node-1' },
  ];

  const mockOnValueChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders textarea with value', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value="initial body text"
        onValueChange={mockOnValueChange}
      />
    );
    expect(screen.getByDisplayValue('initial body text')).toBeInTheDocument();
  });
  it('renders correctly with react-hook-form register', () => {
    const { useForm } = require('react-hook-form');
    function FormComp() {
      const { register } = useForm({
        defaultValues: { body: 'hello from hook form' },
      });
      return <VariableTextarea variables={[]} {...register('body')} />;
    }
    render(<FormComp />);
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    expect(textarea.value).toBe('hello from hook form');
  });

  it('renders textarea with placeholder', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value=""
        placeholder="Enter payload..."
        onValueChange={mockOnValueChange}
      />
    );
    expect(screen.getByPlaceholderText('Enter payload...')).toBeInTheDocument();
  });

  it('shows variable selector button', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).toBeInTheDocument();
  });

  it('actively opens variable selector on typing {{', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: '{"token": "{{', selectionStart: 13 } });

    expect(screen.getByText('auth_token')).toBeInTheDocument();
  });

  it('actively opens variable selector on Ctrl+I shortcut', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value=""
        onValueChange={mockOnValueChange}
      />
    );
    const textarea = screen.getByRole('textbox');
    fireEvent.keyDown(textarea, { key: 'i', ctrlKey: true });

    expect(screen.getByText('auth_token')).toBeInTheDocument();
  });

  it('replaces trailing {{ when variable is selected', async () => {
    const user = userEvent.setup();
    let curVal = '{"token": "{{';
    render(
      <VariableTextarea
        variables={mockVariables}
        value={curVal}
        onValueChange={(val) => {
          curVal = val;
          mockOnValueChange(val);
        }}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]') as HTMLElement;
    await user.click(trigger);

    const varItem = screen.getByText('auth_token');
    await user.click(varItem);

    expect(mockOnValueChange).toHaveBeenCalledWith('{"token": "{{auth_token}}');
  });

  it('hides variable selector when disableVariableSelector is true', () => {
    render(
      <VariableTextarea
        variables={mockVariables}
        value=""
        disableVariableSelector={true}
        onValueChange={mockOnValueChange}
      />
    );
    const trigger = document.querySelector('[aria-haspopup="dialog"]');
    expect(trigger).not.toBeInTheDocument();
  });
});
