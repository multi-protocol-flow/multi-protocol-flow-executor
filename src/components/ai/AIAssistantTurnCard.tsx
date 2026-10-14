/**
 * AI Assistant 复合 Turn 展示卡片 (Turn-based Accordion Card)
 * 将单次用户交互后触发的多轮 ReAct (思考过程、工具调用、中间过渡) 聚合为单一 Bot 视图卡片，
 * 提供可折叠的执行过程面板 (Activity Steps Accordion) 与最终 Markdown 回复交付。
 */

import { useState, useMemo, memo } from 'react';
import {
  Bot,
  Brain,
  Wrench,
  ChevronDown,
  ChevronRight,
  AlertCircle,
} from 'lucide-react';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { AIToolCallItem } from './AIToolCallItem';
import { AIErrorMessageCard } from './AIErrorMessageCard';
import { ThinkingDots } from './ThinkingSpinner';
import { getLL } from '@/i18n/i18n-lazy';
import { formatToolTitle } from '@/services/ai/toolTitleFormatter';
import type { AIMessage, AIToolCall } from '@/types/ai';

export interface AssistantTurnData {
  id: string;
  messages: AIMessage[];
  errorMessage?: AIMessage;
  finalMessage?: AIMessage;
  thoughts: Array<{
    id: string;
    thought: string;
  }>;
  toolCalls: Array<{
    toolCall: AIToolCall;
    result?: string;
  }>;
  intermediateTexts: Array<{
    id: string;
    content: string;
  }>;
}

export type CopilotTurn =
  | { type: 'user'; message: AIMessage }
  | { type: 'summary'; message: AIMessage; archivedMessages: AIMessage[] }
  | { type: 'assistant'; turn: AssistantTurnData };

/**
 * 纯函数：将平铺的 AIMessage[] 列表聚合为按 Turn 组织的 CopilotTurn[]
 */
export function groupMessagesIntoTurns(
  messages: AIMessage[],
  toolResultsMap?: Record<string, string>,
): CopilotTurn[] {
  const turns: CopilotTurn[] = [];
  let currentAssistantMessages: AIMessage[] = [];
  let pendingArchived: AIMessage[] = [];

  const flushAssistantTurn = () => {
    if (currentAssistantMessages.length === 0) return;
    const msgs = currentAssistantMessages;
    currentAssistantMessages = [];

    const errorMsg = msgs.find((m) => m.status === 'error');
    const thoughts: Array<{ id: string; thought: string }> = [];
    const toolCalls: Array<{ toolCall: AIToolCall; result?: string }> = [];
    const assistantMsgsWithContent: AIMessage[] = [];

    for (const m of msgs) {
      if (m.role === 'assistant') {
        if (m.thought && m.thought.trim().length > 0) {
          thoughts.push({ id: m.id, thought: m.thought });
        }
        if (m.tool_calls && m.tool_calls.length > 0) {
          for (const tc of m.tool_calls) {
            const res = tc.output ?? (toolResultsMap ? toolResultsMap[tc.id] : undefined);
            toolCalls.push({ toolCall: tc, result: res });
          }
        }
        if (m.content && m.content.trim().length > 0) {
          assistantMsgsWithContent.push(m);
        }
      }
    }

    let finalMessage: AIMessage | undefined;
    const intermediateTexts: Array<{ id: string; content: string }> = [];

    if (assistantMsgsWithContent.length > 0) {
      finalMessage = assistantMsgsWithContent[assistantMsgsWithContent.length - 1];
      for (let i = 0; i < assistantMsgsWithContent.length - 1; i++) {
        intermediateTexts.push({
          id: assistantMsgsWithContent[i].id,
          content: assistantMsgsWithContent[i].content,
        });
      }
    } else {
      const lastAssistant = [...msgs].reverse().find((m) => m.role === 'assistant');
      finalMessage = lastAssistant;
    }

    turns.push({
      type: 'assistant',
      turn: {
        id: msgs[0].id,
        messages: msgs,
        errorMessage: errorMsg,
        finalMessage,
        thoughts,
        toolCalls,
        intermediateTexts,
      },
    });
  };

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.role === 'system') continue;

    if (msg.isArchived) {
      pendingArchived.push(msg);
      continue;
    }

    if (msg.isSummary) {
      flushAssistantTurn();
      const archivedForThis = pendingArchived;
      pendingArchived = [];
      turns.push({
        type: 'summary',
        message: msg,
        archivedMessages: archivedForThis,
      });
      continue;
    }

    if (msg.role === 'user') {
      flushAssistantTurn();
      turns.push({
        type: 'user',
        message: msg,
      });
      continue;
    }

    if (msg.role === 'assistant' || msg.role === 'tool') {
      currentAssistantMessages.push(msg);
    }
  }

  flushAssistantTurn();

  if (pendingArchived.length > 0) {
    turns.unshift({
      type: 'summary',
      message: {
        id: 'fallback_archived',
        role: 'assistant',
        content: '',
        timestamp: Date.now(),
        compactMeta: {
          compressedMessageCount: pendingArchived.length,
          timestamp: Date.now(),
        },
      },
      archivedMessages: pendingArchived,
    });
  }

  return turns;
}

