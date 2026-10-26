/**
 * FlowTabBar - 多流程标签栏组件
 * 支持流程标签切换、运行状态指示、未保存脏状态拦截、右键上下文菜单与快捷新建
 */
import React, { useState, useRef, useEffect, useCallback, memo } from 'react';
import {
  X,
  Plus,
  Bug,
  Loader2,
  Sliders,
  XCircle,
  ArrowRightToLine,
  Layers,
  MoveHorizontal,
  WrapText,
  Check,
  ChevronDown,
  LocateFixed,
  ClipboardCopy,
  FolderOpen,
} from 'lucide-react';
import * as Popover from '@radix-ui/react-popover';
import { toast } from 'sonner';
import {
  useFlowStore,
  useFlowListStore,
  useUIStore,
  useExecutionStore,
  useDebugStore,
  selectTabs,
  selectActiveTabUuid,
  selectTabSessions,
  selectIsExecuting,
  selectIsDebugging,
  selectFlowTabDisplayMode,
} from '@/stores';
import { flowStorage } from '@/services/storage';
import { copyToClipboard } from '@/lib/clipboard';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import type { FlowTab } from '@/types';
import { revealItemInDir } from '@/services/tauri/settings';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

interface TabContextMenuState {
  x: number;
  y: number;
  tab?: FlowTab;
}

