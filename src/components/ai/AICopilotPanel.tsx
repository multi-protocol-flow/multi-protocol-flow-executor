/**
 * AI Copilot 侧边栏面板组件 - 会话管理与多模态图片增强版
 */

import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import {
  Sparkles,
  X,
  Trash2,
  Lightbulb,
  Settings,
  ArrowRight,
  Loader2,
  Plus,
  History,
  Edit2,
  Check,
  GripVertical,
  GripHorizontal,
  ArrowRightToLine,
  ArrowDown,
  ShieldCheck,
  Archive,
  ChevronDown,
  ExternalLink,
  MoreHorizontal,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { useExecutionStore } from '@/stores/execution';
import { AIConfirmCard } from './AIConfirmCard';
import { AICompactedMessageCard } from './AICompactedMessageCard';
import { AIMessageRow } from './AIMessageRow';
import { AIAssistantTurnCard, groupMessagesIntoTurns } from './AIAssistantTurnCard';
import { AICopilotInput } from './AICopilotInput';
import { AIProviderSelect } from './AIProviderSelect';
import { formatToolTitle } from '@/services/ai/toolTitleFormatter';
import { getLL } from '@/i18n/i18n-lazy';

export function AICopilotPanel() {
  const ll = getLL();
  const {
    sessions,
    currentSessionId,
    activeWorkspace,
    messages,
    agentStatus,
    activeStreamingMessageId,
    activeExecutingToolCallId,
    pendingActions,
    isCopilotOpen,
    panelWidth,
    isFloating,
    floatingPosition,
    setPanelWidth,
    setFloating,
    setFloatingPosition,
    alwaysTrustHighRisk,
    setAlwaysTrustHighRisk,
    toggleAlwaysTrustHighRisk,
    setCopilotOpen,
    setSessionWorkspace,
    selectWorkspaceFolder,
    initSessions,
    createNewSession,
    switchSession,
    renameSession,
    deleteSession,
    sendMessage,
    stopGeneration,
    clearMessages,
    isCompacting,
    compactHistory,
  } = useAIStore();
  const { aiProviders, activeAiProviderId } = useSettingsStore();
  const openSettingsDialog = useUIStore((s) => s.openSettingsDialog);


  // 历史会话下拉弹层状态
  const [showSessionMenu, setShowSessionMenu] = useState(false);
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editTitleInput, setEditTitleInput] = useState('');
  // 大图查看弹窗状态
  const [previewImageUrl, setPreviewImageUrl] = useState<string | null>(null);
  const [showScrollBottom, setShowScrollBottom] = useState(false);

  // 拖拽拉伸宽度状态
  const [isResizing, setIsResizing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  const activeStreamingThoughtRef = useRef<HTMLDivElement | null>(null);

  const activeProvider = aiProviders.find((p) => p.id === activeAiProviderId);
  const currentSession = sessions.find((s) => s.id === currentSessionId);
  const isGenerating =
    agentStatus === 'thinking' ||
    agentStatus === 'executing_tools' ||
    agentStatus === 'waiting_confirmation';


  // 预建工具输出结果映射，避免各消息行在大量消息下重复搜索
  const toolResultsMap = useMemo(() => {
    const map: Record<string, string> = {};
    for (let i = 0; i < messages.length; i++) {
      const m = messages[i];
      if (m.role === 'tool' && m.tool_call_id && m.content) {
        map[m.tool_call_id] = m.content;
      }
    }
    return map;
  }, [messages]);

  // 将平铺消息聚合为 Turn-based 视图数据
  const turns = useMemo(() => {
    return groupMessagesIntoTurns(messages, toolResultsMap);
  }, [messages, toolResultsMap]);

  // 初始化会话列表
  useEffect(() => {
    if (isCopilotOpen) {
      initSessions();
    }
  }, [isCopilotOpen, initSessions]);

  // 自动滚动到底部 (使用 requestAnimationFrame 防抖调度，避免强制同步布局 Layout Thrashing)
  useEffect(() => {
    if (!isCopilotOpen) return;
    const container = messagesContainerRef.current;
    if (!container) {
      messagesEndRef.current?.scrollIntoView?.({ behavior: 'smooth' });
      return;
    }

    let rafId: number;
    rafId = requestAnimationFrame(() => {
      container.scrollTop = container.scrollHeight;
      if (activeStreamingThoughtRef.current) {
        activeStreamingThoughtRef.current.scrollTop = activeStreamingThoughtRef.current.scrollHeight;
      }
    });
    return () => {
      cancelAnimationFrame(rafId);
    };
  }, [messages, isCopilotOpen]);

  const handleMessagesScroll = useCallback(() => {
    const container = messagesContainerRef.current;
    if (!container) return;
    const isFarFromBottom = container.scrollHeight - container.scrollTop - container.clientHeight > 80;
    setShowScrollBottom(isFarFromBottom);
  }, []);

  const scrollToBottom = useCallback((smooth = true) => {
    const container = messagesContainerRef.current;
    if (!container) return;
    if (smooth) {
      container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
    } else {
      container.scrollTop = container.scrollHeight;
    }
  }, []);

  // 打开面板时自动聚焦输入框
  useEffect(() => {
    if (isCopilotOpen) {
      setTimeout(() => textareaRef.current?.focus(), 150);
    }
  }, [isCopilotOpen, currentSessionId]);

  // 点击外部关闭历史会话菜单
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as HTMLElement)) {
        setShowSessionMenu(false);
        setEditingSessionId(null);
      }
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as HTMLElement)) {
        setShowMoreMenu(false);
      }
    };
    if (showSessionMenu || showMoreMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showSessionMenu, showMoreMenu]);

  // 处理拖拽拉伸宽度
  const handleMouseDownResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);

    const startX = e.clientX;
    const startWidth = useAIStore.getState().panelWidth;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = startX - moveEvent.clientX;
      const newWidth = Math.max(320, Math.min(750, startWidth + deltaX));
      setPanelWidth(newWidth);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [setPanelWidth]);

  // 停靠模式下：拖动头部离开右侧吸附
  const handleMouseDownDockedDrag = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button') || target.closest('input') || target.closest('textarea')) {
      return;
    }
    e.preventDefault();

    const startX = e.clientX;
    let hasDetached = false;
    let currentDragWidth = 0;
    let currentDragHeight = 0;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!hasDetached) {
        const deltaX = startX - moveEvent.clientX;
        if (deltaX > 15) {
          hasDetached = true;
          const currentPanelWidth = useAIStore.getState().panelWidth;
          currentDragWidth = Math.max(360, currentPanelWidth);
          currentDragHeight = Math.max(450, window.innerHeight - 120);
          const initialX = Math.max(20, Math.min(window.innerWidth - currentDragWidth - 20, moveEvent.clientX - currentDragWidth / 2));
          const initialY = Math.max(50, Math.min(window.innerHeight - currentDragHeight - 20, moveEvent.clientY - 20));

          setFloatingPosition({
            x: initialX,
            y: initialY,
            width: currentDragWidth,
            height: currentDragHeight,
          });
          setFloating(true);
        }
      } else {
        // 脱离吸附后继续跟随鼠标平滑移动
        const newX = Math.max(0, Math.min(window.innerWidth - currentDragWidth, moveEvent.clientX - currentDragWidth / 2));
        const newY = Math.max(20, Math.min(window.innerHeight - 60, moveEvent.clientY - 20));
        setFloatingPosition({ x: newX, y: newY });
      }
    };

    const handleMouseUp = (upEvent: MouseEvent) => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);

      // 如果释放位置贴近屏幕右边缘 (30px 以内)，自动吸附至右侧
      if (hasDetached && upEvent.clientX >= window.innerWidth - 30) {
        setFloating(false);
      }
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [setFloating, setFloatingPosition]);

  // 浮动模式下：按住头部拖拽移动窗口
  const handleMouseDownFloatingHeader = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button') || target.closest('input') || target.closest('textarea')) {
      return;
    }
    e.preventDefault();

    const currentPos = useAIStore.getState().floatingPosition;
    const startMouseX = e.clientX;
    const startMouseY = e.clientY;
    const startPosX = currentPos.x;
    const startPosY = currentPos.y;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startMouseX;
      const deltaY = moveEvent.clientY - startMouseY;
      const newX = Math.max(0, Math.min(window.innerWidth - currentPos.width, startPosX + deltaX));
      const newY = Math.max(20, Math.min(window.innerHeight - 60, startPosY + deltaY));

      setFloatingPosition({ x: newX, y: newY });
    };

    const handleMouseUp = (upEvent: MouseEvent) => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);

      // 如果释放位置贴近屏幕右边缘 (30px 以内)，自动吸附至右侧
      if (upEvent.clientX >= window.innerWidth - 30) {
        setFloating(false);
      }
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [setFloating, setFloatingPosition]);

  // 浮动模式下：右下角拖拽调整窗口大小
  const handleMouseDownFloatingResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const currentPos = useAIStore.getState().floatingPosition;
    const startX = e.clientX;
    const startY = e.clientY;
    const startW = currentPos.width;
    const startH = currentPos.height;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      const deltaY = moveEvent.clientY - startY;
      const newW = Math.max(340, Math.min(window.innerWidth - currentPos.x - 10, startW + deltaX));
      const newH = Math.max(350, Math.min(window.innerHeight - currentPos.y - 10, startH + deltaY));

      setFloatingPosition({ width: newW, height: newH });
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [setFloatingPosition]);

  const handleUsePrompt = useCallback(
    (promptText: string) => {
      sendMessage(promptText);
    },
    [sendMessage],
  );
  const handleSaveRename = (sessionId: string) => {
    if (editTitleInput.trim()) {
      renameSession(sessionId, editTitleInput.trim());
    }
    setEditingSessionId(null);
  };
  if (!isCopilotOpen) return null;


  return (
    <>
      {/* AI 面板容器：支持右侧吸附与浮动窗口两种形态 */}
      {(() => {
        const effectiveWidth = isFloating ? floatingPosition.width : panelWidth;
        const Container = 'aside';
        const containerClasses = isFloating
          ? 'fixed bg-background border border-border rounded-xl z-50 flex flex-col shadow-2xl animate-in fade-in-50 zoom-in-95 duration-150 overflow-hidden'
          : 'fixed right-0 top-14 bottom-0 bg-background border-l border-border z-30 flex flex-col shadow-2xl animate-in slide-in-from-right duration-200';
        const containerStyle = isFloating
          ? {
              left: `${floatingPosition.x}px`,
              top: `${floatingPosition.y}px`,
              width: `${floatingPosition.width}px`,
              height: `${floatingPosition.height}px`,
              maxHeight: 'calc(100vh - 20px)',
              maxWidth: 'calc(100vw - 20px)',
            }
          : { width: `${panelWidth}px` };

        return (
          <Container
            className={containerClasses}
            style={containerStyle}
            aria-label={isFloating ? 'AI Copilot Floating Panel' : 'AI Copilot Panel'}
            onScroll={(e) => {
              if (e.currentTarget.scrollTop !== 0) {
                e.currentTarget.scrollTop = 0;
              }
            }}
          >
            {/* 停靠模式下：左侧可拖拽拉伸边条 */}
            {!isFloating && (
              <div
                onMouseDown={handleMouseDownResize}
                className={`absolute left-0 top-0 bottom-0 w-1.5 -ml-1 cursor-col-resize z-40 hover:bg-primary/50 transition-colors flex items-center justify-center ${
                  isResizing ? 'bg-primary' : 'bg-transparent'
                }`}
                title={ll.ai.copilot.drag_resize()}
              >
                <GripVertical className="h-3 w-3 text-muted-foreground/40 opacity-0 hover:opacity-100" />
              </div>
            )}

            {/* 头部导航栏 (sticky 锚定在顶部，杜绝内容上顶时滚出视野) */}
            <header
              className={`h-11 border-b border-border/70 px-2.5 flex items-center justify-between bg-background shrink-0 sticky top-0 z-20 select-none gap-2 ${
                isFloating ? 'cursor-move' : ''
              }`}
              onMouseDown={isFloating ? handleMouseDownFloatingHeader : handleMouseDownDockedDrag}
            >
              <div className="flex items-center gap-1.5 min-w-0 flex-1 mr-1">
                {/* 拖拽移动把手：浮动模式下常驻；停靠模式下在较宽 (>= 380) 时展示 */}
                {(isFloating || effectiveWidth >= 380) && (
                  <div
                    className="flex items-center justify-center p-1 rounded hover:bg-muted/70 text-muted-foreground transition-colors shrink-0 cursor-grab active:cursor-grabbing"
                    title={ll.ai.copilot.drag_to_move()}
                    onMouseDown={isFloating ? handleMouseDownFloatingHeader : handleMouseDownDockedDrag}
                  >
                    <GripHorizontal className="h-3.5 w-3.5" />
                  </div>
                )}
                {effectiveWidth >= 360 && (
                  <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/10 text-primary shrink-0">
                    <Sparkles className="h-3.5 w-3.5" />
                  </div>
                )}

                {/* 会话选择器与下拉抽屉触发器 */}
                <div className="relative inline-flex items-center min-w-0 max-w-[220px]" ref={menuRef}>
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 text-xs font-semibold text-foreground hover:bg-muted/60 px-1.5 py-1 rounded-md transition-colors min-w-0 max-w-full cursor-pointer"
                    onClick={() => setShowSessionMenu(!showSessionMenu)}
                    title={ll.ai.copilot.sessions_title()}
                  >
                    <span className="truncate min-w-0 text-left">
                      {currentSession?.title || ll.ai.copilot.title()}
                    </span>
                    <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
                  </button>
                  {/* 历史会话浮动列表 Popover */}
                  {showSessionMenu && (
                <div
                  className="absolute left-0 top-8 w-64 rounded-lg border border-border bg-popover text-popover-foreground shadow-xl z-50 p-1.5 space-y-1 animate-in fade-in zoom-in-95 duration-100"
                  onMouseDown={(e) => e.stopPropagation()}
                >
                  <div className="flex items-center justify-between px-2 py-1 border-b border-border/50 text-[11px] font-medium text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <History className="h-3 w-3" />
                      <span>{ll.ai.copilot.sessions_title()}</span>
                    </span>
                    <button
                      type="button"
                      className="text-primary hover:underline flex items-center gap-0.5"
                      onClick={async () => {
                        setShowSessionMenu(false);
                        await createNewSession();
                      }}
                    >
                      <Plus className="h-3 w-3" />
                      <span>{ll.ai.copilot.new_session()}</span>
                    </button>
                  </div>

                  <div className="max-h-60 overflow-y-auto space-y-0.5">
                    {sessions.map((sess) => {
                      const isCurrent = sess.id === currentSessionId;
                      const isEditing = editingSessionId === sess.id;

                      return (
                        <div
                          key={sess.id}
                          className={`group flex items-center justify-between px-2 py-1.5 rounded-md text-xs transition-colors cursor-pointer ${
                            isCurrent
                              ? 'bg-primary/10 text-primary font-medium'
                              : 'hover:bg-muted/70 text-foreground'
                          }`}
                          onClick={() => {
                            if (!isEditing) {
                              switchSession(sess.id);
                              setShowSessionMenu(false);
                            }
                          }}
                        >
                          {isEditing ? (
                            <div className="flex items-center gap-1 w-full" onClick={(e) => e.stopPropagation()}>
                              <Input
                                value={editTitleInput}
                                onChange={(e) => setEditTitleInput(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter') handleSaveRename(sess.id);
                                  if (e.key === 'Escape') setEditingSessionId(null);
                                }}
                                autoFocus
                                className="h-6 text-xs px-1.5 py-0"
                              />
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-5 w-5 p-0"
                                onClick={() => handleSaveRename(sess.id)}
                              >
                                <Check className="h-3 w-3 text-emerald-500" />
                              </Button>
                            </div>
                          ) : (
                            <>
                              <div className="flex-1 text-left truncate min-w-0 pr-1.5">
                                <div className="truncate">{sess.title}</div>
                                <div className="text-[10px] text-muted-foreground font-normal">
                                  {new Date(sess.updatedAt).toLocaleDateString()}{' '}
                                  · {ll.ai.copilot.messages_count({ count: sess.messageCount })}
                                </div>
                              </div>
                              <div className="opacity-0 group-hover:opacity-100 flex items-center gap-0.5 shrink-0 transition-opacity">
                                <button
                                  type="button"
                                  className="p-1 text-muted-foreground hover:text-foreground"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setEditingSessionId(sess.id);
                                    setEditTitleInput(sess.title);
                                  }}
                                  title={ll.ai.copilot.rename_session()}
                                >
                                  <Edit2 className="h-2.5 w-2.5" />
                                </button>
                                <button
                                  type="button"
                                  className="p-1 text-muted-foreground hover:text-destructive"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    deleteSession(sess.id);
                                  }}
                                  title={ll.ai.copilot.delete_session()}
                                >
                                  <Trash2 className="h-2.5 w-2.5" />
                                </button>
                              </div>
                            </>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>

            {effectiveWidth >= 400 && (
              <AIProviderSelect
                direction="down"
                compact
                disabled={isGenerating}
                maxWidthClass="max-w-[105px]"
              />
            )}
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            {/* 完全信任模式：宽屏 (>= 460) 下胶囊 Switch，标准及窄屏 (< 460) 下紧凑高亮图标按钮 */}
            {effectiveWidth >= 460 ? (
              <button
                type="button"
                role="switch"
                aria-checked={alwaysTrustHighRisk}
                className={`flex items-center gap-1.5 h-7 px-2 rounded-md text-[11px] font-medium border transition-colors cursor-pointer select-none shrink-0 ${
                  alwaysTrustHighRisk
                    ? 'bg-amber-500/10 border-amber-500/35 text-amber-600 dark:bg-amber-950/40 dark:border-amber-600/40 dark:text-amber-300'
                    : 'bg-muted/40 border-border/80 text-muted-foreground hover:bg-muted/60 dark:bg-zinc-900/60 dark:border-zinc-800'
                }`}
                onClick={toggleAlwaysTrustHighRisk}
                title={ll.ai.copilot.trust_mode_tip()}
              >
                <ShieldCheck
                  className={`h-3.5 w-3.5 shrink-0 transition-transform ${
                    alwaysTrustHighRisk
                      ? 'text-amber-600 dark:text-amber-400 scale-105'
                      : 'text-muted-foreground/70'
                  }`}
                />
                <span
                  className={
                    alwaysTrustHighRisk
                      ? 'text-amber-700 dark:text-amber-300 font-semibold'
                      : 'text-muted-foreground'
                  }
                >
                  {ll.ai.copilot.trust_mode()}
                </span>
                <span className="pointer-events-none inline-flex items-center" aria-hidden="true">
                  <Switch
                    checked={alwaysTrustHighRisk}
                    tabIndex={-1}
                    variant="amber"
                    className="scale-75 origin-center"
                  />
                </span>
              </button>
            ) : (
              <button
                type="button"
                role="switch"
                aria-checked={alwaysTrustHighRisk}
                className={`flex items-center justify-center h-7 w-7 rounded-md border transition-colors cursor-pointer select-none shrink-0 ${
                  alwaysTrustHighRisk
                    ? 'bg-amber-500/15 border-amber-500/35 text-amber-600 dark:bg-amber-950/50 dark:border-amber-600/40 dark:text-amber-300 shadow-xs'
                    : 'bg-transparent border-transparent hover:bg-muted/70 text-muted-foreground hover:text-foreground'
                }`}
                onClick={toggleAlwaysTrustHighRisk}
                title={ll.ai.copilot.trust_mode_tip()}
              >
                <ShieldCheck
                  className={`h-3.5 w-3.5 ${alwaysTrustHighRisk ? 'text-amber-600 dark:text-amber-400' : ''}`}
                />
              </button>
            )}
            {isFloating ? (
              <Button
                size="icon"
                variant="ghost"
                className="h-7 w-7 text-muted-foreground hover:text-foreground"
                onClick={() => setFloating(false)}
                title={ll.ai.copilot.dock_to_right()}
              >
                <ArrowRightToLine className="h-3.5 w-3.5" />
              </Button>
            ) : (
              <Button
                size="icon"
                variant="ghost"
                className="h-7 w-7 text-muted-foreground hover:text-foreground"
                onClick={() => setFloating(true)}
                title={ll.ai.copilot.undock_panel()}
              >
                <ExternalLink className="h-3.5 w-3.5" />
              </Button>
            )}

            {/* 新建对话按钮 */}
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7 text-muted-foreground hover:text-foreground"
              onClick={() => createNewSession()}
              title={ll.ai.copilot.new_session()}
            >
              <Plus className="h-4 w-4" />
            </Button>

            {/* 会话维护操作：标准及宽屏 (>= 360) 平铺，超窄屏 (< 360) 收拢至更多菜单 */}
            {effectiveWidth >= 360 ? (
              <>
                {messages.length > 2 && (
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7 text-muted-foreground hover:text-indigo-500 hover:bg-indigo-500/10 transition-colors shrink-0"
                    onClick={() => compactHistory()}
                    disabled={isCompacting || isGenerating}
                    title={ll.ai.copilot.compact_button_tip()}
                  >
                    {isCompacting ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin text-indigo-500" />
                    ) : (
                      <Archive className="h-3.5 w-3.5" />
                    )}
                  </Button>
                )}
                {messages.length > 0 && (
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7 text-muted-foreground hover:text-foreground shrink-0"
                    onClick={clearMessages}
                    title={ll.ai.copilot.clear_history()}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                )}
              </>
            ) : (
              messages.length > 0 && (
                <div className="relative inline-flex items-center shrink-0" ref={moreMenuRef}>
                  <Button
                    size="icon"
                    variant="ghost"
                    className={`h-7 w-7 transition-colors ${
                      showMoreMenu
                        ? 'bg-muted text-foreground'
                        : 'text-muted-foreground hover:text-foreground'
                    }`}
                    onClick={() => setShowMoreMenu(!showMoreMenu)}
                    title={ll.ai.copilot.more_actions()}
                  >
                    <MoreHorizontal className="h-4 w-4" />
                  </Button>
                  {showMoreMenu && (
                    <div
                      className="absolute right-0 top-8 w-44 rounded-lg border border-border bg-popover text-popover-foreground shadow-xl z-50 p-1 space-y-0.5 animate-in fade-in zoom-in-95 duration-100"
                      onMouseDown={(e) => e.stopPropagation()}
                    >
                      {messages.length > 2 && (
                        <button
                          type="button"
                          disabled={isCompacting || isGenerating}
                          className="w-full flex items-center gap-2 px-2 py-1.5 text-xs rounded-md text-foreground hover:bg-muted/70 transition-colors disabled:opacity-50 text-left cursor-pointer"
                          onClick={() => {
                            setShowMoreMenu(false);
                            compactHistory();
                          }}
                        >
                          <Archive className="h-3.5 w-3.5 text-indigo-500 shrink-0" />
                          <span className="truncate">{ll.ai.copilot.command_compact()}</span>
                        </button>
                      )}
                      <button
                        type="button"
                        className="w-full flex items-center gap-2 px-2 py-1.5 text-xs rounded-md text-destructive hover:bg-destructive/10 transition-colors text-left cursor-pointer"
                        onClick={() => {
                          setShowMoreMenu(false);
                          clearMessages();
                        }}
                      >
                        <Trash2 className="h-3.5 w-3.5 shrink-0" />
                        <span className="truncate">{ll.ai.copilot.clear_history()}</span>
                      </button>
                    </div>
                  )}
                </div>
              )
            )}

            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7 text-muted-foreground hover:text-foreground"
              onClick={() => setCopilotOpen(false)}
              title={ll.ai.copilot.close()}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </header>
            {/* 完全信任模式生效横幅提示 */}
            {alwaysTrustHighRisk && (
              <div className="flex items-center justify-between px-3 py-1 bg-amber-500/10 border-b border-amber-500/20 text-[11px] text-amber-700 dark:text-amber-300 shrink-0">
                <div className="flex items-center gap-1.5 truncate">
                  <ShieldCheck className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 shrink-0" />
                  <span className="truncate">{ll.ai.copilot.trust_mode_active_hint()}</span>
                </div>
                <button
                  type="button"
                  className="text-[10px] text-amber-600 dark:text-amber-400 hover:underline shrink-0 ml-2 font-medium cursor-pointer"
                  onClick={() => setAlwaysTrustHighRisk(false)}
                >
                  {ll.ai.copilot.disable_trust_mode()}
                </button>
              </div>
            )}

        {/* 消息列表区域 (不透明背景，独立滚动并添加 min-h-0 防止撑开父容器) */}
        <div className="relative flex-1 min-h-0 flex flex-col">
          <div
            ref={messagesContainerRef}
            onScroll={handleMessagesScroll}
            className="flex-1 min-h-0 overflow-y-auto p-3 space-y-4 text-xs bg-background"
          >
          {/* 未配置 Provider 提示 */}
          {!activeProvider && (
            <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 p-4 text-center space-y-3">
              <Sparkles className="mx-auto h-7 w-7 text-amber-500" />
              <div className="space-y-1">
                <p className="font-medium text-xs text-foreground">
                  {ll.ai.copilot.no_active_provider()}
                </p>
                <p className="text-[11px] text-muted-foreground">
                  {ll.ai.copilot.no_active_provider_desc()}
                </p>
              </div>
              <Button
                size="sm"
                className="h-7 text-xs bg-amber-600 hover:bg-amber-700 text-white mx-auto"
                onClick={() => openSettingsDialog('ai')}
              >
                <Settings className="mr-1.5 h-3.5 w-3.5" />
                {ll.ai.copilot.configure_provider()}
              </Button>
            </div>
          )}

          {/* 空消息时的推荐快捷提示词 */}
          {activeProvider && messages.length === 0 && (
            <div className="py-6 px-2 space-y-5 text-center">
              <div className="space-y-1.5">
                <div className="inline-flex p-2.5 rounded-full bg-primary/10 text-primary">
                  <Sparkles className="h-5 w-5" />
                </div>
                <h3 className="font-semibold text-sm text-foreground">
                  {ll.ai.copilot.welcome_title()}
                </h3>
                <p className="text-xs text-muted-foreground">
                  {ll.ai.copilot.welcome_desc()}
                </p>
              </div>

              <div className="space-y-1.5 text-left pt-2">
                <p className="text-[11px] font-medium text-muted-foreground flex items-center gap-1.5 px-1">
                  <Lightbulb className="h-3 w-3 text-amber-500" />
                  <span>{ll.ai.copilot.suggested_prompts()}</span>
                </p>
                {[
                  ...(useExecutionStore.getState().report?.status === 'failed'
                    ? [ll.ai.copilot.prompt_diagnose_flow()]
                    : []),
                  ll.ai.copilot.prompt_sample_1(),
                  ll.ai.copilot.prompt_sample_2(),
                  ll.ai.copilot.prompt_sample_3(),
                  ll.ai.copilot.prompt_sample_4(),
                ].map((prompt, idx) => (
                  <button
                    key={idx}
                    type="button"
                    className="w-full text-left p-2 rounded-md border border-border/50 bg-muted/30 hover:bg-primary/5 hover:border-primary/40 transition-all group flex items-center justify-between"
                    onClick={() => handleUsePrompt(prompt)}
                  >
                    <span className="text-xs text-foreground/85 line-clamp-2">
                      {prompt}
                    </span>
                    <ArrowRight className="h-3 w-3 text-muted-foreground group-hover:text-primary shrink-0 opacity-0 group-hover:opacity-100 transition-opacity ml-1.5" />
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 消息对话气泡与上下文压缩卡片 (Turn-based 视图层聚合) */}
          {turns.map((turn, idx) => {
            if (turn.type === 'summary') {
              return (
                <AICompactedMessageCard
                  key={turn.message.id}
                  message={turn.message}
                  archivedMessages={turn.archivedMessages}
                />
              );
            }

            if (turn.type === 'user') {
              return (
                <AIMessageRow
                  key={turn.message.id}
                  message={turn.message}
                  onPreviewImage={setPreviewImageUrl}
                />
              );
            }

            if (turn.type === 'assistant') {
              const isTurnStreaming = turn.turn.messages.some(
                (m) => m.id === activeStreamingMessageId,
              );
              const isLatestTurn = idx === turns.length - 1;

              return (
                <AIAssistantTurnCard
                  key={turn.turn.id}
                  turn={turn.turn}
                  isStreaming={isTurnStreaming}
                  isGenerating={isGenerating}
                  isLatest={isLatestTurn}
                  toolResults={toolResultsMap}
                  activeStreamingThoughtRef={activeStreamingThoughtRef}
                  onPreviewImage={setPreviewImageUrl}
                  activeExecutingToolCallId={activeExecutingToolCallId}
                />
              );
            }

            return null;
          })}

          {/* 高风险待确认卡片 */}
          {pendingActions
            .filter((act) => act.status === 'pending')
            .map((act) => (
              <AIConfirmCard key={act.id} action={act} />
            ))}

          {/* 状态加载指示器 */}
          {isGenerating && agentStatus === 'executing_tools' && (() => {
            const activeTool = activeExecutingToolCallId
              ? messages
                  .flatMap((m) => m.tool_calls || [])
                  .find((tc) => tc.id === activeExecutingToolCallId)
              : messages
                  .slice()
                  .reverse()
                  .find((m) => m.role === 'assistant' && m.tool_calls && m.tool_calls.length > 0)
                  ?.tool_calls?.find((tc) => !toolResultsMap[tc.id] && !tc.output);
            const activeToolTitle = activeTool
              ? formatToolTitle(activeTool.function.name, activeTool.function.arguments).fullTitle
              : null;

            return (
              <div className="flex items-center gap-2 text-xs text-muted-foreground px-2.5 py-1.5 bg-primary/5 rounded-lg border border-primary/15 animate-in fade-in duration-150">
                <Loader2 className="h-3.5 w-3.5 animate-spin text-primary shrink-0" />
                <span className="truncate">
                  {activeToolTitle
                    ? ll.ai.copilot.executing_tool_named({ name: activeToolTitle })
                    : ll.ai.copilot.executing_tools()}
                </span>
              </div>
            );
          })()}
            <div ref={messagesEndRef} />
          </div>

          {/* 一键置底按钮 */}
          {showScrollBottom && (
            <Button
              size="icon"
              variant="secondary"
              className="absolute bottom-3 right-4 h-8 w-8 rounded-full shadow-lg border border-border/80 bg-background/90 hover:bg-muted text-muted-foreground hover:text-foreground z-20 transition-all duration-200 animate-in fade-in zoom-in-90"
              onClick={() => scrollToBottom(true)}
              aria-label={ll.ai.copilot.scroll_to_bottom()}
              title={ll.ai.copilot.scroll_to_bottom()}
            >
              <ArrowDown className="h-4 w-4" />
            </Button>
          )}
        </div>

        {/* 底部输入框区 */}
        <AICopilotInput
          textareaRef={textareaRef}
          activeProvider={activeProvider}
          activeWorkspace={activeWorkspace}
          isGenerating={isGenerating}
          onSendMessage={sendMessage}
          onStopGeneration={stopGeneration}
          onSelectWorkspaceFolder={selectWorkspaceFolder}
          onUnlinkWorkspace={() => setSessionWorkspace(null)}
          onSetWorkspaceFolder={setSessionWorkspace}
        />

        {/* 浮动模式下：右下角拉伸调整手柄 */}
        {isFloating && (
          <div
            onMouseDown={handleMouseDownFloatingResize}
            className="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize z-50 flex items-end justify-end p-0.5 text-muted-foreground/40 hover:text-foreground transition-colors select-none"
            title={ll.ai.copilot.drag_resize()}
          >
            <svg className="w-2.5 h-2.5" viewBox="0 0 6 6" fill="currentColor">
              <path d="M5 1v4H1" stroke="currentColor" strokeWidth="1.2" fill="none" />
              <path d="M5 3.5v1.5H3.5" stroke="currentColor" strokeWidth="1.2" fill="none" />
            </svg>
          </div>
        )}
      </Container>
    );
  })()}

      {/* 图片放大预览灯箱弹窗 */}
      {previewImageUrl && (
        <Dialog open={!!previewImageUrl} onOpenChange={() => setPreviewImageUrl(null)}>
          <DialogContent className="max-w-3xl p-2 bg-background border-border/80">
            <DialogTitle className="sr-only">{ll.ai.copilot.image_preview()}</DialogTitle>
            <div className="relative flex items-center justify-center p-2">
              <img
                src={previewImageUrl}
                alt="Preview"
                className="max-h-[80vh] w-auto object-contain rounded-lg shadow-lg"
              />
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
