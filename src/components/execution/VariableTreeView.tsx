/**
 * VariableTreeView - 变量树形查看器
 * 从 DebugPanel 提取的变量显示组件
 * 支持监视模式：执行过程中实时显示变量值变化
 */
import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { ChevronDown, ChevronRight, Copy, Edit2, Check, X, Pin, PinOff, History } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { useExecutionStore } from '@/stores/execution';
import { useDebugStore, selectVariableDiff } from '@/stores/debug';

/** 变量变化记录 */
interface VariableChange {
  name: string;
  newValue: unknown;
  timestamp: number;
  nodeUuid: string;
  animating: boolean;
}

interface VariableTreeViewProps {
  /** 变量数据 */
  variables: Record<string, unknown>;
  /** 是否允许编辑 */
  canEdit?: boolean;
  /** 编辑回调 */
  onEdit?: (name: string, value: unknown) => void;
  /** 是否启用监视模式（实时显示执行中的变量变化） */
  watchMode?: boolean;
  /** 打开变量时间轴回调 */
  onOpenTimeline?: (name: string) => void;
}

function VariableItem({
  name,
  value,
  depth = 0,
  onEdit,
  diffType,
  diffOldVal,
  onOpenTimeline,
}: {
  name: string;
  value: unknown;
  depth?: number;
  onEdit?: (name: string, value: unknown) => void;
  diffType?: 'added' | 'modified';
  diffOldVal?: unknown;
  onOpenTimeline?: () => void;
}) {
  const ll = getLL();
  const [isExpanded, setIsExpanded] = useState(depth < 1);
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState('');

  const isObject = value !== null && typeof value === 'object' && !Array.isArray(value);
  const isArray = Array.isArray(value);
  const isComplex = isObject || isArray;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(value, null, 2));
      toast.success(ll.execution.report.copied_to_clipboard());
    } catch {
      toast.error(ll.execution.report.copy_failed());
    }
  };

  const handleStartEdit = () => {
    setEditValue(JSON.stringify(value, null, 2));
    setIsEditing(true);
  };

  const handleSaveEdit = () => {
    try {
      const parsed = JSON.parse(editValue);
      onEdit?.(name, parsed);
      setIsEditing(false);
      toast.success(ll.execution.toast.variable_modified());
    } catch {
      toast.error(ll.flow.skip_json_error());
    }
  };

  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditValue('');
  };

  const getSimpleDisplay = () => {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (typeof value === 'string') return `"${value}"`;
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'number') return String(value);
    return String(value);
  };

  return (
    <div className="border-b border-border last:border-b-0">
      <div
        className={cn(
          'flex items-center gap-2 px-2 py-1.5 group transition-colors',
          diffType === 'added' && 'bg-green-500/10 hover:bg-green-500/15 border-l-2 border-l-green-500',
          diffType === 'modified' && 'bg-amber-500/10 hover:bg-amber-500/15 border-l-2 border-l-amber-500',
          !diffType && 'hover:bg-accent'
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
      >
        {isComplex ? (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="w-4 h-4 flex items-center justify-center text-muted-foreground hover:text-accent-foreground"
          >
            {isExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
          </button>
        ) : (
          <div className="w-4 h-4" />
        )}

        <span className="text-sm font-medium text-foreground min-w-0 flex-shrink-0">
          {name}
        </span>

        {diffType === 'added' && (
          <span
            className="px-1.5 py-0.5 text-[10px] font-mono font-bold text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-950/80 rounded border border-green-300 dark:border-green-800"
            title={ll.flow.variable_tree_diff_added_title()}
          >
            [+]
          </span>
        )}
        {diffType === 'modified' && (
          <span
            className="px-1.5 py-0.5 text-[10px] font-mono font-bold text-amber-700 dark:text-amber-300 bg-amber-100 dark:bg-amber-950/80 rounded border border-amber-300 dark:border-amber-800 cursor-help"
            title={ll.flow.variable_tree_diff_modified_title({
              oldVal: JSON.stringify(diffOldVal, null, 2),
              newVal: JSON.stringify(value, null, 2),
            })}
          >
            [~]
          </span>
        )}

        {!isComplex && !isEditing && (
          <span className="text-sm text-muted-foreground font-mono flex-1 truncate">
            {getSimpleDisplay()}
          </span>
        )}

        {isComplex && !isExpanded && (
          <span className="text-sm text-muted-foreground flex-1 truncate">
            {isObject ? '{...}' : `[${(value as unknown[]).length}]`}
          </span>
        )}

        {!isEditing && (
          <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              onClick={handleCopy}
              className="w-6 h-6 flex items-center justify-center text-muted-foreground hover:text-accent-foreground hover:bg-accent rounded"
              title={ll.execution.viewers.variable_tree_copy()}
            >
              <Copy className="w-3 h-3" />
            </button>
            {onOpenTimeline && (
              <button
                onClick={onOpenTimeline}
                className="w-6 h-6 flex items-center justify-center text-muted-foreground hover:text-accent-foreground hover:bg-accent rounded"
                title={ll.flow.variable_timeline_open()}
              >
                <History className="w-3 h-3 text-teal-500" />
              </button>
            )}
            {onEdit && !isComplex && (
              <button
                onClick={handleStartEdit}
                className="w-6 h-6 flex items-center justify-center text-muted-foreground hover:text-accent-foreground hover:bg-accent rounded"
                title={ll.execution.viewers.variable_tree_edit()}
              >
                <Edit2 className="w-3 h-3" />
              </button>
            )}
          </div>
        )}

        {isEditing && (
          <>
            <button
              onClick={handleSaveEdit}
              className="w-6 h-6 flex items-center justify-center text-green-600 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-950/50 rounded"
              title={ll.execution.viewers.variable_tree_save()}
            >
              <Check className="w-3 h-3" />
            </button>
            <button
              onClick={handleCancelEdit}
              className="w-6 h-6 flex items-center justify-center text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/50 rounded"
              title={ll.flow.cancel()}
            >
              <X className="w-3 h-3" />
            </button>
          </>
        )}
      </div>

      {isEditing && (
        <div className="px-2 pb-2" style={{ paddingLeft: `${depth * 16 + 8}px` }}>
          <textarea
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            className="w-full h-24 px-2 py-1 text-xs font-mono border border-border rounded resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder={ll.execution.viewers.variable_tree_placeholder()}
          />
        </div>
      )}

      {isComplex && isExpanded && !isEditing && (
        <div className="bg-muted/50">
          {isObject &&
            Object.entries(value as Record<string, unknown>).map(([key, val]) => (
              <VariableItem
                key={key}
                name={key}
                value={val}
                depth={depth + 1}
                onEdit={onEdit ? (_, newVal) => {
                  const updated = { ...(value as Record<string, unknown>) };
                  updated[name] = newVal;
                  onEdit(name, updated);
                } : undefined}
              />
            ))}
          {isArray &&
            (value as unknown[]).map((item, idx) => (
              <VariableItem
                key={idx}
                name={`[${idx}]`}
                value={item}
                depth={depth + 1}
                onEdit={onEdit ? (_, newVal) => {
                  const updated = [...(value as unknown[])];
                  updated[idx] = newVal;
                  onEdit(name, updated);
                } : undefined}
              />
            ))}
        </div>
      )}
    </div>
  );
}

export function VariableTreeView({ variables, canEdit, onEdit, watchMode = false, onOpenTimeline }: VariableTreeViewProps) {
  const ll = getLL();
  const editHandler = canEdit ? onEdit : undefined;
  const isExecuting = useExecutionStore((s) => s.isExecuting);
  const varDiff = useDebugStore(selectVariableDiff);
  // 监视模式：追踪变量变化
  const [variableChanges, setVariableChanges] = useState<Map<string, VariableChange>>(new Map());
  const [pinnedVars, setPinnedVars] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const prevVariablesRef = useRef<Record<string, unknown>>(variables);

  // 监视模式：检测变量变化
  useEffect(() => {
    if (!watchMode || !isExecuting) return;

    const prev = prevVariablesRef.current;
    const changes = new Map(variableChanges);
    let hasChange = false;

    for (const [key, newValue] of Object.entries(variables)) {
      if (JSON.stringify(prev[key]) !== JSON.stringify(newValue)) {
        changes.set(key, {
          name: key,
          newValue,
          timestamp: Date.now(),
          nodeUuid: useExecutionStore.getState().currentNodeUuid || '',
          animating: true,
        });
        hasChange = true;

        // 300ms 后清除动画状态
        setTimeout(() => {
          setVariableChanges((prevMap) => {
            const next = new Map(prevMap);
            const entry = next.get(key);
            if (entry) {
              next.set(key, { ...entry, animating: false });
            }
            return next;
          });
        }, 300);
      }
    }

    if (hasChange) {
      setVariableChanges(changes);
    }
    prevVariablesRef.current = variables;
  }, [variables, watchMode, isExecuting]);

  // 过滤 + 排序变量列表
  const displayEntries = useMemo(() => {
    const entries = Object.entries(variables);
    let filtered = entries;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = entries.filter(([name]) => name.toLowerCase().includes(q));
    }

    // 固定变量置顶
    filtered.sort(([a], [b]) => {
      const aPinned = pinnedVars.has(a) ? 0 : 1;
      const bPinned = pinnedVars.has(b) ? 0 : 1;
      return aPinned - bPinned;
    });

    return filtered;
  }, [variables, searchQuery, pinnedVars]);

  const togglePin = useCallback((name: string) => {
    setPinnedVars((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }, []);

  if (!watchMode) {
    return (
      <div className="h-full overflow-y-auto">
        {Object.keys(variables).length > 0 ? (
          Object.entries(variables).map(([name, value]) => {
            const diffType = varDiff.added.includes(name)
              ? 'added'
              : name in varDiff.modified
              ? 'modified'
              : undefined;
            const diffOldVal = varDiff.modified[name]?.oldVal;

            return (
              <VariableItem
                key={name}
                name={name}
                value={value}
                diffType={diffType}
                diffOldVal={diffOldVal}
                onOpenTimeline={onOpenTimeline ? () => onOpenTimeline(name) : undefined}
                onEdit={editHandler}
              />
            );
          })
        ) : (
          <div className="p-4 text-sm text-muted-foreground text-center">{ll.execution.viewers.variable_tree_empty()}</div>
        )}
      </div>
    );
  }

  // 监视模式渲染
  return (
    <div className="h-full flex flex-col">
      {/* 搜索框 */}
      {watchMode && (
        <div className="px-2 py-1.5 border-b border-border">
          <div className="relative">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={ll.execution.viewers.variable_tree_search_placeholder()}
              className="w-full h-7 pl-2 pr-2 text-xs rounded-md border border-border bg-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {displayEntries.length > 0 ? (
          displayEntries.map(([name, value]) => {
            const change = variableChanges.get(name);
            const isChanged = change !== undefined;
            const isPinned = pinnedVars.has(name);
            const animating = isChanged && change.animating;

            return (
              <WatchVariableItem
                key={name}
                name={name}
                value={value}
                isChanged={isChanged}
                animating={animating}
                isPinned={isPinned}
                onTogglePin={() => togglePin(name)}
                onEdit={editHandler}
              />
            );
          })
        ) : searchQuery ? (
          <div className="p-4 text-sm text-muted-foreground text-center">{ll.execution.viewers.variable_tree_no_match()}</div>
        ) : (
          <div className="p-4 text-sm text-muted-foreground text-center">{ll.execution.viewers.variable_tree_empty()}</div>
        )}
      </div>
    </div>
  );
}

/** 监视模式下的变量项（带变化高亮和 Pin） */
function WatchVariableItem({
  name,
  value,
  isChanged,
  animating,
  isPinned,
  onTogglePin,
  onEdit,
}: {
  name: string;
  value: unknown;
  isChanged: boolean;
  animating: boolean;
  isPinned: boolean;
  onTogglePin: () => void;
  onEdit?: (name: string, value: unknown) => void;
}) {
  const ll = getLL();
  const isObject = value !== null && typeof value === 'object';
  const simpleDisplay = (() => {
    if (value === null) return 'null';
    if (typeof value === 'string') return `"${value.slice(0, 50)}"`;
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'number') return String(value);
    if (Array.isArray(value)) return `[${value.length}]`;
    return '{...}';
  })();

  return (
    <div
      className={cn(
        'flex items-center gap-1.5 px-2 py-1.5 border-b border-border last:border-b-0 group transition-colors',
        animating && 'animate-pulse bg-green-100 dark:bg-green-900/30',
        isChanged && !animating && 'bg-green-50 dark:bg-green-950/20',
      )}
    >
      {/* Pin 按钮 */}
      <button
        type="button"
        onClick={onTogglePin}
        className="w-4 h-4 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-amber-500"
        title={isPinned ? ll.execution.viewers.variable_tree_unpin() : ll.execution.viewers.variable_tree_pin()}
      >
        {isPinned ? <PinOff className="w-3 h-3" /> : <Pin className="w-3 h-3" />}
      </button>

      <span className="text-xs font-medium text-foreground min-w-0 shrink-0">{name}</span>
      <span className="text-xs text-muted-foreground font-mono flex-1 truncate">{simpleDisplay}</span>

      {onEdit && !isObject && (
        <button
          type="button"
          className="w-4 h-4 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-foreground"
          title={ll.execution.viewers.variable_tree_edit()}
          onClick={() => onEdit(name, value)}
        >
          <Edit2 className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}
