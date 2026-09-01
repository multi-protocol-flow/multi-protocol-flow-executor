/**
 * AI Tool Call 徽章与参数展开展示项
 */

import { useState } from 'react';
import { Wrench, ChevronDown, ChevronRight, CheckCircle2 } from 'lucide-react';
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

  let formattedArgs = toolCall.function.arguments;
  try {
    const parsed = JSON.parse(toolCall.function.arguments);
    formattedArgs = JSON.stringify(parsed, null, 2);
  } catch {
    // 保持原文
  }

  return (
    <div className="my-1.5 rounded border border-border/60 bg-muted/40 text-xs">
      <button
        type="button"
        className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left hover:bg-muted/70 transition-colors"
        onClick={() => setIsOpen(!isOpen)}
      >
        <Wrench className="h-3.5 w-3.5 text-primary shrink-0" />
        <span className="font-mono font-medium text-foreground/90">
          {toolCall.function.name}
        </span>
        {isExecuted ? (
          <span className="ml-auto flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400">
            <CheckCircle2 className="h-3 w-3" />
            <span>{ll.ai.tool_call.done()}</span>
          </span>
        ) : (
          <span className="ml-auto text-[11px] text-muted-foreground animate-pulse">
            {ll.ai.tool_call.running()}
          </span>
        )}
        {isOpen ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        )}
      </button>

      {isOpen && (
        <div className="border-t border-border/40 p-2 bg-background/50 font-mono text-[11px] overflow-x-auto max-h-48 whitespace-pre-wrap break-all text-muted-foreground">
          {formattedArgs || '{}'}
        </div>
      )}
    </div>
  );
}
