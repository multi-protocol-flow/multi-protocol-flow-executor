/**
 * VariableChangesCard - 变量变更卡片组件
 * 显示节点执行期间修改变量的记录
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Plus,
  ArrowRight,
} from 'lucide-react';
import type { VariableChange } from '@/types';
import { Badge } from '@/components/ui/badge';
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

/**
 * 获取变更类型标签
 */
function getChangeType(change: VariableChange): 'added' | 'updated' | 'deleted' {
  if (isEmptyValue(change.old_value) && !isEmptyValue(change.new_value)) {
    return 'added';
  }
  if (!isEmptyValue(change.old_value) && isEmptyValue(change.new_value)) {
    return 'deleted';
  }
  return 'updated';
}

export function VariableChangesCard({ changes, defaultOpen = false }: VariableChangesCardProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  // 无变更时不渲染
  if (!changes || changes.length === 0) {
    return null;
  }

  const addedCount = changes.filter(c => getChangeType(c) === 'added').length;
  const updatedCount = changes.filter(c => getChangeType(c) === 'updated').length;

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
        <RefreshCw className="w-4 h-4 text-violet-500" />
        <span className="font-medium text-foreground text-sm">变量变更</span>
        
        {/* 变更统计 */}
        <div className="flex items-center gap-1.5 ml-auto">
          {addedCount > 0 && (
            <Badge variant="outline" className="text-xs bg-emerald-50 text-emerald-700 border-emerald-200">
              <Plus className="w-3 h-3 mr-1" />
              {addedCount} 新增
            </Badge>
          )}
          {updatedCount > 0 && (
            <Badge variant="outline" className="text-xs bg-amber-50 text-amber-700 border-amber-200">
              <RefreshCw className="w-3 h-3 mr-1" />
              {updatedCount} 更新
            </Badge>
          )}
        </div>
      </button>

      {/* 变更列表 */}
      {isOpen && (
        <div className="p-3 border-t border-border bg-background">
          <div className="space-y-2">
            {changes.map((change, index) => {
              const changeType = getChangeType(change);
              
              return (
                <div
                  key={`${change.key}-${index}`}
                  className={cn(
                    'rounded-lg p-2.5 text-sm',
                    changeType === 'added' && 'bg-emerald-50/50 border border-emerald-100',
                    changeType === 'updated' && 'bg-amber-50/50 border border-amber-100',
                    changeType === 'deleted' && 'bg-red-50/50 border border-red-100'
                  )}
                >
                  {/* 变量名 */}
                  <div className="flex items-center gap-2 mb-1.5">
                    <code className="font-mono text-slate-700 font-medium">
                      {change.key}
                    </code>
                    <Badge
                      variant="outline"
                      className={cn(
                        'text-xs',
                        changeType === 'added' && 'bg-emerald-100 text-emerald-700 border-emerald-200',
                        changeType === 'updated' && 'bg-amber-100 text-amber-700 border-amber-200',
                        changeType === 'deleted' && 'bg-red-100 text-red-700 border-red-200'
                      )}
                    >
                      {changeType === 'added' && '新增'}
                      {changeType === 'updated' && '更新'}
                      {changeType === 'deleted' && '删除'}
                    </Badge>
                  </div>

                  {/* 变更内容 */}
                  {changeType === 'added' ? (
                    // 新增：只显示新值
                    <div className="pl-2 border-l-2 border-emerald-200">
                      <div className="text-xs text-muted-foreground mb-1">新值</div>
                      <div className="bg-white/60 rounded p-2">
                        {typeof change.new_value === 'object' ? (
                          <RawJsonViewer data={change.new_value} />
                        ) : (
                          <code className="font-mono text-slate-700">
                            {formatValue(change.new_value)}
                          </code>
                        )}
                      </div>
                    </div>
                  ) : changeType === 'updated' ? (
                    // 更新：显示旧值 → 新值
                    <div className="space-y-1.5">
                      <div className="pl-2 border-l-2 border-slate-200">
                        <div className="text-xs text-muted-foreground mb-1">旧值</div>
                        <div className="bg-white/60 rounded p-2">
                          {typeof change.old_value === 'object' ? (
                            <RawJsonViewer data={change.old_value} />
                          ) : (
                            <code className="font-mono text-slate-500 line-through">
                              {formatValue(change.old_value)}
                            </code>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center justify-center">
                        <ArrowRight className="w-4 h-4 text-amber-500" />
                      </div>
                      <div className="pl-2 border-l-2 border-amber-200">
                        <div className="text-xs text-muted-foreground mb-1">新值</div>
                        <div className="bg-white/60 rounded p-2">
                          {typeof change.new_value === 'object' ? (
                            <RawJsonViewer data={change.new_value} />
                          ) : (
                            <code className="font-mono text-slate-700">
                              {formatValue(change.new_value)}
                            </code>
                          )}
                        </div>
                      </div>
                    </div>
                  ) : (
                    // 删除：只显示旧值
                    <div className="pl-2 border-l-2 border-red-200">
                      <div className="text-xs text-muted-foreground mb-1">已删除的值</div>
                      <div className="bg-white/60 rounded p-2">
                        {typeof change.old_value === 'object' ? (
                          <RawJsonViewer data={change.old_value} />
                        ) : (
                          <code className="font-mono text-slate-500 line-through">
                            {formatValue(change.old_value)}
                          </code>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
