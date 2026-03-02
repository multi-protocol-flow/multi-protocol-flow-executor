/**
 * CanvasToolbar - 画布悬浮工具栏
 * 位于画布顶部居中，包含执行按钮等操作
 */
import { useState } from 'react';
import { Play, Loader2, Bug } from 'lucide-react';
import { useFlowStore, useUIStore, useExecutionStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { executeFlowWithChannel, saveFlowReport } from '@/services/tauri';
import { cn } from '@/lib/utils';
import type { LogEvent } from '@/types/events';

/**
 * 画布悬浮工具栏组件
 */
export function CanvasToolbar() {
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
  const isDisabled = !hasValidFlow || !hasEntryNode || isExecuting;

  return (
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10 flex items-center gap-3 bg-white/90 backdrop-blur-sm rounded-full px-3 py-2 shadow-lg border border-slate-200/50">
      {/* 执行按钮 */}
      <button
        onClick={handleExecute}
        disabled={isDisabled}
        title={isExecuting ? '执行中...' : '执行流程'}
        className={cn(
          'flex items-center justify-center',
          'transition-all duration-150',
          // 正常状态：实心红色播放图标
          !isDisabled && [
            'text-red-500 hover:text-red-600',
            'hover:scale-110',
            'cursor-pointer',
          ],
          // 禁用状态
          isDisabled && [
            'text-slate-300 cursor-not-allowed',
          ]
        )}
      >
        {isExecuting ? (
          <Loader2 className="w-6 h-6 animate-spin" />
        ) : (
          <Play className="w-6 h-6" fill="currentColor" stroke="none" />
        )}
      </button>

      {/* 调试按钮（暂不可用） */}
      <span
        title="调试"
        className="text-green-500 cursor-not-allowed"
      >
        <Bug className="w-6 h-6" />
      </span>
    </div>
  );
}
