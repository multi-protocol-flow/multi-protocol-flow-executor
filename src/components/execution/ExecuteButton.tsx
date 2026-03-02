/**
 * 执行按钮组件
 * 点击后执行当前流程，显示执行状态
 */
import { useState } from 'react';
import { Play, Loader2, Zap } from 'lucide-react';
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
  const reportDir = useSettingsStore((state) => state.reportDir);

  const handleExecute = async () => {
    if (!flow || isExecuting) return;

    setIsExecuting(true);
    setConfigPanelOpen(false);

    // 每次执行前清空日志
    clearLogs();

    // 日志回调函数
    const handleLog = (event: LogEvent) => {
      addLog(event);
    };

    try {
      const result = await executeFlowWithChannel(flow, handleLog);

      if (result.success && result.flow_report) {
        setReport(result.flow_report);

        // 显式保存报告到后端（分离关注点：UI 显示和数据持久化分开）
        try {
          await saveFlowReport(result.flow_report, reportDir);
        } catch (saveError) {
          // 保存失败不影响 UI 显示，只记录日志
          console.error('保存报告失败:', saveError);
        }
      } else {
        setError(result.error || '执行失败');
      }
    } catch (error) {
      setError((error as Error).message);
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
        'bg-white border-2 border-black',
        'hover:bg-slate-50 hover:border-black',
        'active:bg-slate-100',
        'text-black font-medium',
        'rounded-2xl',
        'disabled:bg-slate-100 disabled:border-slate-300',
        'disabled:text-slate-400',
        'transition-all duration-200'
      )}
    >
      {/* 背景动画效果 */}
      <div
        className={cn(
          'absolute inset-0 bg-slate-100',
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
