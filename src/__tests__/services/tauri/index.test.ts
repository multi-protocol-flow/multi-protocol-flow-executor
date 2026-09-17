/**
 * Tauri IPC 服务主出口测试
 * 测试 createFlow 包装器按当前界面语言本地化入口/结束节点的默认名称
 *
 * 注意：全局测试 setup 将 i18n-util 固定为 zh-CN 翻译（见 __tests__/setup.ts），
 * 因此这里断言的是"后端默认名称被 LL.nodes.entry()/end() 覆盖"，而非具体语言值。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createFlow } from '@/services/tauri';
import type { Flow } from '@/types/flow';

// 强制走真实 IPC 分支
vi.mock('@/lib/platform', () => ({ isTauriEnv: () => true }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: vi.fn(),
}));

/** 模拟后端 create_flow 返回（默认名称："开始"/"End"，不随界面语言变化） */
function backendFlow(name: string): Flow {
  return {
    uuid: 'flow-uuid',
    name,
    nodes: [
      { uuid: 'start-flow-uuid', type: 'entry', name: '开始', position: { x: 0, y: 0 }, data: { type: 'entry' } },
      { uuid: 'end-flow-uuid', type: 'end', name: 'End', position: { x: 400, y: 0 }, data: { type: 'end' } },
    ],
    connections: [],
    initial_variables: {},
  };
}

describe('createFlow 服务包装器', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('入口节点名称被本地化（后端默认 "开始" 被覆盖）', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ flow: backendFlow('测试流程') });

    const flow = await createFlow('测试流程');
    const entry = flow.nodes.find((n) => n.type === 'entry');

    expect(entry?.name).toBe('入口');
    expect(entry?.name).not.toBe('开始');
  });

  it('结束节点名称被本地化（后端默认 "End" 被覆盖）', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ flow: backendFlow('测试流程') });

    const flow = await createFlow('测试流程');
    const end = flow.nodes.find((n) => n.type === 'end');

    expect(end?.name).toBe('终止');
    expect(end?.name).not.toBe('End');
  });

  it('非入口/结束节点名称保持后端原样', async () => {
    const flowWithHttp = backendFlow('测试流程');
    flowWithHttp.nodes.push({
      uuid: 'http-flow-uuid',
      type: 'http:request',
      name: '我的请求',
      position: { x: 200, y: 0 },
      data: { type: 'http:request', method: 'get', url: '' },
    });
    vi.mocked(invoke).mockResolvedValueOnce({ flow: flowWithHttp });

    const flow = await createFlow('测试流程');
    const httpNode = flow.nodes.find((n) => n.type === 'http:request');

    expect(httpNode?.name).toBe('我的请求');
  });
});