export interface AIAssistantTurnCardProps {
  turn: AssistantTurnData;
  isStreaming?: boolean;
  isGenerating?: boolean;
  isLatest?: boolean;
  toolResults?: Record<string, string>;
  activeStreamingThoughtRef?: React.RefObject<HTMLDivElement | null>;
  onPreviewImage?: (imageUrl: string) => void;
}

export const AIAssistantTurnCard = memo(function AIAssistantTurnCard({
  turn,
  isStreaming = false,
  isGenerating = false,
  isLatest = false,
  toolResults,
  activeStreamingThoughtRef,
}: AIAssistantTurnCardProps) {
  const ll = getLL();

  // 若当前 Turn 遇到执行报错
  if (turn.errorMessage) {
    return (
      <div className="flex justify-start gap-2 pr-4">
        <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive mt-0.5">
          <AlertCircle className="h-3.5 w-3.5" />
        </div>
        <div className="flex-1 min-w-0">
          <AIErrorMessageCard message={turn.errorMessage} />
        </div>
      </div>
    );
  }

  const hasToolCalls = turn.toolCalls.length > 0;
  const hasThoughts = turn.thoughts.length > 0;
  const hasIntermediateTexts = turn.intermediateTexts.length > 0;
  const hasContent = Boolean(turn.finalMessage?.content && turn.finalMessage.content.trim().length > 0);

  // 既无内容、工具、思考，且不在流式生成中时，不展示空白卡片
  if (!hasToolCalls && !hasThoughts && !hasIntermediateTexts && !hasContent && !isStreaming) {
    return null;
  }

  const operationCount = turn.toolCalls.length + (hasThoughts ? 1 : 0);
  const showActivityAccordion = operationCount > 0 || (isStreaming && !hasContent);

  // 折叠状态策略：历史已完成的消息默认折叠收拢；当前正在流式生成或无正文文本时默认展示工具活动；支持手动展开/收起
  const [userToggled, setUserToggled] = useState<boolean | null>(null);
  const isAccordionOpen =
    userToggled !== null
      ? userToggled
      : isStreaming || (isGenerating && isLatest) || (!hasContent && isLatest);
  // 判断当前流式生成阶段是否有正在执行中的工具
  const activeTool =
    isStreaming && isGenerating
      ? turn.toolCalls.find((tc) => !tc.result && !toolResults?.[tc.toolCall.id])
      : null;

  const activeToolName = useMemo(() => {
    if (!activeTool) return null;
    return formatToolTitle(activeTool.toolCall.function.name, activeTool.toolCall.function.arguments).fullTitle;
  }, [activeTool]);

  return (
    <div className="flex justify-start gap-2 pr-4">
      <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary mt-0.5">
        <Bot className="h-3.5 w-3.5" />
      </div>

      <div className="flex-1 space-y-2 min-w-0">
        {/* 执行步骤折叠栏 (Activity Steps Accordion) */}
        {showActivityAccordion && (
          <div className="rounded border border-border/60 bg-muted/20 text-xs overflow-hidden">
            <button
              type="button"
              className="flex w-full items-center gap-1.5 px-2.5 py-1.5 text-left text-muted-foreground hover:text-foreground hover:bg-muted/40 transition-colors"
              onClick={() => setUserToggled(!isAccordionOpen)}
            >
              {hasThoughts && !hasToolCalls ? (
                <Brain className="h-3 w-3 text-indigo-500 shrink-0" />
              ) : (
                <Wrench className="h-3 w-3 text-muted-foreground shrink-0" />
              )}

              <span className="inline-flex items-center gap-1.5 min-w-0 flex-1 overflow-hidden">
                <span className="font-medium text-[11px] truncate">
                  {isStreaming && isGenerating
                    ? activeToolName
                      ? ll.ai.copilot.executing_tool_named({ name: activeToolName })
                      : ll.ai.copilot.execution_steps_in_progress({ count: Math.max(operationCount, 1) })
                    : ll.ai.copilot.execution_steps({ count: Math.max(operationCount, 1) })}
                </span>
                {isStreaming && isGenerating && (
                  <span className="inline-flex items-center gap-1 text-[10px] text-primary font-normal shrink-0">
                    <ThinkingDots dotClassName="w-1 h-1" />
                  </span>
                )}
              </span>

              {isAccordionOpen ? (
                <ChevronDown className="h-3 w-3 ml-auto shrink-0" />
              ) : (
                <ChevronRight className="h-3 w-3 ml-auto shrink-0" />
              )}
            </button>

            {isAccordionOpen && (
              <div className="border-t border-border/40 p-2 space-y-2 bg-background/40">
                {/* 思考链展示 */}
                {turn.thoughts.map((th) => (
                  <div
                    key={th.id}
                    ref={isStreaming ? activeStreamingThoughtRef : undefined}
                    className="p-2 text-[11px] font-mono text-muted-foreground/90 whitespace-pre-wrap break-words rounded bg-muted/30 border border-border/30 max-h-48 overflow-y-auto"
                  >
                    <div className="flex items-center gap-1 text-[10px] text-indigo-400 font-sans mb-1">
                      <Brain className="h-2.5 w-2.5" />
                      <span>{ll.ai.copilot.thought_process()}</span>
                    </div>
                    {th.thought}
                  </div>
                ))}

                {/* 中间过渡内容 */}
                {turn.intermediateTexts.map((item) => (
                  <div
                    key={item.id}
                    className="text-xs text-muted-foreground bg-muted/20 p-2 rounded border border-border/30"
                  >
                    <MarkdownViewer content={item.content} compact />
                  </div>
                ))}

                {/* 工具调用项展示 */}
                {turn.toolCalls.length > 0 && (
                  <div className="space-y-1">
                    {turn.toolCalls.map(({ toolCall, result }) => {
                      const res = result ?? (toolResults ? toolResults[toolCall.id] : undefined);
                      return (
                        <AIToolCallItem
                          key={toolCall.id}
                          toolCall={toolCall}
                          result={res}
                          isExecuted={!isStreaming && res !== undefined}
                        />
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* 最终回复正文 (Final Delivery) */}
        {hasContent ? (
          <div className="rounded-2xl rounded-tl-sm bg-muted/40 p-3 border border-border/40 text-foreground text-xs leading-relaxed break-words">
            <MarkdownViewer content={turn.finalMessage!.content} compact />
          </div>
        ) : isStreaming && !showActivityAccordion ? (
          <div className="flex items-center gap-1.5 text-muted-foreground py-1">
            <ThinkingDots />
            <span className="text-xs">{ll.ai.copilot.thinking()}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
});
