/**
 * VariableChangesCard - 变量变更卡片组件
 * 显示节点执行期间修改变量的记录
 * 
 * 简化格式：变量名 → 当前值 → 旧值（可选）
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Variable,
} from 'lucide-react';
import type { VariableChange } from '@/types';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { CopyButton } from '@/components/common';
import { RawJsonViewer } from './RawJsonViewer';
interface VariableChangesCardProps {
  /** 变更记录列表 */
  changes: VariableChange[] | undefined | null;
  /** 是否默认展开 */
  defaultOpen?: boolean;
}

/**
 * 判断值是否为空
 */
function isEmptyValue(value: unknown): boolean {
  return value === null || value === undefined;
}

/**
 * 格式化值用于显示
 */
function formatValue(value: unknown): string {
  if (value === null) return 'null';
  if (value === undefined) return 'undefined';
  if (typeof value === 'string') {
    // 字符串截断
    return value.length > 50 ? `${value.slice(0, 50)}...` : value;
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

export const VariableChangesCard = memo(function VariableChangesCard({ changes, defaultOpen = false }: VariableChangesCardProps) {
  const ll = getLL();
  const [isOpen, setIsOpen] = useState(defaultOpen);

  // 无变更时不渲染
  if (!changes || changes.length === 0) {
    return null;
  }

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      {/* 标题栏 */}
      <div className="flex items-center justify-between w-full bg-muted/30 hover:bg-muted/50 transition-colors">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground">
            {isOpen ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </div>
          <Variable className="w-4 h-4 text-violet-500" />
          <span className="font-medium text-foreground text-sm">{ll.execution.viewers.variable_changes()}</span>
        </button>
        <div className="flex items-center gap-2 pr-3">
          <CopyButton
            value={changes}
            format="json"
            label={ll.execution.report.copy_all()}
            size="xs"
          />
          <span className="text-xs text-muted-foreground">
            {changes.length}
          </span>
        </div>
      </div>
      {isOpen && (
        <div className="p-3 border-t border-border bg-background">
          <div className="space-y-2">
            {changes.map((change, index) => {
              const isNewVariable = isEmptyValue(change.old_value);
              
              return (
                <div
                  key={`${change.key}-${index}`}
                  className={cn(
                    'rounded-lg p-2.5 text-sm border',
                    isNewVariable 
                      ? 'bg-emerald-50/50 dark:bg-emerald-950/30 border-emerald-200 dark:border-emerald-800' 
                      : 'bg-amber-50/50 dark:bg-amber-950/30 border-amber-200 dark:border-amber-800'
                  )}
                >
                  {/* 变量名 */}
                  <div className="flex items-center gap-2 mb-2">
                    <div className="flex items-center gap-1">
                      <code className="font-mono text-foreground font-medium select-text">
                        {change.key}
                      </code>
                      <CopyButton
                        value={change.key}
                        title={ll.execution.viewers.copy_var_name()}
                        size="xs"
                        className="h-5 p-0.5"
                      />
                    </div>
                    <span className={cn(
                      'text-xs px-1.5 py-0.5 rounded',
                      isNewVariable 
                        ? 'bg-emerald-100 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300' 
                        : 'bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-300'
                    )}>
                      {isNewVariable ? ll.execution.viewers.var_created() : ll.execution.viewers.var_updated()}
                    </span>
                  </div>
                  {/* 简化显示：当前值 → 旧值（可选） */}
                  <div className="flex items-center gap-3 text-sm">
                    {/* 当前值 */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between mb-0.5">
                        <span className="text-xs text-muted-foreground">{ll.execution.viewers.var_new_value()}</span>
                        <CopyButton
                          value={change.new_value}
                          format="json"
                          title={ll.execution.viewers.copy_new_value()}
                          size="xs"
                          className="h-5 p-0.5"
                        />
                      </div>
                      {typeof change.new_value === 'object' && change.new_value !== null ? (
                        <div className="bg-background/60 rounded p-1.5 max-h-32 overflow-auto">
                          <RawJsonViewer data={change.new_value} />
                        </div>
                      ) : (
                        <code className="font-mono text-foreground text-xs select-text cursor-text">
                          {formatValue(change.new_value)}
                        </code>
                      )}
                    </div>

                    {/* 旧值（仅更新时显示） */}
                    {!isNewVariable && (
                      <>
                        <span className="text-muted-foreground shrink-0">←</span>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center justify-between mb-0.5">
                            <span className="text-xs text-muted-foreground">{ll.execution.viewers.var_old_value()}</span>
                            <CopyButton
                              value={change.old_value}
                              format="json"
                              title={ll.execution.viewers.copy_old_value()}
                              size="xs"
                              className="h-5 p-0.5"
                            />
                          </div>
                          {typeof change.old_value === 'object' && change.old_value !== null ? (
                            <div className="bg-background/60 rounded p-1.5 max-h-32 overflow-auto">
                              <RawJsonViewer data={change.old_value} />
                            </div>
                          ) : (
                            <code className="font-mono text-muted-foreground text-xs line-through select-text cursor-text">
                              {formatValue(change.old_value)}
                            </code>
                          )}
                        </div>
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
});
