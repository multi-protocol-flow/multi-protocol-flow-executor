/**
 * usePluginIframe —— 插件 iframe 双向 postMessage 桥接 hook
 *
 * 宿主侧桥：把插件作者的 HTML 配置面板（沙箱 iframe，仅 `allow-scripts`，
 * 不透明源）与宿主 UI 连接起来。插件 iframe 内 `window.__TAURI_INVOKE__`
 * 天然不可用（F5），postMessage 是唯一通道。
 *
 * 状态机: `loading` →（iframe load + init 发送）→ `ready`（插件发来 ready）
 * → `active`（插件首次 configChanged / requestAction，用户已交互）。
 *
 * 安全（D3 双校验，缺一不可）：
 *   1. `event.origin === origin`（插件 origin 白名单）
 *   2. `event.source === iframeRef.current?.contentWindow`（来源窗口身份）
 *   叠加 `isPluginIframeMessage` 信封/形状校验；任一失败静默丢弃。
 *
 * postMessage 目标 origin：使用 `origin` prop 原样（调用方从
 * `meta.frontend.url` 解析得到 —— macOS/Linux `mpe-plugin://<id>.localhost`、
 * Windows `http://mpe-plugin.<id>.localhost`）。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';
import {
  createActionResultMessage,
  createConfigUpdatedMessage,
  createInitMessage,
  isPluginIframeMessage,
} from '@/types/plugin-iframe';
import type {
  PluginIframeHostApi,
  PluginIframeMessage,
  PluginIframeRequestActionPayload,
} from '@/types/plugin-iframe';

/** iframe 加载失败（onError）时展示的错误文案 */
export const PLUGIN_IFRAME_LOAD_ERROR = 'Failed to load plugin panel';

/** 面板桥接状态 */
export type PluginIframeReadyState = 'loading' | 'ready' | 'active';

export interface UsePluginIframeOptions {
  /** iframe URL（`meta.frontend.url`，如 `mpe-plugin://<id>.localhost/config-panel`） */
  src: string;
  /** 插件 origin —— postMessage 目标 + 入站消息校验白名单（从 src 解析） */
  origin: string;
  /** 当前节点配置（快照语义：postMessage 结构化克隆，插件收到的是深拷贝） */
  config: unknown;
  /** 变量快照（宿主侧组装，不要传 live store 引用） */
  variables: Record<string, unknown>;
  /** host_api 能力声明（PR-5 白名单生成） */
  hostApi: PluginIframeHostApi;
  /** iframe configChanged → 宿主保存 */
  onConfigChange: (config: unknown) => void;
  /** iframe requestAction → 宿主执行，返回结果经 actionResult 回传（PR-5 白名单） */
  onRequestAction?: (action: string, params?: unknown) => Promise<unknown>;
}

export interface UsePluginIframeResult {
  /** 绑定到 <iframe ref={...}> 的 ref（消息来源身份校验依赖它） */
  iframeRef: RefObject<HTMLIFrameElement | null>;
  readyState: PluginIframeReadyState;
  /** 面板错误（协议 error 消息或 iframe onError），null 表示正常 */
  error: string | null;
  /** 自适应高度（resize 消息），null 表示插件未上报 */
  height: number | null;
  /** 绑定到 iframe 的 onLoad：发送 init */
  handleIframeLoad: () => void;
  /** 绑定到 iframe 的 onError：设置加载错误 */
  handleIframeError: () => void;
}

