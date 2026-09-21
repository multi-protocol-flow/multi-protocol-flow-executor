/**
 * pluginRequestActions 白名单测试（PR-5 安全门）
 *
 * 覆盖：
 * (a) 白名单：允许 testConnection / openFileDialog，拒绝未知动作（eval/exec/readFile/任意字符串）
 * (b) 未知动作 × usePluginIframe：onRequestAction 立即拒绝 → actionResult 错误回传（插件不挂起）
 * (c) testConnection（SSRF 原语审计）：目标只来自用户 config（params 的 URL 被忽略）、
 *     硬超时、返回脱敏（不含 body/headers/status，不透传原始错误）
 * (d) openFileDialog：返回路径（string/string[]/null），params 透传 multiple/filters/directory
 * (e) hostApi 声明与白名单完全一致
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { open } from '@tauri-apps/plugin-dialog';
import {
  PLUGIN_HOST_API,
  PLUGIN_REQUEST_ACTIONS,
  TEST_CONNECTION_TIMEOUT_MS,
  createPluginRequestHandler,
  resolvePluginAction,
} from '@/lib/pluginRequestActions';
import type { PluginActionHandler } from '@/lib/pluginRequestActions';
import { usePluginIframe } from '@/hooks/usePluginIframe';
import type { UsePluginIframeOptions } from '@/hooks/usePluginIframe';

// openFileDialog 走 @tauri-apps/plugin-dialog 动态导入 —— 测试内整体替换为 mock
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// uiCall 走 @tauri-apps/api/core 的 invoke —— 测试内整体替换为 mock
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const SRC = 'mpe-plugin://demo.localhost/config-panel';
const ORIGIN = 'mpe-plugin://demo.localhost';

/** 取白名单 handler（断言存在后返回，测试内统一入口） */
function handlerFor(action: string): PluginActionHandler {
  const handler = resolvePluginAction(action);
  if (!handler) throw new Error(`expected a handler for whitelisted action: ${action}`);
  return handler;
}

/** 伪造 iframe：contentWindow.postMessage 为 spy，可注入 message 事件 */
function makeFakeIframe() {
  const postMessage = vi.fn();
  const contentWindow = { postMessage } as unknown as Window;
  const iframe = { contentWindow } as unknown as HTMLIFrameElement;
  return { iframe, contentWindow, postMessage };
}

/** 从伪造 iframe 派发 message 事件（默认 origin = 沙箱真实不透明源 'null'） */
function dispatchMessage(
  fake: ReturnType<typeof makeFakeIframe>,
  data: unknown
) {
  window.dispatchEvent(
    new MessageEvent('message', {
      data,
      origin: 'null',
      source: fake.contentWindow as MessageEventSource,
    })
  );
}

/** 以生产 wiring（createPluginRequestHandler）渲染真实 hook */
function renderWiredHook(config: unknown) {
  const props: UsePluginIframeOptions = {
    src: SRC,
    origin: ORIGIN,
    config,
    variables: {},
    hostApi: PLUGIN_HOST_API,
    nodeType: 'demo:ping',
    locale: 'en-US',
    theme: 'light',
    onConfigChange: vi.fn(),
    onRequestAction: createPluginRequestHandler(config, 'demo:ping'),
  };
  const utils = renderHook((p?: UsePluginIframeOptions) => usePluginIframe(p ?? props));
  const fake = makeFakeIframe();
  utils.result.current.iframeRef.current = fake.iframe;
  return { ...utils, fake, props };
}

/** 取最后一次 postMessage 调用（避免与 init 消息混淆） */
function lastPostMessage(fake: ReturnType<typeof makeFakeIframe>) {
  const calls = fake.postMessage.mock.calls;
  return (calls[calls.length - 1] ?? []) as [unknown, unknown];
}

