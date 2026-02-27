/**
 * 执行历史列表组件
 * 显示历史执行记录，支持选择和重新执行
 */
import { useMemo } from 'react';
import { CheckCircle2, XCircle, RotateCcw, Clock, History } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import {
  listExecutionHistory,
  clearExecutionHistory,
  type ExecutionHistoryItem,
} from '@/services/storage/historyStorage';
import { cn } from '@/lib/utils';

/** 格式化时间戳为可读字符串 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const isToday = date.toDateString() === now.toDateString();

  if (isToday) {
    return date.toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  return date.toLocaleDateString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** 格式化耗时 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

interface ExecutionHistoryProps {
  /** 选择流程回调 */
  onSelectFlow?: (flowUuid: string) => void;
  /** 重新执行回调 */
  onRerun?: (historyItem: ExecutionHistoryItem) => void;
}

export function ExecutionHistory({ onSelectFlow, onRerun }: ExecutionHistoryProps) {
  const history = useMemo(() => listExecutionHistory(), []);

  if (history.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-slate-400">
        <History className="w-10 h-10 mb-3 opacity-40" />
        <p className="text-sm">暂无执行历史</p>
        <p className="text-xs mt-1 opacity-70">执行流程后记录将显示在这里</p>
      </div>
    );
  }

  const handleClearHistory = () => {
    if (window.confirm('确定要清空所有执行历史吗？')) {
      clearExecutionHistory();
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* 头部 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-[#e0e0e0]">
        <span className="text-xs text-slate-500">
          共 {history.length} 条记录
        </span>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleClearHistory}
          className="h-7 text-xs text-slate-500 hover:text-red-500"
        >
          清空
        </Button>
      </div>

      {/* 历史列表 */}
      <ScrollArea className="flex-1">
        <div className="space-y-1 p-2">
          {history.map((item) => {
            const isSuccess = item.status === 'success';

            return (
              <button
                type="button"
                key={item.uuid}
                onClick={() => onSelectFlow?.(item.flow_uuid)}
                className={cn(
                  'group flex items-center justify-between p-2.5 rounded-lg w-full',
                  'border border-transparent hover:border-slate-200',
                  'hover:bg-slate-50 cursor-pointer transition-all duration-150 text-left'
                )}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  {/* 状态图标 */}
                  <div
                    className={cn(
                      'shrink-0 w-7 h-7 rounded-full flex items-center justify-center',
                      isSuccess
                        ? 'bg-emerald-100'
                        : 'bg-red-100'
                    )}
                  >
                    {isSuccess ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                    ) : (
                      <XCircle className="w-3.5 h-3.5 text-red-600" />
                    )}
                  </div>

                  {/* 流程信息 */}
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-slate-700 truncate">
                      {item.flow_name}
                    </p>
                    <div className="flex items-center gap-2 text-xs text-slate-400 mt-0.5">
                      <Clock className="w-3 h-3" />
                      <span>{formatTimestamp(item.timestamp)}</span>
                      <span className="opacity-50">·</span>
                      <span>{formatDuration(item.duration_ms)}</span>
                    </div>
                  </div>
                </div>

                {/* 重新执行按钮 */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRerun?.(item);
                  }}
                  className={cn(
                    'shrink-0 w-8 h-8 opacity-0 group-hover:opacity-100',
                    'transition-opacity duration-150'
                  )}
                  title="重新执行"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                </Button>
              </button>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
