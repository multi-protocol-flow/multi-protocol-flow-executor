/**
 * AI Copilot 侧边栏面板组件 - 会话管理与多模态图片增强版
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import {
  Sparkles,
  X,
  Trash2,
  Send,
  Square,
  Bot,
  User,
  Lightbulb,
  Brain,
  ChevronDown,
  ChevronRight,
  Settings,
  ArrowRight,
  Loader2,
  Plus,
  History,
  Image as ImageIcon,
  Edit2,
  Check,
  GripVertical,
  GripHorizontal,
  ArrowRightToLine,
  ExternalLink,
  Zap,
  AlertCircle,
  Folder,
  ShieldCheck,
  Archive,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { useExecutionStore } from '@/stores/execution';
import { AIToolCallItem } from './AIToolCallItem';
import { AIConfirmCard } from './AIConfirmCard';
import { AIErrorMessageCard } from './AIErrorMessageCard';
import { AICompactedMessageCard } from './AICompactedMessageCard';
import { findMatchingSlashCommands } from '@/services/ai/slashCommands';
import { formatToolTitle } from '@/services/ai/toolTitleFormatter';
import {
  compressAndConvertToBase64,
  extractImagesFromClipboard,
  extractImagesFromDrop,
} from '@/lib/imageUtils';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIMessage } from '@/types/ai';

export function AICopilotPanel() {
  const ll = getLL();
  const {
    sessions,
    currentSessionId,
    activeWorkspace,
    messages,
    agentStatus,
    activeStreamingMessageId,
    pendingActions,
    isCopilotOpen,
    panelWidth,
    isFloating,
    floatingPosition,
    autoExecuteToGoal,
    setPanelWidth,
    setFloating,
    setFloatingPosition,
    toggleAutoExecuteToGoal,
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

  const [inputPrompt, setInputPrompt] = useState('');
  const [attachedImages, setAttachedImages] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [openThoughts, setOpenThoughts] = useState<Record<string, boolean>>({});

  // 历史会话下拉弹层状态
  const [showSessionMenu, setShowSessionMenu] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editTitleInput, setEditTitleInput] = useState('');

  // 大图查看弹窗状态
  const [previewImageUrl, setPreviewImageUrl] = useState<string | null>(null);

  // 拖拽拉伸宽度状态
  const [isResizing, setIsResizing] = useState(false);
  // 斜杠指令选中索引
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);

  const matchingCommands =
    inputPrompt.startsWith('/') && !inputPrompt.includes(' ')
      ? findMatchingSlashCommands(inputPrompt)
      : [];

  useEffect(() => {
    setSelectedCommandIndex(0);
  }, [inputPrompt]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const activeStreamingThoughtRef = useRef<HTMLDivElement | null>(null);

  const activeProvider = aiProviders.find((p) => p.id === activeAiProviderId);
  const currentSession = sessions.find((s) => s.id === currentSessionId);
  const isGenerating =
    agentStatus === 'thinking' ||
    agentStatus === 'executing_tools' ||
    agentStatus === 'waiting_confirmation';

  const latestAssistantMsg = (() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === 'assistant') return messages[i];
    }
    return undefined;
  })();
  const latestAssistantMsgId = latestAssistantMsg?.id;

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
    };
    if (showSessionMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showSessionMenu]);

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
  if (!isCopilotOpen) return null;

  const handleSend = () => {
    const text = inputPrompt.trim();
    if (!text && attachedImages.length === 0) return;
    const imgs = attachedImages.length > 0 ? [...attachedImages] : undefined;
    setInputPrompt('');
    setAttachedImages([]);
    sendMessage(text, imgs);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (matchingCommands.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedCommandIndex((prev) => (prev + 1) % matchingCommands.length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedCommandIndex(
          (prev) => (prev - 1 + matchingCommands.length) % matchingCommands.length,
        );
        return;
      }
      if ((e.key === 'Enter' || e.key === 'Tab') && !e.shiftKey) {
        e.preventDefault();
        const selectedCmd = matchingCommands[selectedCommandIndex];
        if (selectedCmd) {
          setInputPrompt('');
          selectedCmd.run('');
          return;
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setInputPrompt('');
        return;
      }
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handlePaste = async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const images = await extractImagesFromClipboard(e.clipboardData);
    if (images.length > 0) {
      e.preventDefault();
      setAttachedImages((prev) => [...prev, ...images]);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  };

  const handleDragLeave = () => {
    setIsDragOver(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);

    // 优先检查是否拖入本地工程文件夹或项目目录
    const droppedFile = e.dataTransfer.files?.[0] as (File & { path?: string }) | undefined;
    if (droppedFile?.path && !droppedFile.type.startsWith('image/')) {
      const filePath = droppedFile.path;
      const cleanPath = filePath.replace(/[/\\]+$/, '');
      const segments = cleanPath.split(/[/\\]/);
      const name = segments[segments.length - 1] || 'workspace';
      setSessionWorkspace({
        path: filePath,
        name,
      });
      return;
    }

    const images = await extractImagesFromDrop(e.dataTransfer);
    if (images.length > 0) {
      setAttachedImages((prev) => [...prev, ...images]);
    }
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    const newImages: string[] = [];
    for (const file of files) {
      if (file.type.startsWith('image/')) {
        try {
          const base64 = await compressAndConvertToBase64(file);
          newImages.push(base64);
        } catch {
          // 忽略
        }
      }
    }
    if (newImages.length > 0) {
      setAttachedImages((prev) => [...prev, ...newImages]);
    }
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleRemoveImage = (index: number) => {
    setAttachedImages((prev) => prev.filter((_, i) => i !== index));
  };

  const handleUsePrompt = (promptText: string) => {
    sendMessage(promptText);
  };

  const toggleThought = (msgId: string) => {
    setOpenThoughts((prev) => ({ ...prev, [msgId]: !prev[msgId] }));
  };

  const handleSaveRename = (sessionId: string) => {
    if (editTitleInput.trim()) {
      renameSession(sessionId, editTitleInput.trim());
    }
    setEditingSessionId(null);
  };

  return (
    <>
      {/* AI 面板容器：支持右侧吸附与浮动窗口两种形态 */}
      {(() => {
        const effectiveWidth = isFloating ? floatingPosition.width : panelWidth;
        const Container = isFloating ? 'div' : 'aside';
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
                {/* 拖拽移动把手 */}
                <div
                  className="flex items-center justify-center p-1 rounded hover:bg-muted/70 text-muted-foreground transition-colors shrink-0 cursor-grab active:cursor-grabbing"
                  title={ll.ai.copilot.drag_to_move()}
                  onMouseDown={isFloating ? handleMouseDownFloatingHeader : handleMouseDownDockedDrag}
                >
                  <GripHorizontal className="h-3.5 w-3.5" />
                </div>
                {effectiveWidth >= 360 && (
                  <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/10 text-primary shrink-0">
                    <Sparkles className="h-3.5 w-3.5" />
                  </div>
                )}

                {/* 会话选择器与下拉抽屉触发器 */}
                <div className="relative inline-flex items-center min-w-0 flex-1 max-w-[200px]" ref={menuRef}>
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 text-xs font-semibold text-foreground hover:bg-muted/60 px-1.5 py-1 rounded-md transition-colors min-w-0 w-full"
                    onClick={() => setShowSessionMenu(!showSessionMenu)}
                    title={ll.ai.copilot.sessions_title()}
                  >
                    <span className="truncate min-w-0 flex-1 text-left">
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

            {activeProvider && effectiveWidth >= 480 && (
              <span className="truncate text-[10px] px-1.5 py-0.5 rounded font-mono bg-muted text-muted-foreground max-w-[90px] shrink-0">
                {activeProvider.name}
              </span>
            )}
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            {/* 执行到底模式 Switch 开关 */}
            <button
              type="button"
              role="switch"
              aria-checked={autoExecuteToGoal}
              className={`flex items-center gap-1.5 h-7 rounded-md text-[11px] font-medium border transition-colors cursor-pointer select-none shrink-0 ${
                effectiveWidth < 380 ? 'px-1.5' : 'px-2'
              } ${
                autoExecuteToGoal
                  ? 'bg-emerald-500/10 border-emerald-500/35 text-emerald-600 dark:bg-emerald-950/40 dark:border-emerald-600/40 dark:text-emerald-300'
                  : 'bg-muted/40 border-border/80 text-muted-foreground hover:bg-muted/60 dark:bg-zinc-900/60 dark:border-zinc-800'
              }`}
              onClick={toggleAutoExecuteToGoal}
              title={ll.ai.copilot.auto_execute_tip()}
            >
              <Zap
                className={`h-3.5 w-3.5 shrink-0 transition-transform ${
                  autoExecuteToGoal
                    ? 'fill-emerald-500 text-emerald-500 dark:fill-emerald-400 dark:text-emerald-400 scale-105'
                    : 'text-muted-foreground/70'
                }`}
              />
              <span
                className={`${
                  effectiveWidth < 440 ? 'sr-only' : 'inline font-medium'
                } ${autoExecuteToGoal ? 'text-emerald-700 dark:text-emerald-300 font-semibold' : 'text-muted-foreground'}`}
              >
                {ll.ai.copilot.auto_execute()}
              </span>
              {effectiveWidth >= 360 && (
                <span className="pointer-events-none inline-flex items-center" aria-hidden="true">
                  <Switch
                    checked={autoExecuteToGoal}
                    tabIndex={-1}
                    variant="emerald"
                    className="scale-75 origin-center"
                  />
                </span>
              )}
            </button>
            {/* 完全信任模式 Switch 开关 */}
            <button
              type="button"
              role="switch"
              aria-checked={alwaysTrustHighRisk}
              className={`flex items-center gap-1.5 h-7 rounded-md text-[11px] font-medium border transition-colors cursor-pointer select-none shrink-0 ${
                effectiveWidth < 380 ? 'px-1.5' : 'px-2'
              } ${
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
                className={`${
                  effectiveWidth < 460 ? 'sr-only' : 'inline font-medium'
                } ${alwaysTrustHighRisk ? 'text-amber-700 dark:text-amber-300 font-semibold' : 'text-muted-foreground'}`}
              >
                {ll.ai.copilot.trust_mode()}
              </span>
              {effectiveWidth >= 360 && (
                <span className="pointer-events-none inline-flex items-center" aria-hidden="true">
                  <Switch
                    checked={alwaysTrustHighRisk}
                    tabIndex={-1}
                    variant="amber"
                    className="scale-75 origin-center"
                  />
                </span>
              )}
            </button>
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

            {/* 上下文压缩按钮 */}
            {messages.length > 2 && (
              <Button
                size="icon"
                variant="ghost"
                className="h-7 w-7 text-muted-foreground hover:text-indigo-500 hover:bg-indigo-500/10 transition-colors"
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
                className="h-7 w-7 text-muted-foreground hover:text-foreground"
                onClick={clearMessages}
                title={ll.ai.copilot.clear_history()}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
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
        <div
          ref={messagesContainerRef}
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

          {/* 消息对话气泡与上下文压缩卡片 */}
          {(() => {
            const renderNormalMessage = (msg: AIMessage) => {
              if (msg.role === 'system') return null;

              if (msg.role === 'user') {
                return (
                  <div key={msg.id} className="flex justify-end gap-2 pl-6">
                    <div className="max-w-[88%] space-y-1.5">
                      {/* 用户附加的图片展示 */}
                      {msg.images && msg.images.length > 0 && (
                        <div className="flex flex-wrap justify-end gap-1.5">
                          {msg.images.map((imgSrc, idx) => (
                            <button
                              key={idx}
                              type="button"
                              className="overflow-hidden rounded-lg border border-primary/30 max-w-[180px] max-h-[140px] hover:opacity-90 transition-opacity cursor-zoom-in"
                              onClick={() => setPreviewImageUrl(imgSrc)}
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

              if (msg.role === 'assistant') {
                if (msg.status === 'error') {
                  return (
                    <div key={msg.id} className="flex justify-start gap-2 pr-4">
                      <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive mt-0.5">
                        <AlertCircle className="h-3.5 w-3.5" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <AIErrorMessageCard message={msg} />
                      </div>
                    </div>
                  );
                }

                const isStreamingThis = activeStreamingMessageId === msg.id;
                const hasContent = Boolean(msg.content && msg.content.trim().length > 0);
                const hasThought = Boolean(msg.thought && msg.thought.trim().length > 0);
                const hasToolCalls = Boolean(msg.tool_calls && msg.tool_calls.length > 0);

                // 既无正文、又无思考、又无工具调用，且当前没有处于流式中：绝对不渲染多余的空白头像！
                if (!hasContent && !hasThought && !hasToolCalls && !isStreamingThis) {
                  return null;
                }

                const isLatestAssistant = msg.id === latestAssistantMsgId;
                const thoughtOpen =
                  openThoughts[msg.id] !== undefined
                    ? openThoughts[msg.id]
                    : (isStreamingThis || (isGenerating && isLatestAssistant));

                return (
                  <div key={msg.id} className="flex justify-start gap-2 pr-4">
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
                            onClick={() => toggleThought(msg.id)}
                          >
                            <Brain className="h-3 w-3 text-indigo-500 shrink-0" />
                            <span className="font-medium text-[11px] truncate min-w-0 flex-1">
                              {ll.ai.copilot.thought_process()}
                              {isStreamingThis && (
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
                              ref={isStreamingThis ? activeStreamingThoughtRef : undefined}
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
                      ) : isStreamingThis && !msg.thought && (!msg.tool_calls || msg.tool_calls.length === 0) ? (
                        <div className="flex items-center gap-1.5 text-muted-foreground py-1">
                          <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />
                          <span className="text-xs">{ll.ai.copilot.thinking()}</span>
                        </div>
                      ) : null}

                      {/* 工具调用项展示 */}
                      {msg.tool_calls && msg.tool_calls.length > 0 && (
                        <div className="space-y-1">
                          {msg.tool_calls.map((tc) => {
                            const toolMsg = messages.find(
                              (m) => m.role === 'tool' && m.tool_call_id === tc.id,
                            );
                            const resultStr = tc.output ?? toolMsg?.content;
                            return (
                              <AIToolCallItem
                                key={tc.id}
                                toolCall={tc}
                                result={resultStr}
                                isExecuted={!isStreamingThis && resultStr !== undefined}
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
            };

            const elements: React.ReactNode[] = [];
            let pendingArchived: AIMessage[] = [];

            for (let i = 0; i < messages.length; i++) {
              const msg = messages[i];
              if (msg.role === 'system') continue;

              if (msg.isArchived) {
                pendingArchived.push(msg);
                continue;
              }

              if (msg.isSummary) {
                const archivedForThis = pendingArchived;
                pendingArchived = [];
                elements.push(
                  <AICompactedMessageCard
                    key={msg.id}
                    message={msg}
                    archivedMessages={archivedForThis}
                  />,
                );
                continue;
              }

              elements.push(renderNormalMessage(msg));
            }

            if (pendingArchived.length > 0) {
              elements.unshift(
                <AICompactedMessageCard
                  key="fallback_archived"
                  message={{
                    id: 'fallback_archived',
                    role: 'assistant',
                    content: '',
                    timestamp: Date.now(),
                    compactMeta: {
                      compressedMessageCount: pendingArchived.length,
                      timestamp: Date.now(),
                    },
                  }}
                  archivedMessages={pendingArchived}
                />,
              );
            }

            return elements;
          })()}

          {/* 高风险待确认卡片 */}
          {pendingActions
            .filter((act) => act.status === 'pending')
            .map((act) => (
              <AIConfirmCard key={act.id} action={act} />
            ))}

          {/* 状态加载指示器 */}
          {isGenerating && agentStatus === 'executing_tools' && (() => {
            const lastMsg = messages[messages.length - 1];
            const activeTool = lastMsg?.tool_calls?.find(
              (tc) => !messages.some((m) => m.role === 'tool' && m.tool_call_id === tc.id),
            );
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

        {/* 底部输入框区 */}
        <footer
          className="p-3 border-t border-border bg-background shrink-0 space-y-2"
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
        >
          {/* 工作区胶囊组件 (Workspace Context Pill) */}
          <div className="flex items-center justify-between">
            {!activeWorkspace ? (
              <button
                type="button"
                onClick={() => selectWorkspaceFolder()}
                className="inline-flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium text-muted-foreground hover:text-foreground bg-muted/40 hover:bg-muted/70 rounded-md border border-border/60 transition-colors"
                title={ll.ai.copilot.link_workspace_tip()}
              >
                <Folder className="h-3.5 w-3.5 text-primary/70" />
                <span>{ll.ai.copilot.link_workspace()}</span>
              </button>
            ) : (
              <div className="w-full flex items-center justify-between text-[11px] bg-primary/5 border border-primary/20 rounded-md px-2 py-1 text-foreground">
                <div
                  className="flex items-center gap-1.5 min-w-0 max-w-[240px] truncate cursor-help"
                  title={`${activeWorkspace.path}${activeWorkspace.techStack ? ` (${activeWorkspace.techStack})` : ''}`}
                >
                  <Folder className="h-3.5 w-3.5 text-primary shrink-0" />
                  <span className="font-medium text-foreground truncate">{activeWorkspace.name}</span>
                  <span className="text-[10px] text-muted-foreground truncate max-w-[120px]">{activeWorkspace.path}</span>
                  {activeWorkspace.techStack && (
                    <span className="shrink-0 px-1 py-0.2 text-[9px] bg-primary/10 text-primary rounded font-mono">
                      {activeWorkspace.techStack}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-1 shrink-0 ml-2">
                  <button
                    type="button"
                    onClick={() => selectWorkspaceFolder()}
                    className="text-[10px] text-muted-foreground hover:text-foreground px-1 py-0.5 rounded hover:bg-muted transition-colors"
                    title={ll.ai.copilot.switch_workspace_tip()}
                  >
                    {ll.ai.copilot.switch_workspace()}
                  </button>
                  <button
                    type="button"
                    onClick={() => setSessionWorkspace(null)}
                    className="text-muted-foreground hover:text-destructive p-0.5 rounded hover:bg-destructive/10 transition-colors"
                    title={ll.ai.copilot.unlink_workspace_tip()}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* 图片附件待发预览条 */}
          {attachedImages.length > 0 && (
            <div className="flex flex-wrap gap-2 p-1.5 bg-muted/30 rounded-lg border border-border/50">
              {attachedImages.map((imgSrc, idx) => (
                <div key={idx} className="relative group">
                  <img
                    src={imgSrc}
                    alt={`Preview ${idx + 1}`}
                    className="h-12 w-12 object-cover rounded-md border border-border"
                  />
                  <button
                    type="button"
                    className="absolute -top-1 -right-1 h-4 w-4 rounded-full bg-destructive text-destructive-foreground flex items-center justify-center text-[10px] shadow-sm hover:scale-110 transition-transform"
                    onClick={() => handleRemoveImage(idx)}
                    title={ll.ai.copilot.remove_image()}
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          )}

          <div
            className={`relative rounded-lg border bg-background transition-all ${
              isDragOver
                ? 'border-primary ring-2 ring-primary/20 bg-primary/5'
                : 'border-border focus-within:border-primary focus-within:ring-1 focus-within:ring-primary/30'
            }`}
          >
            {/* 斜杠快捷指令自动补全浮层 */}
            {matchingCommands.length > 0 && (
              <div className="absolute bottom-full left-0 right-0 mb-1.5 rounded-lg border border-border bg-popover text-popover-foreground shadow-xl p-1 z-30 space-y-0.5 animate-in fade-in slide-in-from-bottom-1 duration-100">
                <div className="px-2 py-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center justify-between border-b border-border/40 select-none">
                  <span>{ll.ai.copilot.slash_commands_title()}</span>
                  <span className="text-[9px] font-normal font-mono">{ll.ai.copilot.slash_commands_hint()}</span>
                </div>
                <div className="max-h-48 overflow-y-auto space-y-0.5">
                  {matchingCommands.map((cmd, idx) => {
                    const isSelected = idx === selectedCommandIndex;
                    return (
                      <button
                        key={cmd.id}
                        type="button"
                        className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-left text-xs transition-colors cursor-pointer ${
                          isSelected
                            ? 'bg-primary text-primary-foreground font-medium'
                            : 'hover:bg-muted text-foreground'
                        }`}
                        onMouseEnter={() => setSelectedCommandIndex(idx)}
                        onClick={() => {
                          setInputPrompt('');
                          cmd.run('');
                        }}
                      >
                        <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
                          <span className="font-mono font-semibold">{cmd.name}</span>
                          <span
                            className={`truncate text-xs ${
                              isSelected
                                ? 'text-primary-foreground/90'
                                : 'text-muted-foreground'
                            }`}
                          >
                            {cmd.title()}
                          </span>
                        </div>
                        <span
                          className={`text-[10px] truncate max-w-[150px] shrink-0 ${
                            isSelected
                              ? 'text-primary-foreground/75'
                              : 'text-muted-foreground/60'
                          }`}
                        >
                          {cmd.description()}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

            <textarea
              ref={textareaRef}
              value={inputPrompt}
              onChange={(e) => setInputPrompt(e.target.value)}
              onKeyDown={handleKeyDown}
              onPaste={handlePaste}
              placeholder={
                activeProvider
                  ? isDragOver
                    ? ll.ai.copilot.drop_image_here()
                    : isGenerating
                      ? ll.ai.copilot.input_midway_placeholder()
                      : ll.ai.copilot.input_placeholder()
                  : ll.ai.copilot.input_disabled_placeholder()
              }
              rows={2}
              className="w-full resize-none border-0 bg-transparent p-2.5 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 shadow-none min-h-[50px] max-h-32 disabled:cursor-not-allowed disabled:opacity-50"
            />

            {/* 隐藏的图片文件选择器 */}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              onChange={handleFileSelect}
            />

            <div className="flex items-center justify-between px-2.5 pb-2">
              <div className="flex items-center gap-1">
                {/* 附加图片按钮 */}
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-6 w-6 text-muted-foreground hover:text-foreground"
                  onClick={() => fileInputRef.current?.click()}
                  disabled={!activeProvider}
                  title={ll.ai.copilot.attach_image()}
                >
                  <ImageIcon className="h-3.5 w-3.5" />
                </Button>
                <span className="text-[10px] text-muted-foreground">
                  {attachedImages.length > 0
                    ? ll.ai.copilot.image_attached({ count: attachedImages.length })
                    : ll.ai.copilot.image_support()}
                </span>
              </div>

              <div className="flex items-center gap-1.5">
                {isGenerating ? (
                  <>
                    <Button
                      size="sm"
                      variant="destructive"
                      className="h-6 px-2 text-[11px] rounded-md gap-1"
                      onClick={stopGeneration}
                      title={ll.ai.copilot.stop()}
                    >
                      <Square className="h-3 w-3 fill-current" />
                      {ll.ai.copilot.stop()}
                    </Button>
                    {Boolean(inputPrompt.trim() || attachedImages.length > 0) && (
                      <Button
                        size="sm"
                        className="h-6 px-2 text-[11px] rounded-md bg-amber-600 hover:bg-amber-700 text-white gap-1 animate-in fade-in duration-100"
                        onClick={handleSend}
                        title={ll.ai.copilot.send_correction_tip()}
                      >
                        <Send className="h-3 w-3" />
                        <span>{ll.ai.copilot.send_correction()}</span>
                      </Button>
                    )}
                  </>
                ) : (
                  <Button
                    size="sm"
                    className="h-6 w-6 p-0 rounded-md bg-primary hover:bg-primary/90 text-primary-foreground"
                    onClick={handleSend}
                    disabled={(!inputPrompt.trim() && attachedImages.length === 0) || !activeProvider}
                    title={ll.ai.copilot.send()}
                  >
                    <Send className="h-3 w-3" />
                  </Button>
                )}
              </div>
            </div>
          </div>
        </footer>

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
