/**
 * PluginIframe —— 插件 HTML 配置面板容器（沙箱 iframe）
 *
 * 渲染插件作者的 HTML 配置面板并桥接宿主：
 * - sandbox 仅 `allow-scripts`（D2，禁止 allow-same-origin → 不透明源，
 *   阻断插件 HTML 访问父 DOM / `window.__TAURI__`，RCE 防御核心。
 *   注意：不要模仿 EmailViewer 的 `srcDoc + allow-same-origin` 模式）
 * - src 来自 `meta.frontend.url`（macOS/Linux `mpe-plugin://<id>.localhost/...`、
 *   Windows `http://mpe-plugin.<id>.localhost/...`）
 * - 双向 postMessage 桥由 usePluginIframe 管理（D3 origin + source 双校验）
 * - loading 遮罩 → ready 后消失；错误态显示错误 + 回退链接
 *
 * 通用性：props 与节点类型解耦，PR-4 在 NodeConfigPanel 检测
 * `meta?.frontend` 存在时直接渲染（传 `origin` 由 src 解析）。
 */
import { Loader2 } from 'lucide-react';
import { usePluginIframe } from '@/hooks/usePluginIframe';
import type { PluginIframeHostApi } from '@/types/plugin-iframe';

/** 插件未上报 resize 前的兜底高度（px） */
const DEFAULT_IFRAME_HEIGHT = 300;

export interface PluginIframeProps {
  /** iframe URL（`meta.frontend.url`） */
  src: string;
  /** 插件 origin（postMessage 目标 + 入站校验白名单），由 src 解析 */
  origin: string;
  /** 当前节点配置 */
  config: unknown;
  /** 变量快照（宿主侧组装） */
  variables: Record<string, unknown>;
  /** host_api 能力声明（PR-5 白名单） */
  hostApi: PluginIframeHostApi;
  /** 插件表单变更 → 宿主保存 */
  onConfigChange: (config: unknown) => void;
  /** 插件请求宿主能力（testConnection 等，PR-5 白名单） */
  onRequestAction?: (action: string, params?: unknown) => Promise<unknown>;
  /** iframe title（可访问性），默认 "Plugin config panel" */
  title?: string;
  /** 外层容器 className */
  className?: string;
}

export function PluginIframe({
  src,
  origin,
  config,
  variables,
  hostApi,
  onConfigChange,
  onRequestAction,
  title,
  className,
}: PluginIframeProps) {
  const { iframeRef, readyState, error, height, handleIframeLoad, handleIframeError } =
    usePluginIframe({ src, origin, config, variables, hostApi, onConfigChange, onRequestAction });

  // 错误态：显示错误 + 回退链接（不渲染 iframe，沙箱内不再加载内容）
  if (error) {
    return (
      <div
        role="alert"
        className={`border rounded p-4 bg-muted/50 text-sm space-y-2 ${className ?? ''}`}
      >
        <p className="font-medium text-foreground">Plugin panel failed to load</p>
        <p className="text-xs text-muted-foreground break-all">{error}</p>
        <a
          href={src}
          target="_blank"
          rel="noreferrer"
          className="text-xs text-blue-600 hover:underline"
        >
          Open plugin panel in browser
        </a>
        {/* TODO(i18n): PR-3 按计划保持内联英文文案；如需本地化，新增 i18n keys 走 getLL() 模式 */}
      </div>
    );
  }

  return (
    <div className={`relative ${className ?? ''}`}>
      <iframe
        ref={iframeRef}
        src={src}
        title={title ?? 'Plugin config panel'}
        // D2：仅 allow-scripts —— 不透明源，禁 allow-same-origin（RCE 防御）
        sandbox="allow-scripts"
        className="w-full border rounded bg-background"
        style={{ height: height ?? DEFAULT_IFRAME_HEIGHT }}
        onLoad={handleIframeLoad}
        onError={handleIframeError}
      />
      {readyState === 'loading' && (
        <div className="absolute inset-0 flex items-center justify-center bg-background/80">
          <Loader2 className="w-5 h-5 animate-spin text-foreground" />
        </div>
      )}
    </div>
  );
}
