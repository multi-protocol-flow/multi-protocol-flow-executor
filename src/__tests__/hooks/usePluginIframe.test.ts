/**
 * usePluginIframe 桥接 hook 测试
 * 覆盖：init 发送、configUpdated 推送、D3 双校验（origin + source）、
 * resize、requestAction → actionResult 往返、error、生命周期
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { usePluginIframe } from '@/hooks/usePluginIframe';
import type { UsePluginIframeOptions } from '@/hooks/usePluginIframe';

const SRC = 'mpe-plugin://demo.localhost/config-panel';
const ORIGIN = 'mpe-plugin://demo.localhost';
const INITIAL_CONFIG = { url: 'https://example.com', method: 'get' };
const INITIAL_VARIABLES = { api_base: 'https://api.example.com' };
const INITIAL_HOST_API = { testConnection: true, openFileDialog: true };

/** 伪造 iframe：contentWindow.postMessage 为 spy，可注入 message 事件 */
function makeFakeIframe() {
  const postMessage = vi.fn();
  const contentWindow = { postMessage } as unknown as Window;
  const iframe = { contentWindow } as unknown as HTMLIFrameElement;
  return { iframe, contentWindow, postMessage };
}

type PartialOptions = Partial<Omit<UsePluginIframeOptions, 'origin'>>;

function renderBridge(overrides: PartialOptions = {}) {
  const onConfigChange = vi.fn();
  const onRequestAction = vi.fn().mockResolvedValue({ ok: true });
  const props: UsePluginIframeOptions = {
    src: SRC,
    origin: ORIGIN,
    config: INITIAL_CONFIG,
    variables: INITIAL_VARIABLES,
    hostApi: INITIAL_HOST_API,
    onConfigChange,
    onRequestAction,
    ...overrides,
  };
  const utils = renderHook((newProps?: UsePluginIframeOptions) =>
    usePluginIframe(newProps ?? props)
  );
  const fake = makeFakeIframe();
  // 将伪造 iframe 挂到 hook 暴露的 ref 上
  utils.result.current.iframeRef.current = fake.iframe;
  return { ...utils, fake, onConfigChange, onRequestAction, props };
}

/** 从伪造 iframe 派发 message 事件 */
function dispatchMessage(
  fake: ReturnType<typeof makeFakeIframe>,
  data: unknown,
  options: { origin?: string; source?: unknown } = {}
) {
  const { origin = ORIGIN, source = fake.contentWindow } = options;
  window.dispatchEvent(
    new MessageEvent('message', { data, origin, source: source as MessageEventSource })
  );
}

/** 断言伪造场景无任何桥接流量 */
function expectNoBridgeTraffic(
  fake: ReturnType<typeof makeFakeIframe>,
  onConfigChange: ReturnType<typeof vi.fn>
) {
  expect(onConfigChange).not.toHaveBeenCalled();
  expect(fake.postMessage).not.toHaveBeenCalled();
}

