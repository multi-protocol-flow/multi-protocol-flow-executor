/**
 * HttpIngressConfig - HTTP Ingress 监听节点配置面板
 * 配置外部 HTTP Mock 服务的端口、路由匹配模式及支持的 HTTP 方法
 */
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Radio, Globe, ShieldCheck, Copy, Check } from 'lucide-react';
import type { HttpIngressConfig as HttpIngressConfigType } from '@/types';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { Badge } from '@/components/ui/badge';
import { useSettingsStore } from '@/stores/settings';
import { useFlowStore } from '@/stores/flow';

interface HttpIngressConfigProps {
  data: HttpIngressConfigType;
  onSave: (data: HttpIngressConfigType) => void;
  disabled?: boolean;
}

const COMMON_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'OPTIONS'];

export function HttpIngressConfig({ data, onSave, disabled = false }: HttpIngressConfigProps) {
  const ll = getLL();

  const [port, setPort] = useState<number>(data.port ?? 9527);
  const [pathPattern, setPathPattern] = useState<string>(data.path_pattern ?? '/*');
  const [methods, setMethods] = useState<string[]>(data.methods ?? ['GET', 'POST']);
  const [copied, setCopied] = useState(false);

  const defaultIngressPort = useSettingsStore((s) => s.defaultIngressPort) ?? 9527;
  const flow = useFlowStore((s) => s.flow);

  const flowSlug = useMemo(() => {
    if (!flow) return 'flow';
    const nameSlug = flow.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    if (nameSlug) return nameSlug;
    if (flow.uuid && flow.uuid.length <= 36 && !flow.uuid.includes(' ') && /^[a-zA-Z0-9_-]+$/.test(flow.uuid)) {
      return flow.uuid.toLowerCase();
    }
    return flow.uuid.length >= 8 ? flow.uuid.slice(0, 8) : 'flow';
  }, [flow]);

  const isSharedGateway = port === defaultIngressPort;
  const normalizedPath = pathPattern.startsWith('/') ? pathPattern : '/' + pathPattern;
  const effectiveUrl = isSharedGateway
    ? `http://127.0.0.1:${port}/${flowSlug}${normalizedPath === '/*' ? '' : normalizedPath}`
    : `http://127.0.0.1:${port}${normalizedPath}`;

  // 数据变更防抖同步
  const onSaveRef = useRef(onSave);
  useEffect(() => {
    onSaveRef.current = onSave;
  }, [onSave]);

  const syncChanges = useCallback(
    (newPort: number, newPath: string, newMethods: string[]) => {
      onSaveRef.current({
        type: 'http:ingress',
        port: newPort,
        path_pattern: newPath,
        methods: newMethods,
      });
    },
    []
  );

  const handlePortChange = (val: number) => {
    setPort(val);
    syncChanges(val, pathPattern, methods);
  };

  const handlePathChange = (val: string) => {
    setPathPattern(val);
    syncChanges(port, val, methods);
  };

  const toggleMethod = (method: string) => {
    let next: string[];
    if (methods.includes(method)) {
      next = methods.filter((m) => m !== method);
    } else {
      next = [...methods, method];
    }
    setMethods(next);
    syncChanges(port, pathPattern, next);
  };

  return (
    <div className="w-full space-y-4 text-xs">
      {/* 端口配置 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="flex items-center gap-1.5 font-medium text-foreground">
            <Radio className="w-3.5 h-3.5 text-indigo-500" />
            <span>{ll.panels.httpIngress.portLabel()}</span>
          </label>
          <Badge
            variant="outline"
            className={cn(
              'text-[10px] font-normal px-2 py-0.5',
              isSharedGateway
                ? 'bg-indigo-500/10 text-indigo-500 border-indigo-500/30'
                : 'bg-emerald-500/10 text-emerald-500 border-emerald-500/30'
            )}
          >
            {isSharedGateway
              ? ll.panels.httpIngress.sharedGatewayBadge()
              : ll.panels.httpIngress.customPortBadge()}
          </Badge>
        </div>
        <input
          type="number"
          min={1024}
          max={65535}
          value={port}
          disabled={disabled}
          onChange={(e) => handlePortChange(parseInt(e.target.value, 10) || defaultIngressPort)}
          className="w-full h-8 px-2.5 rounded border border-border bg-background font-mono focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
          placeholder={String(defaultIngressPort)}
        />
        <div className="p-2.5 rounded-lg bg-muted/40 border border-border/70 space-y-2">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground font-medium">
            <span>{ll.panels.httpIngress.urlPreviewLabel()}:</span>
            <button
              type="button"
              onClick={() => {
                navigator.clipboard.writeText(effectiveUrl);
                setCopied(true);
                setTimeout(() => setCopied(false), 2000);
              }}
              className="flex items-center gap-1 text-[10px] text-indigo-500 hover:text-indigo-600 transition-colors cursor-pointer px-1.5 py-0.5 rounded hover:bg-indigo-500/10"
              title={ll.settings.copy_path()}
            >
              {copied ? (
                <>
                  <Check className="w-3 h-3 text-emerald-500" />
                  <span className="text-emerald-500">{ll.settings.path_copied()}</span>
                </>
              ) : (
                <>
                  <Copy className="w-3 h-3" />
                  <span>{ll.settings.copy_path()}</span>
                </>
              )}
            </button>
          </div>
          <div className="flex items-center justify-between gap-2 p-1.5 rounded bg-background border border-border/60">
            <code className="text-xs font-mono text-foreground font-medium block truncate select-all flex-1">
              {effectiveUrl}
            </code>
          </div>
          <p className="text-[10px] text-muted-foreground leading-relaxed pt-0.5">
            {isSharedGateway
              ? ll.panels.httpIngress.sharedGatewayHint()
              : ll.panels.httpIngress.customPortHint()}
          </p>
        </div>
      </div>

      {/* 路由路径匹配 */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 font-medium text-foreground">
          <Globe className="w-3.5 h-3.5 text-indigo-500" />
          <span>{ll.panels.httpIngress.pathLabel()}</span>
        </label>
        <input
          type="text"
          value={pathPattern}
          disabled={disabled}
          onChange={(e) => handlePathChange(e.target.value)}
          className="w-full h-8 px-2.5 rounded border border-border bg-background font-mono focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
          placeholder={ll.panels.httpIngress.pathPlaceholder()}
        />
        <p className="text-[11px] text-muted-foreground">
          {ll.panels.httpIngress.pathHintBefore()}
          <code>/*</code>
          {ll.panels.httpIngress.pathHintMiddle()}
          <code>/pay/*</code>
          {ll.panels.httpIngress.pathHintAfter()}
        </p>
      </div>

      {/* HTTP 方法选择 */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 font-medium text-foreground">
          <ShieldCheck className="w-3.5 h-3.5 text-indigo-500" />
          <span>{ll.panels.httpIngress.methodsLabel()}</span>
        </label>
        <div className="flex flex-wrap gap-2 pt-1">
          {COMMON_METHODS.map((m) => {
            const isSelected = methods.includes(m);
            return (
              <button
                key={m}
                type="button"
                disabled={disabled}
                onClick={() => toggleMethod(m)}
                className={cn(
                  'px-2.5 py-1 rounded border text-xs font-mono font-medium transition-colors',
                  isSelected
                    ? 'border-indigo-500 bg-indigo-500/10 text-indigo-700 dark:text-indigo-300'
                    : 'border-border bg-muted/40 text-muted-foreground hover:text-foreground'
                )}
              >
                {m}
              </button>
            );
          })}
        </div>
      </div>

      {/* 请求入参注入说明提示卡 */}
      <div className="rounded-md border border-indigo-500/20 bg-indigo-500/5 p-3 space-y-1.5 text-muted-foreground">
        <p className="font-semibold text-foreground flex items-center gap-1">
          <span>{ll.panels.httpIngress.injectTitle()}</span>
        </p>
        <p className="text-[11px] leading-relaxed">
          {ll.panels.httpIngress.injectDesc()}
        </p>
        <ul className="list-disc list-inside space-y-0.5 text-[11px] font-mono text-foreground/80">
          <li><code>{"{{request.method}}"}</code> — {ll.panels.httpIngress.injectMethod()}</li>
          <li><code>{"{{request.path}}"}</code> — {ll.panels.httpIngress.injectPath()}</li>
          <li><code>{"{{request.query}}"}</code> — {ll.panels.httpIngress.injectQuery()}</li>
          <li><code>{"{{request.headers}}"}</code> — {ll.panels.httpIngress.injectHeaders()}</li>
          <li><code>{"{{request.body}}"}</code> — {ll.panels.httpIngress.injectBody()}</li>
        </ul>
      </div>
    </div>
  );
}
