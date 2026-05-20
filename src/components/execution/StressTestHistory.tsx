/**
 * 压测历史列表组件
 * 显示历史压测报告记录，支持选择查看报告
 */
import { useState, useEffect, useCallback } from 'react';
import { toast } from 'sonner';
import { Clock, History, Loader2, Trash2, BarChart3 } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { useStressTestStore, selectStressTestHistory } from '@/stores/stressTest';
import { stressTestStorage } from '@/services/storage/stressTestStorage';
import { useUIStore } from '@/stores';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import type { StressTestReport } from '@/types/stressTest';

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

export function StressTestHistory() {
  const ll = getLL();
  const history = useStressTestStore(selectStressTestHistory);
  const loadHistory = useStressTestStore((state) => state.loadHistory);
  const openReportDialog = useStressTestStore((state) => state.openReportDialog);
  const showConfirm = useUIStore((state) => state.showConfirm);

  const [loading, setLoading] = useState(false);
  const [localHistory, setLocalHistory] = useState(history);

  // 加载历史列表
  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        await loadHistory();
      } catch (error) {
        console.error(ll.execution.history.toast_load_failed(), error);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [loadHistory]);

  // 同步 store history 到本地状态
  useEffect(() => {
    setLocalHistory(history);
  }, [history]);

  // 点击加载完整报告
  const handleSelectReport = useCallback(async (item: { uuid: string; filePath: string }) => {
    try {
      const report = await stressTestStorage.loadReport(item.filePath);
      // 将报告写入 store 并打开弹窗，同时恢复进度历史数据以支持图表渲染
      useStressTestStore.setState({
        report: report as StressTestReport,
        progressHistory: (report as StressTestReport).progressHistory ?? [],
      });
      openReportDialog();
    } catch (error) {
      console.error(ll.execution.history.toast_load_report_failed(), error);
      toast.error(ll.execution.history.toast_load_report_failed(), {
        description: error instanceof Error ? error.message : ll.execution.history.toast_unknown_error(),
      });
    }
  }, [openReportDialog]);

  // 删除单个报告（带确认）
  const handleDelete = useCallback((item: { uuid: string; filePath: string }) => {
    showConfirm({
      title: ll.execution.history.delete_dialog_stress_title(),
      description: ll.execution.history.delete_dialog_stress_description(),
      confirmText: ll.execution.history.delete(),
      onConfirm: async () => {
        try {
          await stressTestStorage.deleteReport(item.filePath);
          toast.success(ll.execution.history.toast_report_deleted());
          // 重新加载历史列表
          await loadHistory();
        } catch (error) {
          console.error(ll.execution.history.toast_delete_failed(), error);
          toast.error(ll.execution.history.toast_delete_failed(), {
            description: error instanceof Error ? error.message : ll.execution.history.toast_unknown_error(),
          });
        }
      },
    });
  }, [loadHistory, showConfirm]);

  // 加载中状态（初次加载）
  if (loading && localHistory.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-muted-foreground">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">{ll.execution.history.loading()}</p>
      </div>
    );
  }

  // 空状态
  if (localHistory.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-muted-foreground">
        <History className="w-10 h-10 mb-3 opacity-40" />
        <p className="text-sm">{ll.execution.history.empty_stress_title()}</p>
        <p className="text-xs mt-1 opacity-70">{ll.execution.history.empty_stress_subtitle()}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 头部 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
<span className="text-xs text-muted-foreground">
           {ll.execution.history.record_count({ count: localHistory.length })}
         </span>
      </div>

      {/* 历史列表 */}
      <ScrollArea className="flex-1">
        <div className="space-y-1 p-2">
          {localHistory.map((item) => {
            return (
              <div
                key={item.uuid}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleSelectReport(item);
                  }
                }}
                onClick={() => handleSelectReport(item)}
                className={cn(
                  'group flex items-center justify-between p-2.5 rounded-lg w-full',
                  'border border-transparent hover:border-border',
                  'hover:bg-accent cursor-pointer transition-all duration-150 text-left'
                )}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  {/* 图标 */}
                  <div className="shrink-0 w-7 h-7 rounded-full flex items-center justify-center bg-blue-100">
                    <BarChart3 className="w-3.5 h-3.5 text-blue-600" />
                  </div>

                  {/* 流程信息 */}
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <p className="text-sm font-medium text-foreground truncate">
                        {item.flowName}
                      </p>
<span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-xs font-medium bg-violet-100 text-violet-700">
                         {ll.execution.history.badge_stress_test()}
                       </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
                      <Clock className="w-3 h-3" />
                      <span>{formatTimestamp(item.startTime)}</span>
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
                      handleDelete(item);
                    }}
                    className={cn(
                      'w-8 h-8 opacity-0 group-hover:opacity-100',
                      'transition-opacity duration-150 text-muted-foreground hover:text-red-500'
                    )}
                    title={ll.execution.history.delete()}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
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