export const FlowTabBar = memo(function FlowTabBar() {
  const tabs = useFlowStore(selectTabs);
  const activeTabUuid = useFlowStore(selectActiveTabUuid);
  const tabSessions = useFlowStore(selectTabSessions);
  const currentFlow = useFlowStore((s) => s.flow);
  const switchTab = useFlowStore((s) => s.switchTab);
  const closeTab = useFlowStore((s) => s.closeTab);
  const closeOtherTabs = useFlowStore((s) => s.closeOtherTabs);
  const closeTabsToRight = useFlowStore((s) => s.closeTabsToRight);
  const closeAllTabs = useFlowStore((s) => s.closeAllTabs);
  const selectNode = useFlowStore((s) => s.selectNode);

  const openNewFlowDialog = useUIStore((s) => s.openNewFlowDialog);
  const setConfigPanelOpen = useUIStore((s) => s.setConfigPanelOpen);
  const showConfirm = useUIStore((s) => s.showConfirm);

  const isExecuting = useExecutionStore(selectIsExecuting);
  const isDebugging = useDebugStore(selectIsDebugging);
  const flowTabDisplayMode = useUIStore(selectFlowTabDisplayMode);
  const setFlowTabDisplayMode = useUIStore((s) => s.setFlowTabDisplayMode);
  const [isModePopoverOpen, setIsModePopoverOpen] = useState(false);
  const activeTabRef = useRef<HTMLDivElement>(null);

  // 当激活标签切换或显示模式改变为单行滚动时，平滑滚动至视野内
  useEffect(() => {
    if (
      flowTabDisplayMode === 'scroll' &&
      typeof activeTabRef.current?.scrollIntoView === 'function'
    ) {
      activeTabRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
        inline: 'nearest',
      });
    }
  }, [activeTabUuid, flowTabDisplayMode]);

  // 右键菜单状态
  const [contextMenu, setContextMenu] = useState<TabContextMenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // 未保存关闭确认弹窗目标 tab
  const [unsavedTabTarget, setUnsavedTabTarget] = useState<FlowTab | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  // 容器横向滚动 ref
  const tabsContainerRef = useRef<HTMLDivElement>(null);
  // 横向滚动尺寸与位置状态（驱动纯 DOM 3px 超细滚动条）
  const [scrollMetrics, setScrollMetrics] = useState({
    scrollLeft: 0,
    scrollWidth: 0,
    clientWidth: 0,
  });

  const metricsRafIdRef = useRef<number | null>(null);

  const updateScrollMetrics = useCallback(() => {
    const el = tabsContainerRef.current;
    if (!el) return;
    setScrollMetrics((prev) => {
      if (
        prev.scrollLeft === el.scrollLeft &&
        prev.scrollWidth === el.scrollWidth &&
        prev.clientWidth === el.clientWidth
      ) {
        return prev;
      }
      return {
        scrollLeft: el.scrollLeft,
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
      };
    });
  }, []);

  // 监听容器尺寸与标签增减变动（仅监听容器，并加入 rAF 节流防抖，避免布局过渡期间高频重绘）
  useEffect(() => {
    const el = tabsContainerRef.current;
    if (!el) return;
    updateScrollMetrics();

    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(() => {
        if (metricsRafIdRef.current !== null) {
          cancelAnimationFrame(metricsRafIdRef.current);
        }
        metricsRafIdRef.current = requestAnimationFrame(() => {
          metricsRafIdRef.current = null;
          updateScrollMetrics();
        });
      });
      observer.observe(el);
      return () => {
        if (metricsRafIdRef.current !== null) {
          cancelAnimationFrame(metricsRafIdRef.current);
          metricsRafIdRef.current = null;
        }
        observer.disconnect();
      };
    }
  }, [tabs, flowTabDisplayMode, updateScrollMetrics]);

  const isDraggingRef = useRef(false);
  const dragStartRef = useRef({ startX: 0, startScrollLeft: 0 });

  const handleThumbMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const container = tabsContainerRef.current;
    if (!container) return;
    isDraggingRef.current = true;
    dragStartRef.current = {
      startX: e.clientX,
      startScrollLeft: container.scrollLeft,
    };

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDraggingRef.current || !tabsContainerRef.current) return;
      const { scrollWidth, clientWidth } = tabsContainerRef.current;
      const maxScrollLeft = scrollWidth - clientWidth;
      const currentThumbWidth = Math.max(20, (clientWidth / scrollWidth) * clientWidth);
      const maxThumbLeft = clientWidth - currentThumbWidth;
      if (maxThumbLeft <= 0) return;

      const deltaX = ev.clientX - dragStartRef.current.startX;
      tabsContainerRef.current.scrollLeft =
        dragStartRef.current.startScrollLeft + (deltaX / maxThumbLeft) * maxScrollLeft;
      updateScrollMetrics();
    };

    const handleMouseUp = () => {
      isDraggingRef.current = false;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleTrackClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const container = tabsContainerRef.current;
    if (!container) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const ratio = clickX / rect.width;
    const maxScrollLeft = container.scrollWidth - container.clientWidth;
    container.scrollTo({
      left: ratio * maxScrollLeft,
      behavior: 'smooth',
    });
  };

  // 监听点击外部关闭右键菜单
  useEffect(() => {
    if (!contextMenu) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setContextMenu(null);
      }
    };
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [contextMenu]);

  // 切换标签处理（策略 A 拦截）
  const handleSelectTab = useCallback(
    (tab: FlowTab) => {
      if (tab.uuid === activeTabUuid) return;
      const LL = getLL();
      if (isExecuting) {
        toast.info(LL.flow.executing_no_switch());
        return;
      }
      if (isDebugging) {
        toast.info(LL.flow.debug_no_switch());
        return;
      }
      // 切换前主动让活跃输入控件失焦并触发表单刷盘，确保当前流程的修改在本流程内落盘
      if (typeof document !== 'undefined' && document.activeElement instanceof HTMLElement) {
        document.activeElement.blur();
      }
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('flush-config-panel'));
      }
      switchTab(tab.uuid);
    },
    [activeTabUuid, isExecuting, isDebugging, switchTab],
  );

  // 单标签关闭处理（策略 A 拦截与脏状态检查）
  const handleCloseTab = useCallback(
    (tab: FlowTab) => {
      const LL = getLL();
      if (tab.uuid === activeTabUuid) {
        if (isExecuting) {
          toast.info(LL.flow.executing_no_switch());
          return;
        }
        if (isDebugging) {
          toast.info(LL.flow.debug_no_switch());
          return;
        }
      }

      if (tab.isDirty) {
        setUnsavedTabTarget(tab);
        return;
      }

      closeTab(tab.uuid);
    },
    [activeTabUuid, isExecuting, isDebugging, closeTab],
  );

  // 保存并关闭
  const handleSaveAndClose = useCallback(async () => {
    if (!unsavedTabTarget) return;
    setIsSaving(true);
    try {
      const session = tabSessions[unsavedTabTarget.uuid];
      const flowToSave =
        unsavedTabTarget.uuid === activeTabUuid && currentFlow
          ? currentFlow
          : session?.flow;
      if (flowToSave) {
        await flowStorage.saveFlow(flowToSave);
      }
      closeTab(unsavedTabTarget.uuid);
      setUnsavedTabTarget(null);
    } catch (err) {
      console.error('[FlowTabBar] Failed to save flow:', err);
      toast.error(getLL().flow.save_flow_failed());
    } finally {
      setIsSaving(false);
    }
  }, [unsavedTabTarget, tabSessions, activeTabUuid, currentFlow, closeTab]);

  // 放弃修改并关闭
  const handleDiscardAndClose = useCallback(() => {
    if (!unsavedTabTarget) return;
    closeTab(unsavedTabTarget.uuid);
    setUnsavedTabTarget(null);
  }, [unsavedTabTarget, closeTab]);

  // 关闭其他标签
  const handleCloseOtherTabs = useCallback(
    (targetTab: FlowTab) => {
      const LL = getLL();
      if (isExecuting || isDebugging) {
        toast.info(isExecuting ? LL.flow.executing_no_switch() : LL.flow.debug_no_switch());
        return;
      }

      const dirtyOthers = tabs.filter(
        (t) => t.uuid !== targetTab.uuid && t.isDirty,
      );
      if (dirtyOthers.length > 0) {
        showConfirm({
          title: LL.flow.unsaved_tab_confirm_title(),
          description: () =>
            LL.flow.unsaved_other_tabs_confirm_desc({ names: dirtyOthers.map((d) => `"${d.name}"`).join(', ') }),
          confirmText: LL.flow.discard_and_close(),
          onConfirm: () => closeOtherTabs(targetTab.uuid),
        });
      } else {
        closeOtherTabs(targetTab.uuid);
      }
    },
    [tabs, isExecuting, isDebugging, showConfirm, closeOtherTabs],
  );

  // 关闭右侧标签
  const handleCloseTabsToRight = useCallback(
    (targetTab: FlowTab) => {
      const LL = getLL();
      if (isExecuting || isDebugging) {
        toast.info(isExecuting ? LL.flow.executing_no_switch() : LL.flow.debug_no_switch());
        return;
      }

      const idx = tabs.findIndex((t) => t.uuid === targetTab.uuid);
      if (idx === -1) return;
      const rightTabs = tabs.slice(idx + 1);
      const dirtyRight = rightTabs.filter((t) => t.isDirty);

      if (dirtyRight.length > 0) {
        showConfirm({
          title: LL.flow.unsaved_tab_confirm_title(),
          description: () =>
            LL.flow.unsaved_right_tabs_confirm_desc({ names: dirtyRight.map((d) => `"${d.name}"`).join(', ') }),
          confirmText: LL.flow.discard_and_close(),
          onConfirm: () => closeTabsToRight(targetTab.uuid),
        });
      } else {
        closeTabsToRight(targetTab.uuid);
      }
    },
    [tabs, isExecuting, isDebugging, showConfirm, closeTabsToRight],
  );

  // 关闭所有标签
  const handleCloseAllTabs = useCallback(() => {
    const LL = getLL();
    if (isExecuting || isDebugging) {
      toast.info(isExecuting ? LL.flow.executing_no_switch() : LL.flow.debug_no_switch());
      return;
    }

    const dirtyTabs = tabs.filter((t) => t.isDirty);
    if (dirtyTabs.length > 0) {
      showConfirm({
        title: LL.flow.unsaved_tab_confirm_title(),
        description: () =>
          LL.flow.unsaved_all_tabs_confirm_desc({ names: dirtyTabs.map((d) => `"${d.name}"`).join(', ') }),
        confirmText: LL.flow.discard_and_close(),
        onConfirm: () => closeAllTabs(),
      });
    } else {
      closeAllTabs();
    }
  }, [tabs, isExecuting, isDebugging, showConfirm, closeAllTabs]);

  // 打开流程属性面板
  const handleOpenProperties = useCallback(
    (tab: FlowTab) => {
      if (tab.uuid !== activeTabUuid) {
        switchTab(tab.uuid);
      }
      selectNode(null);
      setConfigPanelOpen(true);
    },
    [activeTabUuid, switchTab, selectNode, setConfigPanelOpen],
  );

  // 复制流程文件绝对路径
  const handleCopyFilePath = useCallback(async (tab: FlowTab) => {
    try {
      let targetPath =
        tab.filePath ||
        useFlowListStore.getState().flows.find((f) => f.uuid === tab.uuid)?.file_path;
      if (!targetPath) {
        const flowMetas = await flowStorage.listFlows();
        const found = flowMetas.find((f) => f.uuid === tab.uuid);
        if (found?.filePath) {
          targetPath = found.filePath;
          useFlowListStore.getState().updateFlow(tab.uuid, { file_path: found.filePath });
        }
      }
      if (!targetPath) {
        toast.error(getLL().flow.copy_file_path_not_found());
        return;
      }
      const success = await copyToClipboard(targetPath);
      if (success) {
        toast.success(getLL().flow.copy_file_path_success());
      } else {
        toast.error(getLL().flow.copy_file_path_failed());
      }
    } catch (err) {
      console.error('[FlowTabBar] Failed to copy flow file path:', err);
      toast.error(getLL().flow.copy_file_path_failed());
    }
  }, []);

  // 在系统文件管理器中显示流程文件
  const handleRevealInFileManager = useCallback(async (tab: FlowTab) => {
    try {
      let targetPath =
        tab.filePath ||
        useFlowListStore.getState().flows.find((f) => f.uuid === tab.uuid)?.file_path;
      if (!targetPath) {
        const flowMetas = await flowStorage.listFlows();
        const found = flowMetas.find((f) => f.uuid === tab.uuid);
        if (found?.filePath) {
          targetPath = found.filePath;
          useFlowListStore.getState().updateFlow(tab.uuid, { file_path: found.filePath });
        }
      }
      if (!targetPath) {
        toast.error(getLL().flow.copy_file_path_not_found());
        return;
      }
      await revealItemInDir(targetPath);
    } catch (err) {
      console.error('[FlowTabBar] Failed to reveal in file manager:', err);
      toast.error(getLL().flow.reveal_in_file_manager_failed());
    }
  }, []);

  // 滚轮横向滚动（仅单行滚动模式下生效）
  const handleWheel = (e: React.WheelEvent) => {
    if (flowTabDisplayMode === 'scroll' && e.deltaY !== 0 && tabsContainerRef.current) {
      tabsContainerRef.current.scrollLeft += e.deltaY;
      updateScrollMetrics();
    }
  };

  const targetTabIndex = contextMenu?.tab
    ? tabs.findIndex((t) => t.uuid === contextMenu.tab?.uuid)
    : -1;

  const LL = getLL();
  const hasOverflow =
    flowTabDisplayMode === 'scroll' &&
    scrollMetrics.scrollWidth > scrollMetrics.clientWidth + 2;

  const thumbWidth =
    hasOverflow && scrollMetrics.clientWidth > 0
      ? Math.max(24, (scrollMetrics.clientWidth / scrollMetrics.scrollWidth) * scrollMetrics.clientWidth)
      : 0;

  const maxScrollLeft = Math.max(1, scrollMetrics.scrollWidth - scrollMetrics.clientWidth);
  const maxThumbLeft = Math.max(0, scrollMetrics.clientWidth - thumbWidth);
  const thumbLeft =
    maxScrollLeft > 0 ? (scrollMetrics.scrollLeft / maxScrollLeft) * maxThumbLeft : 0;


  return (
    <>
      <div
        onContextMenu={(e) => {
          if (e.target === e.currentTarget || (tabsContainerRef.current && e.target === tabsContainerRef.current)) {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY });
          }
        }}
        className={cn(
          'bg-background border-b border-border select-none shrink-0 z-10 transition-[height] duration-150',
          flowTabDisplayMode === 'scroll'
            ? 'h-9 min-h-9 flex items-center justify-between px-2 gap-1.5'
            : 'min-h-9 h-auto max-h-36 flex items-start justify-between px-2 py-1 gap-1.5 overflow-y-auto scrollbar-mini'
        )}
      >
        {/* 标签列表区与超细自定义滚动条容器 */}
        <div className="relative flex-1 min-w-0 h-full flex flex-col justify-center">
          <div
            ref={tabsContainerRef}
            onWheel={handleWheel}
            onScroll={updateScrollMetrics}
            className={cn(
              'select-none w-full no-scrollbar scrollbar-none scrollbar-mini',
              flowTabDisplayMode === 'scroll'
                ? 'flex items-center gap-1 overflow-x-auto py-1'
                : 'flex flex-wrap items-center gap-x-1 gap-y-1.5 py-0.5'
            )}
          >
            {tabs.map((tab) => {
              const isActive = tab.uuid === activeTabUuid;
              const isTabExecuting = isActive && isExecuting;
              const isTabDebugging = isActive && isDebugging;

              return (
                <div
                  key={tab.uuid}
                  ref={isActive ? activeTabRef : undefined}
                  role="tab"
                  aria-selected={isActive}
                  onClick={() => handleSelectTab(tab)}
                  onAuxClick={(e) => {
                    if (e.button === 1) {
                      e.preventDefault();
                      e.stopPropagation();
                      handleCloseTab(tab);
                    }
                  }}
                  onDoubleClick={() => handleOpenProperties(tab)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setContextMenu({ x: e.clientX, y: e.clientY, tab });
                  }}
                  className={cn(
                    'group relative flex items-center gap-1.5 h-7 px-2.5 rounded-md text-xs cursor-pointer transition-all shrink-0 max-w-[190px]',
                    isActive
                      ? 'bg-accent text-accent-foreground font-medium shadow-xs border border-border/60'
                      : 'text-muted-foreground hover:bg-accent/40 hover:text-foreground border border-transparent',
                  )}
                  title={tab.name}
                >
                  {/* 运行状态指示器 */}
                  {isTabExecuting && (
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-emerald-500 shrink-0" />
                  )}
                  {isTabDebugging && (
                    <Bug className="w-3.5 h-3.5 text-amber-500 shrink-0 animate-pulse" />
                  )}

                  {/* 流程名称 */}
                  <span className="max-w-[120px] truncate">{tab.name}</span>

                  {/* 未保存小圆点 */}
                  {tab.isDirty && (
                    <span
                      className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0 animate-pulse"
                      title={LL.flow.unsaved()}
                    />
                  )}

                  {/* 关闭按钮 */}
                  <button
                    type="button"
                    aria-label={LL.flow.close_tab()}
                    className="p-0.5 rounded-xs hover:bg-muted-foreground/20 text-muted-foreground hover:text-foreground opacity-60 hover:opacity-100 transition-opacity"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCloseTab(tab);
                    }}
                  >
                    <X className="w-3 h-3" />
                  </button>
                </div>
              );
            })}

            {/* 新建流程按钮 */}
            <button
              type="button"
              onClick={() => openNewFlowDialog()}
              className="flex items-center justify-center h-6 w-6 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors shrink-0 cursor-pointer"
              title={LL.flow.new_flow()}
              data-guide="new-flow-btn"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* 纯 DOM 自定义超细（3px）横向滚动条，彻底消除 Linux GTK / 系统原生粗滚动条 */}
          {hasOverflow && (
            <div
              role="scrollbar"
              aria-orientation="horizontal"
              onClick={handleTrackClick}
              className="absolute bottom-0 left-0 right-0 h-[3px] hover:h-[5px] bg-muted/20 hover:bg-muted/50 rounded-full transition-all cursor-pointer z-20"
            >
              <div
                onMouseDown={handleThumbMouseDown}
                style={{
                  width: `${thumbWidth}px`,
                  transform: `translateX(${thumbLeft}px)`,
                }}
                className="h-full rounded-full bg-muted-foreground/35 hover:bg-muted-foreground/70 active:bg-muted-foreground transition-colors cursor-grab active:cursor-grabbing"
              />
            </div>
          )}
        </div>

        {/* 标签栏右侧操作区（显示模式切换等） */}
        <div className={cn("flex items-center gap-1 shrink-0", flowTabDisplayMode === 'wrap' && "pt-0.5")}>
          <Popover.Root open={isModePopoverOpen} onOpenChange={setIsModePopoverOpen}>
            <Popover.Trigger asChild>
              <button
                type="button"
                aria-label={LL.flow.tab_display_mode()}
                title={`${LL.flow.tab_display_mode()} (${flowTabDisplayMode === 'scroll' ? LL.flow.tab_mode_scroll() : LL.flow.tab_mode_wrap()})`}
                className={cn(
                  "flex items-center gap-1 h-6 px-1.5 rounded-md text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer shrink-0 border border-transparent",
                  isModePopoverOpen && "bg-accent text-foreground border-border/60"
                )}
              >
                {flowTabDisplayMode === 'scroll' ? (
                  <MoveHorizontal className="w-3.5 h-3.5" />
                ) : (
                  <WrapText className="w-3.5 h-3.5" />
                )}
                <ChevronDown className="w-3 h-3 opacity-60" />
              </button>
            </Popover.Trigger>
            <Popover.Portal>
              <Popover.Content
                side="bottom"
                align="end"
                sideOffset={6}
                className={cn(
                  "z-50 w-56 rounded-lg border border-border/60 bg-popover p-1.5 text-popover-foreground shadow-xl",
                  "animate-in fade-in-0 zoom-in-95 duration-100 select-none"
                )}
              >
                <div className="px-2 py-1 text-[11px] font-semibold text-muted-foreground border-b border-border/40 pb-1 mb-1">
                  {LL.flow.tab_display_mode()}
                </div>
                <button
                  type="button"
                  onClick={() => {
                    setFlowTabDisplayMode('scroll');
                    setIsModePopoverOpen(false);
                  }}
                  className={cn(
                    "flex items-start gap-2.5 w-full rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-accent cursor-pointer outline-none",
                    flowTabDisplayMode === 'scroll' && "bg-accent/70 font-medium text-accent-foreground"
                  )}
                >
                  <MoveHorizontal className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <span>{LL.flow.tab_mode_scroll()}</span>
                      {flowTabDisplayMode === 'scroll' && <Check className="w-3.5 h-3.5 text-primary" />}
                    </div>
                    <p className="text-[11px] text-muted-foreground font-normal mt-0.5 leading-normal">
                      {LL.flow.tab_mode_scroll_desc()}
                    </p>
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setFlowTabDisplayMode('wrap');
                    setIsModePopoverOpen(false);
                  }}
                  className={cn(
                    "flex items-start gap-2.5 w-full rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-accent cursor-pointer outline-none mt-0.5",
                    flowTabDisplayMode === 'wrap' && "bg-accent/70 font-medium text-accent-foreground"
                  )}
                >
                  <WrapText className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <span>{LL.flow.tab_mode_wrap()}</span>
                      {flowTabDisplayMode === 'wrap' && <Check className="w-3.5 h-3.5 text-primary" />}
                    </div>
                    <p className="text-[11px] text-muted-foreground font-normal mt-0.5 leading-normal">
                      {LL.flow.tab_mode_wrap_desc()}
                    </p>
                  </div>
                </button>
              </Popover.Content>
            </Popover.Portal>
          </Popover.Root>
        </div>
      </div>

      {/* 右键上下文菜单 */}
      {contextMenu && (
        <div
          ref={menuRef}
          className={cn(
            'fixed z-50 min-w-[160px]',
            'bg-background',
            'rounded-lg shadow-xl',
            'border border-border/50 dark:border-white/20',
            'py-1',
            'animate-in fade-in-0 zoom-in-95 duration-100',
            'select-none',
          )}
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {contextMenu.tab ? (
            <>
              <div
                className="px-3 py-1.5 text-xs text-muted-foreground border-b border-border/40 dark:border-white/10 truncate max-w-[200px]"
                title={contextMenu.tab.name}
              >
                {contextMenu.tab.name}
              </div>

              <div className="py-0.5">
                {/* 关闭当前标签 */}
                <button
                  type="button"
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleCloseTab(tab);
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <X className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.close_tab()}
                </button>

                {/* 关闭其他标签 */}
                <button
                  type="button"
                  disabled={tabs.length <= 1}
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleCloseOtherTabs(tab);
                  }}
                  className={cn(
                    'flex items-center gap-2.5 w-full px-3 py-1.5 text-xs transition-colors outline-none',
                    tabs.length <= 1
                      ? 'text-muted-foreground/40 cursor-not-allowed'
                      : 'text-foreground hover:bg-accent cursor-pointer',
                  )}
                >
                  <XCircle className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.close_other_tabs()}
                </button>

                {/* 关闭右侧标签 */}
                <button
                  type="button"
                  disabled={targetTabIndex === -1 || targetTabIndex === tabs.length - 1}
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleCloseTabsToRight(tab);
                  }}
                  className={cn(
                    'flex items-center gap-2.5 w-full px-3 py-1.5 text-xs transition-colors outline-none',
                    targetTabIndex === -1 || targetTabIndex === tabs.length - 1
                      ? 'text-muted-foreground/40 cursor-not-allowed'
                      : 'text-foreground hover:bg-accent cursor-pointer',
                  )}
                >
                  <ArrowRightToLine className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.close_tabs_to_right()}
                </button>

                {/* 关闭所有标签 */}
                <button
                  type="button"
                  onClick={() => {
                    setContextMenu(null);
                    handleCloseAllTabs();
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <Layers className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.close_all_tabs()}
                </button>

                <div className="my-1 border-t border-border/40 dark:border-white/10" />
                {/* 在列表中定位 */}
                <button
                  type="button"
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    useUIStore.getState().revealFlowInList({ uuid: tab.uuid, filePath: tab.filePath });
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <LocateFixed className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.locate_in_flow_list()}
                </button>

                {/* 复制流程的文件绝对路径 */}
                <button
                  type="button"
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleCopyFilePath(tab);
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <ClipboardCopy className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.copy_file_path()}
                </button>


                {/* 在系统文件管理器中显示 */}
                <button
                  type="button"
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleRevealInFileManager(tab);
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <FolderOpen className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.reveal_in_file_manager()}
                </button>
                {/* 流程属性 */}
                <button
                  type="button"
                  onClick={() => {
                    const tab = contextMenu.tab!;
                    setContextMenu(null);
                    handleOpenProperties(tab);
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <Sliders className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.flow_properties()}
                </button>
              </div>
            </>
          ) : (
            <div className="py-0.5">
              {/* 新建流程 */}
              <button
                type="button"
                onClick={() => {
                  setContextMenu(null);
                  openNewFlowDialog();
                }}
                className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
              >
                <Plus className="w-3.5 h-3.5 text-muted-foreground" />
                {LL.flow.new_flow()}
              </button>

              {/* 关闭所有标签 */}
              {tabs.length > 0 && (
                <button
                  type="button"
                  onClick={() => {
                    setContextMenu(null);
                    handleCloseAllTabs();
                  }}
                  className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
                >
                  <Layers className="w-3.5 h-3.5 text-muted-foreground" />
                  {LL.flow.close_all_tabs()}
                </button>
              )}
            </div>
          )}

          {/* 标签栏布局 */}
          <div className="my-1 border-t border-border/40 dark:border-white/10" />
          <div className="px-3 py-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
            {LL.flow.tab_layout()}
          </div>
          <button
            type="button"
            onClick={() => {
              setFlowTabDisplayMode('scroll');
              setContextMenu(null);
            }}
            className={cn(
              'flex items-center justify-between w-full px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer outline-none',
              flowTabDisplayMode === 'scroll'
                ? 'font-medium text-foreground bg-accent/40'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            <span className="flex items-center gap-2">
              <MoveHorizontal className="w-3.5 h-3.5 text-muted-foreground" />
              {LL.flow.tab_mode_scroll()}
            </span>
            {flowTabDisplayMode === 'scroll' && <Check className="w-3.5 h-3.5 text-primary" />}
          </button>
          <button
            type="button"
            onClick={() => {
              setFlowTabDisplayMode('wrap');
              setContextMenu(null);
            }}
            className={cn(
              'flex items-center justify-between w-full px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer outline-none',
              flowTabDisplayMode === 'wrap'
                ? 'font-medium text-foreground bg-accent/40'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            <span className="flex items-center gap-2">
              <WrapText className="w-3.5 h-3.5 text-muted-foreground" />
              {LL.flow.tab_mode_wrap()}
            </span>
            {flowTabDisplayMode === 'wrap' && <Check className="w-3.5 h-3.5 text-primary" />}
          </button>
        </div>
      )}

      {/* 未保存修改关闭确认对话框 */}
      {unsavedTabTarget && (
        <Dialog
          open={!!unsavedTabTarget}
          onOpenChange={(open) => {
            if (!open && !isSaving) setUnsavedTabTarget(null);
          }}
        >
          <DialogContent className="w-full max-w-[420px]">
            <DialogHeader>
              <DialogTitle>{LL.flow.unsaved_tab_confirm_title()}</DialogTitle>
              <DialogDescription>
                {LL.flow.unsaved_tab_confirm_desc({ name: unsavedTabTarget.name })}
              </DialogDescription>
            </DialogHeader>

            <DialogFooter className="flex flex-col sm:flex-row gap-2 sm:justify-end mt-2">
              <Button
                variant="outline"
                size="sm"
                disabled={isSaving}
                onClick={() => setUnsavedTabTarget(null)}
              >
                {LL.common.CANCEL()}
              </Button>
              <Button
                variant="destructive"
                size="sm"
                disabled={isSaving}
                onClick={handleDiscardAndClose}
              >
                {LL.flow.discard_and_close()}
              </Button>
              <Button
                variant="default"
                size="sm"
                disabled={isSaving}
                onClick={handleSaveAndClose}
              >
                {isSaving && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
                {LL.flow.save_and_close()}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
});
