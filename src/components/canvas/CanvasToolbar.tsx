/**
 * CanvasToolbar - 画布悬浮工具栏
 * 位于画布顶部居中，包含执行按钮等操作
 */
import { useState, useMemo } from 'react';
import { Play, Square, Bug, Activity, Variable as VariableIcon, X, Save } from 'lucide-react';
import { toast } from 'sonner';
import { useFlowStore, useUIStore, useExecutionStore, useDebugStore, selectIsDebugging } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { useWsStreamStore } from '@/stores/wsStream';
import { useStressTestStore, selectIsStressTestRunning } from '@/stores/stressTest';
import { executeFlowWithChannel, saveFlowReport } from '@/services/tauri';
import { flowStorage } from '@/services/storage';
import { cn } from '@/lib/utils';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';
import { DebugToolbar } from './DebugToolbar';
import { StressTestConfigDialog } from '@/components/stress-test/StressTestConfigDialog';
import { StressTestMonitor } from '@/components/stress-test/StressTestMonitor';

import { VariablePanel, extractVariableGroups } from '@/components/panels/VariablePanel';

/**
 * 画布悬浮工具栏组件
 */
export function CanvasToolbar() {
  const [isExecuting, setIsExecuting] = useState(false);
  const flow = useFlowStore((state) => state.flow);
  const isDirty = useFlowStore((state) => state.isDirty);
  const markSaved = useFlowStore((state) => state.markSaved);
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

  const handleSave = async () => {
    if (!flow) return;
    try {
      await flowStorage.saveFlow(flow);
      markSaved();
      toast.success('保存成功', { description: flow.name });
    } catch (err) {
      toast.error('保存失败', { description: (err as Error).message });
    }
  };

  const handleExecute = async () => {
    if (!flow || isExecuting) return;

    setIsExecuting(true);
    setConfigPanelOpen(false);

    // 清空上次增量执行状态，打开底部面板并切换到运行标签
    useExecutionStore.getState().clearExecutionState();
    useUIStore.setState({ isHistoryPanelOpen: true, bottomPanelTab: 'running' });

    // 清理残留调试数据
    useDebugStore.getState().clearDebugData();

    // 每次执行前清空日志
    clearLogs();

    // 日志回调函数
    const handleLog = (event: LogEvent) => {
      addLog(event);
    };

    // 执行事件回调函数（实时节点报告）
    const handleExecutionEvent = (event: ExecutionEvent) => {
      useExecutionStore.getState().handleExecutionEvent(event);
    };

    // WS 流事件回调函数（实时 WS 消息流）
    const handleWsEvent = (event: import('@/types/debug').DebugEvent) => {
      useWsStreamStore.getState().handleWsEvent(event);
    };

    try {
      const result = await executeFlowWithChannel(flow, handleLog, handleExecutionEvent, handleWsEvent);

      if (result.success && result.flow_report) {
        setReport(result.flow_report);

        // 显式保存报告到后端（分离关注点：UI 显示和数据持久化分开）
        try {
          await saveFlowReport(result.flow_report, reportDir);
        } catch (saveError) {
          // 保存失败不影响 UI 显示，但提示用户
          console.error('保存报告失败:', saveError);
          toast.warning('报告保存失败', {
            description: `${(saveError as Error).message}，报告仅在当前会话可见`,
            duration: 8000,
          });
        }

        toast.success('流程执行完成', {
          description: `总耗时: ${result.flow_report.duration_ms}ms | 成功节点: ${result.flow_report.statistics.success_count}/${result.flow_report.total_nodes}`,
          duration: 10000,
          action: {
            label: '查看报告',
            onClick: () => useExecutionStore.getState().openReportDialog(),
          },
        });
      } else {
        setError(result.error || '执行失败');
        toast.error('流程执行失败', {
          description: result.error || '未知错误',
          duration: 10000,
        });
      }
    } catch (error) {
      setError((error as Error).message);
      toast.error('执行异常', {
        description: (error as Error).message,
      });
    } finally {
      setIsExecuting(false);
    }
  };

  const handleStartDebug = async () => {
    if (!flow) return;

    // 找到入口节点
    const entryNode = flow.nodes.find((n) => n.type === 'entry');
    if (!entryNode) return;

    // 收集所有有断点的节点，并确保入口节点始终在断点列表中
    const breakpointUuids = flow.nodes
      .filter((n) => n.hasBreakpoint === true)
      .map((n) => n.uuid);

    const breakpoints = breakpointUuids.includes(entryNode.uuid)
      ? breakpointUuids
      : [entryNode.uuid, ...breakpointUuids];

    // 启动调试时自动打开右侧面板以显示调试面板
    setConfigPanelOpen(true);
    await startDebugSession(flow, breakpoints);
  };

  const hasValidFlow = flow && flow.nodes.length > 0;
  const hasEntryNode = flow?.nodes.some((n) => n.type === 'entry');
  // 压测运行时禁用执行和调试
  const isDisabled = !hasValidFlow || !hasEntryNode || isExecuting || isStressTestRunning;

  // 调试按钮不再依赖用户手动设置断点（入口节点默认作为断点）
  // 压测运行时禁用调试按钮
  const canStartDebug = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning;
  // 执行或调试时禁用压测按钮
  const canStartStressTest = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning;

  // 变量抽屉状态
  const isVariableDrawerOpen = useUIStore((s) => s.isVariableDrawerOpen);
  const toggleVariableDrawer = useUIStore((s) => s.toggleVariableDrawer);
  const variableGroups = useMemo(() => flow ? extractVariableGroups(flow) : [], [flow]);
  const hasVariables = variableGroups.length > 0;
  const totalVariables = useMemo(
    () => variableGroups.reduce((sum, g) => sum + g.variables.length, 0),
    [variableGroups]
  );

  // 调试模式下显示 DebugToolbar
  if (isDebugging) {
    return <DebugToolbar />;
  }

  // 工具栏内容
  const toolbarContent = (
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10 flex items-start gap-2">
      {/* 主工具栏 */}
      <div className="flex items-center gap-3 bg-background/90 backdrop-blur-sm rounded-full px-3 py-2 shadow-lg border border-border/50 dark:border-white/20">
        {/* 保存按钮 */}
        <button
          type="button"
          onClick={handleSave}
          disabled={!flow || !isDirty}
          title={!flow ? '无流程' : isDirty ? '保存流程 (Ctrl+S)' : '已保存'}
          className={cn(
            'flex items-center justify-center',
            'transition-all duration-150',
            flow && isDirty && [
              'text-amber-500 hover:text-amber-600',
              'hover:scale-110',
              'cursor-pointer',
            ],
            !flow || !isDirty && [
              'text-muted-foreground cursor-not-allowed',
            ]
          )}
        >
          <Save className="w-5 h-5" />
        </button>

        {/* 分隔线 */}
        <div className="w-px h-5 bg-border/50" />

        {/* 执行/停止按钮 */}
        {isExecuting ? (
          <button
            type="button"
            onClick={() => useExecutionStore.getState().cancelExecution()}
            title="停止执行"
            className="flex items-center justify-center transition-all duration-150 text-red-600 hover:text-red-700 hover:scale-110 cursor-pointer"
          >
            <Square className="w-6 h-6" fill="currentColor" />
          </button>
        ) : (
          <button
            type="button"
            onClick={handleExecute}
            disabled={isDisabled}
            title="执行流程"
            className={cn(
              'flex items-center justify-center',
              'transition-all duration-150',
              !isDisabled && [
                'text-red-500 hover:text-red-600',
                'hover:scale-110',
                'cursor-pointer',
              ],
              isDisabled && [
                'text-muted-foreground cursor-not-allowed',
              ]
            )}
          >
            <Play className="w-6 h-6" fill="currentColor" stroke="none" />
          </button>
        )}

        {/* 调试按钮 */}
        <button
          type="button"
          onClick={handleStartDebug}
          disabled={!canStartDebug}
          title={
            isExecuting
              ? '调试（执行中不可用）'
              : isStressTestRunning
              ? '调试（压测进行中不可用）'
              : '开始调试'
          }
          className={cn(
            'flex items-center justify-center',
            'transition-all duration-150',
            canStartDebug && [
              'text-green-500 hover:text-green-600',
              'hover:scale-110',
              'cursor-pointer',
            ],
            !canStartDebug && [
              'text-muted-foreground cursor-not-allowed',
            ]
          )}
        >
          <Bug className="w-6 h-6" />
        </button>

        {/* 压测按钮 */}
        <button
          type="button"
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
            canStartStressTest && [
              'text-blue-500 hover:text-blue-600',
              'hover:scale-110',
              'cursor-pointer',
            ],
            !canStartStressTest && [
              'text-muted-foreground cursor-not-allowed',
            ]
          )}
        >
          <Activity className="w-6 h-6" />
        </button>
      </div>

      {/* 变量按钮 - 独立元素，紧贴工具栏右侧 */}
      {hasVariables && (
        <div className="relative">
          <button
            type="button"
            onClick={toggleVariableDrawer}
            className={cn(
              'flex items-center justify-center',
              'w-10 h-10 rounded-full',
              'bg-background/90 backdrop-blur-sm shadow-lg border border-border/50 dark:border-white/20',
              'transition-all duration-150 hover:shadow-lg',
              'cursor-pointer',
              isVariableDrawerOpen
                ? 'text-violet-600 border-violet-200 dark:border-violet-500/50'
                : 'text-muted-foreground hover:text-foreground'
            )}
            title={isVariableDrawerOpen ? '收起变量面板' : `变量 (${totalVariables})`}
          >
            <div className="relative">
              <VariableIcon className="w-6 h-6" />
              <span className="absolute -top-1.5 -right-2 min-w-[14px] h-[14px] flex items-center justify-center rounded-full bg-violet-100 text-violet-600 dark:bg-violet-900/40 dark:text-violet-400 text-[9px] font-bold leading-none px-0.5">
                {totalVariables}
              </span>
            </div>
          </button>

          {/* 变量抽屉面板 */}
          {isVariableDrawerOpen && (
            <div className={cn(
              'absolute top-full mt-2 left-1/2 -translate-x-1/2',
              'w-72 max-h-[calc(100vh-200px)]',
              'bg-background/95 backdrop-blur-xl rounded-xl',
              'shadow-xl border border-border/50 dark:border-white/20',
              'overflow-hidden',
              'animate-in fade-in-0 zoom-in-95 duration-200'
            )}>
              {/* 标题栏 + 关闭按钮 */}
              <div className="flex items-center justify-between px-3 py-2 border-b border-border">
                <div className="flex items-center gap-2">
                  <VariableIcon className="w-4 h-4 text-violet-600" />
                  <span className="text-sm font-semibold text-foreground">变量</span>
                  <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
                    {totalVariables}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={toggleVariableDrawer}
                  className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                  title="收起"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>
              {/* 变量面板内容 */}
              <div className="h-[calc(100%-40px)] overflow-hidden">
                <VariablePanel flow={flow!} nodes={flow!.nodes} showHeader={false} />
              </div>
            </div>
          )}
        </div>
      )}
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
    </>
  );
}
