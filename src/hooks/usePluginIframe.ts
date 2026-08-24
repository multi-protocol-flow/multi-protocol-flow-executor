/**
 * usePluginIframe —— 插件 iframe 双向 postMessage 桥接 hook
 *
 * 宿主侧桥：把插件作者的 HTML 配置面板（沙箱 iframe，仅 `allow-scripts`，
 * 不透明源）与宿主 UI 连接起来。插件 iframe 内 `window.__TAURI_INVOKE__`
 * 天然不可用（F5），postMessage 是唯一通道。
 *
 * 状态机: `loading` →（iframe load + init 发送，未 ready 前周期重发）→ `ready`
 * （插件发来 ready）→ `active`（插件首次 configChanged / requestAction，用户已交互）。
 *
 * 安全（D3，真实浏览器实证修正）：
 * 沙箱 iframe（无 `allow-same-origin`）的源是**不透明的**，浏览器将其序列化
 * 为字符串 `'null'`（跨平台一致：macOS/Linux 的 `mpe-plugin://` 与 Windows 的
 * `http://mpe-plugin.<id>.localhost` 在沙箱内都报告 `event.origin === 'null'`）。
 * 因此：
 *   1. 发送：目标 origin 必须是 `'*'` —— 对不透明源传 `'null'` 会抛
 *      `Invalid target origin`，传具体 origin（如 `http://mpe-plugin.*`）会被
 *      浏览器静默丢弃（target origin 与接收窗口实际 origin 'null' 不匹配）。
 *      使用 `'*'` 安全：目的地被 `iframeRef.current.contentWindow` 引用钉死
 *      （永远只发给**我们的** iframe），接收侧身份由 `event.source` 校验。
 *   2. 接收：`event.source === contentWindow` 是**权威**防伪闸（不变）；
 *      origin 检查接受不透明源 `'null'` 或 `origin` prop 白名单（纵深防御，
 *      拒绝任何真实第三方源——如宿主页面 XSS 或其他 frame 的伪造消息）。
 *   叠加 `isPluginIframeMessage` 信封/形状校验；任一失败静默丢弃。
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
  PluginIframeNodeSnapshot,
  PluginIframeRequestActionPayload,
} from '@/types/plugin-iframe';

/** iframe 加载失败（onError）时展示的错误文案 */
export const PLUGIN_IFRAME_LOAD_ERROR = 'Failed to load plugin panel';

/**
 * init 重发间隔（WebKitGTK 竞态兜底，实证见 handleIframeLoad 注释）。
 * 插件面板一般在 React effect 里安装 message 监听器，可能晚于 iframe
 * `load` 事件；load 时发出的首个 init 会丢失。宿主按此间隔周期重发
 * init，直至收到 ready，保证监听器就绪后 ≤ 一个间隔内收到 init。
 */
export const INIT_RETRY_INTERVAL_MS = 300;

/** 面板桥接状态 */
export type PluginIframeReadyState = 'loading' | 'ready' | 'active';

export interface UsePluginIframeOptions {
  /** iframe URL（`meta.frontend.url`，如 `mpe-plugin://<id>.localhost/config-panel`） */
  src: string;
  /** 插件 origin —— 入站消息校验白名单（从 src 解析；沙箱不透明源 'null' 恒被接受） */
  origin: string;
  /** 当前节点配置（快照语义：postMessage 结构化克隆，插件收到的是深拷贝） */
  config: unknown;
  /** 变量快照（宿主侧组装，不要传 live store 引用） */
  variables: Record<string, unknown>;
  /** host_api 能力声明（PR-5 白名单生成） */
  hostApi: PluginIframeHostApi;
  /** 当前编辑节点的类型 id（透传给 init 载荷，如 `mcp:connect`） */
  nodeType: string;
  /** 宿主当前 UI 语言（BCP-47，透传给 init 载荷） */
  locale: string;
  /** 同插件节点快照（可选；connection 下拉等跨节点引用用） */
  nodes?: PluginIframeNodeSnapshot[];
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
  // iframe 是否已触发 load —— init 重发兜底只在 load 之后运行
  const [loaded, setLoaded] = useState(false);

