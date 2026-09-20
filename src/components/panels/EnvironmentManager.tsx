/**
 * 环境与变量管理组件
 *
 * 包含：
 * 1. EnvironmentSelector: 画布工具栏上的环境快速切换下拉菜单
 * 2. EnvironmentManagerDialog: 上下文感知的环境与变量管理对话框
 *    - `sidebar` 模式（纯全局视角）：仅管理环境 YAML 文件与本地覆盖
 *    - `canvas` 模式（执行级联视角）：额外提供流程内置兜底变量与级联对比矩阵
 */

import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Globe,
  Plus,
  Trash2,
  X,
  Settings2,
  FileCode,
  Shield,
  Layers,
  ArrowRightLeft,
  FolderGit2,
  AlertTriangle,
  Power,
} from 'lucide-react';
import { useFlowStore } from '@/stores/flow';
import { useUIStore } from '@/stores/ui';
import { useEnvironmentStore } from '@/stores/environment';
import { environmentStorage } from '@/services/storage/environmentStorage';
import { cn } from '@/lib/utils';
import type { EnvironmentSet, EnvironmentComparisonItem } from '@/types/environment';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';

/** 级联矩阵列宽定义（表头与数据行共用，避免元素重叠）
 *  右侧操作列固定宽度并始终可见，其余列按比例伸缩 + 文本截断 */
const MATRIX_GRID =
  'grid grid-cols-[minmax(84px,1fr)_minmax(104px,1.3fr)_68px_minmax(84px,1fr)_minmax(84px,1fr)_112px] gap-x-2 items-center';

/** 删除范围 */
type DeleteScope = 'all' | 'local_only';

// ============================================================================
// 1. 画布工具栏环境选择器
// ============================================================================

