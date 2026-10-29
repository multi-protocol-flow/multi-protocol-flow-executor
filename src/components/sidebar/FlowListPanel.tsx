/**
 * FlowListPanel - 流程列表面板
 * 显示已保存的流程列表，支持加载流程，支持文件夹分组树形结构
 */
import { useEffect, useRef, useState, useCallback, useMemo, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import {
  Plus, FileJson, MoreVertical, Copy, Trash2, Pencil, Bug, RefreshCw,
  FolderPlus, Folder, FolderOpen, ChevronDown, ChevronRight, Move,
  Search, ArrowUpDown, Check, ClipboardCopy, FileDown, LocateFixed, Bot,
} from 'lucide-react';
import { toast } from 'sonner';
import { copyToClipboard } from '@/lib/clipboard';
import { Button } from '@/components/ui/button';
import {
  useFlowListStore,
  useFlowStore,
  useUIStore,
  useExecutionStore,
  useDebugStore,
  selectIsExecuting,
  selectIsDebugging,
  selectLocateFlowTarget,
} from '@/stores';
import { useSettingsStore } from '@/stores/settings';
import { flowStorage } from '@/services/storage';
import type { FolderInfo } from '@/services/storage';
import { DEMO_FLOW, WS_DEMO_FLOW, BUILTIN_FLOW_VERSION } from '@/templates/builtinFlows';
import { cn } from '@/lib/utils';
import type { Flow } from '@/types/flow';
import { getLL } from '@/i18n/i18n-lazy';
import { useCurrentLocale } from '@/hooks/useLocale';
import { validateFlow } from '@/services/tauri/flow';
import { ValidationErrorDialog } from '@/components/dialogs/ValidationErrorDialog';
import { revealItemInDir } from '@/services/tauri/settings';

export function FlowListPanel({ showHeader = true, disabled = false, debugFlowUuid, onCreateFolderTrigger }: { showHeader?: boolean; disabled?: boolean; debugFlowUuid?: string | null; onCreateFolderTrigger?: (fn: () => void) => void }) {
  const { setFlows, addFlow: addFlowToList, removeFlow: removeFlowFromList } = useFlowListStore();
  const { setFlow, flow: currentFlow } = useFlowStore();
  const { openNewFlowDialog, showConfirm } = useUIStore();
  const isExecuting = useExecutionStore(selectIsExecuting);
  const isDebugging = useDebugStore(selectIsDebugging);
  const flowsDir = useSettingsStore((state) => state.flowsDir);
  const settingsLoading = useSettingsStore((state) => state.isLoading);
  const locale = useCurrentLocale();

  const activeTabUuid = useFlowStore((state) => state.activeTabUuid);
  const tabs = useFlowStore((state) => state.tabs);
  const locateFlowTarget = useUIStore(selectLocateFlowTarget);
  const [highlightingFlowUuid, setHighlightingFlowUuid] = useState<string | null>(null);
  const highlightTimerRef = useRef<number | null>(null);
  const lastLocateTimestampRef = useRef<number>(0);

  useEffect(() => {
    return () => {
      if (highlightTimerRef.current) {
        window.clearTimeout(highlightTimerRef.current);
      }
    };
  }, []);
  const formatRelativeTime = (timestamp: number): string => {
    const now = Date.now();
    const diff = now - timestamp;
    const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return rtf.format(0, 'second');
    if (minutes < 60) return rtf.format(-minutes, 'minute');
    if (hours < 24) return rtf.format(-hours, 'hour');
    if (days < 7) return rtf.format(-days, 'day');
    return new Date(timestamp).toLocaleDateString(locale);
  };

  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);

  // 验证错误对话框状态
  const [validationErrors, setValidationErrors] = useState<{ code: string; message: string }[]>([]);
  const [validationWarnings, setValidationWarnings] = useState<string[]>([]);
  const [validationType, setValidationType] = useState<'validation' | 'corrupted'>('validation');
  const [showValidationDialog, setShowValidationDialog] = useState(false);

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

  const openExportFlowDialog = useUIStore((s) => s.openExportFlowDialog);
  const openExportMcpDialog = useUIStore((s) => s.openExportMcpDialog);

  // 拖拽流程标识，避免拖拽释放时误触发流程切换
  const isDraggingFlowRef = useRef(false);
  // 记录最近一次刷新完成的时间戳，避免应用启动时重复触发全盘流程与目录扫描
  const lastRefreshTimeRef = useRef(0);


  /** 将 flowStorage 元数据映射为 FlowListStore 元数据 */
  const mapMetadata = useCallback((meta: { uuid: string; name: string; updatedAt: number; folderPath: string; filePath?: string }) => ({
    uuid: meta.uuid,
    name: meta.name,
    updated_at: meta.updatedAt,
    folder_path: meta.folderPath ?? '',
    file_path: meta.filePath,
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
    // 等待全局设置加载完毕，避免在 flowsDir 初始空值下先加载系统默认路径
    if (settingsLoading) return;

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

        // 检查内置模板是否需要更新（仅当流程列表中存在该模板时才检查）
        const updateBuiltinFlow = async (flow: Flow) => {
          if (!flowMetas.some((m) => m.uuid === flow.uuid)) {
            return;
          }
          try {
            const existingFlow = await flowStorage.loadFlow(flow.uuid);
            if (existingFlow) {
              const existingVersion = (existingFlow as Flow & { _meta?: { version: number } })._meta?.version || 0;
              if (existingVersion < BUILTIN_FLOW_VERSION) {
                // 版本过期，更新内置模板
                await flowStorage.saveFlow(flow);
                // 如果当前正在查看该流程，刷新显示
                if (useFlowStore.getState().flow?.uuid === flow.uuid) {
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
      lastRefreshTimeRef.current = Date.now();
      setIsLoading(false);
    };

    init();

    return () => { cancelled = true; };
  }, [settingsLoading, flowsDir, setFlows, setFlow, loadFlowList, loadFolders, mapMetadata]);

  // 窗口聚焦时刷新流程列表
  // 窗口聚焦时刷新流程列表（带防抖节流，至少间隔 3 秒，避免启动阶段窗口切换导致连续重复全盘扫描）
  useEffect(() => {
    const handleFocus = async () => {
      if (Date.now() - lastRefreshTimeRef.current < 3000) return;
      lastRefreshTimeRef.current = Date.now();
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

  const handleSelectFlow = async (metaOrUuid: string | typeof flows[number]) => {
    const LL = getLL();
    if (isExecuting) {
      toast.info(LL.flow.executing_no_switch());
      return;
    }
    if (isDebugging) {
      toast.info(LL.flow.debug_no_switch());
      return;
    }

    const uuid = typeof metaOrUuid === 'string' ? metaOrUuid : metaOrUuid.uuid;
    const filePath = typeof metaOrUuid === 'string'
      ? undefined
      : (metaOrUuid.file_path || ('filePath' in metaOrUuid && typeof metaOrUuid.filePath === 'string' ? metaOrUuid.filePath : undefined));
    const loadTarget = filePath || uuid;

    const { tabs, switchTab, openTab } = useFlowStore.getState();
    const existingTab = tabs.find((t) => t.uuid === uuid || (filePath && t.filePath === filePath));
    if (existingTab) {
      switchTab(existingTab.uuid);
      return;
    }
    const start = performance.now();
    try {
      const flow = await flowStorage.loadFlow(loadTarget);
      const loadTime = performance.now() - start;
      if (flow) {
        const openStart = performance.now();
        const opened = openTab(flow, filePath || loadTarget);
        const openTime = performance.now() - openStart;
        console.debug(`[FlowOpen] disk load: ${loadTime.toFixed(1)}ms, tab open: ${openTime.toFixed(1)}ms, total: ${(performance.now() - start).toFixed(1)}ms`);
        if (!opened) return;
        // 后台异步验证流程结构，不阻塞主渲染与 Tab 切换
        validateFlow(flow)
          .then((result) => {
            if (!result.valid && result.errors.length > 0) {
              setValidationErrors(result.errors);
              setValidationWarnings(result.warnings || []);
              setValidationType('validation');
              setShowValidationDialog(true);
            }
          })
          .catch(() => {
            // 验证调用失败（如非 Tauri 环境）静默忽略
          });
      }
    } catch (err) {
      // 加载流程文件失败，透传后端原始错误消息
      const errMsg = err instanceof Error ? err.message : String(err);
      setValidationErrors([{ code: 'file_corrupted', message: errMsg }]);
      setValidationType('corrupted');
      setShowValidationDialog(true);
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

  // 移动菜单中展开的文件夹路径（用于折叠/展开子文件夹）
  const [expandedMoveMenuFolders, setExpandedMoveMenuFolders] = useState<Set<string>>(new Set());
  const [expandedMoveFolderMenuFolders, setExpandedMoveFolderMenuFolders] = useState<Set<string>>(new Set());

  // 排序下拉菜单状态
  const [sortMenuOpen, setSortMenuOpen] = useState(false);
  const sortMenuRef = useRef<HTMLDivElement>(null);
  const [sortMenuPosition, setSortMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

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
      if (sortMenuOpen && sortMenuRef.current && !sortMenuRef.current.contains(e.target as Node)) {
        setSortMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuUuid, openFolderMenuPath, moveMenuUuid, moveFolderMenuPath, sortMenuOpen]);

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
      title: getLL().flow.delete_folder_title(),
      description: getLL().flow.delete_folder_desc(),
      confirmText: getLL().common.DELETE(),
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

  /** 切换移动菜单中文件夹的展开/折叠 */
  const toggleExpandedMoveFolder = useCallback((path: string) => {
    setExpandedMoveMenuFolders(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  /** 切换文件夹移动菜单中文件夹的展开/折叠 */
  const toggleExpandedMoveFolderMenu = useCallback((path: string) => {
    setExpandedMoveFolderMenuFolders(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

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
      title: getLL().flow.delete_flow_title(),
      description: getLL().flow.delete_flow_desc(),
      confirmText: getLL().common.DELETE(),
      onConfirm: async () => {
        const flowMeta = flows.find((f) => f.uuid === uuid);
        await flowStorage.deleteFlow(uuid);
        removeFlowFromList(uuid);
        // 如果删除的是打开的流程标签，关闭该标签
        const { tabs, closeTab } = useFlowStore.getState();
        const openedTab = tabs.find(
          (t) => t.uuid === uuid || (flowMeta?.file_path && t.filePath === flowMeta.file_path)
        );
        if (openedTab) {
          closeTab(openedTab.uuid);
        } else if (currentFlow?.uuid === uuid) {
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
      await refreshAll();
      // 切换到复制后的流程
      setFlow(duplicatedFlow);
    } catch (err) {
      console.error('[FlowListPanel] 复制流程失败:', err);
    }
  };

  const handleCopyFilePath = async (uuid: string) => {
    setOpenMenuUuid(null);
    try {
      let targetPath = flows.find((f) => f.uuid === uuid)?.file_path;
      if (!targetPath) {
        const flowMetas = await flowStorage.listFlows();
        const found = flowMetas.find((f) => f.uuid === uuid);
        if (found?.filePath) {
          targetPath = found.filePath;
          useFlowListStore.getState().updateFlow(uuid, { file_path: found.filePath });
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
      console.error('[FlowListPanel] 复制流程文件路径失败:', err);
      toast.error(getLL().flow.copy_file_path_failed());
    }
  };

  const handleRevealInFileManager = async (uuid: string) => {
    try {
      let targetPath = flows.find((f) => f.uuid === uuid)?.file_path;
      if (!targetPath) {
        const flowMetas = await flowStorage.listFlows();
        const found = flowMetas.find((f) => f.uuid === uuid);
        if (found?.filePath) {
          targetPath = found.filePath;
          useFlowListStore.getState().updateFlow(uuid, { file_path: found.filePath });
        }
      }
      if (!targetPath) {
        toast.error(getLL().flow.copy_file_path_not_found());
        return;
      }
      await revealItemInDir(targetPath);
    } catch (err) {
      console.error('[FlowListPanel] 在文件管理器中显示失败:', err);
      toast.error(getLL().flow.reveal_in_file_manager_failed());
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
    useFlowStore.getState().updateTabName(renamingUuid, trimmedName);
    // 持久化：加载完整 flow，修改名称后保存
    try {
      const flowMeta = flows.find((f) => f.uuid === renamingUuid);
      const targetIdentifier = flowMeta?.file_path || renamingUuid;
      const renamedMeta = await flowStorage.renameFlow(targetIdentifier, trimmedName);
      if (renamedMeta) {
        useFlowListStore.getState().updateFlow(renamingUuid, {
          uuid: renamedMeta.uuid,
          name: renamedMeta.name,
          file_path: renamedMeta.filePath,
          updated_at: renamedMeta.updatedAt,
        });
      }
      await refreshAll();
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
  const expandFolders = useFlowListStore((state) => state.expandFolders);
  const toggleFolder = useFlowListStore((state) => state.toggleFolder);
  const searchKeyword = useFlowListStore((state) => state.searchKeyword);
  const setSearchKeyword = useFlowListStore((state) => state.setSearchKeyword);
  const sortBy = useFlowListStore((state) => state.sortBy);
  const setSortBy = useFlowListStore((state) => state.setSortBy);
  const sortOrder = useFlowListStore((state) => state.sortOrder);
  const setSortOrder = useFlowListStore((state) => state.setSortOrder);

  const handleSelectSort = useCallback((newSortBy: 'name' | 'updated_at', newSortOrder: 'asc' | 'desc') => {
    setSortBy(newSortBy);
    setSortOrder(newSortOrder);
    setSortMenuOpen(false);
  }, [setSortBy, setSortOrder]);
  // ===== 定位流程 =====
  const handleLocateFlow = useCallback((target?: { uuid?: string; filePath?: string }) => {
    const targetTab =
      (target?.uuid ? tabs.find((t) => t.uuid === target.uuid) : null) ??
      tabs.find((t) => t.uuid === activeTabUuid);
    const targetUuid = target?.uuid ?? targetTab?.uuid ?? currentFlow?.uuid;
    const targetFilePath = target?.filePath ?? targetTab?.filePath;
    const targetName = targetTab?.name ?? currentFlow?.name;

    if (!targetUuid && !targetFilePath && !targetName) {
      toast.info(getLL().flow.locate_flow_not_found());
      return;
    }

    // 1. 精确匹配：文件路径完全相同，或列表项标识与目标 UUID 匹配
    let targetFlow = flows.find(
      (f) =>
        (targetFilePath && f.file_path && f.file_path === targetFilePath) ||
        (targetUuid && f.uuid === targetUuid)
    );

    // 2. 兜底回退：若未通过 UUID/路径命中（例如 AI 或未绑路径创建的流程），尝试按名称在列表中匹配
    let isAmbiguousMatch = false;
    if (!targetFlow && targetName) {
      const nameMatches = flows.filter((f) => f.name === targetName);
      if (nameMatches.length === 1) {
        targetFlow = nameMatches[0];
      } else if (nameMatches.length > 1) {
        targetFlow = nameMatches.find((f) => !f.folder_path) || nameMatches[0];
        isAmbiguousMatch = true;
      }
    }

    if (!targetFlow) {
      toast.info(getLL().flow.locate_flow_not_found());
      return;
    }

    // 3. 自动回填修复：若对应 Tab 缺失 filePath 且匹配无歧义，将找到的目标文件绝对路径回填更新
    if (targetFlow.file_path && targetTab && !targetTab.filePath && !isAmbiguousMatch) {
      useFlowStore.getState().updateTabFilePath(targetTab.uuid, targetFlow.file_path);
    }
    if (targetFlow.folder_path) {
      const segments = targetFlow.folder_path.split('/').filter(Boolean);
      const ancestorPaths: string[] = [];
      let acc = '';
      for (const seg of segments) {
        acc = acc ? `${acc}/${seg}` : seg;
        ancestorPaths.push(acc);
      }
      expandFolders(ancestorPaths);
    }

    // 2. 如果存在搜索过滤且目标被过滤隐藏了，清空搜索以确保其可见
    if (searchKeyword.trim()) {
      const keyword = searchKeyword.toLowerCase();
      if (!targetFlow.name.toLowerCase().includes(keyword)) {
        setSearchKeyword('');
      }
    }

    // 3. 滚动并添加发光高亮动效
    const executeScrollAndHighlight = () => {
      const element = document.querySelector(`[data-flow-uuid="${targetFlow.uuid}"]`) as HTMLElement | null;
      if (element) {
        if (typeof element.scrollIntoView === 'function') {
          element.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
        setHighlightingFlowUuid(targetFlow.uuid);
        if (highlightTimerRef.current) {
          window.clearTimeout(highlightTimerRef.current);
        }
        highlightTimerRef.current = window.setTimeout(() => {
          setHighlightingFlowUuid(null);
          highlightTimerRef.current = null;
        }, 1500);
      }
    };

    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(executeScrollAndHighlight);
    });
  }, [tabs, activeTabUuid, currentFlow, flows, expandFolders, searchKeyword, setSearchKeyword]);

  useEffect(() => {
    if (locateFlowTarget && locateFlowTarget.timestamp > lastLocateTimestampRef.current) {
      lastLocateTimestampRef.current = locateFlowTarget.timestamp;
      handleLocateFlow(locateFlowTarget);
    }
  }, [locateFlowTarget, handleLocateFlow]);


  // ===== 构建树形结构 =====
  const treeData = useMemo(() => {
    // 过滤流程
    let filteredFlows = flows;
    if (searchKeyword.trim()) {
      const keyword = searchKeyword.toLowerCase();
      filteredFlows = flows.filter((f) => f.name.toLowerCase().includes(keyword));
    }

    // 按文件夹分组
    const folderMap = new Map<string, typeof flows>();
    const rootFlows: typeof flows = [];

    for (const flow of filteredFlows) {
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

    // 排序
    const sortFn = (a: typeof flows[number], b: typeof flows[number]) => {
      let comparison: number;
      if (sortBy === 'name') {
        comparison = a.name.localeCompare(b.name, undefined, { numeric: true });
      } else {
        comparison = a.updated_at - b.updated_at;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    };
    rootFlows.sort(sortFn);
    for (const [, folderFlows] of folderMap) {
      folderFlows.sort(sortFn);
    }

    // 合并所有文件夹路径（搜索时隐藏空文件夹）
    const allFolderPaths = new Set(folderMap.keys());
    if (!searchKeyword.trim()) {
      for (const folder of folders) {
        allFolderPaths.add(folder.path);
      }
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
    const sortPaths = (paths: string[]) => paths.sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
    sortPaths(rootFolderPaths);
    for (const [, children] of folderChildrenMap) {
      sortPaths(children);
    }

    return { rootFlows, folderMap, rootFolderPaths, folderChildrenMap };
  }, [flows, folders, searchKeyword, sortBy, sortOrder]);

  // ===== 构建文件夹树（用于移动菜单） =====
  const folderTree = useMemo(() => {
    const allPaths = new Set(folders.map(f => f.path));
    const childrenMap = new Map<string, string[]>();
    const rootPaths: string[] = [];

    for (const folder of folders) {
      const parentPath = folder.path.includes('/')
        ? folder.path.substring(0, folder.path.lastIndexOf('/'))
        : '';
      if (parentPath && allPaths.has(parentPath)) {
        const children = childrenMap.get(parentPath) ?? [];
        children.push(folder.path);
        childrenMap.set(parentPath, children);
      } else {
        rootPaths.push(folder.path);
      }
    }

    rootPaths.sort((a, b) => a.localeCompare(b, locale));
    for (const [, children] of childrenMap) {
      children.sort((a, b) => a.localeCompare(b, locale));
    }

    return { rootPaths, childrenMap };
  }, [folders, locale]);

  // ===== 渲染流程项 =====
  const renderFlowItem = (flowMeta: typeof flows[number], depth: number) => {
    const activeTab = tabs.find((t) => t.uuid === activeTabUuid);
    const isSelected = currentFlow?.uuid === flowMeta.uuid || (activeTab?.filePath && flowMeta.file_path && activeTab.filePath === flowMeta.file_path);
    const isHighlighting = highlightingFlowUuid === flowMeta.uuid;
    const isDebugFlow = disabled && debugFlowUuid === flowMeta.uuid;

    const canDrag = !disabled && renamingUuid !== flowMeta.uuid;

    const handleDragStart = (e: React.DragEvent) => {
      if (!canDrag) {
        e.preventDefault();
        return;
      }
      isDraggingFlowRef.current = true;
      const payload = JSON.stringify({
        uuid: flowMeta.uuid,
        name: flowMeta.name,
        filePath: flowMeta.file_path,
      });
      e.dataTransfer.setData('application/reactflow/subflow', payload);
      e.dataTransfer.setData('text/plain', payload);
      e.dataTransfer.effectAllowed = 'all';
    };

    const handleDragEnd = () => {
      window.setTimeout(() => {
        isDraggingFlowRef.current = false;
      }, 100);
    };

    const itemTitle = disabled
      ? getLL().flow.debug_no_switch()
      : `${flowMeta.name} (${getLL().flow.drag_to_canvas_as_subflow()})`;

    return (
      <div
        key={flowMeta.uuid}
        data-flow-uuid={flowMeta.uuid}
        role="button"
        tabIndex={disabled ? -1 : 0}
        draggable={canDrag}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onClick={() => {
          if (disabled) return;
          if (renamingUuid === flowMeta.uuid) return;
          if (isDraggingFlowRef.current) return;
          handleSelectFlow(flowMeta);
        }}
        onContextMenu={(e) => {
          if (disabled) return;
          e.preventDefault();
          e.stopPropagation();
          const rect = e.currentTarget.getBoundingClientRect();
          setMenuPosition(computeMenuPosition(rect, 190, 210));
          setOpenMenuUuid(flowMeta.uuid);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            if (disabled) return;
            if (renamingUuid === flowMeta.uuid) return;
            handleSelectFlow(flowMeta);
          }
        }}
        title={itemTitle}
        style={{ marginLeft: `${depth * 14}px` }}
        className={cn(
          'group flex items-center gap-2.5 px-3 h-10.5 rounded-lg text-xs text-left transition-all duration-150',
          'border select-none',
          !disabled && 'cursor-pointer active:cursor-grabbing',
          isSelected && !disabled && 'bg-primary/10 border-primary/60 text-foreground font-medium shadow-[0_0_12px_rgba(var(--primary-rgb),0.1)] ring-1 ring-primary/25',
          !isSelected && !disabled && 'border-border/40 bg-background/50 hover:bg-accent/50 hover:border-border text-foreground/80 hover:text-foreground',
          isHighlighting && '!ring-2 !ring-primary !border-primary !bg-primary/20 animate-pulse duration-500',
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
              isSelected ? 'text-primary' : 'text-muted-foreground'
            )}
          />
        )}
        <div className="flex-1 min-w-0 flex flex-col justify-center py-0.5">
          {renamingUuid === flowMeta.uuid ? (
            <input
              ref={renameInputRef}
              type="text"
              value={renamingName}
              onChange={(e) => setRenamingName(e.target.value)}
              onBlur={() => handleConfirmRename()}
              onKeyDown={handleRenameKeyDown}
              onClick={(e) => e.stopPropagation()}
              className="w-full px-1.5 py-0.5 text-xs font-medium bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
            />
          ) : (
            <>
              <p className={cn("truncate font-medium text-xs leading-snug", isSelected ? "text-foreground" : "text-foreground/90")}>
                {flowMeta.name}
              </p>
              <p className="text-[11px] text-muted-foreground/75 truncate leading-none mt-0.5">
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
                setMenuPosition(computeMenuPosition(rect, 190, 180));
                setOpenMenuUuid(flowMeta.uuid);
              }
            }}
            className={cn(
              'h-6 w-6 shrink-0 transition-opacity duration-150',
              'opacity-0 group-hover:opacity-100',
              'hover:bg-accent'
            )}
            title={getLL().flow.more_ops()}
          >
            <MoreVertical className="w-3.5 h-3.5" />
          </Button>

          {openMenuUuid === flowMeta.uuid && createPortal(
            <div
              ref={menuRef}
              className="fixed z-[9999] min-w-[190px] whitespace-nowrap rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
              style={{ top: menuPosition.top, left: menuPosition.left }}
            >
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleStartRename(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
              >
                <Pencil className="w-3.5 h-3.5" />
                {getLL().flow.rename()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDuplicateFlow(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
              >
                <Copy className="w-3.5 h-3.5" />
                {getLL().flow.duplicate_flow()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleCopyFilePath(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
              >
                <ClipboardCopy className="w-3.5 h-3.5" />
                {getLL().flow.copy_file_path()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setOpenMenuUuid(null);
                  handleRevealInFileManager(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
              >
                <FolderOpen className="w-3.5 h-3.5" />
                {getLL().flow.reveal_in_file_manager()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  // 显示移动子菜单
                  setOpenMenuUuid(null);
                  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                  setExpandedMoveMenuFolders(new Set());
                  setMoveMenuPosition(computeMenuPosition(rect, 180, 300, 4, 'right'));
                  setMoveMenuUuid(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 py-1.5 px-2.5 text-xs hover:bg-accent transition-colors"
              >
                <Move className="w-3.5 h-3.5" />
                {getLL().flow.move_to()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setOpenMenuUuid(null);
                  openExportFlowDialog(flowMeta);
                }}
                className="flex w-full items-center gap-2 py-1.5 px-2.5 text-xs hover:bg-accent transition-colors"
              >
                <FileDown className="w-3.5 h-3.5" />
                {getLL().flow.export_flow_menu()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setOpenMenuUuid(null);
                  openExportMcpDialog(flowMeta);
                }}
                className="flex w-full items-center gap-2 py-1.5 px-2.5 text-xs hover:bg-accent transition-colors text-amber-500 font-medium"
              >
                <Bot className="w-3.5 h-3.5" />
                {getLL().flow.publish_as_mcp_server()}
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteFlow(flowMeta.uuid);
                }}
                className="flex w-full items-center gap-2 py-1.5 px-2.5 text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-950/30 transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
                {getLL().flow.delete_flow()}
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
  const renderFolderItem = (folderPath: string, depth: number, childFolderElements?: React.ReactNode) => {
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
          className={cn(
            'group flex items-center gap-1.5 h-8.5 px-2 rounded-md text-xs text-left transition-colors cursor-pointer select-none',
            isExpanded
              ? 'bg-accent/40 hover:bg-accent/60 text-foreground font-semibold'
              : 'hover:bg-accent/50 text-foreground/85 font-medium'
          )}
          style={{ marginLeft: `${depth * 14}px`, paddingRight: '6px' }}
          onContextMenu={(e) => {
            e.preventDefault();
            const rect = e.currentTarget.getBoundingClientRect();
            setFolderMenuPosition(computeMenuPosition(rect, 120, 110));
            setOpenFolderMenuPath(folderPath);
          }}
        >
          {isExpanded ? (
            <ChevronDown className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
          )}
          {isExpanded ? (
            <FolderOpen className="w-4 h-4 shrink-0 text-amber-500" />
          ) : (
            <Folder className="w-4 h-4 shrink-0 text-amber-500" />
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
              className="flex-1 min-w-0 px-1.5 py-0.5 text-xs bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
            />
          ) : (
            <span className="truncate flex-1 min-w-0">{displayName}</span>
          )}

          {/* 文件夹更多操作按钮（位于统计左侧） */}
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
              'h-6 w-6 shrink-0 transition-opacity duration-150',
              'opacity-0 group-hover:opacity-100',
              'hover:bg-accent'
            )}
            title={getLL().flow.folder_ops()}
          >
            <MoreVertical className="w-3.5 h-3.5" />
          </Button>

          {/* 数字统计放到最右侧 */}
          {renamingFolderPath !== folderPath && (
            <span className="text-[11px] text-muted-foreground/75 font-mono shrink-0 tabular-nums ml-0.5">
              {`(${folderFlows.length})`}
            </span>
          )}
        </div>

        {/* 文件夹展开内容：子文件夹（优先）→ 流程 → 新建文件夹输入 */}
        {isExpanded && (
          <div className="space-y-1 mt-0.5">
            {/* 子文件夹优先渲染 */}
            {childFolderElements}
            {folderFlows.map(flow => renderFlowItem(flow, depth + 1))}
            {folderFlows.length === 0 && !childFolderElements && !(isCreatingFolder && newFolderParentPath === folderPath) && (
              <div
                className="text-[11px] text-muted-foreground/60 italic px-2 py-1"
                style={{ marginLeft: `${(depth + 1) * 14}px` }}
              >
                {getLL().flow.empty_folder()}
              </div>
            )}
            {isCreatingFolder && newFolderParentPath === folderPath && (
              <div
                className="w-full flex items-center gap-1.5 h-8 rounded-md text-xs px-2"
                style={{ marginLeft: `${(depth + 1) * 14}px`, paddingRight: '8px' }}
              >
                <ChevronRight className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
                <Folder className="w-4 h-4 shrink-0 text-amber-500" />
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
                  placeholder={getLL().flow.folder_name_placeholder()}
                  className="flex-1 min-w-0 px-1 py-0 text-xs bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
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
          <span className="text-sm font-medium text-foreground">{getLL().flow.flow_list_title()}</span>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : () => handleLocateFlow()}
              disabled={disabled || (!currentFlow && !activeTabUuid)}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={getLL().flow.locate_in_flow_list()}
            >
              <LocateFixed className="w-3.5 h-3.5 text-foreground" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : handleRefresh}
              disabled={disabled || isRefreshing}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={getLL().flow.refresh_list()}
            >
              <RefreshCw className={cn("w-3.5 h-3.5 text-foreground", isRefreshing && "animate-spin")} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : () => handleStartCreateFolder()}
              disabled={disabled}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={disabled ? getLL().flow.debug_no_create_folder() : getLL().flow.new_folder()}
            >
              <FolderPlus className="w-3.5 h-3.5 text-foreground" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={disabled ? undefined : handleNewFlow}
              disabled={disabled}
              className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent disabled:opacity-50"
              title={disabled ? getLL().flow.debug_no_new_flow() : getLL().flow.new_flow()}
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
          <span className="ml-2 text-sm">{getLL().flow.loading()}</span>
        </div>
      )}

      {/* 流程列表 */}
      {!isLoading && (
        <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin">
          {/* 搜索和排序栏 */}
          <div className="px-2 pt-2 pb-1 flex items-center gap-1.5">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
              <input
                type="text"
                value={searchKeyword}
                onChange={(e) => setSearchKeyword(e.target.value)}
                placeholder={getLL().flow.flow_search_placeholder()}
                className="w-full h-7 pl-7 pr-2 text-xs rounded-md border border-border bg-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            {/* 排序下拉按钮 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={(e) => {
                if (sortMenuOpen) {
                  setSortMenuOpen(false);
                } else {
                  const rect = e.currentTarget.getBoundingClientRect();
                  setSortMenuPosition(computeMenuPosition(rect, 140, 132));
                  setSortMenuOpen(true);
                }
              }}
              className="h-7 px-1.5 rounded-md hover:bg-accent text-xs gap-0.5 shrink-0"
              title={`${sortBy === 'name' ? getLL().flow.sort_by_name() : getLL().flow.sort_by_time()} ${sortOrder === 'asc' ? '↑' : '↓'}`}
            >
              <ArrowUpDown className="w-3 h-3" />
              <ChevronDown className="w-2.5 h-2.5 text-muted-foreground" />
            </Button>
          </div>
          <div className="p-2 space-y-1">
            {/* 新建文件夹输入框（始终可见，不依赖 showHeader） */}
            {isCreatingFolder && !newFolderParentPath && (
              <div
                className="w-full flex items-center gap-1.5 h-8 rounded-md text-xs px-2"
                style={{ paddingLeft: '8px', paddingRight: '8px' }}
              >
                <ChevronRight className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
                <Folder className="w-4 h-4 shrink-0 text-amber-500" />
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
                  placeholder={getLL().flow.folder_name_placeholder()}
                  className="flex-1 min-w-0 px-1 py-0 text-xs bg-background border border-primary rounded outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            )}

            {/* 文件夹节点（递归渲染，子文件夹优先于流程） */}
            {(() => {
              const renderFolderTree = (folderPaths: string[], depth: number): ReactNode => {
                return folderPaths.map(folderPath => {
                  const children = treeData.folderChildrenMap.get(folderPath) ?? [];
                  const childElements = expandedFolders.has(folderPath) && children.length > 0
                    ? <div key={`children-${folderPath}`} className="space-y-1 mt-0.5">{renderFolderTree(children, depth + 1)}</div>
                    : null;
                  return (
                    <div key={`folder-tree-${folderPath}`}>
                      {renderFolderItem(folderPath, depth, childElements)}
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
                <p className="text-xs font-medium">{getLL().flow.no_flows()}</p>
                <p className="text-[11px] mt-1 opacity-70">{getLL().flow.create_flow_hint()}</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 排序下拉菜单 Portal */}
      {sortMenuOpen && createPortal(
        <div
          ref={sortMenuRef}
          className="fixed z-[9999] min-w-[140px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95 py-1"
          style={{ top: sortMenuPosition.top, left: sortMenuPosition.left }}
        >
          <button type="button" onClick={() => handleSelectSort('name', 'asc')}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors">
            <Check className={sortBy === 'name' && sortOrder === 'asc' ? 'w-3.5 h-3.5' : 'w-3.5 h-3.5 invisible'} />
            <span>{getLL().flow.sort_by_name()} {getLL().flow.sort_asc()}</span>
          </button>
          <button type="button" onClick={() => handleSelectSort('name', 'desc')}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors">
            <Check className={sortBy === 'name' && sortOrder === 'desc' ? 'w-3.5 h-3.5' : 'w-3.5 h-3.5 invisible'} />
            <span>{getLL().flow.sort_by_name()} {getLL().flow.sort_desc()}</span>
          </button>
          <button type="button" onClick={() => handleSelectSort('updated_at', 'asc')}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors">
            <Check className={sortBy === 'updated_at' && sortOrder === 'asc' ? 'w-3.5 h-3.5' : 'w-3.5 h-3.5 invisible'} />
            <span>{getLL().flow.sort_by_time()} {getLL().flow.sort_asc()}</span>
          </button>
          <button type="button" onClick={() => handleSelectSort('updated_at', 'desc')}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors">
            <Check className={sortBy === 'updated_at' && sortOrder === 'desc' ? 'w-3.5 h-3.5' : 'w-3.5 h-3.5 invisible'} />
            <span>{getLL().flow.sort_by_time()} {getLL().flow.sort_desc()}</span>
          </button>
        </div>,
        document.body
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
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            {getLL().flow.new_flow()}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setOpenFolderMenuPath(null);
              handleStartCreateFolder(openFolderMenuPath ?? undefined);
            }}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" />
            {getLL().flow.new_subfolder()}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setOpenFolderMenuPath(null);
              const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
              setExpandedMoveFolderMenuFolders(new Set());
              setMoveFolderMenuPosition(computeMenuPosition(rect, 180, 300, 4, 'right'));
              setMoveFolderMenuPath(openFolderMenuPath);
            }}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
          >
            <Move className="w-3.5 h-3.5" />
            {getLL().flow.move_to()}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleStartRenameFolder(openFolderMenuPath);
            }}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
          >
            <Pencil className="w-3.5 h-3.5" />
            {getLL().flow.rename()}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleDeleteFolder(openFolderMenuPath);
            }}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-950/30 transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
            {getLL().flow.delete_folder()}
          </button>
        </div>,
        document.body
      )}

      {/* 移动到子菜单 Portal */}
      {moveMenuUuid !== null && createPortal(
        <div
          ref={moveMenuRef}
          className="fixed z-[10000] min-w-[180px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
          style={{ top: moveMenuPosition.top, left: moveMenuPosition.left }}
        >
          <div className="max-h-[300px] overflow-y-auto py-1">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleMoveFlow(moveMenuUuid, '');
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
            >
              <FileJson className="w-3.5 h-3.5 shrink-0" />
              {getLL().flow.root_dir()}
            </button>
            {(() => {
              const renderFolderItems = (folderPaths: string[], depth: number): ReactNode[] => {
                const items: ReactNode[] = [];
                for (const path of folderPaths) {
                  const folder = folders.find(f => f.path === path);
                  if (!folder) continue;
                  const children = folderTree.childrenMap.get(path) ?? [];
                  const hasChildren = children.length > 0;
                  const isExpanded = expandedMoveMenuFolders.has(path);
                  const isCurrentFlowFolder = flows.find(f => f.uuid === moveMenuUuid)?.folder_path === path;

                  items.push(
                    <div key={path} className="flex items-center hover:bg-accent transition-colors"
                      style={{ paddingLeft: `${depth * 16}px` }}
                    >
                      {/* 折叠/展开按钮（有子文件夹时显示） */}
                      {hasChildren ? (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleExpandedMoveFolder(path);
                          }}
                          className="p-0.5 shrink-0 text-muted-foreground hover:text-foreground"
                        >
                          {isExpanded ? (
                            <ChevronDown className="w-3 h-3" />
                          ) : (
                            <ChevronRight className="w-3 h-3" />
                          )}
                        </button>
                      ) : (
                        <span className="w-4 shrink-0" />
                      )}
                      {/* 文件夹名称（点击选择目标） */}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleMoveFlow(moveMenuUuid, path);
                        }}
                        disabled={isCurrentFlowFolder}
                        className="flex items-center gap-1.5 px-2 py-1.5 text-xs disabled:opacity-40 disabled:cursor-not-allowed flex-1 min-w-0"
                      >
                        <Folder className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                        <span className="truncate">{folder.name}</span>
                        {isCurrentFlowFolder && (
                          <span className="text-[10px] text-muted-foreground shrink-0">{getLL().flow.current_label()}</span>
                        )}
                      </button>
                    </div>,
                  );
                  if (hasChildren && isExpanded) {
                    items.push(...renderFolderItems(children, depth + 1));
                  }
                }
                return items;
              };
              return renderFolderItems(folderTree.rootPaths, 0);
            })()}
          </div>
        </div>,
        document.body
      )}

      {/* 文件夹移动子菜单 Portal */}
      {moveFolderMenuPath !== null && createPortal(
        <div
          ref={moveFolderMenuRef}
          className="fixed z-[10000] min-w-[180px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
          style={{ top: moveFolderMenuPosition.top, left: moveFolderMenuPosition.left }}
        >
          <div className="max-h-[300px] overflow-y-auto py-1">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleMoveFolder(moveFolderMenuPath, '');
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
            >
              <FileJson className="w-3.5 h-3.5 shrink-0" />
              {getLL().flow.root_dir()}
            </button>
            {(() => {
              const renderFolderItems = (folderPaths: string[], depth: number): ReactNode[] => {
                const items: ReactNode[] = [];
                for (const path of folderPaths) {
                  // 排除自身和自身的子目录
                  if (path === moveFolderMenuPath) continue;
                  if (path.startsWith(moveFolderMenuPath + '/')) continue;
                  const folder = folders.find(f => f.path === path);
                  if (!folder) continue;
                  const children = folderTree.childrenMap.get(path) ?? [];
                  const hasChildren = children.length > 0;
                  const isExpanded = expandedMoveFolderMenuFolders.has(path);

                  items.push(
                    <div key={path} className="flex items-center hover:bg-accent transition-colors"
                      style={{ paddingLeft: `${depth * 16}px` }}
                    >
                      {/* 折叠/展开按钮（有子文件夹时显示） */}
                      {hasChildren ? (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleExpandedMoveFolderMenu(path);
                          }}
                          className="p-0.5 shrink-0 text-muted-foreground hover:text-foreground"
                        >
                          {isExpanded ? (
                            <ChevronDown className="w-3 h-3" />
                          ) : (
                            <ChevronRight className="w-3 h-3" />
                          )}
                        </button>
                      ) : (
                        <span className="w-4 shrink-0" />
                      )}
                      {/* 文件夹名称（点击选择目标） */}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleMoveFolder(moveFolderMenuPath, path);
                        }}
                        className="flex items-center gap-1.5 px-2 py-1.5 text-xs flex-1 min-w-0"
                      >
                        <Folder className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                        <span className="truncate">{folder.name}</span>
                      </button>
                    </div>,
                  );
                  if (hasChildren && isExpanded) {
                    items.push(...renderFolderItems(children, depth + 1));
                  }
                }
                return items;
              };
              return renderFolderItems(folderTree.rootPaths, 0);
            })()}
          </div>
        </div>,
        document.body
      )}

      {/* 验证错误对话框 */}
      <ValidationErrorDialog
        open={showValidationDialog}
        type={validationType}
        errors={validationErrors}
        warnings={validationWarnings}
        onClose={() => setShowValidationDialog(false)}
      />
    </div>
  );
}
