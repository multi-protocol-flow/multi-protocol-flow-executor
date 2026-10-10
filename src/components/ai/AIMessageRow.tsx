/**
 * AI 单条消息气泡组件 (Memoized)
 * 隔离单条历史消息与深度思考过程的重绘，避免由于父组件按键输入或外部状态更新导致全量渲染
 */

import { memo } from 'react';
import {
  Bot,
  User,
  Zap,
  Brain,
  ChevronDown,
  ChevronRight,
  Loader2,
  AlertCircle,
} from 'lucide-react';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { AIToolCallItem } from './AIToolCallItem';
import { AIErrorMessageCard } from './AIErrorMessageCard';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIMessage } from '@/types/ai';

export interface AIMessageRowProps {
  message: AIMessage;
  isStreaming?: boolean;
  isGenerating?: boolean;
  isLatestAssistant?: boolean;
  isThoughtOpen?: boolean;
  toolResults?: Record<string, string>;
  activeStreamingThoughtRef?: React.RefObject<HTMLDivElement | null>;
  onToggleThought?: (messageId: string) => void;
  onPreviewImage?: (imageUrl: string) => void;
}

export const AIMessageRow = memo(function AIMessageRow({
  message: msg,
  isStreaming = false,
  isGenerating = false,
  isLatestAssistant = false,
  isThoughtOpen,
  toolResults,
  activeStreamingThoughtRef,
  onToggleThought,
  onPreviewImage,
}: AIMessageRowProps) {
  const ll = getLL();

  if (msg.role === 'system') return null;

  // 用户消息渲染
  if (msg.role === 'user') {
    return (
      <div className="flex justify-end gap-2 pl-6">
        <div className="max-w-[88%] space-y-1.5">
          {/* 用户附加的图片展示 */}
          {msg.images && msg.images.length > 0 && (
            <div className="flex flex-wrap justify-end gap-1.5">
              {msg.images.map((imgSrc, idx) => (
                <button
                  key={idx}
                  type="button"
                  className="overflow-hidden rounded-lg border border-primary/30 max-w-[180px] max-h-[140px] hover:opacity-90 transition-opacity cursor-zoom-in"
                  onClick={() => onPreviewImage?.(imgSrc)}
                >
                  <img
                    src={imgSrc}
                    alt={`Upload ${idx + 1}`}
                    className="w-full h-full object-cover rounded-lg"
                  />
                </button>
              ))}
            </div>
          )}
          {msg.isCorrection && (
            <div className="flex items-center justify-end gap-1 text-[10px] text-amber-500 font-medium pb-0.5">
              <Zap className="h-2.5 w-2.5 fill-current" />
              <span>{ll.ai.copilot.steer_badge()}</span>
            </div>
          )}

          {msg.content && (
            <div className="rounded-2xl rounded-tr-sm bg-primary px-3.5 py-2 text-primary-foreground text-xs shadow-xs break-words whitespace-pre-wrap overflow-hidden max-w-full">
              {msg.content}
            </div>
          )}
        </div>
        <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary/20 text-primary mt-0.5">
          <User className="h-3.5 w-3.5" />
        </div>
      </div>
    );
  }

  // 助手消息渲染
  if (msg.role === 'assistant') {
    if (msg.status === 'error') {
      return (
        <div className="flex justify-start gap-2 pr-4">
          <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive mt-0.5">
            <AlertCircle className="h-3.5 w-3.5" />
          </div>
          <div className="flex-1 min-w-0">
            <AIErrorMessageCard message={msg} />
          </div>
        </div>
      );
    }

    const hasContent = Boolean(msg.content && msg.content.trim().length > 0);
    const hasThought = Boolean(msg.thought && msg.thought.trim().length > 0);
    const hasToolCalls = Boolean(msg.tool_calls && msg.tool_calls.length > 0);

    // 既无正文、又无思考、又无工具调用，且当前没有处于流式中：不渲染空白头像
    if (!hasContent && !hasThought && !hasToolCalls && !isStreaming) {
      return null;
    }

    const thoughtOpen =
      isThoughtOpen !== undefined
        ? isThoughtOpen
        : isStreaming || (isGenerating && isLatestAssistant);

    return (
      <div className="flex justify-start gap-2 pr-4">
        <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary mt-0.5">
          <Bot className="h-3.5 w-3.5" />
        </div>

        <div className="flex-1 space-y-2 min-w-0">
          {/* 深度思考过程展示 (DeepSeek R1 / Reasoning) */}
          {msg.thought && (
            <div className="rounded border border-border/60 bg-muted/30 text-xs overflow-hidden">
              <button
                type="button"
                className="flex w-full items-center gap-1.5 px-2.5 py-1.5 text-left text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                onClick={() => onToggleThought?.(msg.id)}
              >
                <Brain className="h-3 w-3 text-indigo-500 shrink-0" />
                <span className="font-medium text-[11px] truncate min-w-0 flex-1">
                  {ll.ai.copilot.thought_process()}
                  {isStreaming && (
                    <span className="ml-1.5 text-[10px] text-primary animate-pulse font-normal">
                      ({ll.ai.copilot.thinking()})
                    </span>
                  )}
                </span>
                <span className="text-[10px] text-muted-foreground/70 font-mono mr-1">
                  {msg.thought.length} 字
                </span>
                {thoughtOpen ? (
                  <ChevronDown className="h-3 w-3 ml-auto shrink-0" />
                ) : (
                  <ChevronRight className="h-3 w-3 ml-auto shrink-0" />
                )}
              </button>

              {thoughtOpen && (
                <div
                  ref={isStreaming ? activeStreamingThoughtRef : undefined}
                  className="border-t border-border/40 p-2 text-[11px] font-mono text-muted-foreground/90 whitespace-pre-wrap break-words bg-background/40 max-h-48 overflow-y-auto [contain:content]"
                >
                  {msg.thought}
                </div>
              )}
            </div>
          )}

          {/* 正文内容渲染（放在工具调用前，保证意图和方案规划先于工具卡片呈现） */}
          {msg.content ? (
            <div className="rounded-2xl rounded-tl-sm bg-muted/40 p-3 border border-border/40 text-foreground text-xs leading-relaxed break-words">
              <MarkdownViewer content={msg.content} compact />
            </div>
          ) : isStreaming && !msg.thought && (!msg.tool_calls || msg.tool_calls.length === 0) ? (
            <div className="flex items-center gap-1.5 text-muted-foreground py-1">
              <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />
              <span className="text-xs">{ll.ai.copilot.thinking()}</span>
            </div>
          ) : null}

          {/* 工具调用项展示 */}
          {msg.tool_calls && msg.tool_calls.length > 0 && (
            <div className="space-y-1">
              {msg.tool_calls.map((tc) => {
                const resultStr = tc.output ?? (toolResults ? toolResults[tc.id] : undefined);
                return (
                  <AIToolCallItem
                    key={tc.id}
                    toolCall={tc}
                    result={resultStr}
                    isExecuted={!isStreaming && resultStr !== undefined}
                  />
                );
              })}
            </div>
          )}
        </div>
      </div>
    );
  }

  return null;
});