export function EnvironmentSelector() {
  const ll = getLL().panels.environment;
  const flow = useFlowStore((s) => s.flow);
  const { environments, activeEnvironmentName, setActiveEnvironment, loadEnvironments } =
    useEnvironmentStore();
  const openEnvironmentManagerDialog = useUIStore((s) => s.openEnvironmentManagerDialog);
  const [showDropdown, setShowDropdown] = useState(false);

  useEffect(() => {
    void loadEnvironments();
  }, [loadEnvironments]);

  /** 切换激活环境：加载失败（文件被删 / YAML 非法）时提示而不是静默失败 */
  const activateEnvironment = async (name: string | null) => {
    try {
      await setActiveEnvironment(name);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  if (!flow) return null;

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setShowDropdown(!showDropdown)}
        className={cn(
          'flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs',
          'transition-all duration-150 cursor-pointer select-none',
          'hover:bg-accent border border-transparent',
          activeEnvironmentName
            ? 'text-primary font-medium bg-primary/10 border-primary/20'
            : 'text-muted-foreground hover:text-foreground',
        )}
        title={activeEnvironmentName ? ll.currentEnv({ name: activeEnvironmentName }) : ll.selectEnv()}
      >
        <Globe className="w-3.5 h-3.5 shrink-0" />
        <span className="max-w-[100px] truncate">
          {activeEnvironmentName ?? ll.defaultVars()}
        </span>
      </button>

      {showDropdown && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setShowDropdown(false)} />
          <div
            className={cn(
              'absolute top-full left-0 mt-1 z-50',
              'w-56 bg-background/95 backdrop-blur-xl rounded-lg',
              'shadow-xl border border-border/60 py-1 select-none',
            )}
          >
            {/* 无环境（仅流程内置变量） */}
            <button
              type="button"
              onClick={() => {
                void activateEnvironment(null);
                setShowDropdown(false);
              }}
              className={cn(
                'w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer flex items-center justify-between',
                !activeEnvironmentName && 'text-primary font-medium bg-primary/5',
              )}
            >
              <div className="flex items-center gap-2">
                <span
                  className={cn(
                    'w-2 h-2 rounded-full border shrink-0',
                    !activeEnvironmentName
                      ? 'bg-primary border-primary ring-2 ring-primary/20'
                      : 'border-muted-foreground/40',
                  )}
                />
                <span>{ll.defaultVars()}</span>
              </div>
              <span className="text-[10px] text-muted-foreground/70">
                {Object.keys(flow.initial_variables ?? {}).length}
              </span>
            </button>

            {/* 环境列表 */}
            {environments.map((env) => {
              const isActive = activeEnvironmentName === env.name;
              return (
                <button
                  key={env.name}
                  type="button"
                  onClick={() => {
                    // 再次点击当前环境 = 取消激活，回到「仅流程内置变量」
                    void activateEnvironment(activeEnvironmentName === env.name ? null : env.name);
                    setShowDropdown(false);
                  }}
                  className={cn(
                    'w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer flex items-center justify-between',
                    isActive && 'text-primary font-medium bg-primary/5',
                  )}
                >
                  <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
                    <span
                      className={cn(
                        'w-2 h-2 rounded-full border shrink-0',
                        isActive
                          ? 'bg-emerald-500 border-emerald-500 ring-2 ring-emerald-500/20'
                          : 'border-muted-foreground/40',
                      )}
                    />
                    <span className="truncate">{env.name}</span>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 text-muted-foreground">
                    {env.hasLocalOverride && (
                      <span className="text-[9px] leading-4 text-amber-500 font-mono px-1 rounded border border-amber-500/30">
                        local
                      </span>
                    )}
                    <span className="text-[10px] tabular-nums">({env.variableCount})</span>
                  </div>
                </button>
              );
            })}

            {/* 管理入口 */}
            <div className="border-t border-border/50 mt-1 pt-1">
              <button
                type="button"
                onClick={() => {
                  setShowDropdown(false);
                  openEnvironmentManagerDialog('canvas');
                }}
                className="w-full text-left px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer flex items-center gap-2"
              >
                <Settings2 className="w-3.5 h-3.5" />
                <span>{ll.manageEnvs()}</span>
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

// ============================================================================
// 2. 变量行编辑列表（主环境 / 本地覆盖共用）
// ============================================================================

function VariableEditorList({
  variables,
  onAdd,
  onUpdate,
  onRemove,
}: {
  variables: Record<string, unknown>;
  onAdd: (key: string, value: unknown) => void;
  onUpdate: (key: string, value: unknown) => void;
  onRemove: (key: string) => void;
}) {
  const ll = getLL().panels.environment;
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  /** 正在编辑但尚未提交的值：逐键写盘会带来「每敲一个字符一次 IPC + 一次 toast」 */
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const parseValue = (raw: string): unknown => {
    try {
      return JSON.parse(raw);
    } catch {
      return raw;
    }
  };

  const formatValue = (val: unknown): string =>
    typeof val === 'string' ? val : JSON.stringify(val);

  /** 提交单个草稿（失焦 / 回车），值未变化时不再触发写入 */
  const commitDraft = (key: string) => {
    const draft = drafts[key];
    if (draft === undefined) return;
    setDrafts((prev) => {
      const { [key]: _committed, ...rest } = prev;
      return rest;
    });
    const current = formatValue(variables[key]);
    if (draft !== current) {
      onUpdate(key, parseValue(draft));
    }
  };

  const commitAllDrafts = () => {
    for (const key of Object.keys(drafts)) commitDraft(key);
  };

  const handleAdd = () => {
    const key = newKey.trim();
    if (!key) return;
    commitAllDrafts();
    onAdd(key, parseValue(newValue));
    setNewKey('');
    setNewValue('');
  };

  // 卸载（切换环境 / 关闭弹窗）时兜底提交，避免输入内容丢失
  const draftsRef = useRef(drafts);
  draftsRef.current = drafts;
  const commitRef = useRef<(key: string) => void>(() => {});
  commitRef.current = commitDraft;
  useEffect(
    () => () => {
      for (const key of Object.keys(draftsRef.current)) commitRef.current(key);
    },
    [],
  );

  const entries = Object.entries(variables);

  return (
    <>
      <div className="flex-1 min-h-0 rounded-lg border border-border bg-muted/5 overflow-y-auto scrollbar-thin p-2 space-y-1.5">
        {entries.map(([key, val]) => (
          <div key={key} className="flex items-center gap-2 group">
            <span
              className="text-xs font-mono font-medium text-foreground w-40 shrink-0 truncate"
              title={key}
            >
              {key}
            </span>
            <input
              type="text"
              value={drafts[key] ?? formatValue(val)}
              onChange={(e) => setDrafts((prev) => ({ ...prev, [key]: e.target.value }))}
              onBlur={() => commitDraft(key)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  commitDraft(key);
                  e.currentTarget.blur();
                }
              }}
              className="h-7 flex-1 min-w-0 rounded border border-border bg-background px-2 text-xs font-mono focus:border-primary focus:outline-none"
            />
            <button
              type="button"
              aria-label={getLL().common.DELETE()}
              onClick={() => {
                setDrafts((prev) => {
                  const { [key]: _removed, ...rest } = prev;
                  return rest;
                });
                onRemove(key);
              }}
              className="p-1 rounded text-muted-foreground hover:text-rose-500 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer shrink-0"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        ))}
        {entries.length === 0 && (
          <div className="py-8 text-center text-xs text-muted-foreground/60">
            {ll.noDefaultVarsHint()}
          </div>
        )}
      </div>

      <div className="mt-3 flex items-center gap-2 pt-2 border-t border-border shrink-0">
        <input
          type="text"
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          placeholder={ll.keyPlaceholder()}
          className="h-8 w-40 shrink-0 rounded-md border border-border bg-background px-2.5 text-xs font-mono focus:border-primary focus:outline-none"
        />
        <input
          type="text"
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder={ll.valuePlaceholder()}
          className="h-8 flex-1 min-w-0 rounded-md border border-border bg-background px-2.5 text-xs font-mono focus:border-primary focus:outline-none"
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleAdd();
          }}
        />
        <button
          type="button"
          onClick={handleAdd}
          disabled={!newKey.trim()}
          className="h-8 px-3 shrink-0 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer flex items-center gap-1.5"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{ll.addVariable()}</span>
        </button>
      </div>
    </>
  );
}

