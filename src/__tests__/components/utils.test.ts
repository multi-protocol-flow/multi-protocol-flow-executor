import { describe, expect, it } from 'vitest';
import { createDefaultNodeData } from '@/components/canvas/utils';

describe('createDefaultNodeData', () => {
  it('createDefaultNodeData_plugin_preserves_config', () => {
    // D9 (plugin architecture): 签名已宽化为 AnyNodeType —— 任意插件类型字符串无需 cast
    const pluginType = 'plugin:my-node';

    const data = createDefaultNodeData(pluginType);

    // Must NOT be silently rewritten to 'entry'
    expect(data.type).toBe('plugin:my-node');
  });

  it('createDefaultNodeData_arbitrary_string_type_keeps_type_and_defaults_shape', () => {
    // 任意字符串类型（插件 type_id，如 "vendor:widget"）→ 返回 { type, ...defaults } 形状
    const data = createDefaultNodeData('vendor:widget');

    expect(data).toBeDefined();
    expect(typeof data).toBe('object');
    // 默认分支保留自身 type，不被静默改写
    expect(data.type).toBe('vendor:widget');
    // 默认分支合并 default_config（无缓存时至少包含 type 键）
    expect(Object.keys(data)).toContain('type');
  });

  it('createDefaultNodeData_known_type_uses_default_config', () => {
    const data = createDefaultNodeData('http:request');

    expect(data.type).toBe('http:request');
    if (data.type === 'http:request') {
      expect(data.url).toBe('');
      expect(data.method).toBe('get');
    }
  });
});
