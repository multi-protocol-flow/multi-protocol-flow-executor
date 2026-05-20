/**
 * VariableTreeView - 变量树形查看器
 * 从 DebugPanel 提取的变量显示组件
 */
import { useState } from 'react';
import { ChevronDown, ChevronRight, Copy, Edit2, Check, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

interface VariableTreeViewProps {
  /** 变量数据 */
  variables: Record<string, unknown>;
  /** 是否允许编辑 */
  canEdit?: boolean;
  /** 编辑回调 */
  onEdit?: (name: string, value: unknown) => void;
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

export function VariableTreeView({ variables, canEdit, onEdit }: VariableTreeViewProps) {
  const editHandler = canEdit ? onEdit : undefined;

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
