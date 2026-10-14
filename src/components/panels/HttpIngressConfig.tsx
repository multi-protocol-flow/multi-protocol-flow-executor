/**
 * HttpIngressConfig - HTTP Ingress 监听节点配置面板
 * 配置外部 HTTP Mock 服务的端口、路由匹配模式及支持的 HTTP 方法
 */
import { useState, useEffect, useCallback, useRef } from 'react';
import { Radio, Globe, ShieldCheck } from 'lucide-react';
import type { HttpIngressConfig as HttpIngressConfigType } from '@/types';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

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
        <label className="flex items-center gap-1.5 font-medium text-foreground">
          <Radio className="w-3.5 h-3.5 text-indigo-500" />
          <span>{ll.panels.httpIngress.portLabel()}</span>
        </label>
        <input
          type="number"
          min={1024}
          max={65535}
          value={port}
          disabled={disabled}
          onChange={(e) => handlePortChange(parseInt(e.target.value, 10) || 9527)}
          className="w-full h-8 px-2.5 rounded border border-border bg-background font-mono focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
          placeholder="9527"
        />
        <p className="text-[11px] text-muted-foreground">
          {ll.panels.httpIngress.portHintBefore()}
          <code>http://127.0.0.1:{port}</code>
          {ll.panels.httpIngress.portHintAfter()}
        </p>
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
