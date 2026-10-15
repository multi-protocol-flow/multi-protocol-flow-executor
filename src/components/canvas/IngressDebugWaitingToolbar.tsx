/**
 * IngressDebugWaitingToolbar - Ingress 调试就绪状态工具栏
 * 当以 Debug 模式启动 Ingress 监听、等待外部真实 HTTP 请求打入触发断点时展示
 */
import { memo, useMemo, useState } from 'react';
import { Bug, Radio, Copy, Terminal, Square, Check } from 'lucide-react';
import { copyToClipboard } from '@/lib/clipboard';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import type { Flow } from '@/types/flow';
import { isHttpIngressConfig } from '@/types';

interface IngressDebugWaitingToolbarProps {
  port: number;
  flow: Flow | null;
  onStop: () => Promise<void> | void;
}

export const IngressDebugWaitingToolbar = memo(function IngressDebugWaitingToolbar({
  port,
  flow,
  onStop,
}: IngressDebugWaitingToolbarProps) {
  const ll = getLL();
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [copiedCurl, setCopiedCurl] = useState(false);

  // 解析 Ingress 节点的路径与允许的方法
  const { pathPattern, firstMethod } = useMemo(() => {
    const ingressNode = flow?.nodes.find((n) => n.type === 'http:ingress');
    if (ingressNode && isHttpIngressConfig(ingressNode.data)) {
      const rawPath = ingressNode.data.path_pattern || '/*';
      const cleanPath = rawPath.replace(/\/\*$/, '') || '/';
      const method = ingressNode.data.methods?.[0] || 'POST';
      return { pathPattern: cleanPath, firstMethod: method };
    }
    return { pathPattern: '/', firstMethod: 'POST' };
  }, [flow]);

  const targetUrl = `http://127.0.0.1:${port}${pathPattern === '/' ? '' : pathPattern}`;
  const curlCommand = `curl -X ${firstMethod} ${targetUrl} -H "Content-Type: application/json" -d '{"debug": true}'`;

  const handleCopyUrl = async () => {
    const ok = await copyToClipboard(targetUrl);
    if (ok) {
      setCopiedUrl(true);
      toast.success(`${ll.flow.ingress_debug_copy_url()}: ${targetUrl}`);
      setTimeout(() => setCopiedUrl(false), 2000);
    }
  };

  const handleCopyCurl = async () => {
    const ok = await copyToClipboard(curlCommand);
    if (ok) {
      setCopiedCurl(true);
      toast.success(ll.flow.ingress_debug_copy_curl(), {
        description: curlCommand,
      });
      setTimeout(() => setCopiedCurl(false), 2000);
    }
  };

  return (
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10 flex items-center gap-2 bg-background/95 backdrop-blur-md border border-indigo-500/40 dark:border-indigo-400/40 rounded-full px-3.5 py-1.5 shadow-xl transition-all duration-200">
      {/* 状态徽标与小呼吸灯 */}
      <div className="flex items-center gap-1.5 pl-1 shrink-0">
        <span className="relative flex h-2.5 w-2.5">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-indigo-400 opacity-75" />
          <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-indigo-500" />
        </span>
        <div className="flex items-center text-indigo-600 dark:text-indigo-400 font-medium text-xs gap-1 select-none">
          <Bug className="w-3.5 h-3.5" />
          <Radio className="w-3.5 h-3.5 animate-pulse" />
          <span>{ll.flow.ingress_debug_listening_title()}</span>
        </div>
      </div>

      <div className="w-px h-4 bg-border/60 mx-1 shrink-0" />

      {/* 监听 URL 与复制按钮 */}
      <div className="flex items-center gap-1 shrink-0">
        <code className="text-xs font-mono px-2 py-0.5 rounded bg-indigo-500/10 text-indigo-600 dark:text-indigo-300 border border-indigo-500/20 max-w-[220px] truncate select-all">
          {targetUrl}
        </code>
        <button
          type="button"
          onClick={handleCopyUrl}
          title={ll.flow.ingress_debug_copy_url()}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors cursor-pointer"
        >
          {copiedUrl ? (
            <Check className="w-3.5 h-3.5 text-emerald-500" />
          ) : (
            <Copy className="w-3.5 h-3.5" />
          )}
        </button>
        <button
          type="button"
          onClick={handleCopyCurl}
          title={ll.flow.ingress_debug_copy_curl()}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors cursor-pointer"
        >
          {copiedCurl ? (
            <Check className="w-3.5 h-3.5 text-emerald-500" />
          ) : (
            <Terminal className="w-3.5 h-3.5" />
          )}
        </button>
      </div>

      {/* 提示文案 */}
      <span className="hidden lg:inline-block text-[11px] text-muted-foreground max-w-[240px] truncate select-none px-1">
        {ll.flow.ingress_debug_waiting_hint()}
      </span>

      <div className="w-px h-4 bg-border/60 mx-1 shrink-0" />

      {/* 停止调试监听按钮 */}
      <button
        type="button"
        onClick={() => void onStop()}
        title={ll.flow.ingress_debug_stop()}
        className={cn(
          'flex items-center justify-center p-1.5 rounded-full',
          'text-red-500 hover:bg-red-500/10 hover:text-red-600',
          'transition-all duration-150 cursor-pointer shrink-0'
        )}
      >
        <Square className="w-3.5 h-3.5" fill="currentColor" stroke="none" />
      </button>
    </div>
  );
});
