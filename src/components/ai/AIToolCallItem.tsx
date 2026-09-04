/**
 * AI Tool Call 徽章与参数展开展示项
 */

import { useState, useCallback } from 'react';
import { Wrench, ChevronDown, ChevronRight, CheckCircle2, Copy, Check } from 'lucide-react';
import type { AIToolCall } from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';

interface AIToolCallItemProps {
  toolCall: AIToolCall;
  isExecuted?: boolean;
}

export function AIToolCallItem({
  toolCall,
  isExecuted = true,
}: AIToolCallItemProps) {
  const ll = getLL();
  const [isOpen, setIsOpen] = useState(false);
  const [hasCopied, setHasCopied] = useState(false);

  let formattedArgs = toolCall.function.arguments;
  try {
    const parsed = JSON.parse(toolCall.function.arguments);
    formattedArgs = JSON.stringify(parsed, null, 2);
  } catch {
    // 保持原文
  }

  const handleCopy = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      try {
        await navigator.clipboard.writeText(formattedArgs || '{}');
        setHasCopied(true);
        setTimeout(() => setHasCopied(false), 1500);
      } catch {
        // ignore clipboard error
      }
    },
    [formattedArgs],
  );

  return (
    <div className="my-1.5 w-full max-w-full overflow-hidden rounded border border-border/60 bg-muted/40 text-xs">
      <button
        type="button"
        className="flex w-full min-w-0 items-center gap-1.5 px-2.5 py-1.5 text-left hover:bg-muted/70 transition-colors"
        onClick={() => setIsOpen(!isOpen)}
      >
        <Wrench className="h-3.5 w-3.5 text-primary shrink-0" />
        <span
          className="font-mono font-medium text-foreground/90 truncate flex-1 min-w-0"
          title={toolCall.function.name}
        >
          {toolCall.function.name}
        </span>
        {isExecuted ? (
          <span className="ml-auto shrink-0 flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400 whitespace-nowrap pl-1">
            <CheckCircle2 className="h-3 w-3 shrink-0" />
            <span>{ll.ai.tool_call.done()}</span>
          </span>
        ) : (
          <span className="ml-auto shrink-0 text-[11px] text-muted-foreground animate-pulse whitespace-nowrap pl-1">
            {ll.ai.tool_call.running()}
          </span>
        )}
        {isOpen ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0 ml-0.5" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0 ml-0.5" />
        )}
      </button>

      {isOpen && (
        <div className="border-t border-border/40 bg-background/50 text-[11px]">
          <div className="flex items-center justify-between px-2.5 py-1 bg-muted/30 border-b border-border/30 text-[10px] text-muted-foreground font-mono">
            <span>{ll.ai.tool_call.call_args()}</span>
            <button
              type="button"
              className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors px-1 py-0.5 rounded hover:bg-muted"
              onClick={handleCopy}
              title={ll.ai.tool_call.copy_arguments()}
            >
              {hasCopied ? (
                <>
                  <Check className="h-2.5 w-2.5 text-emerald-500" />
                  <span className="text-emerald-500">{ll.ai.tool_call.copied()}</span>
                </>
              ) : (
                <>
                  <Copy className="h-2.5 w-2.5" />
                  <span>{ll.ai.tool_call.copy_arguments()}</span>
                </>
              )}
            </button>
          </div>
          <pre className="p-2 font-mono text-[11px] overflow-x-auto max-h-48 whitespace-pre-wrap break-all text-muted-foreground selection:bg-primary/20">
            {formattedArgs || '{}'}
          </pre>
        </div>
      )}
    </div>
  );
}
