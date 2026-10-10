/**
 * AI 上下文压缩卡片组件
 * 展示由 /compact 生成的紧凑结构化会话状态，并支持按需展开查看已归档历史记录
 */

import { useState } from 'react';
import {
  Archive,
  ChevronDown,
  ChevronRight,
  History,
  Sparkles,
  User,
  Bot,
  Wrench,
} from 'lucide-react';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIMessage } from '@/types/ai';

interface AICompactedMessageCardProps {
  message: AIMessage;
  archivedMessages?: AIMessage[];
}

export function AICompactedMessageCard({
  message,
  archivedMessages = [],
}: AICompactedMessageCardProps) {
  const ll = getLL();
  const [showSummary, setShowSummary] = useState(true);
  const [showArchived, setShowArchived] = useState(false);

  const count = message.compactMeta?.compressedMessageCount ?? archivedMessages.length;
  const tokens = message.compactMeta?.originalTokenEstimate ?? 0;

  return (
    <div className="rounded-xl border border-indigo-500/30 bg-indigo-500/5 dark:bg-indigo-950/20 text-xs overflow-hidden transition-all shadow-xs">
      {/* 顶部标题栏 */}
      <div className="flex items-center justify-between px-3 py-2 bg-indigo-500/10 border-b border-indigo-500/20 select-none">
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <div className="flex h-5 w-5 items-center justify-center rounded-md bg-indigo-500/20 text-indigo-600 dark:text-indigo-400 shrink-0">
            <Archive className="h-3 w-3" />
          </div>
          <span className="font-semibold text-foreground truncate">
            {ll.ai.copilot.compact_card_title({ count })}
          </span>
          {tokens > 0 && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-indigo-500/20 text-indigo-700 dark:text-indigo-300 shrink-0">
              {ll.ai.copilot.compact_card_tokens({ tokens })}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {/* 展开/折叠摘要正文 */}
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground px-1.5 py-0.5 rounded hover:bg-muted/50 transition-colors"
            onClick={() => setShowSummary(!showSummary)}
          >
            <span>
              {showSummary
                ? ll.ai.copilot.compact_card_collapse_summary()
                : ll.ai.copilot.compact_card_expand_summary()}
            </span>
            {showSummary ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </button>
        </div>
      </div>

      {/* 结构化状态摘要渲染区域 */}
      {showSummary && (
        <div className="p-3 border-b border-indigo-500/15 bg-background/50 leading-relaxed text-foreground/90">
          <div className="flex items-center gap-1 text-[10px] font-semibold text-indigo-600 dark:text-indigo-400 mb-1.5 uppercase tracking-wider">
            <Sparkles className="h-3 w-3" />
            <span>{ll.ai.copilot.compact_card_snapshot_badge()}</span>
          </div>
          <MarkdownViewer content={message.content} compact />
        </div>
      )}

      {/* 底部已归档历史展开抽屉入口 */}
      {archivedMessages.length > 0 && (
        <div className="px-3 py-1.5 bg-muted/20">
          <button
            type="button"
            className="inline-flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
            onClick={() => setShowArchived(!showArchived)}
          >
            <History className="h-3 w-3 text-indigo-500" />
            <span>
              {showArchived
                ? ll.ai.copilot.compact_card_hide_archived()
                : `${ll.ai.copilot.compact_card_view_archived()} (${archivedMessages.length})`}
            </span>
            {showArchived ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </button>

          {/* 展开的已归档消息列表 */}
          {showArchived && (
            <div className="mt-2 pt-2 border-t border-border/40 space-y-2.5 max-h-60 overflow-y-auto pr-1">
              {archivedMessages.map((archived) => {
                const isUser = archived.role === 'user';
                const isTool = archived.role === 'tool';

                return (
                  <div
                    key={archived.id}
                    className={`flex items-start gap-2 p-2 rounded-lg border text-[11px] opacity-80 ${
                      isUser
                        ? 'bg-primary/5 border-primary/20 text-foreground'
                        : isTool
                        ? 'bg-muted/40 border-border/50 text-muted-foreground font-mono'
                        : 'bg-muted/30 border-border/40 text-foreground'
                    }`}
                  >
                    <div className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full mt-0.5">
                      {isUser ? (
                        <User className="h-3 w-3 text-primary" />
                      ) : isTool ? (
                        <Wrench className="h-3 w-3 text-amber-500" />
                      ) : (
                        <Bot className="h-3 w-3 text-indigo-500" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between text-[10px] text-muted-foreground mb-0.5">
                        <span className="font-semibold uppercase">
                          {isTool ? `Tool: ${archived.name || 'call'}` : archived.role}
                        </span>
                        <span>{new Date(archived.timestamp).toLocaleTimeString()}</span>
                      </div>
                      <div className="line-clamp-4 whitespace-pre-wrap break-words">
                        {archived.content || (archived.thought ? `[Thought]: ${archived.thought}` : '(empty)')}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
