/**
 * FlowTabBar - 多流程标签栏组件
 * 支持流程标签切换、运行状态指示、未保存脏状态拦截、右键上下文菜单与快捷新建
 */
import React, { useState, useRef, useEffect, useCallback } from 'react';
import {
  X,
  Plus,
  Bug,
  Loader2,
  Sliders,
  XCircle,
  ArrowRightToLine,
  Layers,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  useFlowStore,
  useUIStore,
  useExecutionStore,
  useDebugStore,
  selectTabs,
  selectActiveTabUuid,
  selectTabSessions,
  selectIsExecuting,
  selectIsDebugging,
} from '@/stores';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import type { FlowTab } from '@/types';
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
  tab: FlowTab;
}

export function FlowTabBar() {
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

  // 右键菜单状态
  const [contextMenu, setContextMenu] = useState<TabContextMenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // 未保存关闭确认弹窗目标 tab
  const [unsavedTabTarget, setUnsavedTabTarget] = useState<FlowTab | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  // 容器横向滚动 ref
  const tabsContainerRef = useRef<HTMLDivElement>(null);

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
      console.error('[FlowTabBar] 保存失败:', err);
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
            `${dirtyOthers.map((d) => `「${d.name}」`).join('、')} 有未保存的修改。确定放弃并关闭其他标签吗？`,
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
            `${dirtyRight.map((d) => `「${d.name}」`).join('、')} 有未保存的修改。确定放弃并关闭右侧标签吗？`,
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
          `${dirtyTabs.map((d) => `「${d.name}」`).join('、')} 有未保存的修改。确定放弃并关闭所有标签吗？`,
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

  // 滚轮横向滚动
  const handleWheel = (e: React.WheelEvent) => {
    if (e.deltaY !== 0 && tabsContainerRef.current) {
      tabsContainerRef.current.scrollLeft += e.deltaY;
    }
  };

  const targetTabIndex = contextMenu
    ? tabs.findIndex((t) => t.uuid === contextMenu.tab.uuid)
    : -1;

  const LL = getLL();

  return (
    <>
      <div className="h-9 bg-background border-b border-border flex items-center justify-between px-2 gap-1 select-none shrink-0 z-10">
        {/* 标签滚动列表 */}
        <div
          ref={tabsContainerRef}
          onWheel={handleWheel}
          className="flex items-center gap-1 overflow-x-auto select-none no-scrollbar flex-1 min-w-0 py-1"
        >
          {tabs.map((tab) => {
            const isActive = tab.uuid === activeTabUuid;
            const isTabExecuting = isActive && isExecuting;
            const isTabDebugging = isActive && isDebugging;

            return (
              <div
                key={tab.uuid}
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
            className="flex items-center justify-center h-6 w-6 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors shrink-0"
            title={LL.flow.new_flow()}
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
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
                const tab = contextMenu.tab;
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
                const tab = contextMenu.tab;
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
                const tab = contextMenu.tab;
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

            {/* 流程属性 */}
            <button
              type="button"
              onClick={() => {
                const tab = contextMenu.tab;
                setContextMenu(null);
                handleOpenProperties(tab);
              }}
              className="flex items-center gap-2.5 w-full px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer outline-none"
            >
              <Sliders className="w-3.5 h-3.5 text-muted-foreground" />
              {LL.flow.flow_properties()}
            </button>
          </div>
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
}