// ============================================================================
// 3. 双上下文环境管理对话框
// ============================================================================

export function EnvironmentManagerDialog() {
  const ll = getLL().panels.environment;
  const context = useUIStore((s) => s.environmentManagerContext);
  const pendingDeleteEnv = useUIStore((s) => s.environmentManagerPendingDelete);
  const closeEnvironmentManagerDialog = useUIStore((s) => s.closeEnvironmentManagerDialog);

  const flow = useFlowStore((s) => s.flow);
  const updateInitialVariables = useFlowStore((s) => s.updateInitialVariables);

  const {
    environments,
    activeEnvironmentName,
    loadEnvironments,
    setActiveEnvironment,
    createEnvironment,
    saveEnvironment,
    deleteEnvironment,
    deleteEnvironmentLocalOverride,
  } = useEnvironmentStore();

  /** 当前选中的 Tab 项：`__fallback__`（仅 canvas 模式）或具体环境名称 */
  const [selectedEnvName, setSelectedEnvName] = useState<string>('__fallback__');
  const [baseEnvData, setBaseEnvData] = useState<EnvironmentSet | null>(null);
  const [localEnvData, setLocalEnvData] = useState<EnvironmentSet | null>(null);
  const [hasLocalOverride, setHasLocalOverride] = useState(false);
  const [activeSubTab, setActiveSubTab] = useState<'base' | 'local' | 'matrix'>('base');

  /** 新建环境输入 */
  const [newEnvName, setNewEnvName] = useState('');
  /** 待确认删除的环境与删除范围 */
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const [deleteScope, setDeleteScope] = useState<DeleteScope>('all');
  const [isDeleting, setIsDeleting] = useState(false);

  const newEnvInputRef = useRef<HTMLInputElement>(null);

  /** 环境列表是否已完成首次加载（决定能否安全地挑选默认选中项） */
  const [listSettled, setListSettled] = useState(false);
  useEffect(() => {
    let cancelled = false;
    void loadEnvironments().finally(() => {
      if (!cancelled) setListSettled(true);
    });
    return () => {
      cancelled = true;
    };
  }, [loadEnvironments]);

  // 来自侧边栏的删除请求：直接进入删除确认视图
  useEffect(() => {
    if (pendingDeleteEnv) {
      setSelectedEnvName(pendingDeleteEnv);
      setPendingDelete(pendingDeleteEnv);
      setDeleteScope('all');
    }
  }, [pendingDeleteEnv]);

  const initializedRef = useRef(false);

  // 首次打开时按上下文挑选默认选中项（仅执行一次，之后尊重用户选择）
  //
  // 必须等列表加载完成再决定：否则「列表尚未返回」会被误判成「激活环境不存在」，
  // 从而把 canvas 模式锁死在「流程内置变量」视图上。
  useEffect(() => {
    if (initializedRef.current || !listSettled) return;
    initializedRef.current = true;

    if (context === 'sidebar') {
      setSelectedEnvName(environments.length > 0 ? environments[0].name : '');
      setActiveSubTab('base');
      return;
    }

    const preferred =
      activeEnvironmentName && environments.some((e) => e.name === activeEnvironmentName)
        ? activeEnvironmentName
        : '__fallback__';
    setSelectedEnvName(preferred);
    setActiveSubTab(preferred === '__fallback__' ? 'base' : 'matrix');
  }, [context, listSettled, environments, activeEnvironmentName]);

  // 拉取选中环境的主文件与本地覆盖
  useEffect(() => {
    if (!selectedEnvName || selectedEnvName === '__fallback__') {
      setBaseEnvData(null);
      setLocalEnvData(null);
      setHasLocalOverride(false);
      return;
    }

    let isMounted = true;
    void (async () => {
      try {
        const base = await environmentStorage.loadEnvironment(selectedEnvName, false);
        let local: EnvironmentSet | null = null;
        try {
          local = await environmentStorage.loadEnvironment(selectedEnvName, true);
        } catch {
          local = null;
        }
        if (!isMounted) return;
        setBaseEnvData(base);
        setLocalEnvData(local);
        setHasLocalOverride(local !== null);
      } catch {
        if (!isMounted) return;
        setBaseEnvData(null);
        setLocalEnvData(null);
        setHasLocalOverride(false);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, [selectedEnvName]);

  /** 切换激活环境：加载失败（文件被删 / YAML 非法）时提示而不是静默失败 */
  const activateEnvironment = async (name: string | null) => {
    try {
      await setActiveEnvironment(name);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  const handleCreateEnv = async () => {
    const trimmed = newEnvName.trim();
    if (!trimmed) {
      newEnvInputRef.current?.focus();
      return;
    }
    try {
      await createEnvironment(trimmed, {});
      setNewEnvName('');
      setSelectedEnvName(trimmed);
      setActiveSubTab(context === 'canvas' ? 'matrix' : 'base');
      toast.success(getLL().common.save_success());
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  const persistEnvData = async (isLocal: boolean, variables: Record<string, unknown>) => {
    if (!selectedEnvName || selectedEnvName === '__fallback__') return;
    const target: EnvironmentSet = {
      name: selectedEnvName,
      description: isLocal ? undefined : baseEnvData?.description,
      variables,
    };
    try {
      await saveEnvironment(target, isLocal);
      if (isLocal) {
        setLocalEnvData(target);
      } else {
        setBaseEnvData(target);
      }
      toast.success(getLL().common.save_success());
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  const handleToggleLocalOverride = async () => {
    if (!selectedEnvName || selectedEnvName === '__fallback__') return;
    try {
      if (hasLocalOverride) {
        await deleteEnvironmentLocalOverride(selectedEnvName);
        setLocalEnvData(null);
        setHasLocalOverride(false);
        setActiveSubTab('base');
        toast.success(ll.localOverrideDeleted({ name: selectedEnvName }));
      } else {
        await saveEnvironment({ name: selectedEnvName, variables: {} }, true);
        setLocalEnvData({ name: selectedEnvName, variables: {} });
        setHasLocalOverride(true);
        setActiveSubTab('local');
        toast.success(ll.localOverrideCreated({ name: selectedEnvName }));
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  const handleConfirmDelete = async () => {
    if (!pendingDelete) return;
    setIsDeleting(true);
    try {
      if (deleteScope === 'local_only') {
        await deleteEnvironmentLocalOverride(pendingDelete);
        toast.success(ll.localOverrideDeleted({ name: pendingDelete }));
        setHasLocalOverride(false);
        setLocalEnvData(null);
        // 覆盖层已不存在，停留在 local 页会显示一个空编辑器
        if (activeSubTab === 'local') setActiveSubTab('base');
      } else {
        await deleteEnvironment(pendingDelete);
        toast.success(ll.envDeleted({ name: pendingDelete }));
        const fallbackName = environments.find((e) => e.name !== pendingDelete)?.name;
        if (context === 'canvas') {
          // canvas 模式始终存在「流程内置变量」页签，删到最后一个环境时回到它
          const nextName = fallbackName ?? '__fallback__';
          setSelectedEnvName(nextName);
          setActiveSubTab(nextName === '__fallback__' ? 'base' : 'matrix');
        } else {
          setSelectedEnvName(fallbackName ?? '');
          setActiveSubTab('base');
        }
      }
      setPendingDelete(null);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleAddFallbackVariable = (key: string, value: unknown) => {
    updateInitialVariables({ ...(flow?.initial_variables ?? {}), [key]: value });
  };

  const handleRemoveFallbackVariable = (key: string) => {
    const { [key]: _removed, ...rest } = flow?.initial_variables ?? {};
    updateInitialVariables(rest);
  };

  const handleUpdateFallbackVariable = (key: string, value: unknown) => {
    updateInitialVariables({ ...(flow?.initial_variables ?? {}), [key]: value });
  };

  // 级联对比矩阵
  const comparisonItems = useMemo<EnvironmentComparisonItem[]>(() => {
    if (context !== 'canvas' || !selectedEnvName || selectedEnvName === '__fallback__') {
      return [];
    }

    const flowVars = flow?.initial_variables ?? {};
    const baseVars = baseEnvData?.variables ?? {};
    const localVars = localEnvData?.variables ?? {};

    const allKeys = Array.from(
      new Set([...Object.keys(flowVars), ...Object.keys(baseVars), ...Object.keys(localVars)]),
    ).sort();

    return allKeys.map((key) => {
      const hasFlow = key in flowVars;
      const hasBase = key in baseVars;
      const hasLocal = key in localVars;

      let effectiveValue: unknown;
      let source: EnvironmentComparisonItem['source'];
      if (hasLocal) {
        effectiveValue = localVars[key];
        source = 'local_override';
      } else if (hasBase) {
        effectiveValue = baseVars[key];
        source = 'environment';
      } else {
        effectiveValue = flowVars[key];
        source = 'flow_fallback';
      }

      return {
        key,
        effectiveValue,
        source,
        envValue: hasBase ? baseVars[key] : undefined,
        localValue: hasLocal ? localVars[key] : undefined,
        fallbackValue: hasFlow ? flowVars[key] : undefined,
        isOverridden: (hasBase || hasLocal) && hasFlow,
      };
    });
  }, [context, selectedEnvName, flow?.initial_variables, baseEnvData, localEnvData]);

  const formatValue = (value: unknown): string =>
    value === undefined ? '—' : typeof value === 'string' ? value : JSON.stringify(value);

  const isFallbackSelected = selectedEnvName === '__fallback__';
  const pendingDeleteHasLocal = environments.find((e) => e.name === pendingDelete)?.hasLocalOverride ?? false;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-xs"
        onClick={closeEnvironmentManagerDialog}
      />

      <div className="relative z-10 w-full max-w-5xl h-[640px] bg-background rounded-xl shadow-2xl border border-border flex flex-col overflow-hidden animate-in fade-in-0 zoom-in-95">
        {/* 顶部标题栏 */}
        <div className="flex items-center justify-between px-5 py-3.5 border-b border-border bg-muted/20 shrink-0">
          <div className="flex items-center gap-2.5 min-w-0">
            <Globe className="w-5 h-5 text-primary shrink-0" />
            <div className="min-w-0">
              <h2 className="text-sm font-semibold text-foreground truncate">
                {context === 'sidebar' ? ll.globalTitle() : ll.canvasTitle()}
              </h2>
              <p className="text-[11px] text-muted-foreground truncate">
                {context === 'sidebar' ? ll.globalSubtitle() : ll.canvasSubtitle()}
              </p>
            </div>
          </div>
          <button
            type="button"
            aria-label={getLL().common.CANCEL()}
            onClick={closeEnvironmentManagerDialog}
            className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer shrink-0"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 主体两栏布局 */}
        <div className="flex-1 flex min-h-0 overflow-hidden">
          {/* 左栏：环境列表 */}
          <div className="w-56 border-r border-border flex flex-col bg-muted/10 shrink-0">
            <div className="flex-1 overflow-y-auto p-2 space-y-1 scrollbar-thin">
              {context === 'canvas' && (
                <>
                  <button
                    type="button"
                    onClick={() => {
                      setSelectedEnvName('__fallback__');
                      setActiveSubTab('base');
                    }}
                    className={cn(
                      'w-full flex items-center justify-between px-2.5 py-2 rounded-lg text-xs transition-all cursor-pointer select-none text-left',
                      isFallbackSelected
                        ? 'bg-primary/10 text-primary font-medium border border-primary/20'
                        : 'text-foreground hover:bg-accent border border-transparent',
                    )}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <FileCode className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                      <span className="truncate">{ll.flowFallback()}</span>
                    </div>
                    <span className="text-[10px] tabular-nums text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full shrink-0">
                      {Object.keys(flow?.initial_variables ?? {}).length}
                    </span>
                  </button>
                  <div className="px-2 pt-2 pb-1 text-[10px] font-semibold text-muted-foreground/60 uppercase tracking-wider">
                    {ll.globalEnvsSection()}
                  </div>
                </>
              )}

              {environments.map((env) => {
                const isSelected = selectedEnvName === env.name;
                const isActive = activeEnvironmentName === env.name;
                return (
                  <div
                    key={env.name}
                    className={cn(
                      'group w-full flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-xs transition-all',
                      isSelected
                        ? 'bg-primary/10 text-primary font-medium border border-primary/20'
                        : 'text-foreground hover:bg-accent border border-transparent',
                    )}
                  >
                    {/* 激活开关：radio 语义，点击即切换全局激活环境 */}
                    <button
                      type="button"
                      role="radio"
                      aria-checked={isActive}
                      title={isActive ? ll.clickToDeactivate() : ll.clickToActivate()}
                      onClick={() => void activateEnvironment(isActive ? null : env.name)}
                      className="shrink-0 p-0.5 rounded-full cursor-pointer"
                    >
                      <span
                        className={cn(
                          'block w-2.5 h-2.5 rounded-full border transition-all',
                          isActive
                            ? 'bg-emerald-500 border-emerald-500 ring-2 ring-emerald-500/25'
                            : 'border-muted-foreground/40 hover:border-foreground',
                        )}
                      />
                    </button>

                    <button
                      type="button"
                      onClick={() => {
                        setSelectedEnvName(env.name);
                        setPendingDelete(null);
                        if (context === 'canvas') setActiveSubTab('matrix');
                      }}
                      className="flex-1 min-w-0 flex items-center justify-between gap-1.5 text-left cursor-pointer"
                    >
                      <span className="truncate">{env.name}</span>
                      <span className="flex items-center gap-1 shrink-0">
                        {env.hasLocalOverride && (
                          <span className="text-[9px] leading-4 text-amber-500 font-mono px-1 rounded border border-amber-500/30">
                            local
                          </span>
                        )}
                        <span className="text-[10px] tabular-nums text-muted-foreground/70">
                          {env.variableCount}
                        </span>
                      </span>
                    </button>

                    <button
                      type="button"
                      aria-label={ll.deleteEnv()}
                      title={ll.deleteEnv()}
                      onClick={() => {
                        setSelectedEnvName(env.name);
                        setPendingDelete(env.name);
                        setDeleteScope('all');
                      }}
                      className="shrink-0 p-0.5 rounded text-muted-foreground hover:text-rose-500 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                );
              })}
            </div>

            {/* 新建环境 */}
            <div className="p-2 border-t border-border space-y-1.5 shrink-0">
              <div className="flex items-center gap-1.5">
                <input
                  ref={newEnvInputRef}
                  type="text"
                  value={newEnvName}
                  onChange={(e) => setNewEnvName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void handleCreateEnv();
                  }}
                  placeholder={ll.newEnvPlaceholder()}
                  className="h-7 flex-1 min-w-0 rounded-md border border-border bg-background px-2 text-xs focus:border-primary focus:outline-none"
                />
                <button
                  type="button"
                  aria-label={ll.addEnv()}
                  title={ll.addEnv()}
                  onClick={() => void handleCreateEnv()}
                  disabled={!newEnvName.trim()}
                  className="h-7 w-7 shrink-0 rounded-md bg-primary text-primary-foreground flex items-center justify-center hover:bg-primary/90 disabled:opacity-40 disabled:cursor-not-allowed transition-colors cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>

          {/* 右栏：内容区 */}
          <div className="flex-1 flex flex-col min-w-0 bg-background overflow-hidden">
            {/* 删除确认视图 */}
            {pendingDelete ? (
              <div className="flex-1 flex flex-col p-5 overflow-y-auto scrollbar-thin">
                <div className="flex items-center gap-2 mb-2">
                  <AlertTriangle className="w-4 h-4 text-rose-500 shrink-0" />
                  <h3 className="text-sm font-semibold text-foreground">{ll.deleteDialogTitle()}</h3>
                </div>
                <p className="text-xs text-muted-foreground mb-4 leading-relaxed">
                  {ll.deleteDialogDesc({ name: pendingDelete })}
                </p>

                <div className="space-y-2">
                  <label
                    className={cn(
                      'flex items-start gap-2.5 p-3 rounded-lg border cursor-pointer transition-colors select-none',
                      deleteScope === 'all'
                        ? 'border-rose-500/40 bg-rose-500/5'
                        : 'border-border hover:bg-accent/40',
                    )}
                  >
                    <input
                      type="radio"
                      name="env-delete-scope"
                      checked={deleteScope === 'all'}
                      onChange={() => setDeleteScope('all')}
                      className="mt-0.5 h-3.5 w-3.5 text-rose-500 focus:ring-rose-500"
                    />
                    <span className="min-w-0">
                      <span className="block text-xs font-medium text-foreground">
                        {ll.deleteScopeAll()}
                      </span>
                      <span className="block text-[11px] text-muted-foreground mt-0.5 leading-relaxed">
                        {ll.deleteScopeAllDesc()}
                      </span>
                    </span>
                  </label>

                  <label
                    className={cn(
                      'flex items-start gap-2.5 p-3 rounded-lg border cursor-pointer transition-colors select-none',
                      !pendingDeleteHasLocal && 'opacity-50 cursor-not-allowed',
                      deleteScope === 'local_only' && pendingDeleteHasLocal
                        ? 'border-amber-500/40 bg-amber-500/5'
                        : 'border-border hover:bg-accent/40',
                    )}
                  >
                    <input
                      type="radio"
                      name="env-delete-scope"
                      checked={deleteScope === 'local_only'}
                      disabled={!pendingDeleteHasLocal}
                      onChange={() => setDeleteScope('local_only')}
                      className="mt-0.5 h-3.5 w-3.5 text-amber-500 focus:ring-amber-500"
                    />
                    <span className="min-w-0">
                      <span className="flex items-center gap-1.5 text-xs font-medium text-foreground">
                        <Shield className="w-3 h-3 text-amber-500 shrink-0" />
                        {ll.deleteScopeLocalOnly()}
                      </span>
                      <span className="block text-[11px] text-muted-foreground mt-0.5 leading-relaxed">
                        {ll.deleteScopeLocalOnlyDesc()}
                      </span>
                    </span>
                  </label>
                </div>

                <div className="mt-auto pt-4 flex justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => setPendingDelete(null)}
                    disabled={isDeleting}
                    className="h-8 px-3 rounded-md border border-border bg-background text-xs hover:bg-accent transition-colors cursor-pointer disabled:opacity-50"
                  >
                    {getLL().common.CANCEL()}
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleConfirmDelete()}
                    disabled={isDeleting}
                    className="h-8 px-3 rounded-md bg-rose-600 text-white text-xs font-medium hover:bg-rose-700 transition-colors cursor-pointer disabled:opacity-50"
                  >
                    {ll.deleteConfirm()}
                  </button>
                </div>
              </div>
            ) : isFallbackSelected ? (
              /* 流程内置兜底变量编辑 */
              <div className="flex-1 flex flex-col p-5 overflow-hidden">
                <div className="mb-4 shrink-0">
                  <div className="flex items-center gap-2 mb-1">
                    <FileCode className="w-4 h-4 text-amber-500 shrink-0" />
                    <h3 className="text-sm font-semibold text-foreground">{ll.flowFallback()}</h3>
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {ll.flowFallbackDesc()}
                  </p>
                </div>
                <VariableEditorList
                  variables={flow?.initial_variables ?? {}}
                  onAdd={handleAddFallbackVariable}
                  onUpdate={handleUpdateFallbackVariable}
                  onRemove={handleRemoveFallbackVariable}
                />
              </div>
            ) : selectedEnvName ? (
              /* 具体环境 */
              <div className="flex-1 flex flex-col p-5 overflow-hidden">
                {/* 环境头部：名称 + 激活开关 + 本地覆盖开关 + 删除 */}
                <div className="flex flex-wrap items-center justify-between gap-2 pb-3 mb-3 border-b border-border shrink-0">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="text-sm font-semibold text-foreground truncate">
                      {selectedEnvName}
                    </span>
                    <button
                      type="button"
                      onClick={() =>
                        void activateEnvironment(
                          activeEnvironmentName === selectedEnvName ? null : selectedEnvName,
                        )
                      }
                      className={cn(
                        'flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium transition-colors cursor-pointer border shrink-0',
                        activeEnvironmentName === selectedEnvName
                          ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                          : 'bg-muted text-muted-foreground hover:text-foreground border-transparent',
                      )}
                      title={
                        activeEnvironmentName === selectedEnvName
                          ? ll.clickToDeactivate()
                          : ll.activateThisEnv()
                      }
                    >
                      <Power className="w-3 h-3" />
                      <span>
                        {activeEnvironmentName === selectedEnvName
                          ? ll.activated()
                          : ll.activateThisEnv()}
                      </span>
                    </button>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    <button
                      type="button"
                      onClick={() => void handleToggleLocalOverride()}
                      title={ll.localOverrideDesc()}
                      className={cn(
                        'flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs border transition-colors cursor-pointer',
                        hasLocalOverride
                          ? 'border-amber-500/40 text-amber-600 dark:text-amber-400 bg-amber-500/5 hover:bg-amber-500/10'
                          : 'border-border text-muted-foreground hover:text-foreground hover:bg-accent',
                      )}
                    >
                      <Shield className="w-3.5 h-3.5" />
                      <span>
                        {hasLocalOverride
                          ? ll.localOverrideDisableAction()
                          : ll.localOverrideEnableAction()}
                      </span>
                    </button>

                    <button
                      type="button"
                      aria-label={ll.deleteEnv()}
                      title={ll.deleteEnv()}
                      onClick={() => {
                        setPendingDelete(selectedEnvName);
                        setDeleteScope('all');
                      }}
                      className="p-1.5 rounded-md text-muted-foreground hover:text-rose-500 hover:bg-rose-500/10 transition-colors cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Sub Tab */}
                <div className="flex items-center gap-1 pb-2 border-b border-border/50 text-xs shrink-0 overflow-x-auto scrollbar-thin">
                  {context === 'canvas' && (
                    <button
                      type="button"
                      onClick={() => setActiveSubTab('matrix')}
                      className={cn(
                        'px-3 py-1 rounded-md font-medium transition-colors cursor-pointer flex items-center gap-1.5 whitespace-nowrap',
                        activeSubTab === 'matrix'
                          ? 'bg-primary/10 text-primary'
                          : 'text-muted-foreground hover:text-foreground',
                      )}
                    >
                      <Layers className="w-3.5 h-3.5" />
                      <span>{ll.tabMatrix()}</span>
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => setActiveSubTab('base')}
                    className={cn(
                      'px-3 py-1 rounded-md font-medium transition-colors cursor-pointer flex items-center gap-1.5 whitespace-nowrap',
                      activeSubTab === 'base'
                        ? 'bg-primary/10 text-primary'
                        : 'text-muted-foreground hover:text-foreground',
                    )}
                  >
                    <FolderGit2 className="w-3.5 h-3.5" />
                    <span>{`${ll.tabBase()} (${selectedEnvName}.yaml)`}</span>
                  </button>
                  {hasLocalOverride && (
                    <button
                      type="button"
                      onClick={() => setActiveSubTab('local')}
                      className={cn(
                        'px-3 py-1 rounded-md font-medium transition-colors cursor-pointer flex items-center gap-1.5 whitespace-nowrap',
                        activeSubTab === 'local'
                          ? 'bg-primary/10 text-primary'
                          : 'text-muted-foreground hover:text-foreground',
                      )}
                    >
                      <Shield className="w-3.5 h-3.5 text-amber-500" />
                      <span>{`${ll.tabLocal()} (${selectedEnvName}.local.yaml)`}</span>
                    </button>
                  )}
                </div>

                {activeSubTab === 'matrix' ? (
                  /* 级联对比矩阵：固定列宽网格，长值截断，避免重叠 */
                  <div className="flex-1 min-h-0 mt-2 rounded-lg border border-border overflow-hidden flex flex-col">
                    <div className="overflow-auto scrollbar-thin flex-1">
                      <div className="min-w-0">
                        <div
                          className={cn(
                            MATRIX_GRID,
                            'sticky top-0 z-10 bg-muted/60 backdrop-blur-sm px-3 py-2 border-b border-border text-[11px] font-medium text-muted-foreground',
                          )}
                        >
                          <span>{ll.keyPlaceholder()}</span>
                          <span>{ll.effectiveValue()}</span>
                          <span>{ll.source()}</span>
                          <span>{ll.envValue()}</span>
                          <span>{ll.fallbackValue()}</span>
                          <span className="text-right">{ll.actions()}</span>
                        </div>

                        {comparisonItems.map((item) => (
                          <div
                            key={item.key}
                            className={cn(
                              MATRIX_GRID,
                              'px-3 py-2 border-b border-border/50 last:border-b-0 hover:bg-muted/20 transition-colors text-xs',
                            )}
                          >
                            <span
                              className="font-mono font-medium text-foreground truncate"
                              title={item.key}
                            >
                              {item.key}
                            </span>
                            <span
                              className="font-mono text-primary font-semibold truncate"
                              title={formatValue(item.effectiveValue)}
                            >
                              {formatValue(item.effectiveValue)}
                            </span>
                            <span className="min-w-0">
                              {item.isOverridden ? (
                                <span className="inline-block px-1.5 py-0.5 rounded text-[10px] font-medium leading-4 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20 whitespace-nowrap">
                                  {ll.statusOverridden()}
                                </span>
                              ) : item.source === 'flow_fallback' ? (
                                <span className="inline-block px-1.5 py-0.5 rounded text-[10px] font-medium leading-4 bg-muted text-muted-foreground whitespace-nowrap">
                                  {ll.statusFallbackOnly()}
                                </span>
                              ) : (
                                <span className="inline-block px-1.5 py-0.5 rounded text-[10px] font-medium leading-4 bg-sky-500/10 text-sky-600 dark:text-sky-400 border border-sky-500/20 whitespace-nowrap">
                                  {ll.statusEnvOnly()}
                                </span>
                              )}
                            </span>
                            <span
                              className="font-mono text-muted-foreground truncate"
                              title={formatValue(item.envValue)}
                            >
                              {formatValue(item.envValue)}
                            </span>
                            <span
                              className="font-mono text-muted-foreground truncate"
                              title={formatValue(item.fallbackValue)}
                            >
                              {formatValue(item.fallbackValue)}
                            </span>
                            <span className="flex justify-end min-w-0">
                              {item.source === 'flow_fallback' ? (
                                <button
                                  type="button"
                                  onClick={() =>
                                    void persistEnvData(false, {
                                      ...(baseEnvData?.variables ?? {}),
                                      [item.key]: item.fallbackValue,
                                    })
                                  }
                                  className="text-[11px] text-primary hover:underline cursor-pointer whitespace-nowrap"
                                >
                                  {ll.copyToEnv()}
                                </button>
                              ) : (
                                <button
                                  type="button"
                                  title={ll.syncToFallback()}
                                  onClick={() => {
                                    updateInitialVariables({
                                      ...(flow?.initial_variables ?? {}),
                                      [item.key]: item.effectiveValue,
                                    });
                                    toast.success(ll.syncToFallback());
                                  }}
                                  className="text-[11px] text-muted-foreground hover:text-foreground cursor-pointer flex items-center gap-1 whitespace-nowrap"
                                >
                                  <ArrowRightLeft className="w-3 h-3 shrink-0" />
                                  <span className="truncate">{ll.syncToFallback()}</span>
                                </button>
                              )}
                            </span>
                          </div>
                        ))}

                        {comparisonItems.length === 0 && (
                          <div className="py-10 text-center text-xs text-muted-foreground/60">
                            {ll.noComparisonData()}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ) : (
                  /* 键值编辑（主环境 / 本地覆盖） */
                  <div className="flex-1 flex flex-col mt-2 overflow-hidden">
                    <VariableEditorList
                      variables={
                        (activeSubTab === 'local' ? localEnvData?.variables : baseEnvData?.variables) ?? {}
                      }
                      onAdd={(key, value) =>
                        void persistEnvData(activeSubTab === 'local', {
                          ...((activeSubTab === 'local' ? localEnvData?.variables : baseEnvData?.variables) ?? {}),
                          [key]: value,
                        })
                      }
                      onUpdate={(key, value) =>
                        void persistEnvData(activeSubTab === 'local', {
                          ...((activeSubTab === 'local' ? localEnvData?.variables : baseEnvData?.variables) ?? {}),
                          [key]: value,
                        })
                      }
                      onRemove={(key) => {
                        const current =
                          (activeSubTab === 'local' ? localEnvData?.variables : baseEnvData?.variables) ?? {};
                        const { [key]: _removed, ...rest } = current;
                        void persistEnvData(activeSubTab === 'local', rest);
                      }}
                    />
                  </div>
                )}
              </div>
            ) : (
              <div className="flex-1 flex items-center justify-center text-xs text-muted-foreground/60 px-6 text-center">
                {ll.selectOrCreateEnv()}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
