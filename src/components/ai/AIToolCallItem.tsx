/**
 * AI Tool Call 徽章、参数与执行详情展示项
 */

import { useState, useCallback, useMemo, memo } from 'react';
import {
  Wrench,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Copy,
  Check,
  Loader2,
} from 'lucide-react';
import type { AIToolCall } from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';
import { formatToolTitle } from '@/services/ai/toolTitleFormatter';

interface AIToolCallItemProps {
  toolCall: AIToolCall;
  result?: string;
  isExecuted?: boolean;
}

export const AIToolCallItem = memo(function AIToolCallItem({
  toolCall,
  result,
  isExecuted = true,
}: AIToolCallItemProps) {
  const ll = getLL();
  const [isOpen, setIsOpen] = useState(false);
  const [hasCopied, setHasCopied] = useState(false);
  // 默认 tab：若未传 result（如纯参数展示测试或正在执行中），默认显示 args；若已有结果，默认显示 result
  const [activeTab, setActiveTab] = useState<'args' | 'result'>(
    result !== undefined ? 'result' : 'args',
  );

  const rawResult = result ?? toolCall.output;
  const titleInfo = useMemo(
    () =>
      formatToolTitle(
        toolCall.function.name,
        toolCall.function.arguments,
        rawResult,
      ),
    [toolCall.function.name, toolCall.function.arguments, rawResult],
  );

  const {
    formattedArgs,
    formattedResult,
    isResultSuccess,
    summaryMessage,
    errorMessage,
  } = useMemo(() => {
    let args = toolCall.function.arguments;
    try {
      const parsed = JSON.parse(toolCall.function.arguments);
      args = JSON.stringify(parsed, null, 2);
    } catch {
      // 保持原文
    }

    let formatted = rawResult || '';
    let success = true;
    let summary: string | null = null;
    let error: string | null = null;

    if (rawResult) {
      try {
        const parsedResult = JSON.parse(rawResult);
        formatted = JSON.stringify(parsedResult, null, 2);
        if (parsedResult && typeof parsedResult === 'object') {
          if (parsedResult.success === false || parsedResult.error) {
            success = false;
            error =
              typeof parsedResult.error === 'string'
                ? parsedResult.error
                : typeof parsedResult.message === 'string'
                  ? parsedResult.message
                  : null;
          } else if (typeof parsedResult.message === 'string') {
            summary = parsedResult.message;
          } else if (typeof parsedResult.summary === 'string') {
            summary = parsedResult.summary;
          }
        }
      } catch {
        formatted = rawResult;
      }
    }

    return {
      formattedArgs: args,
      formattedResult: formatted,
      isResultSuccess: success,
      summaryMessage: summary,
      errorMessage: error,
    };
  }, [toolCall.function.arguments, rawResult]);
  const handleCopy = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      const contentToCopy = activeTab === 'args' ? formattedArgs : formattedResult;
      try {
        await navigator.clipboard.writeText(contentToCopy);
        setHasCopied(true);
        setTimeout(() => setHasCopied(false), 2000);
      } catch {
        // ignore clipboard error
      }
    },
    [activeTab, formattedArgs, formattedResult],
  );

  return (
    <div className="my-1.5 w-full max-w-full overflow-hidden rounded-lg border border-border/60 bg-muted/40 text-xs shadow-2xs">
      <button
        className="flex w-full min-w-0 items-center gap-1.5 px-2.5 py-1.5 text-left hover:bg-muted/70 transition-colors"
        onClick={() => setIsOpen(!isOpen)}
        title={titleInfo.fullTitle}
      >
        <Wrench className="h-3.5 w-3.5 text-primary shrink-0" />
        <div className="flex items-center gap-1 min-w-0 flex-1 overflow-hidden">
          {/* 工具名：始终优先展示，不被挤压 */}
          <span
            className={`font-semibold text-foreground/95 text-[11px] truncate min-w-0 ${
              titleInfo.target ? 'shrink-0' : 'flex-1'
            }`}
            title={toolCall.function.name}
          >
            {titleInfo.toolDisplayName}
          </span>

          {/* 冒号与目标名：左对齐紧挨工具名；面板变窄时优先在此处收缩截断 */}
          {titleInfo.target && (
            <span className="flex items-center gap-0.5 min-w-0 flex-1 overflow-hidden">
              <span className="text-muted-foreground/60 shrink-0 font-medium text-[11px]">:</span>
              <span
                className="font-mono text-primary bg-primary/10 dark:bg-primary/15 px-1.5 py-0.5 rounded text-[10.5px] truncate min-w-0 border border-primary/20 font-medium"
                title={titleInfo.target}
              >
                {titleInfo.target}
              </span>
            </span>
          )}

          {/* 隐藏工具函数名仅在国际化名不同时渲染，防止重复匹配 */}
          {titleInfo.toolDisplayName !== toolCall.function.name && (
            <span className="sr-only">{toolCall.function.name}</span>
          )}
        </div>
        {/* 状态徽章 */}
        {!isExecuted ? (
          <span className="ml-auto shrink-0 flex items-center gap-1 text-[11px] text-muted-foreground whitespace-nowrap pl-1">
            <Loader2 className="h-3 w-3 animate-spin text-primary shrink-0" />
            <span className="shrink-0 whitespace-nowrap">{ll.ai.tool_call.running()}</span>
          </span>
        ) : !isResultSuccess ? (
          <span className="ml-auto shrink-0 flex items-center gap-1 text-[11px] text-destructive whitespace-nowrap pl-1">
            <XCircle className="h-3 w-3 shrink-0 text-destructive" />
            <span>{ll.ai.tool_call.failed()}</span>
          </span>
        ) : (
          <span className="ml-auto shrink-0 flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400 whitespace-nowrap pl-1">
            <CheckCircle2 className="h-3 w-3 shrink-0" />
            <span>{ll.ai.tool_call.done()}</span>
          </span>
        )}

        {isOpen ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0 ml-0.5" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0 ml-0.5" />
        )}
      </button>

      {isOpen && (
        <div className="border-t border-border/40 bg-background/60 text-[11px]">
          {/* Tab 切换栏与一键复制 */}
          <div className="flex items-center justify-between px-2.5 py-1 bg-muted/30 border-b border-border/30 text-[10px] text-muted-foreground">
            <div className="flex items-center gap-2">
              <div className="flex items-center gap-1 p-0.5 bg-muted/60 rounded border border-border/40">
              <button
                type="button"
                onClick={() => setActiveTab('args')}
                className={`px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors ${
                  activeTab === 'args'
                    ? 'bg-background text-foreground shadow-2xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {ll.ai.tool_call.call_args()}
              </button>

              <button
                type="button"
                onClick={() => setActiveTab('result')}
                className={`px-2 py-0.5 rounded text-[10.5px] font-medium flex items-center gap-1 transition-colors ${
                  activeTab === 'result'
                    ? 'bg-background text-foreground shadow-2xs font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                <span>{ll.ai.tool_call.execution_result()}</span>
                {isExecuted && (
                  <span
                    className={`h-1.5 w-1.5 rounded-full ${
                      !isResultSuccess ? 'bg-destructive' : 'bg-emerald-500'
                    }`}
                  />
                )}
              </button>
              </div>
              <span
                className="font-mono text-[10.5px] text-muted-foreground/75 hidden sm:inline"
                title={toolCall.function.name}
              >
                ({toolCall.function.name})
              </span>
            </div>
            <button
              type="button"
              className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors px-1.5 py-0.5 rounded hover:bg-muted"
              onClick={handleCopy}
              title={
                activeTab === 'args'
                  ? ll.ai.tool_call.copy_arguments()
                  : ll.ai.tool_call.copy_result()
              }
            >
              {hasCopied ? (
                <>
                  <Check className="h-2.5 w-2.5 text-emerald-500" />
                  <span className="text-emerald-500">{ll.ai.tool_call.copied()}</span>
                </>
              ) : (
                <>
                  <Copy className="h-2.5 w-2.5" />
                  <span>
                    {activeTab === 'args'
                      ? ll.ai.tool_call.copy_arguments()
                      : ll.ai.tool_call.copy_result()}
                  </span>
                </>
              )}
            </button>
          </div>

          {/* Tab 内容区 */}
          {activeTab === 'args' ? (
            <pre className="p-2.5 font-mono text-[11px] overflow-x-auto max-h-52 whitespace-pre-wrap break-all text-muted-foreground selection:bg-primary/20">
              {formattedArgs || '{}'}
            </pre>
          ) : (
            <div>
              {!isExecuted ? (
                <div className="p-4 flex items-center justify-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />
                  <span>{ll.ai.tool_call.running()}</span>
                </div>
              ) : (
                <>
                  {errorMessage && (
                    <div className="m-2 p-2 rounded-md bg-destructive/10 border border-destructive/25 text-destructive text-[11px] flex items-start gap-1.5 leading-snug">
                      <AlertCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                      <span className="break-all font-mono">{errorMessage}</span>
                    </div>
                  )}

                  {summaryMessage && (
                    <div className="m-2 p-2 rounded-md bg-emerald-500/10 border border-emerald-500/25 text-emerald-600 dark:text-emerald-400 text-[11px] flex items-start gap-1.5 leading-snug">
                      <CheckCircle2 className="h-3.5 w-3.5 shrink-0 mt-0.5 text-emerald-500" />
                      <span className="break-words">{summaryMessage}</span>
                    </div>
                  )}

                  <pre className="p-2.5 font-mono text-[11px] overflow-x-auto max-h-52 whitespace-pre-wrap break-all text-muted-foreground selection:bg-primary/20">
                    {formattedResult || ll.ai.tool_call.no_result()}
                  </pre>
                </>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
});
