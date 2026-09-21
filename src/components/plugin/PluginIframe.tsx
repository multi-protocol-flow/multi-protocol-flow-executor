/**
 * PluginIframe —— 插件 HTML 配置面板容器（沙箱 iframe）
 *
 * 渲染插件作者的 HTML 配置面板并桥接宿主：
 * - sandbox 仅 `allow-scripts`（D2，禁止 allow-same-origin → 不透明源，
 *   阻断插件 HTML 访问父 DOM / `window.__TAURI__`，RCE 防御核心。
 *   注意：不要模仿 `srcDoc + allow-same-origin` 模式）
 * - src 来自 `meta.frontend.url`（macOS/Linux `mpe-plugin://<id>.localhost/...`、
 *   Windows `http://mpe-plugin.<id>.localhost/...`）
 * - 双向 postMessage 桥由 usePluginIframe 管理（D3 origin + source 双校验）
 * - loading 遮罩 → ready 后消失；错误态显示错误 + 回退链接
 *
 * 通用性：props 与节点类型解耦，PR-4 在 NodeConfigPanel 检测
 * `meta?.frontend` 存在时直接渲染（传 `origin` 由 src 解析）。
 */
import { Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { usePluginIframe } from '@/hooks/usePluginIframe';
import type {
  PluginIframeHostApi,
  PluginIframeNodeSnapshot,
  PluginIframeTheme,
} from '@/types/plugin-iframe';

/** 插件未上报 resize 前的兜底高度（px） */
const DEFAULT_IFRAME_HEIGHT = 300;

/**
 * iframe 元素边框补偿（px）。
 *
 * 宿主全局 preflight 是 `box-sizing: border-box`，且本组件给 iframe 加了
 * `border`（1px）——因此元素高度等于「上报的内容高度」时，内部视口实际只有
 * `高度 - 2`。插件（如 grpc）上报 `document.body.scrollHeight` 后内容恒定
 * 溢出 2px → 内部出现一条几乎滚不动的死滚动条，滚轮只能滚宿主面板。
 * 补偿使内部视口 == 内容高度（WebKitGTK 实测：deClient == deScroll）。
 * 与下方 iframe 的 `border` class 强耦合：改边框宽度必须同步此常量。
 */
const IFRAME_CHROME_PX = 2;

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
  /** 当前编辑节点的类型 id（透传 init 载荷，如 `mcp:connect`） */
  nodeType: string;
  /** 宿主当前 UI 语言（BCP-47，透传 init 载荷） */
  locale: string;
  /** 宿主已解析主题（light/dark，透传 init 载荷并在变化时推 themeChanged） */
  theme: PluginIframeTheme;
  /** 同插件节点快照（可选；connection 下拉等跨节点引用用） */
  nodes?: PluginIframeNodeSnapshot[];
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
  nodeType,
  locale,
  theme,
  nodes,
  onConfigChange,
  onRequestAction,
  title,
  className,
}: PluginIframeProps) {
  const { iframeRef, readyState, error, height, handleIframeLoad, handleIframeError } =
    usePluginIframe({
      src,
      origin,
      config,
      variables,
      hostApi,
      nodeType,
      locale,
      theme,
      nodes,
      onConfigChange,
      onRequestAction,
    });

  // 错误态：显示错误 + 回退链接（不渲染 iframe，沙箱内不再加载内容）
  if (error) {
    return (
      <div
        role="alert"
        className={cn('border rounded p-4 bg-muted/50 text-sm space-y-2', className)}
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
    <div className={cn('relative', className)}>
      <iframe
        ref={iframeRef}
        src={src}
        title={title ?? 'Plugin config panel'}
        // D2：仅 allow-scripts —— 不透明源，禁 allow-same-origin（RCE 防御）
        sandbox="allow-scripts"
        className="w-full border rounded bg-background"
        style={{ height: (height ?? DEFAULT_IFRAME_HEIGHT) + IFRAME_CHROME_PX }}
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
