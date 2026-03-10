/**
 * VariableChangesCard - 变量变更卡片组件
 * 显示节点执行期间修改变量的记录
 * 
 * 简化格式：变量名 → 当前值 → 旧值（可选）
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Variable,
} from 'lucide-react';
import type { VariableChange } from '@/types';
import { cn } from '@/lib/utils';
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

export function VariableChangesCard({ changes, defaultOpen = false }: VariableChangesCardProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  // 无变更时不渲染
  if (!changes || changes.length === 0) {
    return null;
  }

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      {/* 标题栏 */}
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full flex items-center gap-2 p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
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
        <span className="font-medium text-foreground text-sm">变量变更</span>
        <span className="ml-auto text-xs text-muted-foreground">
          {changes.length} 项
        </span>
      </button>

      {/* 变更列表 */}
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
                      ? 'bg-emerald-50/50 border-emerald-200' 
                      : 'bg-amber-50/50 border-amber-200'
                  )}
                >
                  {/* 变量名 */}
                  <div className="flex items-center gap-2 mb-2">
                    <code className="font-mono text-slate-700 font-medium">
                      {change.key}
                    </code>
                    <span className={cn(
                      'text-xs px-1.5 py-0.5 rounded',
                      isNewVariable 
                        ? 'bg-emerald-100 text-emerald-700' 
                        : 'bg-amber-100 text-amber-700'
                    )}>
                      {isNewVariable ? '新增' : '更新'}
                    </span>
                  </div>

                  {/* 简化显示：当前值 → 旧值（可选） */}
                  <div className="flex items-center gap-3 text-sm">
                    {/* 当前值 */}
                    <div className="flex-1 min-w-0">
                      <div className="text-xs text-muted-foreground mb-0.5">当前值</div>
                      {typeof change.new_value === 'object' && change.new_value !== null ? (
                        <div className="bg-white/60 rounded p-1.5 max-h-32 overflow-auto">
                          <RawJsonViewer data={change.new_value} />
                        </div>
                      ) : (
                        <code className="font-mono text-slate-700 text-xs">
                          {formatValue(change.new_value)}
                        </code>
                      )}
                    </div>

                    {/* 旧值（仅更新时显示） */}
                    {!isNewVariable && (
                      <>
                        <span className="text-muted-foreground shrink-0">←</span>
                        <div className="flex-1 min-w-0">
                          <div className="text-xs text-muted-foreground mb-0.5">旧值</div>
                          {typeof change.old_value === 'object' && change.old_value !== null ? (
                            <div className="bg-white/60 rounded p-1.5 max-h-32 overflow-auto">
                              <RawJsonViewer data={change.old_value} />
                            </div>
                          ) : (
                            <code className="font-mono text-slate-500 text-xs line-through">
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
}
