/**
 * LoopConfig 面板的不变量：保存时必须原样保留内联子图 `body`
 *
 * 面板只编辑标量字段，`body` 由画布下钻编辑；一旦面板在 `onSave` 时丢掉 `body`，
 * 循环体节点会被静默清空（用户可见的数据丢失）。
 */
import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LoopConfig } from '@/components/panels/LoopConfig';
import type { LoopConfig as LoopConfigType } from '@/types';

const baseConfig: LoopConfigType = {
  type: 'flow:loop',
  source: 'count',
  count: 3,
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
};

describe('LoopConfig', () => {
  it('save_keeps_body_and_applies_scalar_edits', () => {
    const saved: LoopConfigType[] = [];
    render(<LoopConfig data={baseConfig} onSave={(data) => saved.push(data)} />);

    fireEvent.change(screen.getByLabelText(/循环次数|Count|Iterations/i), {
      target: { value: '7' },
    });

    expect(saved.length).toBeGreaterThan(0);
    const last = saved[saved.length - 1];
    expect(last.count).toBe(7);
    expect(last.body).toEqual(baseConfig.body);
  });
});
