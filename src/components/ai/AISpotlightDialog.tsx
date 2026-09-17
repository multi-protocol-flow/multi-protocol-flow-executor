/**
 * AI Spotlight 居中快捷指令弹窗 (Cmd/Ctrl + K)
 */

import { useState, useRef, useEffect } from 'react';
import {
  Sparkles,
  Send,
  Square,
  Bot,
  User,
  PanelRightClose,
  Loader2,
  X,
  AlertCircle,
  Brain,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { AIToolCallItem } from './AIToolCallItem';
import { AIConfirmCard } from './AIConfirmCard';
import { AIErrorMessageCard } from './AIErrorMessageCard';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIMessage } from '@/types/ai';

export function AISpotlightDialog() {
  const ll = getLL();
  const {
    isSpotlightOpen,
    setSpotlightOpen,
    setCopilotOpen,
    messages,
    agentStatus,
    activeStreamingMessageId,
    pendingActions,
    sendMessage,
    stopGeneration,
  } = useAIStore();

  const { aiProviders, activeAiProviderId } = useSettingsStore();
  const activeProvider = aiProviders.find((p) => p.id === activeAiProviderId);

  const [prompt, setPrompt] = useState('');
  const [openThoughts, setOpenThoughts] = useState<Record<string, boolean>>({});
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const toggleThought = (id: string) => {
    setOpenThoughts((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const isGenerating =
    agentStatus === 'thinking' ||
    agentStatus === 'executing_tools' ||
    agentStatus === 'waiting_confirmation';

  // 打开时自动聚焦输入框
  useEffect(() => {
    if (isSpotlightOpen) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isSpotlightOpen]);

  // 新消息滚动 (使用 requestAnimationFrame 防抖调度，避免强制同步重排)
  useEffect(() => {
    if (!isSpotlightOpen) return;
    let rafId: number;
    rafId = requestAnimationFrame(() => {
      scrollRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
    return () => {
      cancelAnimationFrame(rafId);
    };
  }, [messages, isSpotlightOpen]);

  const handleSend = () => {
    const text = prompt.trim();
    if (!text || isGenerating) return;
    setPrompt('');
    sendMessage(text);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSend();
    }
  };

  const handleDockToCopilot = () => {
    setSpotlightOpen(false);
    setCopilotOpen(true);
  };

  // 获取最近的对话消息 (Spotlight 模式只展示最近 4 条消息)
  const recentMessages = messages.slice(-4);

  return (
    <Dialog open={isSpotlightOpen} onOpenChange={setSpotlightOpen}>
      <DialogContent className="max-w-2xl p-0 gap-0 overflow-hidden border-border/80 shadow-2xl bg-background/95 backdrop-blur-md">
        <DialogTitle className="sr-only">{ll.ai.spotlight.title()}</DialogTitle>

        {/* 顶部搜索输入条 */}
        <div className="flex items-center gap-2 px-3 py-3 border-b border-border/60 bg-muted/20">
          <Sparkles className="h-5 w-5 text-primary shrink-0 ml-1" />
          <input
            ref={inputRef}
            type="text"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              activeProvider
                ? ll.ai.spotlight.input_placeholder()
                : ll.ai.copilot.no_active_provider()
            }
            disabled={!activeProvider}
            className="flex-1 bg-transparent border-0 text-sm font-medium text-foreground focus:outline-none placeholder:text-muted-foreground/60"
          />

          {activeProvider && (
            <span
              className="text-[10px] px-2 py-0.5 rounded font-mono bg-muted text-muted-foreground shrink-0 max-w-[120px] truncate"
              title={activeProvider.name}
            >
              {activeProvider.name}
            </span>
          )}

          {isGenerating ? (
            <Button
              size="sm"
              variant="destructive"
              className="h-7 px-2 text-xs rounded-md gap-1 shrink-0"
              onClick={stopGeneration}
            >
              <Square className="h-3 w-3 fill-current" />
              {ll.ai.copilot.stop()}
            </Button>
          ) : (
            <Button
              size="sm"
              className="h-7 px-3 text-xs bg-primary hover:bg-primary/90 text-primary-foreground shrink-0"
              onClick={handleSend}
              disabled={!prompt.trim() || !activeProvider}
            >
              <Send className="mr-1 h-3 w-3" />
              {ll.ai.copilot.send()}
            </Button>
          )}

          <Button
            size="icon"
            variant="ghost"
            className="h-7 w-7 text-muted-foreground hover:text-foreground shrink-0"
            onClick={handleDockToCopilot}
            title={ll.ai.spotlight.dock_to_sidebar()}
          >
            <PanelRightClose className="h-4 w-4" />
          </Button>

          <Button
            size="icon"
            variant="ghost"
            className="h-7 w-7 text-muted-foreground hover:text-foreground shrink-0"
            onClick={() => setSpotlightOpen(false)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* 交互反馈与响应展示区 */}
        {recentMessages.length > 0 && (
          <div className="max-h-[360px] overflow-y-auto p-4 space-y-3 text-xs">
            {recentMessages.map((msg) => {
              if (msg.role === 'system') return null;

              if (msg.role === 'user') {
                return (
                  <div key={msg.id} className="flex justify-end gap-2">
                    <div className="rounded-xl bg-primary px-3 py-1.5 text-primary-foreground text-xs shadow-xs max-w-[85%] break-words whitespace-pre-wrap overflow-hidden">
                      {msg.content}
                    </div>
                    <div className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/20 text-primary mt-0.5">
                      <User className="h-3 w-3" />
                    </div>
                  </div>
                );
              }

              if (msg.role === 'assistant') {
                if (msg.status === 'error') {
                  return (
                    <div key={msg.id} className="flex justify-start gap-2">
                      <div className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive mt-0.5">
                        <AlertCircle className="h-3 w-3" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <AIErrorMessageCard message={msg} compact />
                      </div>
                    </div>
                  );
                }

                const isStreaming = activeStreamingMessageId === msg.id;
                const hasContent = Boolean(msg.content && msg.content.trim().length > 0);
                const hasThought = Boolean(msg.thought && msg.thought.trim().length > 0);
                const hasToolCalls = Boolean(msg.tool_calls && msg.tool_calls.length > 0);

                if (!hasContent && !hasThought && !hasToolCalls && !isStreaming) {
                  return null;
                }
                return (
                  <div key={msg.id} className="flex justify-start gap-2">
                    <div className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary mt-0.5">
                      <Bot className="h-3 w-3" />
                    </div>
                    <div className="flex-1 space-y-1.5 min-w-0">
                      {/* 深度思考过程展示 (DeepSeek R1 / Reasoning) */}
                      {msg.thought && (() => {
                        let latestAssistantMsg: AIMessage | undefined;
                        for (let i = recentMessages.length - 1; i >= 0; i--) {
                          if (recentMessages[i].role === 'assistant') {
                            latestAssistantMsg = recentMessages[i];
                            break;
                          }
                        }
                        const isLatestAssistant = latestAssistantMsg?.id === msg.id;
                        const thoughtOpen =
                          openThoughts[msg.id] !== undefined
                            ? openThoughts[msg.id]
                            : (isStreaming || (isGenerating && isLatestAssistant));

                        return (
                          <div className="rounded border border-border/60 bg-muted/30 text-[11px] overflow-hidden">
                            <button
                              type="button"
                              className="flex w-full items-center gap-1.5 px-2 py-1 text-left text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                              onClick={() => toggleThought(msg.id)}
                            >
                              <Brain className="h-3 w-3 text-indigo-500 shrink-0" />
                              <span className="font-medium truncate min-w-0 flex-1">
                                {ll.ai.copilot.thought_process()}
                                {isStreaming && (
                                  <span className="ml-1 text-[10px] text-primary animate-pulse font-normal">
                                    ({ll.ai.copilot.thinking()})
                                  </span>
                                )}
                              </span>
                              <span className="text-[10px] text-muted-foreground/70 font-mono mr-1">
                                {msg.thought.length} 字
                              </span>
                              {thoughtOpen ? (
                                <ChevronDown className="h-3 w-3 shrink-0" />
                              ) : (
                                <ChevronRight className="h-3 w-3 shrink-0" />
                              )}
                            </button>
                            {thoughtOpen && (
                              <div className="border-t border-border/40 p-2 text-[10px] font-mono text-muted-foreground/90 whitespace-pre-wrap break-words bg-background/40 max-h-36 overflow-y-auto [contain:content]">
                                {msg.thought}
                              </div>
                            )}
                          </div>
                        );
                      })()}

                      {/* 正文内容渲染（放在工具调用前） */}
                      {msg.content ? (
                        <div className="rounded-xl bg-muted/40 p-2.5 border border-border/40 text-foreground text-xs leading-relaxed break-words">
                          <MarkdownViewer content={msg.content} compact />
                        </div>
                      ) : isStreaming && !msg.thought && (!msg.tool_calls || msg.tool_calls.length === 0) ? (
                        <div className="flex items-center gap-1.5 text-muted-foreground py-1">
                          <Loader2 className="h-3 w-3 animate-spin text-primary" />
                          <span className="text-xs">{ll.ai.copilot.thinking()}</span>
                        </div>
                      ) : null}

                      {/* 工具调用项展示 */}
                      {msg.tool_calls && msg.tool_calls.length > 0 && (
                        <div className="space-y-1">
                          {msg.tool_calls.map((tc) => {
                            const toolMsg = recentMessages.find(
                              (m) => m.role === 'tool' && m.tool_call_id === tc.id,
                            );
                            const resultStr = tc.output ?? toolMsg?.content;
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
            })}

            {/* 待确认卡片 */}
            {pendingActions
              .filter((act) => act.status === 'pending')
              .map((act) => (
                <AIConfirmCard key={act.id} action={act} />
              ))}

            <div ref={scrollRef} />
          </div>
        )}

        {/* 底部小提示 */}
        <div className="px-3 py-1.5 bg-muted/40 border-t border-border/40 flex items-center justify-between text-[11px] text-muted-foreground">
          <span>{ll.ai.spotlight.shortcut_hint()}</span>
          <span>{ll.ai.spotlight.esc_to_close()}</span>
        </div>
      </DialogContent>
    </Dialog>
  );
}
