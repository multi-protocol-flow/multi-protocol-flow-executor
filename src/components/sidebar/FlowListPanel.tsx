/**
 * FlowListPanel - 流程列表面板
 * 显示已保存的流程列表，支持加载流程，支持文件夹分组树形结构
 */
import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { createPortal } from 'react-dom';
import {
  Plus, FileJson, MoreVertical, Copy, Trash2, Pencil, Bug, RefreshCw,
  FolderPlus, Folder, FolderOpen, ChevronDown, ChevronRight, Move,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useFlowListStore, useFlowStore, useUIStore } from '@/stores';
import { flowStorage } from '@/services/storage';
import type { FolderInfo } from '@/services/storage';
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

export function FlowListPanel({ showHeader = true, disabled = false, debugFlowUuid, onCreateFolderTrigger }: { showHeader?: boolean; disabled?: boolean; debugFlowUuid?: string | null; onCreateFolderTrigger?: (fn: () => void) => void }) {
  const { setFlows, addFlow: addFlowToList, removeFlow: removeFlowFromList } = useFlowListStore();
  const { setFlow, flow: currentFlow, isDirty, markSaved } = useFlowStore();
  const { openNewFlowDialog, showConfirm } = useUIStore();

  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);

  // 文件夹相关状态
  const [folders, setFolders] = useState<FolderInfo[]>([]);
  const [isCreatingFolder, setIsCreatingFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const newFolderInputRef = useRef<HTMLInputElement>(null);
  const [newFolderParentPath, setNewFolderParentPath] = useState('');

  // 文件夹重命名状态
  const [renamingFolderPath, setRenamingFolderPath] = useState<string | null>(null);
  const [renamingFolderName, setRenamingFolderName] = useState('');
  const renameFolderInputRef = useRef<HTMLInputElement>(null);

  /** 将 flowStorage 元数据映射为 FlowListStore 元数据 */
  const mapMetadata = useCallback((meta: { uuid: string; name: string; updatedAt: number; folderPath: string }) => ({
    uuid: meta.uuid,
    name: meta.name,
    updated_at: meta.updatedAt,
    folder_path: meta.folderPath ?? '',
  }), []);

  /** 加载文件夹列表 */
  const loadFolders = useCallback(async () => {
    try {
      const folderList = await flowStorage.listFolders();
      setFolders(folderList);
    } catch (err) {
      console.error('[FlowListPanel] 加载文件夹列表失败:', err);
    }
  }, []);

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

  /** 刷新流程列表和文件夹列表 */
  const refreshAll = useCallback(async () => {
    const [flowMetas] = await Promise.all([loadFlowList(), loadFolders()]);
    if (flowMetas) {
      setFlows(flowMetas.map(mapMetadata));
    }
  }, [loadFlowList, loadFolders, setFlows, mapMetadata]);

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
            updated_at: Date.now(),
            folder_path: '',
          },
          {
            uuid: WS_DEMO_FLOW.uuid,
            name: WS_DEMO_FLOW.name,
            updated_at: Date.now(),
            folder_path: '',
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

      // 加载文件夹列表
      await loadFolders();

      setIsLoading(false);
    };

    init();

    return () => { cancelled = true; };
  }, [setFlows, setFlow, currentFlow?.uuid, loadFlowList, loadFolders, mapMetadata]);

  // 窗口聚焦时刷新流程列表
  useEffect(() => {
    const handleFocus = async () => {
      await refreshAll();
    };
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [refreshAll]);

  /** 手动刷新 */
  const handleRefresh = useCallback(async () => {
    setIsRefreshing(true);
    try {
      await refreshAll();
    } finally {
      setIsRefreshing(false);
    }
  }, [refreshAll]);

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

  // ===== 上下文菜单状态 =====
  const [openMenuUuid, setOpenMenuUuid] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // 文件夹上下文菜单
  const [openFolderMenuPath, setOpenFolderMenuPath] = useState<string | null>(null);
  const folderMenuRef = useRef<HTMLDivElement>(null);
  const [folderMenuPosition, setFolderMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // 移动到文件夹子菜单
  const [moveMenuUuid, setMoveMenuUuid] = useState<string | null>(null);
  const moveMenuRef = useRef<HTMLDivElement>(null);
  const [moveMenuPosition, setMoveMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // 文件夹移动子菜单
  const [moveFolderMenuPath, setMoveFolderMenuPath] = useState<string | null>(null);
  const moveFolderMenuRef = useRef<HTMLDivElement>(null);
  const [moveFolderMenuPosition, setMoveFolderMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  /**
   * 计算菜单定位，考虑视口边界自动翻转方向
   * @param anchorRect 触发元素的 boundingClientRect
   * @param menuWidth 菜单宽度
   * @param menuHeight 菜单预估高度
   * @param gap 菜单与触发元素的间距
   * @param side 方向：'bottom' 在下方弹出，'right' 在右侧弹出（子菜单）
   */
  const computeMenuPosition = (
    anchorRect: DOMRect,
    menuWidth: number,
    menuHeight: number,
    gap: number = 4,
    side: 'bottom' | 'right' = 'bottom',
  ): { top: number; left: number } => {
    const viewportH = window.innerHeight;
    const viewportW = window.innerWidth;

    if (side === 'bottom') {
      const spaceBelow = viewportH - anchorRect.bottom - gap;
      const showAbove = spaceBelow < menuHeight && anchorRect.top > menuHeight + gap;
      const top = showAbove
        ? anchorRect.top - menuHeight - gap
        : anchorRect.bottom + gap;
      const left = Math.max(4, Math.min(anchorRect.right - menuWidth, viewportW - menuWidth - 4));
      return { top, left };
    }

    // side === 'right'
    const spaceRight = viewportW - anchorRect.right - gap;
    const showLeft = spaceRight < menuWidth && anchorRect.left > menuWidth + gap;
    const top = Math.max(4, Math.min(anchorRect.top, viewportH - menuHeight - 4));
    const left = showLeft
      ? anchorRect.left - menuWidth - gap
      : anchorRect.right + gap;
    return { top, left };
  };

  // 重命名流程状态
  const [renamingUuid, setRenamingUuid] = useState<string | null>(null);
  const [renamingName, setRenamingName] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);

  // 点击外部关闭菜单
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (openMenuUuid !== null && menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpenMenuUuid(null);
      }
      if (openFolderMenuPath !== null && folderMenuRef.current && !folderMenuRef.current.contains(e.target as Node)) {
        setOpenFolderMenuPath(null);
      }
      if (moveMenuUuid !== null && moveMenuRef.current && !moveMenuRef.current.contains(e.target as Node)) {
        setMoveMenuUuid(null);
      }
      if (moveFolderMenuPath !== null && moveFolderMenuRef.current && !moveFolderMenuRef.current.contains(e.target as Node)) {
        setMoveFolderMenuPath(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuUuid, openFolderMenuPath, moveMenuUuid, moveFolderMenuPath]);

  // ===== 文件夹操作 =====

  /** 开始创建文件夹 */
  const handleStartCreateFolder = (parentPath?: string) => {
    setIsCreatingFolder(true);
    setNewFolderParentPath(parentPath ?? '');
    setNewFolderName('');
    requestAnimationFrame(() => {
      newFolderInputRef.current?.focus();
    });
  };

  // 注册创建文件夹回调给父组件（LeftSidebarPanel 标题栏按钮使用）
  useEffect(() => {
    if (onCreateFolderTrigger) {
      onCreateFolderTrigger(() => {
        setIsCreatingFolder(true);
        setNewFolderParentPath('');
        setNewFolderName('');
        requestAnimationFrame(() => {
          newFolderInputRef.current?.focus();
        });
      });
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onCreateFolderTrigger]);

  /** 确认创建文件夹 */
  const handleConfirmCreateFolder = async () => {
    const trimmedName = newFolderName.trim();
    if (!trimmedName) {
      setIsCreatingFolder(false);
      setNewFolderName('');
      setNewFolderParentPath('');
      return;
    }
    try {
      const fullPath = newFolderParentPath
        ? `${newFolderParentPath}/${trimmedName}`
        : trimmedName;
      await flowStorage.createFolder(fullPath);
      await refreshAll();
    } catch (err) {
      console.error('[FlowListPanel] 创建文件夹失败:', err);
    }
    setIsCreatingFolder(false);
    setNewFolderName('');
    setNewFolderParentPath('');
  };

  const handleCancelCreateFolder = () => {
    setIsCreatingFolder(false);
    setNewFolderName('');
    setNewFolderParentPath('');
  };

  /** 删除文件夹 */
  const handleDeleteFolder = (folderPath: string) => {
    setOpenFolderMenuPath(null);
    showConfirm({
      title: '删除文件夹',
      description: '确定要删除此文件夹吗？仅空文件夹可被删除。',
      confirmText: '删除',
      onConfirm: async () => {
        try {
          await flowStorage.deleteFolder(folderPath);
          await refreshAll();
        } catch (err) {
          console.error('[FlowListPanel] 删除文件夹失败:', err);
        }
      },
    });
  };

  /** 开始重命名文件夹 */
  const handleStartRenameFolder = (folderPath: string) => {
    setOpenFolderMenuPath(null);
    const folder = folders.find(f => f.path === folderPath);
    if (!folder) return;
    setRenamingFolderPath(folderPath);
    setRenamingFolderName(folder.name);
    requestAnimationFrame(() => {
      renameFolderInputRef.current?.focus();
      renameFolderInputRef.current?.select();
    });
  };

  /** 确认重命名文件夹 */
  const handleConfirmRenameFolder = async () => {
    if (!renamingFolderPath) return;
    const trimmedName = renamingFolderName.trim();
    if (!trimmedName) {
      setRenamingFolderPath(null);
      setRenamingFolderName('');
      return;
    }
    try {
      await flowStorage.renameFolder(renamingFolderPath, trimmedName);
      await refreshAll();
    } catch (err) {
      console.error('[FlowListPanel] 重命名文件夹失败:', err);
    }
    setRenamingFolderPath(null);
    setRenamingFolderName('');
  };

  const handleCancelRenameFolder = () => {
    setRenamingFolderPath(null);
    setRenamingFolderName('');
  };

  const handleRenameFolderKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirmRenameFolder();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancelRenameFolder();
    }
  };

  /** 移动流程到文件夹 */
  const handleMoveFlow = async (uuid: string, targetPath: string) => {
    setMoveMenuUuid(null);
    setOpenMenuUuid(null);
    try {
      await flowStorage.moveFlowToFolder(uuid, targetPath);
      await refreshAll();
    } catch (err) {
      console.error('[FlowListPanel] 移动流程失败:', err);
    }
  };

  /** 移动文件夹 */
  const handleMoveFolder = async (sourcePath: string, targetPath: string) => {
    setMoveFolderMenuPath(null);
    setOpenFolderMenuPath(null);
    try {
      await flowStorage.moveFolder(sourcePath, targetPath);
      await refreshAll();
    } catch (err) {
      console.error('[FlowListPanel] 移动文件夹失败:', err);
    }
  };

  // ===== 流程操作 =====

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
      // 保留原流程所在的文件夹
      const originalFlow = flows.find(f => f.uuid === uuid);
      const folderPath = originalFlow?.folder_path ?? '';
      // 如果原流程在文件夹中，将复制的流程也移动到同一文件夹
      if (folderPath) {
        await flowStorage.moveFlowToFolder(duplicatedFlow.uuid, folderPath);
      }
      addFlowToList({
        uuid: duplicatedFlow.uuid,
        name: duplicatedFlow.name,
        updated_at: Date.now(),
        folder_path: folderPath,
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
  const expandedFolders = useFlowListStore((state) => state.expandedFolders);
  const toggleFolder = useFlowListStore((state) => state.toggleFolder);

  // ===== 构建树形结构 =====
  const treeData = useMemo(() => {
    // 按文件夹分组
    const folderMap = new Map<string, typeof flows>();
    const rootFlows: typeof flows = [];

    for (const flow of flows) {
      const folderPath = flow.folder_path || '';
      if (folderPath === '') {
        rootFlows.push(flow);
      } else {
        const existing = folderMap.get(folderPath);
        if (existing) {
          existing.push(flow);
        } else {
          folderMap.set(folderPath, [flow]);
        }
      }
    }

    // 排序根目录流程
    rootFlows.sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'));

    // 排序每个文件夹内的流程
    for (const [, folderFlows] of folderMap) {
      folderFlows.sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'));
    }

    // 合并所有文件夹路径（包括空文件夹）
    const allFolderPaths = new Set(folderMap.keys());
    for (const folder of folders) {
      allFolderPaths.add(folder.path);
    }

    // 构建文件夹树结构
    const folderChildrenMap = new Map<string, string[]>(); // parentPath -> childPaths
    const rootFolderPaths: string[] = [];

    for (const path of allFolderPaths) {
      const parentPath = path.includes('/') ? path.substring(0, path.lastIndexOf('/')) : '';
      if (parentPath && allFolderPaths.has(parentPath)) {
        const children = folderChildrenMap.get(parentPath) ?? [];
        children.push(path);
        folderChildrenMap.set(parentPath, children);
      } else {
        rootFolderPaths.push(path);
      }
    }

    // 排序
    const sortPaths = (paths: string[]) => paths.sort((a, b) => a.localeCompare(b, 'zh-CN'));
    sortPaths(rootFolderPaths);
    for (const [, children] of folderChildrenMap) {
      sortPaths(children);
    }

    return { rootFlows, folderMap, rootFolderPaths, folderChildrenMap };
  }, [flows, folders]);

  // ===== 渲染流程项 =====
  const renderFlowItem = (flowMeta: typeof flows[number], depth: number) => {
    const isSelected = currentFlow?.uuid === flowMeta.uuid;
    const isDebugFlow = disabled && debugFlowUuid === flowMeta.uuid;

    return (
      <div
        key={flowMeta.uuid}
        role="button"
        tabIndex={disabled ? -1 : 0}
        onClick={() => {
          if (disabled) return;
          if (renamingUuid === flowMeta.uuid) return;
          handleSelectFlow(flowMeta.uuid);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            if (disabled) return;
            if (renamingUuid === flowMeta.uuid) return;
            handleSelectFlow(flowMeta.uuid);
          }
        }}
        title={disabled ? '调试中不可切换流程' : undefined}
          style={{ marginLeft: `${depth * 20}px` }}
          className={cn(
          'group flex items-center gap-2 px-3 h-12 rounded-lg text-sm text-left transition-all duration-150',
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
                setMenuPosition(computeMenuPosition(rect, 120, 140));
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
                  // 显示移动子菜单
                  setOpenMenuUuid(null);
                  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                  setMoveMenuPosition(computeMenuPosition(rect, 140, 140, 4, 'right'));
                  setMoveMenuUuid(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
              >
                <Move className="w-3.5 h-3.5" />
                移动到...
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
  };

  // ===== 渲染文件夹项 =====
  const renderFolderItem = (folderPath: string, depth: number) => {
    const folder = folders.find(f => f.path === folderPath);
    const displayName = folder?.name ?? folderPath.split('/').pop() ?? folderPath;
    const isExpanded = expandedFolders.has(folderPath);
    const folderFlows = treeData.folderMap.get(folderPath) ?? [];

    return (
      <div key={`folder-${folderPath}`}>
        {/* 文件夹行 - 使用 div 避免 button 嵌套 button */}
        <div
          role="button"
          tabIndex={0}
          onClick={() => toggleFolder(folderPath)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              toggleFolder(folderPath);
            }
          }}
          className="group flex items-center gap-1.5 h-8 rounded-md text-sm text-left transition-colors hover:bg-accent/50 cursor-pointer"
          style={{ marginLeft: `${depth * 20}px`, paddingRight: '12px' }}
          onContextMenu={(e) => {
            e.preventDefault();
            const rect = e.currentTarget.getBoundingClientRect();
            setFolderMenuPosition(computeMenuPosition(rect, 120, 110));
            setOpenFolderMenuPath(folderPath);
          }}
        >
          {isExpanded ? (
            <ChevronDown className="w-3 h-3 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="w-3 h-3 shrink-0 text-muted-foreground" />
          )}
          {isExpanded ? (
            <FolderOpen className="w-3.5 h-3.5 shrink-0 text-amber-500" />
          ) : (
            <Folder className="w-3.5 h-3.5 shrink-0 text-amber-500" />
          )}

          {renamingFolderPath === folderPath ? (
            <input
              ref={renameFolderInputRef}
              type="text"
              value={renamingFolderName}
              onChange={(e) => setRenamingFolderName(e.target.value)}
              onBlur={() => handleConfirmRenameFolder()}
              onKeyDown={handleRenameFolderKeyDown}
              onClick={(e) => e.stopPropagation()}
              className="flex-1 min-w-0 px-1 py-0 text-sm bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
            />
          ) : (
            <span className="truncate text-muted-foreground text-xs font-medium">{displayName}</span>
          )}

          {/* 文件夹更多操作按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={(e) => {
              e.stopPropagation();
              if (openFolderMenuPath === folderPath) {
                setOpenFolderMenuPath(null);
              } else {
                const rect = e.currentTarget.getBoundingClientRect();
                setFolderMenuPosition(computeMenuPosition(rect, 120, 110));
                setOpenFolderMenuPath(folderPath);
              }
            }}
            className={cn(
              'h-5 w-5 shrink-0 ml-auto transition-opacity duration-150',
              'opacity-0 group-hover:opacity-100',
              'hover:bg-accent'
            )}
            title="文件夹操作"
          >
            <MoreVertical className="w-3 h-3" />
          </Button>
        </div>

        {/* 文件夹下的流程列表 */}
        {isExpanded && (
          <div className="space-y-0.5 mt-0.5">
            {folderFlows.map(flow => renderFlowItem(flow, depth + 1))}
            {folderFlows.length === 0 && !(isCreatingFolder && newFolderParentPath === folderPath) && (
              <div
                className="text-xs text-muted-foreground/60 italic px-3 py-1"
                style={{ marginLeft: `${(depth + 1) * 20}px` }}
              >
                空文件夹
              </div>
            )}
            {isCreatingFolder && newFolderParentPath === folderPath && (
              <div
                className="w-full flex items-center gap-1.5 h-8 rounded-md text-sm"
                style={{ marginLeft: `${(depth + 1) * 20}px`, paddingRight: '12px' }}
              >
                <ChevronRight className="w-3 h-3 shrink-0 text-muted-foreground" />
                <Folder className="w-3.5 h-3.5 shrink-0 text-amber-500" />
                <input
                  ref={newFolderInputRef}
                  type="text"
                  value={newFolderName}
                  onChange={(e) => setNewFolderName(e.target.value)}
                  onBlur={() => handleConfirmCreateFolder()}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleConfirmCreateFolder();
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      handleCancelCreateFolder();
                    }
                  }}
                  placeholder="文件夹名称..."
                  className="flex-1 min-w-0 px-1 py-0 text-sm bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className={cn("flex flex-col h-full", showHeader && "h-52 border-b border-border")}>
      {/* 标题栏（仅在 showHeader=true 时渲染，即非 LeftSidebarPanel 场景） */}
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
              onClick={disabled ? undefined : () => handleStartCreateFolder()}
              disabled={disabled}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={disabled ? "调试中不可创建文件夹" : "新建文件夹"}
            >
              <FolderPlus className="w-3.5 h-3.5 text-foreground" />
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
            {/* 新建文件夹输入框（始终可见，不依赖 showHeader） */}
            {isCreatingFolder && !newFolderParentPath && (
              <div
                className="w-full flex items-center gap-1.5 h-8 rounded-md text-sm"
                style={{ paddingLeft: '8px', paddingRight: '12px' }}
              >
                <ChevronRight className="w-3 h-3 shrink-0 text-muted-foreground" />
                <Folder className="w-3.5 h-3.5 shrink-0 text-amber-500" />
                <input
                  ref={newFolderInputRef}
                  type="text"
                  value={newFolderName}
                  onChange={(e) => setNewFolderName(e.target.value)}
                  onBlur={() => handleConfirmCreateFolder()}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleConfirmCreateFolder();
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      handleCancelCreateFolder();
                    }
                  }}
                  placeholder="文件夹名称..."
                  className="flex-1 min-w-0 px-1 py-0 text-sm bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            )}

            {/* 文件夹节点（递归渲染） */}
            {(() => {
              const renderFolderTree = (folderPaths: string[], depth: number) => {
                return folderPaths.map(folderPath => {
                  const children = treeData.folderChildrenMap.get(folderPath) ?? [];
                  return (
                    <div key={`folder-tree-${folderPath}`}>
                      {renderFolderItem(folderPath, depth)}
                      {expandedFolders.has(folderPath) && children.length > 0 && (
                        <div className="space-y-0.5 mt-0.5">
                          {renderFolderTree(children, depth + 1)}
                        </div>
                      )}
                    </div>
                  );
                });
              };
              return renderFolderTree(treeData.rootFolderPaths, 0);
            })()}

            {/* 根目录流程（排在文件夹后面） */}
            {treeData.rootFlows.map(flow => renderFlowItem(flow, 0))}

            {flows.length === 0 && treeData.rootFolderPaths.length === 0 && !isCreatingFolder && (
              <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
                <FileJson className="w-8 h-8 mb-2 opacity-40" />
                <p className="text-sm">暂无流程</p>
                <p className="text-xs mt-1 opacity-70">点击 + 创建新流程</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 文件夹上下文菜单 Portal */}
      {openFolderMenuPath !== null && createPortal(
        <div
          ref={folderMenuRef}
          className="fixed z-[9999] min-w-[120px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
          style={{ top: folderMenuPosition.top, left: folderMenuPosition.left }}
        >
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setOpenFolderMenuPath(null);
              openNewFlowDialog(openFolderMenuPath ?? undefined);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            新建流程
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setOpenFolderMenuPath(null);
              handleStartCreateFolder(openFolderMenuPath ?? undefined);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" />
            新建子文件夹
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setOpenFolderMenuPath(null);
              const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
              setMoveFolderMenuPosition(computeMenuPosition(rect, 140, 140, 4, 'right'));
              setMoveFolderMenuPath(openFolderMenuPath);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
          >
            <Move className="w-3.5 h-3.5" />
            移动到...
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleStartRenameFolder(openFolderMenuPath);
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
              handleDeleteFolder(openFolderMenuPath);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm text-red-500 hover:bg-red-50 transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
            删除文件夹
          </button>
        </div>,
        document.body
      )}

      {/* 移动到子菜单 Portal */}
      {moveMenuUuid !== null && createPortal(
        <div
          ref={moveMenuRef}
          className="fixed z-[10000] min-w-[140px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
          style={{ top: moveMenuPosition.top, left: moveMenuPosition.left }}
        >
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleMoveFlow(moveMenuUuid, '');
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
          >
            <FileJson className="w-3.5 h-3.5" />
            根目录
          </button>
          {folders.map(folder => (
            <button
              key={folder.path}
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleMoveFlow(moveMenuUuid, folder.path);
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
            >
              <Folder className="w-3.5 h-3.5 text-amber-500" />
              {folder.name}
            </button>
          ))}
        </div>,
        document.body
      )}

      {/* 文件夹移动子菜单 Portal */}
      {moveFolderMenuPath !== null && createPortal(
        <div
          ref={moveFolderMenuRef}
          className="fixed z-[10000] min-w-[140px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
          style={{ top: moveFolderMenuPosition.top, left: moveFolderMenuPosition.left }}
        >
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleMoveFolder(moveFolderMenuPath, '');
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
          >
            <FileJson className="w-3.5 h-3.5" />
            根目录
          </button>
          {folders
            .filter(f => {
              // 排除自身和自身的子目录
              if (f.path === moveFolderMenuPath) return false;
              if (f.path.startsWith(moveFolderMenuPath + '/')) return false;
              return true;
            })
            .map(folder => (
              <button
                key={folder.path}
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleMoveFolder(moveFolderMenuPath, folder.path);
                }}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
              >
                <Folder className="w-3.5 h-3.5 text-amber-500" />
                {folder.name}
              </button>
            ))}
        </div>,
        document.body
      )}
    </div>
  );
}
