/**
 * DynamicVariablesHelpDialog and dynamicVariable types tests
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DynamicVariablesHelpDialog } from '@/components/dialogs/DynamicVariablesHelpDialog';
import {
  DYNAMIC_VARIABLES,
  isDynamicVariable,
  getDynamicVariablesByCategory,
} from '@/types/dynamicVariable';

describe('Dynamic Variables Metadata & Helpers', () => {
  it('defines 40+ dynamic variables', () => {
    expect(DYNAMIC_VARIABLES.length).toBeGreaterThanOrEqual(40);
  });

  it('correctly identifies dynamic variables with isDynamicVariable', () => {
    expect(isDynamicVariable('$guid')).toBe(true);
    expect(isDynamicVariable('$timestamp')).toBe(true);
    expect(isDynamicVariable('{{$guid}}')).toBe(true);
    expect(isDynamicVariable('  {{$timestamp}}  ')).toBe(true);
    expect(isDynamicVariable('$randomEmail')).toBe(true);
    expect(isDynamicVariable('$randomInt')).toBe(true);
    expect(isDynamicVariable('notADynamicVar')).toBe(false);
    expect(isDynamicVariable('{{custom_var}}')).toBe(false);
  });

  it('generates non-empty samples for all dynamic variables', () => {
    for (const item of DYNAMIC_VARIABLES) {
      const sample = item.generateSample();
      expect(sample).toBeDefined();
      expect(typeof sample).toBe('string');
      expect(sample.length).toBeGreaterThan(0);
    }
  });

  it('groups variables across all 8 categories', () => {
    const grouped = getDynamicVariablesByCategory();
    expect(grouped.common.length).toBeGreaterThan(0);
    expect(grouped.datetime.length).toBeGreaterThan(0);
    expect(grouped.numeric.length).toBeGreaterThan(0);
    expect(grouped.network.length).toBeGreaterThan(0);
    expect(grouped.identity.length).toBeGreaterThan(0);
    expect(grouped.contact.length).toBeGreaterThan(0);
    expect(grouped.text.length).toBeGreaterThan(0);
    expect(grouped.location.length).toBeGreaterThan(0);
  });
});

describe('DynamicVariablesHelpDialog', () => {
  it('renders dialog when open is true', () => {
    const onOpenChange = vi.fn();
    render(<DynamicVariablesHelpDialog open={true} onOpenChange={onOpenChange} />);

    expect(screen.getByText('{{$guid}}')).toBeInTheDocument();
    expect(screen.getByText('{{$timestamp}}')).toBeInTheDocument();
  });
});
