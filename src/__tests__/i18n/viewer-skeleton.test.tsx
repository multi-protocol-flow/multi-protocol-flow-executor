/**
 * Temp test for viewer i18n skeleton.
 * Required to prevent noUnusedLocals build failure on viewer translation keys.
 * Will be expanded in Task 22.
 */
import { describe, it, expect } from 'vitest';
import { getLL } from '@/i18n/i18n-lazy';

describe('viewer i18n skeleton', () => {
  it('provides placeholder and timestamp keys', () => {
    const LL = getLL();
    // Reference viewer keys so TypeScript doesn't flag them as unused
    const placeholder = LL.viewer.placeholder();
    const timestamp = LL.viewer.timestamp();
    expect(typeof placeholder).toBe('string');
    expect(typeof timestamp).toBe('string');
  });
});
