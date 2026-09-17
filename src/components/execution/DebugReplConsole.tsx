/**
 * DebugReplConsole - 断点即时 REPL 求值器 (Debug Scratchpad)
 * 在调试断点暂停时提供即时 JavaScript 与 JSONPath 变量求值与测试沙箱
 */
import { useState, useRef, useEffect, useCallback } from 'react';
import { Terminal, Play, Trash2, Copy, ChevronUp, ChevronDown, Check, Code, Braces } from 'lucide-react';
import { useDebugStore } from '@/stores/debug';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

type ReplMode = 'javascript' | 'jsonpath';

interface ReplHistoryEntry {
  id: string;
  expr: string;
  mode: ReplMode;
  result?: unknown;
  error?: string;
  timestamp: number;
}

/**
 * JSONPath 安全求值器
 */
export function evaluateJsonPath(data: unknown, path: string): unknown {
  if (!path || path === '$') return data;
  const cleanPath = path.startsWith('$.') ? path.slice(2) : path.startsWith('$') ? path.slice(1) : path;
  if (!cleanPath) return data;

  const parts = cleanPath.replace(/\[(\d+)\]/g, '.$1').split('.').filter(Boolean);
  let current: unknown = data;
  for (const part of parts) {
    if (current === null || current === undefined) return undefined;
    if (typeof current === 'object') {
      current = (current as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return current;
}

/**
 * JavaScript 安全求值沙箱（传入当前 variables 与 $）
 */
export function evaluateJsExpression(expr: string, variables: Record<string, unknown>): unknown {
  const fn = new Function('variables', '$', `with(variables) { return (${expr}); }`);
  return fn(variables, variables);
}

export function DebugReplConsole() {
  const ll = getLL();
  const isPaused = useDebugStore((s) => s.status === 'paused');
  const variables = useDebugStore((s) => s.variables);

  const [isExpanded, setIsExpanded] = useState(false);
  const [mode, setMode] = useState<ReplMode>('javascript');
  const [inputExpr, setInputExpr] = useState('');
  const [history, setHistory] = useState<ReplHistoryEntry[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const historyListRef = useRef<HTMLDivElement>(null);

  // 自动滚动到最新历史
  useEffect(() => {
    if (isExpanded && historyListRef.current) {
      historyListRef.current.scrollTop = historyListRef.current.scrollHeight;
    }
  }, [history, isExpanded]);

  const handleEvaluate = useCallback(() => {
    const trimmed = inputExpr.trim();
    if (!trimmed) return;

    const entryId = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const effectiveMode: ReplMode = trimmed.startsWith('$') ? 'jsonpath' : mode;

    let result: unknown;
    let error: string | undefined;

    try {
      if (effectiveMode === 'jsonpath') {
        result = evaluateJsonPath(variables, trimmed);
      } else {
        result = evaluateJsExpression(trimmed, variables);
      }
    } catch (err) {
      error = (err as Error).message || String(err);
    }

    setHistory((prev) => [
      ...prev,
      {
        id: entryId,
        expr: trimmed,
        mode: effectiveMode,
        result,
        error,
        timestamp: Date.now(),
      },
    ]);

    setInputExpr('');
    setHistoryIndex(null);
  }, [inputExpr, mode, variables]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleEvaluate();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length === 0) return;
      const nextIdx = historyIndex === null ? history.length - 1 : Math.max(0, historyIndex - 1);
      setHistoryIndex(nextIdx);
      setInputExpr(history[nextIdx].expr);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex === null) return;
      const nextIdx = historyIndex + 1;
      if (nextIdx < history.length) {
        setHistoryIndex(nextIdx);
        setInputExpr(history[nextIdx].expr);
      } else {
        setHistoryIndex(null);
        setInputExpr('');
      }
    }
  };

  const handleCopyExtraction = async (entry: ReplHistoryEntry) => {
    // 转换为 {{expr}} 模板语法
    let templateVar = entry.expr.trim();
    if (templateVar.startsWith('$.')) {
      templateVar = templateVar.slice(2);
    } else if (templateVar.startsWith('$')) {
      templateVar = templateVar.slice(1);
    }
    const extractExpr = `{{${templateVar}}}`;

    try {
      await navigator.clipboard.writeText(extractExpr);
      setCopiedId(entry.id);
      setTimeout(() => setCopiedId(null), 2000);
      toast.success(`${ll.flow.debug_repl_copy_success()}: ${extractExpr}`);
    } catch {
      toast.error(ll.execution.report.copy_failed());
    }
  };

  return (
    <div className="border-t border-border bg-card flex flex-col shrink-0 transition-all duration-200">
      {/* 折叠/展开 标题栏 */}
      <div
        className={cn(
          'flex items-center justify-between px-3 py-1.5 cursor-pointer select-none text-xs border-b border-border/60 hover:bg-muted/40 transition-colors',
          isExpanded ? 'bg-muted/30' : 'bg-card'
        )}
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <Terminal className="w-3.5 h-3.5 text-teal-500" />
          <span className="font-medium text-foreground">{ll.flow.debug_repl_title()}</span>

          <span
            className={cn(
              'px-1.5 py-0.2 rounded text-[10px] font-mono',
              isPaused
                ? 'bg-green-500/10 text-green-700 dark:text-green-400 border border-green-300 dark:border-green-800'
                : 'bg-muted text-muted-foreground border border-border'
            )}
          >
            {isPaused ? ll.flow.debug_repl_paused_ready() : ll.flow.debug_repl_running_disabled()}
          </span>
        </div>

        <div className="flex items-center gap-1">
          {isExpanded ? (
            <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
          ) : (
            <ChevronUp className="w-3.5 h-3.5 text-muted-foreground" />
          )}
        </div>
      </div>

      {/* 展开后的控制台区域 */}
      {isExpanded && (
        <div className="h-56 flex flex-col bg-background/80">
          {/* 输出记录区域 */}
          <div ref={historyListRef} className="flex-1 overflow-y-auto p-3 space-y-3 font-mono text-xs">
            {history.length === 0 ? (
              <div className="h-full flex items-center justify-center text-muted-foreground/60 text-xs">
                {isPaused
                  ? ll.flow.debug_repl_placeholder()
                  : ll.flow.debug_repl_running_disabled()}
              </div>
            ) : (
              history.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded border border-border bg-card/60 p-2 text-xs space-y-1 group hover:border-border transition-colors shadow-2xs"
                >
                  {/* 输入头 */}
                  <div className="flex items-center justify-between text-muted-foreground">
                    <div className="flex items-center gap-1.5">
                      <span className="px-1 rounded text-[10px] bg-muted uppercase font-sans">
                        {entry.mode === 'jsonpath' ? 'JSONPath' : 'JS'}
                      </span>
                      <span className="text-foreground font-semibold">&gt; {entry.expr}</span>
                    </div>

                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={() => handleCopyExtraction(entry)}
                        className="px-1.5 py-0.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground text-[10px] flex items-center gap-1"
                        title={ll.flow.debug_repl_copy_expr()}
                      >
                        {copiedId === entry.id ? (
                          <Check className="w-3 h-3 text-green-500" />
                        ) : (
                          <Copy className="w-3 h-3" />
                        )}
                        <span>{ll.flow.debug_repl_copy_expr()}</span>
                      </button>
                    </div>
                  </div>

                  {/* 求值输出 */}
                  {entry.error ? (
                    <div className="text-rose-600 dark:text-rose-400 font-sans text-[11px] bg-rose-50/50 dark:bg-rose-950/20 rounded p-1.5 border border-rose-200 dark:border-rose-900/50">
                      {entry.error}
                    </div>
                  ) : (
                    <pre className="text-emerald-700 dark:text-emerald-400 text-xs overflow-x-auto p-1.5 bg-muted/40 rounded max-h-32">
                      {entry.result === undefined
                        ? 'undefined'
                        : typeof entry.result === 'string'
                        ? `"${entry.result}"`
                        : JSON.stringify(entry.result, null, 2)}
                    </pre>
                  )}
                </div>
              ))
            )}
          </div>

          {/* 底部输入操作栏 */}
          <div className="p-2 border-t border-border bg-card/90 flex items-center gap-2">
            {/* 模式选择 */}
            <div className="flex rounded border border-border bg-muted/50 p-0.5 shrink-0">
              <button
                type="button"
                onClick={() => setMode('javascript')}
                className={cn(
                  'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                  mode === 'javascript' ? 'bg-background shadow-xs text-foreground' : 'text-muted-foreground hover:text-foreground'
                )}
                title={ll.flow.debug_repl_mode_js()}
              >
                <Code className="w-3 h-3" />
                <span>JS</span>
              </button>
              <button
                type="button"
                onClick={() => setMode('jsonpath')}
                className={cn(
                  'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                  mode === 'jsonpath' ? 'bg-background shadow-xs text-foreground' : 'text-muted-foreground hover:text-foreground'
                )}
                title={ll.flow.debug_repl_mode_jsonpath()}
              >
                <Braces className="w-3 h-3" />
                <span>JSONPath</span>
              </button>
            </div>

            {/* 表达式输入框 */}
            <input
              ref={inputRef}
              type="text"
              value={inputExpr}
              onChange={(e) => setInputExpr(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={!isPaused}
              placeholder={
                isPaused
                  ? ll.flow.debug_repl_placeholder()
                  : ll.flow.debug_repl_running_disabled()
              }
              className="flex-1 h-7 px-2 text-xs font-mono rounded border border-border bg-background placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50 disabled:cursor-not-allowed"
            />

            {/* 求值按钮 */}
            <button
              onClick={handleEvaluate}
              disabled={!isPaused || !inputExpr.trim()}
              className="h-7 px-3 text-xs rounded bg-primary text-primary-foreground font-medium hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1 shrink-0 shadow-xs"
            >
              <Play className="w-3 h-3" />
              <span>{ll.flow.debug_repl_eval_btn()}</span>
            </button>

            {/* 清空历史按钮 */}
            {history.length > 0 && (
              <button
                onClick={() => setHistory([])}
                className="h-7 w-7 flex items-center justify-center rounded border border-border hover:bg-muted text-muted-foreground hover:text-foreground shrink-0"
                title={ll.flow.debug_repl_clear_btn()}
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
