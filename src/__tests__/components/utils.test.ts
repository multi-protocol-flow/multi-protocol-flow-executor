import { describe, expect, it } from 'vitest';
import { createDefaultNodeData } from '@/components/canvas/utils';
import type { NodeType } from '@/types';

describe('createDefaultNodeData', () => {
  it('createDefaultNodeData_plugin_preserves_config', () => {
    // F2 (plugin architecture) widens NodeType to include plugin: prefixed types.
    // Cast keeps the test valid today (closed union) and after the widening lands.
    const pluginType = 'plugin:my-node' as NodeType;

    const data = createDefaultNodeData(pluginType);

    // Must NOT be silently rewritten to 'entry'
    expect(data.type).toBe('plugin:my-node');
  });

  it('createDefaultNodeData_known_type_uses_default_config', () => {
    const data = createDefaultNodeData('http');

    expect(data.type).toBe('http');
    if (data.type === 'http') {
      expect(data.url).toBe('');
      expect(data.method).toBe('get');
    }
  });
});