describe('usePluginIframe', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('init 发送（iframe load）', () => {
    it('onLoad 时发送 init，携带 config + 变量快照 + host_api 能力声明', () => {
      const { result, fake } = renderBridge();

      act(() => {
        result.current.handleIframeLoad();
      });

      expect(fake.postMessage).toHaveBeenCalledTimes(1);
      const [payload, targetOrigin] = fake.postMessage.mock.calls[0] ?? [];
      expect(targetOrigin).toBe(ORIGIN);
      expect(payload).toEqual({
        type: 'init',
        payload: {
          config: INITIAL_CONFIG,
          variables: INITIAL_VARIABLES,
          host_api: INITIAL_HOST_API,
        },
      });
    });

    it('contentWindow 不可用时静默跳过', () => {
      const { result, fake } = renderBridge();
      result.current.iframeRef.current = null;

      act(() => {
        result.current.handleIframeLoad();
      });

      expect(fake.postMessage).not.toHaveBeenCalled();
    });
  });

  describe('configChanged 流转', () => {
    it('ready 后收到 configChanged → 调用 onConfigChange 且状态变 active', () => {
      const { result, fake, onConfigChange } = renderBridge();
      const newConfig = { url: 'https://changed.example.com' };

      act(() => {
        dispatchMessage(fake, { type: 'ready', payload: {} });
      });
      expect(result.current.readyState).toBe('ready');

      act(() => {
        dispatchMessage(fake, { type: 'configChanged', payload: { config: newConfig } });
      });

      expect(onConfigChange).toHaveBeenCalledTimes(1);
      expect(onConfigChange).toHaveBeenCalledWith(newConfig);
      expect(result.current.readyState).toBe('active');
    });

    it('ready 未到（loading）时收到 configChanged 也流转到 active', () => {
      const { result, fake, onConfigChange } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'configChanged', payload: { config: { a: 1 } } });
      });

      expect(onConfigChange).toHaveBeenCalledWith({ a: 1 });
      expect(result.current.readyState).toBe('active');
    });

    it('收到 ready 时宿主回发最新 config 同步竞态', () => {
      const { result, fake } = renderBridge();

      act(() => {
        result.current.handleIframeLoad();
        dispatchMessage(fake, { type: 'ready', payload: {} });
      });

      const calls = fake.postMessage.mock.calls;
      expect(calls).toHaveLength(2);
      const [initPayload] = calls[0] ?? [];
      const [syncPayload] = calls[1] ?? [];
      expect(initPayload?.type).toBe('init');
      expect(syncPayload).toEqual({ type: 'configUpdated', payload: { config: INITIAL_CONFIG } });
    });
  });

  describe('configUpdated 推送', () => {
    it('ready 后 config prop 变更 → 发送 configUpdated', () => {
      const { rerender, fake, props } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'ready', payload: {} });
      });

      const nextConfig = { url: 'https://next.example.com' };
      act(() => {
        rerender({ ...props, config: nextConfig });
      });

      expect(fake.postMessage).toHaveBeenCalledWith(
        { type: 'configUpdated', payload: { config: nextConfig } },
        ORIGIN
      );
    });

    it('ready 之前 config 变更不发送 configUpdated（init 已携带，待 ready 同步）', () => {
      const { rerender, fake, props } = renderBridge();

      act(() => {
        rerender({ ...props, config: { url: 'https://early.example.com' } });
      });

      expect(fake.postMessage).not.toHaveBeenCalled();
    });
  });

  describe('D3 安全校验（origin + source 双校验）', () => {
    it('伪造消息（错误 origin）被静默丢弃', () => {
      const { fake, onConfigChange } = renderBridge();

      act(() => {
        dispatchMessage(
          fake,
          { type: 'configChanged', payload: { config: { evil: true } } },
          { origin: 'https://evil.example.com' }
        );
      });

      expectNoBridgeTraffic(fake, onConfigChange);
    });

    it('伪造消息（错误 event.source）被静默丢弃', () => {
      const { fake, onConfigChange } = renderBridge();
      const otherWindow = { postMessage: vi.fn() } as unknown as Window;

      act(() => {
        dispatchMessage(
          fake,
          { type: 'configChanged', payload: { config: { evil: true } } },
          { source: otherWindow }
        );
      });

      expectNoBridgeTraffic(fake, onConfigChange);
    });

    it('非协议消息（非对象 data）被静默丢弃', () => {
      const { fake, onConfigChange } = renderBridge();

      act(() => {
        dispatchMessage(fake, 'just a string');
      });

      expectNoBridgeTraffic(fake, onConfigChange);
    });

    it('畸形协议消息（payload 形状不符）被静默丢弃', () => {
      const { fake, onConfigChange } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'resize', payload: { height: 'not-a-number' } });
      });

      expectNoBridgeTraffic(fake, onConfigChange);
    });

    it('iframe 未挂载（无 contentWindow）时消息被丢弃', () => {
      const { result, fake, onConfigChange } = renderBridge();
      result.current.iframeRef.current = null;

      act(() => {
        dispatchMessage(fake, { type: 'configChanged', payload: { config: { x: 1 } } });
      });

      expectNoBridgeTraffic(fake, onConfigChange);
    });
  });

  describe('resize 自适应高度', () => {
    it('收到 resize 更新 height（初始为 null）', () => {
      const { result, fake } = renderBridge();
      expect(result.current.height).toBeNull();

      act(() => {
        dispatchMessage(fake, { type: 'resize', payload: { height: 480 } });
      });

      expect(result.current.height).toBe(480);
    });

    it('多次 resize 覆盖旧值', () => {
      const { result, fake } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'resize', payload: { height: 300 } });
        dispatchMessage(fake, { type: 'resize', payload: { height: 640 } });
      });

      expect(result.current.height).toBe(640);
    });
  });

  describe('requestAction → actionResult 往返', () => {
    it('调用 onRequestAction 并把结果回传 iframe', async () => {
      const { fake, onRequestAction } = renderBridge();

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-1', action: 'testConnection', params: { host: 'x' } },
        });
      });

      expect(onRequestAction).toHaveBeenCalledWith('testConnection', { host: 'x' });
      expect(fake.postMessage).toHaveBeenCalledWith(
        { type: 'actionResult', payload: { requestId: 'req-1', result: { ok: true } } },
        ORIGIN
      );
    });

    it('requestAction 无 params 时透传 undefined', async () => {
      const { fake, onRequestAction } = renderBridge();

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-2', action: 'ping' },
        });
      });

      expect(onRequestAction).toHaveBeenCalledWith('ping', undefined);
    });

    it('handler 抛 Error → actionResult 带 error 回传', async () => {
      const { fake } = renderBridge({
        onRequestAction: vi.fn().mockRejectedValue(new Error('connection refused')),
      });

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-3', action: 'testConnection' },
        });
      });

      expect(fake.postMessage).toHaveBeenCalledWith(
        { type: 'actionResult', payload: { requestId: 'req-3', result: null, error: 'connection refused' } },
        ORIGIN
      );
    });

    it('handler 抛非 Error 异常也回传 error', async () => {
      const { fake } = renderBridge({
        onRequestAction: vi.fn().mockRejectedValue('raw string error'),
      });

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-4', action: 'custom' },
        });
      });

      const calls = fake.postMessage.mock.calls;
      const lastCall = calls[calls.length - 1] ?? [];
      expect(lastCall[0]).toEqual({
        type: 'actionResult',
        payload: { requestId: 'req-4', result: null, error: 'raw string error' },
      });
    });

    it('未注册 onRequestAction → 立即回传 error 防止插件挂起', async () => {
      const { fake } = renderBridge({ onRequestAction: undefined });

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-5', action: 'testConnection' },
        });
      });

      const calls = fake.postMessage.mock.calls;
      const lastCall = calls[calls.length - 1] ?? [];
      expect(lastCall[0]?.type).toBe('actionResult');
      expect(lastCall[0]?.payload?.requestId).toBe('req-5');
      expect(typeof lastCall[0]?.payload?.error).toBe('string');
    });

    it('requestAction 流转到 active 状态', async () => {
      const { result, fake } = renderBridge();

      await act(async () => {
        dispatchMessage(fake, {
          type: 'requestAction',
          payload: { requestId: 'req-6', action: 'testConnection' },
        });
      });

      expect(result.current.readyState).toBe('active');
    });
  });

  describe('error 消息', () => {
    it('收到 error → 设置 error 状态', () => {
      const { result, fake } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'error', payload: { message: 'panel crashed' } });
      });

      expect(result.current.error).toBe('panel crashed');
    });

    it('handleIframeError（iframe onError）设置加载错误', () => {
      const { result } = renderBridge();

      act(() => {
        result.current.handleIframeError();
      });

      expect(result.current.error).toBeTruthy();
    });
  });

  describe('生命周期', () => {
    it('卸载时移除 message 监听器', () => {
      const removeSpy = vi.spyOn(window, 'removeEventListener');
      const { unmount } = renderBridge();

      unmount();

      expect(removeSpy).toHaveBeenCalledWith('message', expect.any(Function));
      removeSpy.mockRestore();
    });

    it('src 变更时重置 readyState/error/height', () => {
      const { rerender, result, fake, props } = renderBridge();

      act(() => {
        dispatchMessage(fake, { type: 'ready', payload: {} });
        dispatchMessage(fake, { type: 'resize', payload: { height: 500 } });
        dispatchMessage(fake, { type: 'error', payload: { message: 'boom' } });
      });
      expect(result.current.readyState).toBe('ready');
      expect(result.current.height).toBe(500);
      expect(result.current.error).toBe('boom');

      act(() => {
        rerender({ ...props, src: 'mpe-plugin://other.localhost/config-panel' });
      });

      expect(result.current.readyState).toBe('loading');
      expect(result.current.height).toBeNull();
      expect(result.current.error).toBeNull();
    });
  });
});