export function usePluginIframe(options: UsePluginIframeOptions): UsePluginIframeResult {
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const [readyState, setReadyState] = useState<PluginIframeReadyState>('loading');
  const [error, setError] = useState<string | null>(null);
  const [height, setHeight] = useState<number | null>(null);

  // 最新 props 快照：window message 监听器只注册一次，经 ref 读取最新回调，
  // 避免每次渲染重建监听器。
  const latestRef = useRef<UsePluginIframeOptions>(options);
  latestRef.current = options;

  // 插件是否已就绪（ready 消息）——决定 configUpdated 是否立即推送
  const readyRef = useRef(false);

  /** 向插件 iframe 发消息（目标 origin = origin prop 原样） */
  const sendToIframe = useCallback((message: PluginIframeMessage): void => {
    const contentWindow = iframeRef.current?.contentWindow;
    if (!contentWindow) return;
    contentWindow.postMessage(message, latestRef.current.origin);
  }, []);

  /** iframe load：发送 init（config + 变量快照 + host_api 能力声明） */
  const handleIframeLoad = useCallback((): void => {
    const { config: cfg, variables: vars, hostApi: api } = latestRef.current;
    sendToIframe(createInitMessage(cfg, vars, api));
  }, [sendToIframe]);

  /** iframe onError：加载失败 */
  const handleIframeError = useCallback((): void => {
    setError(PLUGIN_IFRAME_LOAD_ERROR);
  }, []);

  /** 处理 requestAction：调 handler，结果/异常经 actionResult 回传（永不抛） */
  const handleRequestAction = useCallback(
    async (payload: PluginIframeRequestActionPayload): Promise<void> => {
      const { requestId, action, params } = payload;
      const handler = latestRef.current.onRequestAction;
      if (!handler) {
        // 未注册 handler → 立即回传错误，避免插件永久挂起等待
        sendToIframe(createActionResultMessage(requestId, null, 'No action handler registered'));
        return;
      }
      try {
        const result = await handler(action, params);
        sendToIframe(createActionResultMessage(requestId, result));
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        sendToIframe(createActionResultMessage(requestId, null, message));
      }
    },
    [sendToIframe]
  );

  /** 入站消息处理：D3 双校验 + 协议守卫，通过后按 type 分派 */
  const handleMessage = useCallback(
    (event: MessageEvent): void => {
      const contentWindow = iframeRef.current?.contentWindow;
      if (!contentWindow) return;
      // D3 校验 1：来源 origin 必须等于插件白名单 origin
      if (event.origin !== latestRef.current.origin) return;
      // D3 校验 2：来源窗口必须就是本 iframe 的 contentWindow
      if (event.source !== contentWindow) return;
      // 协议守卫：信封形状 + payload 形状（未知 type / 畸形 → 丢弃）
      const message = event.data;
      if (!isPluginIframeMessage(message)) return;

      // 仅处理 iframe → 宿主方向；宿主方向消息（init 等）视为畸形丢弃
      switch (message.type) {
        case 'ready':
          readyRef.current = true;
          setReadyState('ready');
          // 竞态同步：ready 前 config 已变更过 → 回发最新 config 保证一致
          sendToIframe(createConfigUpdatedMessage(latestRef.current.config));
          break;
        case 'configChanged':
          setReadyState('active');
          latestRef.current.onConfigChange(message.payload.config);
          break;
        case 'requestAction':
          setReadyState('active');
          void handleRequestAction(message.payload);
          break;
        case 'resize':
          setHeight(message.payload.height);
          break;
        case 'error':
          setError(message.payload.message);
          break;
        case 'init':
        case 'configUpdated':
        case 'actionResult':
          break;
      }
    },
    [sendToIframe, handleRequestAction]
  );

  // 监听 window message（整个面板生命周期）
  useEffect(() => {
    window.addEventListener('message', handleMessage);
    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [handleMessage]);

  // config prop 变更 → 已就绪则推送 configUpdated（未就绪由 ready 同步兜底）
  const previousConfigRef = useRef<unknown>(options.config);
  useEffect(() => {
    if (previousConfigRef.current === options.config) return;
    previousConfigRef.current = options.config;
    if (!readyRef.current) return;
    sendToIframe(createConfigUpdatedMessage(options.config));
  }, [options.config, sendToIframe]);

  // src 变更（切换插件/页面）→ 重置状态，等待新 iframe 重新走状态机
  useEffect(() => {
    readyRef.current = false;
    setReadyState('loading');
    setError(null);
    setHeight(null);
  }, [options.src]);

  return { iframeRef, readyState, error, height, handleIframeLoad, handleIframeError };
}
