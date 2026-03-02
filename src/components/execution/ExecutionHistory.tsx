/**
 * 执行历史列表组件
 * 显示历史执行记录，支持选择查看报告
 */
import { toast } from 'sonner';
import { useState, useEffect, useCallback } from 'react';
import { CheckCircle2, XCircle, RotateCcw, Clock, History, Loader2, Trash2 } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { loadReport, listReports, deleteReport } from '@/services/storage/reportStorage';
import type { ReportMetadata } from '@/types/ipc';
import { useExecutionStore } from '@/stores/execution';
import { useSettingsStore } from '@/stores/settings';
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
  /** 选择报告回调（点击历史项时触发） */
  onSelectReport?: (reportUuid: string) => void;
  /** 重新执行回调 */
  onRerun?: (report: ReportMetadata) => void;
}

export function ExecutionHistory({ onSelectReport, onRerun }: ExecutionHistoryProps) {
  const [history, setHistory] = useState<ReportMetadata[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingReportUuid, setLoadingReportUuid] = useState<string | null>(null);
  const setReport = useExecutionStore((state) => state.setReport);
  const reportDir = useSettingsStore((state) => state.reportDir);

  // 加载历史列表
  useEffect(() => {
    const loadHistory = async () => {
      setLoading(true);
      try {
        const reports = await listReports(reportDir);
        setHistory(reports);
      } catch (error) {
        console.error('加载历史列表失败:', error);
        toast.error('加载历史列表失败', {
          description: error instanceof Error ? error.message : '未知错误',
        });
      } finally {
        setLoading(false);
      }
    };
    loadHistory();
  }, []);

  // 点击加载完整报告
  const handleSelectReport = useCallback(async (reportUuid: string) => {
    setLoadingReportUuid(reportUuid);
    try {
      const report = await loadReport(reportUuid, reportDir);
      setReport(report);
      onSelectReport?.(reportUuid);
    } catch (error) {
      console.error('加载报告失败:', error);
      toast.error('加载报告失败', {
        description: error instanceof Error ? error.message : '未知错误',
      });
    } finally {
      setLoadingReportUuid(null);
    }
  }, [setReport, onSelectReport]);

  // 删除单个报告
  const handleDelete = useCallback(async (uuid: string) => {
    try {
      await deleteReport(uuid, reportDir);
      setHistory((prev) => prev.filter((item) => item.uuid !== uuid));
      toast.success('报告已删除');
    } catch (error) {
      console.error('删除报告失败:', error);
      const errorMessage = error instanceof Error ? error.message : '未知错误';
      if (errorMessage.includes('not found') || errorMessage.includes('不存在')) {
        toast.error('报告文件已删除', {
          description: '该报告文件可能已被手动删除',
        });
      } else if (errorMessage.includes('permission') || errorMessage.includes('权限')) {
        toast.error('删除报告失败', {
          description: '没有足够的权限删除该文件',
        });
      } else {
        toast.error('删除报告失败', {
          description: errorMessage,
        });
      }
    }
  }, []);

  // 清空所有历史
  const handleClearHistory = useCallback(async () => {
    if (window.confirm('确定要清空所有执行历史吗？')) {
      try {
        await Promise.all(history.map((item) => deleteReport(item.uuid, reportDir)));
        setHistory([]);
        toast.success('执行历史已清空');
      } catch (error) {
        console.error('清空执行历史失败:', error);
        toast.error('清空执行历史失败', {
          description: error instanceof Error ? error.message : '未知错误',
        });
      }
    }
  }, [history]);

  // 加载中状态（初次加载）
  if (loading && history.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-slate-400">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">加载中...</p>
      </div>
    );
  }

  // 空状态
  if (history.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-slate-400">
        <History className="w-10 h-10 mb-3 opacity-40" />
        <p className="text-sm">暂无执行历史</p>
        <p className="text-xs mt-1 opacity-70">执行流程后记录将显示在这里</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 头部 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
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
            const isLoading = loadingReportUuid === item.uuid;

            return (
              <div
                key={item.uuid}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleSelectReport(item.uuid);
                  }
                }}
                onClick={() => handleSelectReport(item.uuid)}
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

                {/* 操作按钮 */}
                <div className="flex items-center gap-1 shrink-0">
                  {/* 删除按钮 */}
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(item.uuid);
                    }}
                    className={cn(
                      'w-8 h-8 opacity-0 group-hover:opacity-100',
                      'transition-opacity duration-150 text-slate-400 hover:text-red-500'
                    )}
                    title="删除"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </Button>

                  {/* 重新执行按钮 */}
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => {
                      e.stopPropagation();
                      onRerun?.(item);
                    }}
                    disabled={isLoading}
                    className={cn(
                      'w-8 h-8 opacity-0 group-hover:opacity-100',
                      'transition-opacity duration-150'
                    )}
                    title="重新执行"
                  >
                    {isLoading ? (
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    ) : (
                      <RotateCcw className="w-3.5 h-3.5" />
                    )}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
