/**
 * 执行按钮组件
 * 点击后执行当前流程，显示执行状态
 */
import { useState } from 'react';
import { Play, Loader2, Zap } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { useFlowStore, useUIStore, useExecutionStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { executeFlowWithChannel, saveFlowReport } from '@/services/tauri';
import { cn } from '@/lib/utils';
import type { LogEvent } from '@/types/events';

export function ExecuteButton() {
  const [isExecuting, setIsExecuting] = useState(false);
  const flow = useFlowStore((state) => state.flow);
  const setReport = useExecutionStore((state) => state.setReport);
  const setError = useExecutionStore((state) => state.setError);
  const addLog = useConsoleStore((state) => state.addLog);
  const clearLogs = useConsoleStore((state) => state.clearLogs);
  const setConfigPanelOpen = useUIStore((state) => state.setConfigPanelOpen);
  const clearExecutionState = useExecutionStore((state) => state.clearExecutionState);
  const reportDir = useSettingsStore((state) => state.reportDir);

  const handleExecute = async () => {
    if (!flow || isExecuting) return;

    setIsExecuting(true);
    setConfigPanelOpen(false);

    // 清空上次增量执行状态，打开底部面板并切换到运行标签
    clearExecutionState();
    useUIStore.setState({ isHistoryPanelOpen: true, bottomPanelTab: 'running' });

    // 每次执行前清空日志
    clearLogs();

    // 日志回调函数
    const handleLog = (event: LogEvent) => {
      addLog(event);
    };

    // 执行事件回调函数（实时节点报告）
    const handleExecutionEvent = (event: import('@/types/execution_event').ExecutionEvent) => {
      useExecutionStore.getState().handleExecutionEvent(event);
    };

    // WS 流事件回调函数（实时 WS 消息流）
    const handleWsEvent = (event: import('@/types/debug').DebugEvent) => {
      useExecutionStore.getState().handleWsEvent(event);
    };

    try {
      const result = await executeFlowWithChannel(flow, handleLog, handleExecutionEvent, handleWsEvent);

      if (result.success && result.flow_report) {
        setReport(result.flow_report);

        // 显式保存报告到后端（分离关注点：UI 显示和数据持久化分开）
        try {
          await saveFlowReport(result.flow_report, reportDir);
        } catch (saveError) {
          // 保存失败不影响 UI 显示，只记录日志
          console.error('保存报告失败:', saveError);
        }

        toast.success('执行完成', {
          description: `${result.flow_report.flow_name} · 耗时 ${result.flow_report.duration_ms}ms · ${result.flow_report.statistics.success_count} 成功 · ${result.flow_report.statistics.failed_count} 失败`,
          action: { label: '查看报告', onClick: () => useExecutionStore.getState().openReportDialog() },
          duration: 10000,
        });
      } else {
        setError(result.error || '执行失败');
        toast.error('执行失败', {
          description: result.error || '未知错误',
          duration: 10000,
        });
      }
    } catch (error) {
      setError((error as Error).message);
      toast.error('执行失败', {
        description: (error as Error).message,
        duration: 10000,
      });
    } finally {
      setIsExecuting(false);
    }
  };

  const hasValidFlow = flow && flow.nodes.length > 0;
  const hasEntryNode = flow?.nodes.some((n) => n.type === 'entry');

  return (
    <Button
      variant="ghost"
      onClick={handleExecute}
      disabled={!hasValidFlow || !hasEntryNode || isExecuting}
      className={cn(
        'relative overflow-hidden gap-2 min-w-[100px] h-9',
        'bg-background border-2 border-border',
        'hover:bg-accent hover:border-border',
        'active:bg-accent',
        'text-foreground font-medium',
        'rounded-2xl',
        'disabled:bg-muted disabled:border-border',
        'disabled:text-muted-foreground',
        'transition-all duration-200'
      )}
    >
      {/* 背景动画效果 */}
      <div
        className={cn(
          'absolute inset-0 bg-muted',
          'translate-x-[-100%]',
          isExecuting && 'animate-shimmer'
        )}
      />

      {isExecuting ? (
        <>
          <Loader2 className="w-4 h-4 animate-spin" />
          <span className="relative z-10">执行中</span>
        </>
      ) : (
        <>
          <Zap className="w-4 h-4" />
          <span className="relative z-10">执行流程</span>
          <Play className="w-3 h-3 ml-1 opacity-70" />
        </>
      )}
    </Button>
  );
}
