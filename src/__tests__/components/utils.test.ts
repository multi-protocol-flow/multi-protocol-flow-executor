import { describe, expect, it } from 'vitest';
import { convertFlowToReactFlow, createDefaultNodeData } from '@/components/canvas/utils';
import type { Flow } from '@/types';

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

  it('createDefaultNodeData_loop_has_body_and_report', () => {
    const data = createDefaultNodeData('flow:loop');

    expect(data.type).toBe('flow:loop');
    if (data.type === 'flow:loop') {
      expect(data.source).toBe('count');
      expect(data.body).toEqual({ nodes: [], connections: [] });
      expect(data.report).toEqual({ detail_iterations: 20, always_keep_failed: true });
      expect(data.max_iterations).toBe(100);
    }
  });

  it('createDefaultNodeData_delay_has_duration_and_jitter', () => {
    const data = createDefaultNodeData('flow:delay');

    expect(data.type).toBe('flow:delay');
    if (data.type === 'flow:delay') {
      expect(data.duration_ms).toBe('1000');
      expect(data.jitter_ms).toBe(0);
    }
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

describe('convertFlowToReactFlow loop scope', () => {
  const flow: Flow = {
    uuid: 'f1',
    name: 'scope',
    nodes: [
      {
        uuid: 'loop-1',
        type: 'flow:loop',
        name: 'Poll',
        position: { x: 0, y: 0 },
        data: {
          type: 'flow:loop',
          source: 'count',
          count: 2,
          max_iterations: 100,
          concurrency: 1,
          break_on_error: true,
          report: { detail_iterations: 20, always_keep_failed: true },
          body: {
            nodes: [
              {
                uuid: 'body-1',
                type: 'flow:delay',
                name: 'body delay',
                position: { x: 0, y: 0 },
                data: { type: 'flow:delay', duration_ms: '10', jitter_ms: 0 },
              },
            ],
            connections: [],
          },
        },
      },
    ],
    connections: [],
  };

  it('loopPath 空时渲染阶段子图，非空时渲染 loop body', () => {
    expect(convertFlowToReactFlow(flow, 'main', []).nodes.map((n) => n.id)).toEqual(['loop-1']);
    expect(convertFlowToReactFlow(flow, 'main', ['loop-1']).nodes.map((n) => n.id)).toEqual([
      'body-1',
    ]);
    // 失效路径回落阶段层
    expect(convertFlowToReactFlow(flow, 'main', ['missing']).nodes.map((n) => n.id)).toEqual([
      'loop-1',
    ]);
  });
});