describe('resolvePluginAction 白名单（安全闸）', () => {
  it.each(['testConnection', 'openFileDialog'] as const)('允许 %s', (action) => {
    expect(resolvePluginAction(action)).toBeTypeOf('function');
  });

  it('白名单枚举内的每个动作都能解析出 handler（声明不漂移）', () => {
    for (const action of PLUGIN_REQUEST_ACTIONS) {
      expect(resolvePluginAction(action)).toBeTypeOf('function');
    }
  });

  it.each([
    'eval',
    'exec',
    'readFile',
    'writeFile',
    'shell',
    'openExternal',
    'listFiles',
    'kill',
    'fetch',
    'test_connection',
    'TestConnection',
    'testConnection.evil',
    'testConnection()',
    'url',
    '',
    ' ',
    'alert(1)',
  ])('拒绝未知动作 %j', (action) => {
    expect(resolvePluginAction(action)).toBeUndefined();
  });
});

describe('hostApi 能力声明', () => {
  it('与白名单完全一致：键集合相同且全部声明为 true', () => {
    expect(Object.keys(PLUGIN_HOST_API).sort()).toEqual([...PLUGIN_REQUEST_ACTIONS].sort());
    for (const action of PLUGIN_REQUEST_ACTIONS) {
      expect(PLUGIN_HOST_API[action]).toBe(true);
    }
  });
});

