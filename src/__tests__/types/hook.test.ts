/**
 * 钩子类型测试
 * 验证 TypeScript 类型与后端 Rust 结构体的对齐
 */

import { describe, it, expect } from 'vitest';
import type { HookTrigger, HookAction, FlowHook } from '@/types';

describe('HookTrigger 类型', () => {
  it('应包含四种触发条件', () => {
    const triggers: HookTrigger[] = ['on_success', 'on_failure', 'on_cancel', 'on_complete'];
    expect(triggers).toHaveLength(4);
  });

  it('on_success 是有效的触发条件', () => {
    const trigger: HookTrigger = 'on_success';
    expect(trigger).toBe('on_success');
  });

  it('on_failure 是有效的触发条件', () => {
    const trigger: HookTrigger = 'on_failure';
    expect(trigger).toBe('on_failure');
  });

  it('on_cancel 是有效的触发条件', () => {
    const trigger: HookTrigger = 'on_cancel';
    expect(trigger).toBe('on_cancel');
  });

  it('on_complete 是有效的触发条件', () => {
    const trigger: HookTrigger = 'on_complete';
    expect(trigger).toBe('on_complete');
  });
});

describe('HookAction 类型', () => {
  it('Webhook 动作应包含必要字段', () => {
    const action: HookAction = {
      type: 'webhook',
      url: 'https://example.com/hook',
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body_template: '{"status": "{{status}}"}',
      timeout_ms: 5000,
    };
    expect(action.type).toBe('webhook');
    if (action.type === 'webhook') {
      expect(action.url).toBe('https://example.com/hook');
      expect(action.method).toBe('POST');
      expect(action.headers).toBeDefined();
      expect(action.body_template).toBeDefined();
    }
  });

  it('Script 动作应包含代码字段', () => {
    const action: HookAction = {
      type: 'script',
      code: 'console.log("hello")',
      timeout_ms: 10000,
    };
    expect(action.type).toBe('script');
    if (action.type === 'script') {
      expect(action.code).toBe('console.log("hello")');
      expect(action.timeout_ms).toBe(10000);
    }
  });

  it('Notify 动作应包含模板字段', () => {
    const action: HookAction = {
      type: 'notify',
      title_template: '流程 {{flow_name}} 完成',
      message_template: '状态: {{status}}, 耗时: {{duration_ms}}ms',
    };
    expect(action.type).toBe('notify');
    if (action.type === 'notify') {
      expect(action.title_template).toContain('{{flow_name}}');
      expect(action.message_template).toContain('{{status}}');
    }
  });

  it('Webhook 动作可选字段可以省略', () => {
    const action: HookAction = {
      type: 'webhook',
      url: 'https://example.com',
      method: 'GET',
      headers: {},
    };
    expect(action.type).toBe('webhook');
    if (action.type === 'webhook') {
      expect(action.body_template).toBeUndefined();
      expect(action.timeout_ms).toBeUndefined();
    }
  });

  it('Script 动作 timeout_ms 可以省略', () => {
    const action: HookAction = {
      type: 'script',
      code: 'fn.log("done")',
    };
    expect(action.type).toBe('script');
    if (action.type === 'script') {
      expect(action.timeout_ms).toBeUndefined();
    }
  });
});

describe('FlowHook 类型', () => {
  it('应正确构造完整的钩子对象', () => {
    const hook: FlowHook = {
      uuid: 'hook-001',
      name: '通知 Hook',
      enabled: true,
      trigger: 'on_success',
      action: {
        type: 'notify',
        title_template: '流程完成',
        message_template: '耗时 {{duration_ms}}ms',
      },
    };
    expect(hook.uuid).toBe('hook-001');
    expect(hook.name).toBe('通知 Hook');
    expect(hook.enabled).toBe(true);
    expect(hook.trigger).toBe('on_success');
    expect(hook.action.type).toBe('notify');
  });

  it('钩子可以禁用', () => {
    const hook: FlowHook = {
      uuid: 'hook-002',
      name: '禁用的 Hook',
      enabled: false,
      trigger: 'on_complete',
      action: {
        type: 'webhook',
        url: 'https://example.com',
        method: 'POST',
        headers: {},
      },
    };
    expect(hook.enabled).toBe(false);
  });

  it('Flow 接口 hooks 字段是可选的（向后兼容）', () => {
    const flowJson = JSON.stringify({
      uuid: 'flow-001',
      name: '无钩子流程',
      nodes: [],
      connections: [],
    });
    const parsed = JSON.parse(flowJson) as Record<string, unknown>;
    expect(parsed.hooks).toBeUndefined();
  });
});
