/**
 * EnvironmentPanel - 左侧面板中的全局环境列表
 *
 * 与数据源面板一致：由外层的 CollapsiblePanel 提供折叠 / 拖拽调高能力，
 * 本组件只负责列表渲染、激活切换与右键操作。
 * 创建 / 删除等写入操作统一交给「环境与变量管理」弹窗处理。
 */
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Globe, Copy, MoreVertical, Pencil, Settings2, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { useEnvironmentStore } from '@/stores/environment';
import { useUIStore } from '@/stores/ui';
import { environmentStorage } from '@/services/storage/environmentStorage';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

export function EnvironmentPanel() {
  const ll = getLL();
  const envLl = ll.panels.environment;
  const {
    environments,
    activeEnvironmentName,
    loadEnvironments,
    setActiveEnvironment,
    renameEnvironment,
    saveEnvironment,
  } = useEnvironmentStore();
  const openEnvironmentManagerDialog = useUIStore((s) => s.openEnvironmentManagerDialog);

  const [renamingOldName, setRenamingOldName] = useState<string | null>(null);
  const [renamingNewName, setRenamingNewName] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);

  const [openMenuName, setOpenMenuName] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0 });

  /** 切换激活环境：加载失败（文件被删 / YAML 非法）时提示而不是静默失败 */
  const activateEnvironment = async (name: string | null) => {
    try {
      await setActiveEnvironment(name);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    if (useEnvironmentStore.getState().environments.length === 0) {
      void loadEnvironments();
    }
  }, [loadEnvironments]);
  useEffect(() => {
    if (renamingOldName) renameInputRef.current?.focus();
  }, [renamingOldName]);

  useEffect(() => {
    if (openMenuName === null) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpenMenuName(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuName]);

  const handleConfirmRename = async () => {
    if (!renamingOldName) return;
    const trimmed = renamingNewName.trim();
    if (!trimmed || trimmed === renamingOldName) {
      setRenamingOldName(null);
      setRenamingNewName('');
      return;
    }
    try {
      await renameEnvironment(renamingOldName, trimmed);
      toast.success(ll.common.save_success());
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setRenamingOldName(null);
      setRenamingNewName('');
    }
  };

  const handleDuplicate = async (name: string) => {
    try {
      const source = await environmentStorage.loadEnvironment(name);
      let newName = `${name}-copy`;
      let counter = 1;
      while (environments.some((e) => e.name === newName)) {
        counter += 1;
        newName = `${name}-copy-${counter}`;
      }
      await saveEnvironment(
        { name: newName, description: source.description, variables: { ...source.variables } },
        false,
      );
      toast.success(ll.common.save_success());
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  if (environments.length === 0) {
    return (
      <div className="px-3 py-4 text-center text-xs text-muted-foreground/70 leading-relaxed">
        {ll.flow.no_envs_hint()}
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden scrollbar-thin px-1.5 py-1 space-y-0.5">
      {environments.map((env) => {
        const isActive = activeEnvironmentName === env.name;

        if (renamingOldName === env.name) {
          return (
            <div key={env.name} className="flex items-center gap-1.5 px-1.5 py-1">
              <Globe className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
              <input
                ref={renameInputRef}
                type="text"
                value={renamingNewName}
                onChange={(e) => setRenamingNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleConfirmRename();
                  if (e.key === 'Escape') {
                    setRenamingOldName(null);
                    setRenamingNewName('');
                  }
                }}
                onBlur={() => void handleConfirmRename()}
                className="h-6 flex-1 min-w-0 text-xs px-1.5 rounded border border-primary bg-background focus:outline-none"
              />
            </div>
          );
        }

        return (
          <div
            key={env.name}
            role="button"
            tabIndex={0}
            onClick={() => void activateEnvironment(isActive ? null : env.name)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                void activateEnvironment(isActive ? null : env.name);
              }
            }}
            title={isActive ? envLl.clickToDeactivate() : envLl.clickToActivate()}
            className={cn(
              'group flex items-center gap-2 px-2 py-1.5 rounded-md text-xs transition-colors cursor-pointer select-none',
              isActive
                ? 'bg-primary/10 text-primary font-medium'
                : 'text-foreground hover:bg-accent/60',
            )}
          >
            <span
              className={cn(
                'w-2 h-2 rounded-full shrink-0 transition-all',
                isActive
                  ? 'bg-emerald-500 ring-2 ring-emerald-500/30'
                  : 'border border-muted-foreground/40',
              )}
            />
            <span className="truncate flex-1 min-w-0">{env.name}</span>

            <div className="flex items-center gap-1 shrink-0">
              {env.hasLocalOverride && (
                <span className="text-[10px] leading-4 text-amber-600 dark:text-amber-400 px-1 rounded border border-amber-500/30">
                  local
                </span>
              )}
              <span className="text-[11px] leading-4 text-muted-foreground/70 px-1 rounded bg-muted/50 tabular-nums">
                {env.variableCount}
              </span>
              <button
                type="button"
                aria-label={ll.flow.more_ops()}
                onClick={(e) => {
                  e.stopPropagation();
                  if (openMenuName === env.name) {
                    setOpenMenuName(null);
                    return;
                  }
                  const rect = e.currentTarget.getBoundingClientRect();
                  setMenuPosition({
                    top: rect.bottom + 4,
                    left: Math.max(8, rect.right - 160),
                  });
                  setOpenMenuName(env.name);
                }}
                className={cn(
                  'p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-accent transition-opacity',
                  openMenuName === env.name ? 'opacity-100' : 'opacity-0 group-hover:opacity-100',
                )}
              >
                <MoreVertical className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        );
      })}

      {openMenuName !== null &&
        createPortal(
          <div
            ref={menuRef}
            className="fixed z-[9999] min-w-[160px] whitespace-nowrap rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95 py-1"
            style={{ top: menuPosition.top, left: menuPosition.left }}
          >
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setRenamingOldName(openMenuName);
                setRenamingNewName(openMenuName);
                setOpenMenuName(null);
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
            >
              <Pencil className="w-3.5 h-3.5" />
              {ll.flow.rename()}
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                const name = openMenuName;
                setOpenMenuName(null);
                void handleDuplicate(name);
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent transition-colors"
            >
              <Copy className="w-3.5 h-3.5" />
              {ll.flow.duplicate_env()}
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                const name = openMenuName;
                setOpenMenuName(null);
                openEnvironmentManagerDialog('sidebar', name);
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-950/30 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
              {ll.flow.delete_env()}
            </button>
          </div>,
          document.body,
        )}
    </div>
  );
}

/** 环境面板标题栏右侧的「打开管理弹窗」按钮（供 CollapsiblePanel headerRight 使用） */
export function EnvironmentPanelHeaderAction() {
  const ll = getLL();
  const openEnvironmentManagerDialog = useUIStore((s) => s.openEnvironmentManagerDialog);
  const environmentCount = useEnvironmentStore((s) => s.environments.length);

  // 保证当标题栏挂载时（即使环境面板主体处于折叠状态），环境数量角标也能立即在后台拉取并显示
  useEffect(() => {
    if (useEnvironmentStore.getState().environments.length === 0) {
      void useEnvironmentStore.getState().loadEnvironments();
    }
  }, []);

  return (
    <div className="flex items-center gap-0.5" onClick={(e) => e.stopPropagation()}>
      <span className="text-xs text-muted-foreground font-normal mr-1 tabular-nums">
        ({environmentCount})
      </span>
      <button
        type="button"
        aria-label={ll.panels.environment.manageEnvs()}
        title={ll.flow.open_env_manager()}
        onClick={() => openEnvironmentManagerDialog('sidebar')}
        className="h-6 w-6 rounded-md hover:bg-accent flex items-center justify-center text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
      >
        <Settings2 className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}
