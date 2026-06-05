/**
 * VariableTreeView - 变量树形查看器
 * 从 DebugPanel 提取的变量显示组件
 * 支持监视模式：执行过程中实时显示变量值变化
 */
import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { ChevronDown, ChevronRight, Copy, Edit2, Check, X, Eye, EyeOff, Pin, PinOff } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { useExecutionStore } from '@/stores/execution';

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
}

function VariableItem({
  name,
  value,
  depth = 0,
  onEdit,
}: {
  name: string;
  value: unknown;
  depth?: number;
  onEdit?: (name: string, value: unknown) => void;
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
          'flex items-center gap-2 px-2 py-1.5 group',
          'hover:bg-accent transition-colors'
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
              title="复制"
            >
              <Copy className="w-3 h-3" />
            </button>
            {onEdit && !isComplex && (
              <button
                onClick={handleStartEdit}
                className="w-6 h-6 flex items-center justify-center text-muted-foreground hover:text-accent-foreground hover:bg-accent rounded"
                title="编辑"
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
              title="保存"
            >
              <Check className="w-3 h-3" />
            </button>
            <button
              onClick={handleCancelEdit}
              className="w-6 h-6 flex items-center justify-center text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/50 rounded"
              title="取消"
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
            placeholder="输入 JSON 格式的值"
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

export function VariableTreeView({ variables, canEdit, onEdit, watchMode = false }: VariableTreeViewProps) {
  const editHandler = canEdit ? onEdit : undefined;
  const nodeReportsMap = useExecutionStore((s) => s.nodeReportsMap);
  const isExecuting = useExecutionStore((s) => s.isExecuting);

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
        const existing = changes.get(key);
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
          Object.entries(variables).map(([name, value]) => (
            <VariableItem
              key={name}
              name={name}
              value={value}
              onEdit={editHandler}
            />
          ))
        ) : (
          <div className="p-4 text-sm text-muted-foreground text-center">暂无变量</div>
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
              placeholder="搜索变量..."
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
          <div className="p-4 text-sm text-muted-foreground text-center">未找到匹配变量</div>
        ) : (
          <div className="p-4 text-sm text-muted-foreground text-center">暂无变量</div>
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
        title={isPinned ? '取消固定' : '固定到顶部'}
      >
        {isPinned ? <PinOff className="w-3 h-3" /> : <Pin className="w-3 h-3" />}
      </button>

      <span className="text-xs font-medium text-foreground min-w-0 shrink-0">{name}</span>
      <span className="text-xs text-muted-foreground font-mono flex-1 truncate">{simpleDisplay}</span>

      {onEdit && !isObject && (
        <button
          type="button"
          className="w-4 h-4 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-foreground"
          title="编辑"
          onClick={() => onEdit(name, value)}
        >
          <Edit2 className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}
