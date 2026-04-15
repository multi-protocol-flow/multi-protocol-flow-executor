/**
 * FlowListPanel - 流程列表面板
 * 显示已保存的流程列表，支持加载流程
 */
import { useEffect, useRef, useState, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { Plus, FileJson, MoreVertical, Copy, Trash2, Pencil, Bug, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useFlowListStore, useFlowStore, useUIStore } from '@/stores';
import { flowStorage } from '@/services/storage';
import { DEMO_FLOW, WS_DEMO_FLOW, BUILTIN_FLOW_VERSION } from '@/templates/builtinFlows';
import { cn } from '@/lib/utils';
import type { Flow } from '@/types/flow';

/** 格式化时间为相对时间 */
function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes} 分钟前`;
  if (hours < 24) return `${hours} 小时前`;
  if (days < 7) return `${days} 天前`;
  return new Date(timestamp).toLocaleDateString('zh-CN');
}

export function FlowListPanel({ showHeader = true, disabled = false, debugFlowUuid }: { showHeader?: boolean; disabled?: boolean; debugFlowUuid?: string | null }) {
  const { setFlows, addFlow: addFlowToList, removeFlow: removeFlowFromList } = useFlowListStore();
  const { setFlow, flow: currentFlow, isDirty, markSaved } = useFlowStore();
  const { openNewFlowDialog, showConfirm } = useUIStore();

  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);

  /** 将 flowStorage 元数据映射为 FlowListStore 元数据 */
  const mapMetadata = useCallback((meta: { uuid: string; name: string; updatedAt: number }) => ({
    uuid: meta.uuid,
    name: meta.name,
    updated_at: meta.updatedAt,
  }), []);

  /** 加载流程列表 */
  const loadFlowList = useCallback(async () => {
    try {
      const flowMetas = await flowStorage.listFlows();
      return flowMetas;
    } catch (err) {
      console.error('[FlowListPanel] 加载流程列表失败:', err);
      return [];
    }
  }, []);

  // 加载流程列表 + 自动更新内置模板
  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      const flowMetas = await loadFlowList();
      if (cancelled) return;

      if (flowMetas.length === 0) {
        // 首次启动：加载模板
        await Promise.all([
          flowStorage.saveFlow(DEMO_FLOW),
          flowStorage.saveFlow(WS_DEMO_FLOW),
        ]);
        if (cancelled) return;
        setFlow(DEMO_FLOW);
        setFlows([
          {
            uuid: DEMO_FLOW.uuid,
            name: DEMO_FLOW.name,
            updated_at: Date.now()
          },
          {
            uuid: WS_DEMO_FLOW.uuid,
            name: WS_DEMO_FLOW.name,
            updated_at: Date.now()
          }
        ]);
      } else {
        setFlows(flowMetas.map(mapMetadata));

        // 检查内置模板是否需要更新
        const updateBuiltinFlow = async (flow: Flow) => {
          try {
            const existingFlow = await flowStorage.loadFlow(flow.uuid);
            if (existingFlow) {
              const existingVersion = (existingFlow as Flow & { _meta?: { version: number } })._meta?.version || 0;
              if (existingVersion < BUILTIN_FLOW_VERSION) {
                // 版本过期，更新内置模板
                await flowStorage.saveFlow(flow);
                // 如果当前正在查看该流程，刷新显示
                if (currentFlow?.uuid === flow.uuid) {
                  setFlow(flow);
                }
              }
            }
          } catch (err) {
            console.error('[FlowListPanel] 更新内置模板失败:', err);
          }
        };

        await Promise.all([
          updateBuiltinFlow(DEMO_FLOW),
          updateBuiltinFlow(WS_DEMO_FLOW),
        ]);
      }

      setIsLoading(false);
    };

    init();

    return () => { cancelled = true; };
  }, [setFlows, setFlow, currentFlow?.uuid, loadFlowList, mapMetadata]);

  // 窗口聚焦时刷新流程列表
  useEffect(() => {
    const handleFocus = async () => {
      const flowMetas = await loadFlowList();
      setFlows(flowMetas.map(mapMetadata));
    };
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [setFlows, loadFlowList, mapMetadata]);

  /** 手动刷新 */
  const handleRefresh = useCallback(async () => {
    setIsRefreshing(true);
    try {
      const flowMetas = await loadFlowList();
      setFlows(flowMetas.map(mapMetadata));
    } finally {
      setIsRefreshing(false);
    }
  }, [setFlows, loadFlowList, mapMetadata]);

  const handleSelectFlow = async (uuid: string) => {
    // 切换流程前，自动保存当前流程（如果有未保存的修改）
    if (currentFlow && isDirty) {
      await flowStorage.saveFlow(currentFlow);
      markSaved();
      // 更新流程列表中的时间戳
      const flowMetas = await loadFlowList();
      setFlows(flowMetas.map(mapMetadata));
    }

    const flow = await flowStorage.loadFlow(uuid);
    if (flow) {
      setFlow(flow);
    }
  };

  const [openMenuUuid, setOpenMenuUuid] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // 重命名状态
  const [renamingUuid, setRenamingUuid] = useState<string | null>(null);
  const [renamingName, setRenamingName] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);

  // 点击外部关闭菜单
  useEffect(() => {
    if (openMenuUuid === null) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpenMenuUuid(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuUuid]);

  const handleDeleteFlow = (uuid: string) => {
    setOpenMenuUuid(null);
    showConfirm({
      title: '删除流程',
      description: '确定要删除此流程吗？删除后无法恢复。',
      confirmText: '删除',
      onConfirm: async () => {
        await flowStorage.deleteFlow(uuid);
        removeFlowFromList(uuid);
        // 如果删除的是当前流程，清空画布
        if (currentFlow?.uuid === uuid) {
          setFlow(null);
        }
      },
    });
  };

  const handleDuplicateFlow = async (uuid: string) => {
    setOpenMenuUuid(null);
    try {
      const duplicatedFlow = await flowStorage.copyFlow(uuid);
      addFlowToList({
        uuid: duplicatedFlow.uuid,
        name: duplicatedFlow.name,
        updated_at: Date.now(),
      });
      // 切换到复制后的流程
      setFlow(duplicatedFlow);
    } catch (err) {
      console.error('[FlowListPanel] 复制流程失败:', err);
    }
  };

  const handleNewFlow = () => {
    openNewFlowDialog();
  };

  const handleStartRename = (uuid: string) => {
    setOpenMenuUuid(null);
    const flowMeta = flows.find((f) => f.uuid === uuid);
    if (!flowMeta) return;
    setRenamingUuid(uuid);
    setRenamingName(flowMeta.name);
    // 延迟聚焦以等待 DOM 更新
    requestAnimationFrame(() => {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    });
  };

  const handleConfirmRename = async () => {
    if (!renamingUuid) return;
    const trimmedName = renamingName.trim();
    if (!trimmedName) {
      // 名称为空，取消重命名
      setRenamingUuid(null);
      setRenamingName('');
      return;
    }
    // 更新 flowList store
    useFlowListStore.getState().updateFlow(renamingUuid, { name: trimmedName, updated_at: Date.now() });
    // 更新 flowStore（如果正在编辑此流程）
    const currentFlowState = useFlowStore.getState();
    if (currentFlowState.flow?.uuid === renamingUuid) {
      currentFlowState.updateFlowName(trimmedName);
    }
    // 持久化：加载完整 flow，修改名称后保存
    try {
      const fullFlow = await flowStorage.loadFlow(renamingUuid);
      if (fullFlow) {
        await flowStorage.saveFlow({ ...fullFlow, name: trimmedName });
      }
    } catch (err) {
      console.error('[FlowListPanel] 重命名保存失败:', err);
    }
    setRenamingUuid(null);
    setRenamingName('');
  };

  const handleCancelRename = () => {
    setRenamingUuid(null);
    setRenamingName('');
  };

  const handleRenameKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirmRename();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancelRename();
    }
  };

  const flows = useFlowListStore((state) => state.flows);

  return (
    <div className={cn("flex flex-col h-full", showHeader && "h-52 border-b border-border")}>
      {/* 标题栏 */}
      {showHeader && (
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <span className="text-sm font-medium text-foreground">流程列表</span>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : handleRefresh}
              disabled={disabled || isRefreshing}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title="刷新列表"
            >
              <RefreshCw className={cn("w-3.5 h-3.5 text-foreground", isRefreshing && "animate-spin")} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : handleNewFlow}
              disabled={disabled}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={disabled ? "调试中不可新建流程" : "新建流程"}
            >
              <Plus className="w-4 h-4 text-foreground" />
            </Button>
          </div>
        </div>
      )}

      {/* 加载状态 */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center py-6 text-muted-foreground">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span className="ml-2 text-sm">加载中...</span>
        </div>
      )}

      {/* 流程列表 */}
      {!isLoading && (
        <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin">
          <div className="p-2 space-y-1">
            {flows.map((flowMeta) => {
              const isSelected = currentFlow?.uuid === flowMeta.uuid;
              const isDebugFlow = disabled && debugFlowUuid === flowMeta.uuid;

              return (
                <div
                  key={flowMeta.uuid}
                  onClick={() => {
                    if (disabled) return;
                    if (renamingUuid === flowMeta.uuid) return;
                    handleSelectFlow(flowMeta.uuid);
                  }}
                  title={disabled ? '调试中不可切换流程' : undefined}
                  className={cn(
                    'group w-full flex items-center gap-2 px-3 h-12 rounded-lg text-sm text-left transition-all duration-150',
                    'bg-background border border-border',
                    !disabled && 'hover:bg-accent',
                    isSelected && !disabled && 'bg-background border-border',
                    !isSelected && !disabled && 'text-muted-foreground',
                    disabled && 'opacity-50 pointer-events-none',
                    isDebugFlow && '!opacity-100 !pointer-events-none border-primary/50 bg-primary/5'
                  )}
                >
                  {isDebugFlow ? (
                    <Bug className="w-4 h-4 shrink-0 text-amber-500" />
                  ) : (
                    <FileJson
                      className={cn(
                        'w-4 h-4 shrink-0 transition-colors',
                        isSelected ? 'text-emerald-500' : 'text-muted-foreground'
                      )}
                    />
                  )}
                  <div className="flex-1 min-w-0">
                    {renamingUuid === flowMeta.uuid ? (
                      <input
                        ref={renameInputRef}
                        type="text"
                        value={renamingName}
                        onChange={(e) => setRenamingName(e.target.value)}
                        onBlur={() => handleConfirmRename()}
                        onKeyDown={handleRenameKeyDown}
                        onClick={(e) => e.stopPropagation()}
                        className="w-full px-1 py-0 text-sm font-medium bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
                      />
                    ) : (
                      <>
                        <p className="truncate font-medium">{flowMeta.name}</p>
                        <p className="text-xs text-muted-foreground truncate">
                          {formatRelativeTime(flowMeta.updated_at)}
                        </p>
                      </>
                    )}
                  </div>

                  {/* 更多操作菜单按钮 */}
                  {!disabled && (
                  <div className="relative">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation();
                        if (openMenuUuid === flowMeta.uuid) {
                          setOpenMenuUuid(null);
                        } else {
                          const rect = e.currentTarget.getBoundingClientRect();
                          setMenuPosition({ top: rect.bottom + 4, left: rect.right - 120 });
                          setOpenMenuUuid(flowMeta.uuid);
                        }
                      }}
                      className={cn(
                        'h-6 w-6 shrink-0 transition-opacity duration-150',
                        'opacity-0 group-hover:opacity-100',
                        'hover:bg-accent'
                      )}
                      title="更多操作"
                    >
                      <MoreVertical className="w-3.5 h-3.5" />
                    </Button>

                    {openMenuUuid === flowMeta.uuid && createPortal(
                      <div
                        ref={menuRef}
                        className="fixed z-[9999] min-w-[120px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
                        style={{ top: menuPosition.top, left: menuPosition.left }}
                      >
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleStartRename(flowMeta.uuid);
                          }}
                          className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
                        >
                          <Pencil className="w-3.5 h-3.5" />
                          重命名
                        </button>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDuplicateFlow(flowMeta.uuid);
                          }}
                          className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
                        >
                          <Copy className="w-3.5 h-3.5" />
                          复制流程
                        </button>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteFlow(flowMeta.uuid);
                          }}
                          className="flex w-full items-center gap-2 px-3 py-1.5 text-sm text-red-500 hover:bg-red-50 transition-colors"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                          删除流程
                        </button>
                      </div>,
                      document.body
                    )}
                  </div>
                  )}
                </div>
              );
            })}

            {flows.length === 0 && (
              <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
                <FileJson className="w-8 h-8 mb-2 opacity-40" />
                <p className="text-sm">暂无流程</p>
                <p className="text-xs mt-1 opacity-70">点击 + 创建新流程</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
