/**
 * PluginIframe 组件测试
 * 覆盖：sandbox 精确属性（D2 仅 allow-scripts）、loading 态、错误 UI、init 发送
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, fireEvent, render } from '@testing-library/react';
import { PluginIframe } from '@/components/plugin/PluginIframe';
import type { PluginIframeProps } from '@/components/plugin/PluginIframe';

const SRC = 'mpe-plugin://demo.localhost/config-panel';
const ORIGIN = 'mpe-plugin://demo.localhost';

function makeProps(overrides: Partial<PluginIframeProps> = {}): PluginIframeProps {
  return {
    src: SRC,
    origin: ORIGIN,
    config: { url: 'https://example.com' },
    variables: { api_base: 'https://api.example.com' },
    hostApi: { testConnection: true },
    onConfigChange: vi.fn(),
    ...overrides,
  };
}

function renderPanel(props: PluginIframeProps) {
  const utils = render(<PluginIframe {...props} />);
  const iframe = utils.container.querySelector('iframe');
  if (!iframe) throw new Error('iframe not rendered');
  return { ...utils, iframe };
}

/** 注入来自 iframe 的 message 事件 */
function dispatchFromIframe(
  iframe: HTMLIFrameElement,
  data: unknown,
  origin: string = ORIGIN
) {
  act(() => {
    window.dispatchEvent(
      new MessageEvent('message', {
        data,
        origin,
        source: iframe.contentWindow as unknown as MessageEventSource,
      })
    );
  });
}

describe('PluginIframe', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('沙箱（D2 — 安全核心）', () => {
    it('sandbox 属性精确为 allow-scripts（禁止 allow-same-origin）', () => {
      const { iframe } = renderPanel(makeProps());

      expect(iframe.getAttribute('sandbox')).toBe('allow-scripts');
    });

    it('src 与 title 正确设置', () => {
      const { iframe } = renderPanel(makeProps({ title: 'Demo Plugin Panel' }));

      expect(iframe.getAttribute('src')).toBe(SRC);
      expect(iframe.getAttribute('title')).toBe('Demo Plugin Panel');
    });

    it('默认 title 兜底', () => {
      const { iframe } = renderPanel(makeProps());
      expect(iframe.getAttribute('title')).toBeTruthy();
    });
  });

  describe('加载态', () => {
    it('loading 时显示 spinner', () => {
      const { container } = renderPanel(makeProps());
      expect(container.querySelector('.animate-spin')).not.toBeNull();
    });

    it('收到 ready 后 spinner 消失', () => {
      const { container, iframe } = renderPanel(makeProps());

      dispatchFromIframe(iframe, { type: 'ready', payload: {} });

      expect(container.querySelector('.animate-spin')).toBeNull();
    });
  });

  describe('错误态', () => {
    it('iframe 内 error 消息 → 显示错误信息与回退链接', () => {
      const { container, iframe } = renderPanel(makeProps());

      dispatchFromIframe(iframe, { type: 'error', payload: { message: 'panel crashed' } });

      expect(container.textContent).toContain('panel crashed');
      expect(container.textContent).toContain('failed to load');
      const link = container.querySelector('a');
      expect(link?.getAttribute('href')).toBe(SRC);
    });

    // 注：iframe onError（React 19 资源元素）在 jsdom 中无法合成触发——
    // 真实浏览器正常；状态转换已由 hook 测试 handleIframeError 覆盖。

    it('错误态不再渲染 iframe（沙箱不会继续加载恶意内容）', () => {
      const { container, iframe } = renderPanel(makeProps());

      dispatchFromIframe(iframe, { type: 'error', payload: { message: 'boom' } });

      expect(container.querySelector('iframe')).toBeNull();
    });
  });

  describe('init 发送', () => {
    it('iframe load 事件触发 init（含 config + 变量 + host_api）', () => {
      const { iframe } = renderPanel(makeProps());

      const postMessageSpy = vi
        .spyOn(iframe.contentWindow as unknown as Window, 'postMessage')
        .mockImplementation(() => {});

      fireEvent.load(iframe);

      expect(postMessageSpy).toHaveBeenCalledWith(
        {
          type: 'init',
          payload: {
            config: { url: 'https://example.com' },
            variables: { api_base: 'https://api.example.com' },
            host_api: { testConnection: true },
          },
        },
        ORIGIN
      );
      postMessageSpy.mockRestore();
    });
  });
});
