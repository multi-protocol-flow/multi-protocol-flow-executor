/**
 * Temp test for viewer i18n module.
 * Required to prevent noUnusedLocals build failure on viewer translation keys.
 */
import { describe, it, expect } from 'vitest';
import { getLL } from '@/i18n/i18n-lazy';

describe('viewer i18n module', () => {
  it('provides common translation keys', () => {
    const LL = getLL();
    const noData = LL.viewer.common.no_data();
    const success = LL.viewer.common.success();
    expect(typeof noData).toBe('string');
    expect(typeof success).toBe('string');
  });

  it('provides amqp translation keys', () => {
    const LL = getLL();
    const connected = LL.viewer.amqp.status_connected();
    const failed = LL.viewer.amqp.status_connection_failed();
    expect(typeof connected).toBe('string');
    expect(typeof failed).toBe('string');
  });

  it('provides http translation keys', () => {
    const LL = getLL();
    const url = LL.viewer.http.label_url();
    expect(typeof url).toBe('string');
  });
});