  // 最新 props 快照：window message 监听器只注册一次，经 ref 读取最新回调，
  // 避免每次渲染重建监听器。
  const latestRef = useRef<UsePluginIframeOptions>(options);
  latestRef.current = options;

  // 插件是否已就绪（ready 消息）——决定 configUpdated 是否立即推送
  const readyRef = useRef(false);

  /**
   * 向插件 iframe 发消息。
   * 目标 origin 固定 `'*'`：沙箱 iframe 源不透明（'null'）——
   * 传 `'null'` 抛 Invalid target origin，传具体 origin 被静默丢弃，
   * `'*'` 是唯一能送达的合法目标（D3 实证）。安全性由接收方
   * `event.source === contentWindow` 身份校验兜底；postMessage 的
   * `'*'` 只影响送达范围，不影响消息内容。
   */
  const sendToIframe = useCallback((message: PluginIframeMessage): void => {
    const contentWindow = iframeRef.current?.contentWindow;
    if (!contentWindow) return;
    contentWindow.postMessage(message, '*');
  }, []);

  /** iframe load：发送 init（config + 变量快照 + host_api 能力声明 + 节点上下文） */
  const handleIframeLoad = useCallback((): void => {
    setLoaded(true);
    const {
      config: cfg,
      variables: vars,
      hostApi: api,
      nodeType,
      locale,
      nodes,
    } = latestRef.current;
    sendToIframe(createInitMessage(cfg, vars, api, nodeType, locale, nodes));
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

  /** 入站消息处理：D3 校验（source 权威 + origin 纵深）+ 协议守卫，通过后按 type 分派 */
  const handleMessage = useCallback(
    (event: MessageEvent): void => {
      const contentWindow = iframeRef.current?.contentWindow;
      if (!contentWindow) return;
      // D3 权威校验：来源窗口必须就是本 iframe 的 contentWindow（防伪主闸）
      if (event.source !== contentWindow) return;
      // D3 纵深校验：沙箱 iframe 的源不透明，浏览器恒报 event.origin === 'null'；
      // 接受 'null'（沙箱）或 origin prop 白名单（非沙箱场景），拒绝其他任何
      // 真实（非不透明）来源 —— 阻止宿主页面内其他 frame / XSS 伪造消息。
      const isOpaqueSandboxOrigin = event.origin === 'null';
      if (!isOpaqueSandboxOrigin && event.origin !== latestRef.current.origin) return;
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
    setLoaded(false);
    setReadyState('loading');
    setError(null);
    setHeight(null);
  }, [options.src]);

  // init 重发兜底（WebKitGTK 实证）：iframe `load` 事件可早于插件安装
  // message 监听器（React effect 在 load 之后才跑），load 时发出的首个
  // init 会被丢弃 → 插件永不回 ready → 宿主永久 loading 转圈。
  // Chromium 的调度恰好先跑 effect 再派发 load，所以浏览器里不重现；
  // 协议未约定插件必须同步监听，宿主作为协议拥有方周期重发直至 ready。
  useEffect(() => {
    if (!loaded || readyRef.current) return;
    let retryCount = 0;
    const maxRetries = 25; // ~7.5s
    const timer = window.setInterval(() => {
      if (readyRef.current) {
        window.clearInterval(timer);
        return;
      }
      retryCount++;
      if (retryCount >= maxRetries) {
        window.clearInterval(timer);
        setError('Plugin panel initialization timed out (no ready signal received)');
        return;
      }
      const {
        config: cfg,
        variables: vars,
        hostApi: api,
        nodeType,
        locale,
        nodes,
      } = latestRef.current;
      sendToIframe(createInitMessage(cfg, vars, api, nodeType, locale, nodes));
    }, INIT_RETRY_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [loaded, options.src, sendToIframe]);

  return { iframeRef, readyState, error, height, handleIframeLoad, handleIframeError };
}