describe('testConnection（SSRF 原语审计）', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('成功：只返回脱敏摘要 { ok, latency_ms }，不含任何响应内容', async () => {
    const response = {
      ok: true,
      status: 200,
      headers: new Headers({ 'x-internal-token': 'leak-me', 'set-cookie': 'session=secret' }),
      body: 'internal secret body',
      text: vi.fn().mockResolvedValue('internal secret body'),
    };
    const fetchMock = vi.fn().mockResolvedValue(response);
    vi.stubGlobal('fetch', fetchMock);

    const result = await handlerFor('testConnection')(undefined, {
      config: { url: 'http://127.0.0.1:6379' },
      nodeType: 'demo:ping',
    });

    expect(result).toEqual({ ok: true, latency_ms: expect.any(Number) });
    const serialized = JSON.stringify(result);
    expect(serialized).not.toContain('internal secret');
    expect(serialized).not.toContain('leak-me');
    expect(serialized).not.toContain('session=secret');
  });

  it('目标只来自用户 config：params 中的 URL 被完全忽略', async () => {
    const fetchMock = vi.fn().mockResolvedValue({});
    vi.stubGlobal('fetch', fetchMock);

    await handlerFor('testConnection')(
      { url: 'https://evil.example.com', target: 'http://evil2.example.com' },
      { config: { url: 'https://user-typed.example.com/ping' }, nodeType: 'demo:ping' }
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [targetUrl] = fetchMock.mock.calls[0] ?? [];
    expect(targetUrl).toBe('https://user-typed.example.com/ping');
  });

  it('params 只能指定 config 字段名（默认 url），值仍从 config 解析', async () => {
    const fetchMock = vi.fn().mockResolvedValue({});
    vi.stubGlobal('fetch', fetchMock);

    await handlerFor('testConnection')(
      { field: 'endpoint' },
      { config: { endpoint: 'https://svc.local/health' }, nodeType: 'demo:ping' }
    );

    const [targetUrl] = fetchMock.mock.calls[0] ?? [];
    expect(targetUrl).toBe('https://svc.local/health');
  });

  it('config 无 url 字段 → invalid_target，不发请求', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    const result = await handlerFor('testConnection')(undefined, { config: {}, nodeType: 'demo:ping' });

    expect(result).toEqual({ ok: false, error: 'invalid_target' });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('config.url 非字符串 / 非 http(s) / 非法 URL / 字段名非法 → invalid_target', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    const configs: unknown[] = [
      { url: 42 },
      { url: null },
      { url: '' },
      { url: 'file:///etc/passwd' },
      { url: 'ws://internal.example.com/socket' },
      { url: 'javascript:alert(1)' },
      { url: 'data:text/html,secret' },
      { url: 'not a url' },
      { url: 'ftp://internal.example.com' },
    ];
    for (const cfg of configs) {
      const result = await handlerFor('testConnection')(undefined, { config: cfg, nodeType: 'demo:ping' });
      expect(result).toEqual({ ok: false, error: 'invalid_target' });
    }
    // 非法字段名（路径穿越 / 原型键等形态）→ invalid_target
    for (const field of ['a/b', 'a.b', '__proto__', 'constructor', '-x', 'x y']) {
      const result = await handlerFor('testConnection')(
        { field },
        { config: { url: 'https://example.com' }, nodeType: 'demo:ping' }
      );
      expect(result).toEqual({ ok: false, error: 'invalid_target' });
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('网络失败 → connection_failed 错误码，不透传原始错误信息', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockRejectedValue(new TypeError('Failed to fetch http://secret.internal:8080/admin'))
    );

    const result = await handlerFor('testConnection')(undefined, {
      config: { url: 'https://internal.example.com/health' },
      nodeType: 'demo:ping',
    });

    expect(result).toEqual({
      ok: false,
      error: 'connection_failed',
      latency_ms: expect.any(Number),
    });
    const serialized = JSON.stringify(result);
    expect(serialized).not.toContain('Failed to fetch');
    expect(serialized).not.toContain('secret.internal');
  });

  it('超过硬超时 → timeout 错误码（插件无法挂起宿主）', async () => {
    vi.useFakeTimers();
    try {
      vi.stubGlobal(
        'fetch',
        vi.fn(
          (_url: string, init?: RequestInit) =>
            new Promise((_resolve, reject) => {
              init?.signal?.addEventListener('abort', () => {
                reject(new DOMException('Aborted', 'AbortError'));
              });
            })
        )
      );

      const pending = handlerFor('testConnection')(undefined, {
        config: { url: 'https://internal.example.com/health' },
        nodeType: 'demo:ping',
      });
      await vi.advanceTimersByTimeAsync(TEST_CONNECTION_TIMEOUT_MS);
      const result = await pending;

      expect(result).toMatchObject({ ok: false, error: 'timeout' });
    } finally {
      vi.useRealTimers();
    }
  });

  it('无 context.config → invalid_target，不发请求', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    const result = await handlerFor('testConnection')();

    expect(result).toEqual({ ok: false, error: 'invalid_target' });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe('openFileDialog（路径返回 + 参数透传）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('返回所选文件的路径字符串', async () => {
    vi.mocked(open).mockResolvedValue('/home/user/data/file.json');

    const result = await handlerFor('openFileDialog')();

    expect(result).toBe('/home/user/data/file.json');
    expect(open).toHaveBeenCalledWith({ multiple: false, directory: false });
  });

  it('多选结果 → 返回路径数组', async () => {
    vi.mocked(open).mockResolvedValue(['/a.txt', '/b.txt']);

    expect(await handlerFor('openFileDialog')({ multiple: true })).toEqual(['/a.txt', '/b.txt']);
  });

  it('取消（null）→ 返回 null', async () => {
    vi.mocked(open).mockResolvedValue(null);

    expect(await handlerFor('openFileDialog')()).toBeNull();
  });

  it('插件 params 控制对话框：multiple/filters/directory 透传', async () => {
    vi.mocked(open).mockResolvedValue(['/a.proto', '/b.proto']);

    const result = await handlerFor('openFileDialog')({
      multiple: true,
      filters: [{ name: 'Proto', extensions: ['proto'] }],
      directory: false,
    });

    expect(result).toEqual(['/a.proto', '/b.proto']);
    expect(open).toHaveBeenCalledWith({
      multiple: true,
      directory: false,
      filters: [{ name: 'Proto', extensions: ['proto'] }],
    });
  });

  it('directory 模式透传', async () => {
    vi.mocked(open).mockResolvedValue('/proto/dir');

    expect(await handlerFor('openFileDialog')({ directory: true })).toBe('/proto/dir');
    expect(open).toHaveBeenCalledWith({ multiple: false, directory: true });
  });
});

describe('uiCall（宿主转发发现查询）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue({ tools: [{ name: 'echo' }] });
  });

  it('合法透传：method + payload → invoke(plugin_ui_call)，nodeType 绑定到上下文', async () => {
    const result = await handlerFor('uiCall')(
      { method: 'mcp.discover', payload: { transport: 'stdio', command: 'npx' } },
      { config: {}, nodeType: 'mcp:connect' }
    );

    expect(result).toEqual({ tools: [{ name: 'echo' }] });
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('plugin_ui_call', {
      nodeType: 'mcp:connect',
      method: 'mcp.discover',
      payload: { transport: 'stdio', command: 'npx' },
    });
  });

  it('payload 缺省 → 传 null（Rust Option 侧 None）', async () => {
    await handlerFor('uiCall')(
      { method: 'mcp.discover' },
      { config: {}, nodeType: 'mcp:connect' }
    );

    expect(invoke).toHaveBeenCalledWith('plugin_ui_call', {
      nodeType: 'mcp:connect',
      method: 'mcp.discover',
      payload: null,
    });
  });

  it('params 非对象 / 缺 method / method 非字符串 → 拒绝（Action not allowed 风格）', async () => {
    const cases: unknown[] = [
      undefined,
      null,
      'mcp.discover',
      { method: '' },
      { method: '   ' },
      { method: 42 },
      {},
    ];
    for (const params of cases) {
      await expect(
        handlerFor('uiCall')(params, { config: {}, nodeType: 'mcp:connect' })
      ).rejects.toThrow(/Action not allowed/);
    }
    expect(invoke).not.toHaveBeenCalled();
  });

  it('缺 context.nodeType → 拒绝，不发 invoke', async () => {
    await expect(handlerFor('uiCall')({ method: 'mcp.discover' })).rejects.toThrow(
      /Action not allowed/
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it('未知动作仍拒绝（uiCall 不放开其他白名单外动作）', () => {
    expect(resolvePluginAction('uiCall.evil')).toBeUndefined();
    expect(resolvePluginAction('uicall')).toBeUndefined();
    expect(resolvePluginAction('plugin_ui_call')).toBeUndefined();
  });

  it('invoke 失败 → 错误原样上抛（usePluginIframe 经 actionResult 回传）', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('unknown plugin node type mcp:nope'));

    await expect(
      handlerFor('uiCall')({ method: 'mcp.discover' }, { config: {}, nodeType: 'mcp:nope' })
    ).rejects.toThrow('unknown plugin node type mcp:nope');
  });
});

describe('白名单 × usePluginIframe 集成（hook 未修改，复用 PR-3 拒绝路径）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('未知动作 → onRequestAction 立即拒绝 → actionResult 错误回传（插件不挂起）', async () => {
    const { fake } = renderWiredHook({ url: 'https://example.com' });

    await act(async () => {
      dispatchMessage(fake, {
        type: 'requestAction',
        payload: { requestId: 'req-1', action: 'exec' },
      });
    });

    const [message] = lastPostMessage(fake);
    expect(message).toEqual({
      type: 'actionResult',
      payload: { requestId: 'req-1', result: null, error: 'Action not allowed: exec' },
    });
  });

  it('白名单动作经真实 wiring 执行 → actionResult 携带脱敏结果', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({}));
    const { fake } = renderWiredHook({ url: 'https://internal.example.com/health' });

    await act(async () => {
      dispatchMessage(fake, {
        type: 'requestAction',
        payload: { requestId: 'req-2', action: 'testConnection' },
      });
    });

    const [message] = lastPostMessage(fake);
    expect(message).toEqual({
      type: 'actionResult',
      payload: { requestId: 'req-2', result: { ok: true, latency_ms: expect.any(Number) } },
    });
  });
});
