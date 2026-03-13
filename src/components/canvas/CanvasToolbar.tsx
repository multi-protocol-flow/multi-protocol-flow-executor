/**
 * CanvasToolbar - 画布悬浮工具栏
 * 位于画布顶部居中，包含执行按钮等操作
 */
import { useState } from 'react';
import { Play, Loader2, Bug, Activity } from 'lucide-react';
import { useFlowStore, useUIStore, useExecutionStore, useDebugStore, selectIsDebugging } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { useStressTestStore, selectIsStressTestRunning } from '@/stores/stressTest';
import { executeFlowWithChannel, saveFlowReport } from '@/services/tauri';
import { cn } from '@/lib/utils';
import type { LogEvent } from '@/types/events';
import { DebugToolbar } from './DebugToolbar';
import { StressTestConfigDialog } from '@/components/stress-test/StressTestConfigDialog';
import { StressTestMonitor } from '@/components/stress-test/StressTestMonitor';
import { StressTestResult } from '@/components/stress-test/StressTestResult';

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
  const isDebugging = useDebugStore(selectIsDebugging);
  const startDebugSession = useDebugStore((state) => state.startDebugSession);
  
  // 压力测试状态
  const isStressTestRunning = useStressTestStore(selectIsStressTestRunning);
  const showStressTestConfigDialog = useUIStore((state) => state.showStressTestConfigDialog);
  const openStressTestConfigDialog = useUIStore((state) => state.openStressTestConfigDialog);
  const closeStressTestConfigDialog = useUIStore((state) => state.closeStressTestConfigDialog);

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

  const handleStartDebug = async () => {
    if (!flow) return;

    // 收集所有有断点的节点
    const breakpoints = flow.nodes
      .filter((n) => n.hasBreakpoint === true)
      .map((n) => n.uuid);

    if (breakpoints.length === 0) {
      return;
    }

    // 启动调试时自动打开右侧面板以显示调试面板
    setConfigPanelOpen(true);
    await startDebugSession(flow, breakpoints);
  };

  const hasValidFlow = flow && flow.nodes.length > 0;
  const hasEntryNode = flow?.nodes.some((n) => n.type === 'entry');
  // 压测运行时禁用执行和调试
  const isDisabled = !hasValidFlow || !hasEntryNode || isExecuting || isStressTestRunning;

  // 收集断点
  const breakpoints = flow?.nodes.filter((n) => n.hasBreakpoint === true) || [];
  const hasBreakpoints = breakpoints.length > 0;
  // 压测运行时禁用调试
  const canStartDebug = hasValidFlow && hasEntryNode && hasBreakpoints && !isExecuting && !isDebugging && !isStressTestRunning;
  // 执行或调试时禁用压测按钮
  const canStartStressTest = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning;

  // 调试模式下显示 DebugToolbar
  if (isDebugging) {
    return <DebugToolbar />;
  }

  // 工具栏内容
  const toolbarContent = (
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

      {/* 调试按钮 */}
      <button
        onClick={handleStartDebug}
        disabled={!canStartDebug}
        title={
          !hasBreakpoints
            ? '调试（需要至少设置一个断点）'
            : isExecuting
            ? '调试（执行中不可用）'
            : isStressTestRunning
            ? '调试（压测进行中不可用）'
            : '开始调试'
        }
        className={cn(
          'flex items-center justify-center',
          'transition-all duration-150',
          // 正常状态：绿色虫子图标
          canStartDebug && [
            'text-green-500 hover:text-green-600',
            'hover:scale-110',
            'cursor-pointer',
          ],
          // 禁用状态
          !canStartDebug && [
            'text-slate-300 cursor-not-allowed',
          ]
        )}
      >
        <Bug className="w-6 h-6" />
      </button>

      {/* 压测按钮 */}
      <button
        onClick={openStressTestConfigDialog}
        disabled={!canStartStressTest}
        title={
          isExecuting
            ? '压测（执行中不可用）'
            : isDebugging
            ? '压测（调试中不可用）'
            : isStressTestRunning
            ? '压测（已运行）'
            : '压力测试'
        }
        className={cn(
          'flex items-center justify-center',
          'transition-all duration-150',
          // 正常状态：蓝色活动图标
          canStartStressTest && [
            'text-blue-500 hover:text-blue-600',
            'hover:scale-110',
            'cursor-pointer',
          ],
          // 禁用状态
          !canStartStressTest && [
            'text-slate-300 cursor-not-allowed',
          ]
        )}
      >
        <Activity className="w-6 h-6" />
      </button>
    </div>
  );

  return (
    <>
      {toolbarContent}
      
      {/* 压力测试配置对话框 */}
      <StressTestConfigDialog
        open={showStressTestConfigDialog}
        onOpenChange={(open) => !open && closeStressTestConfigDialog()}
      />
      
      {/* 压力测试监控面板（仅在运行时显示） */}
      <StressTestMonitor />
      
      {/* 压力测试结果（仅在完成时显示） */}
      <StressTestResult />
    </>
  );
}
