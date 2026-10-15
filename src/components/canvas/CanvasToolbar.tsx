/**
 * CanvasToolbar - 画布悬浮工具栏
 * 位于画布顶部居中，包含撤销/重做、执行按钮、变量按钮
 */
import { useState, useMemo, useEffect, lazy, Suspense } from 'react';
import { Play, Square, Bug, Activity, Variable as VariableIcon, X, Undo2, Redo2, Database, Plus, Radio } from 'lucide-react';
import { toast } from 'sonner';
import * as Popover from '@radix-ui/react-popover';
import {
  useFlowStore,
  useUIStore,
  useExecutionStore,
  useDebugStore,
  selectIsDebugging,
  selectCanUndo,
  selectCanRedo,
  selectFlow,
  selectNodes,
  getAllNodes,
  selectIsExecuting,
} from '@/stores';
import { notifyError } from '@/lib/notify';
import { useConsoleStore } from '@/stores/console';
import { useSettingsStore } from '@/stores/settings';
import { useWsStreamStore } from '@/stores/wsStream';
import { useStressTestStore, selectIsStressTestRunning } from '@/stores/stressTest';
import { executeFlowWithChannel, validateFlow } from '@/services/tauri';
import { startIngressListener, stopIngressListener, getActiveIngressPorts } from '@/services/tauri/flow';
import { isHttpIngressConfig, isEntryNodeType } from '@/types';
import { cn } from '@/lib/utils';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';
import { ValidationErrorDialog } from '@/components/dialogs/ValidationErrorDialog';
import { DebugToolbar } from './DebugToolbar';
import { useDatasetExecutionStore } from '@/stores/datasetExecution';

import { EnvironmentSelector } from '@/components/panels/EnvironmentManager';
import { useEnvironmentStore } from '@/stores/environment';
import { VariablePanel, extractVariableGroups } from '@/components/panels/VariablePanel';
import { getLL } from '@/i18n/i18n-lazy';
import { hotkeyLabel } from '@/lib/hotkeyLabels';

const StressTestConfigDialog = lazy(() =>
  import('@/components/stress-test/StressTestConfigDialog').then((m) => ({ default: m.StressTestConfigDialog }))
);
const DatasetExecutionPanel = lazy(() =>
  import('@/components/execution/DatasetExecutionPanel').then((m) => ({ default: m.DatasetExecutionPanel }))
);

/**
 * 画布悬浮工具栏组件
 */
export function CanvasToolbar() {
  const ll = getLL();
  const isGlobalExecuting = useExecutionStore(selectIsExecuting);
  const [localExecuting, setLocalExecuting] = useState(false);
  const isExecuting = localExecuting || isGlobalExecuting;
  const flow = useFlowStore(selectFlow);
  const hasEnvironments = useEnvironmentStore((s) => s.environments.length > 0);
  // 验证错误对话框状态
  const [validationErrors, setValidationErrors] = useState<{ code: string; message: string }[]>([]);
  const [showValidationDialog, setShowValidationDialog] = useState(false);
  const undo = useFlowStore((state) => state.undo);
  const redo = useFlowStore((state) => state.redo);
  const canUndo = useFlowStore(selectCanUndo);
  const canRedo = useFlowStore(selectCanRedo);
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

  // 数据驱动执行状态
  const showDatasetExecutionPanel = useUIStore((state) => state.showDatasetExecutionPanel);
  const openDatasetExecutionPanel = useUIStore((state) => state.openDatasetExecutionPanel);
  const closeDatasetExecutionPanel = useUIStore((state) => state.closeDatasetExecutionPanel);
  // Ingress 流程级 Mock 监听状态
  const ingressNode = flow?.nodes.find((n) => n.type === 'http:ingress');
  const configuredIngressPort = ingressNode && isHttpIngressConfig(ingressNode.data)
    ? ingressNode.data.port
    : 9527;
  // 后端“实际在监听”的端口。必须记住它而不是只记布尔值：用户监听后改端口时，
  // 旧监听仍在运行，此时停止必须停旧端口，否则会漏掉一个常驻监听。
  const [listeningPort, setListeningPort] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    getActiveIngressPorts()
      .then((ports) => {
        if (cancelled) return;
        setListeningPort((prev) => {
          if (ports.includes(configuredIngressPort)) return configuredIngressPort;
          // 端口被改动时旧监听仍在运行：保留原有在听端口，不要误显示为“未监听”
          if (prev !== null && ports.includes(prev)) return prev;
          return null;
        });
      })
      .catch(() => {
        if (!cancelled) setListeningPort(null);
      });
    return () => {
      cancelled = true;
    };
  }, [configuredIngressPort]);

  const handleToggleIngress = async () => {
    if (!flow || !ingressNode) return;
    try {
      if (listeningPort !== null) {
        await stopIngressListener(listeningPort);
        toast.success(ll.flow.ingress_listener_stopped({ port: listeningPort }));
        setListeningPort(null);
      } else {
        const port = configuredIngressPort;
        // 流程含断点时以调试模式启动：外部请求挂起等待人工单步，直到 http:respond 执行
        const useDebug = flow.nodes.some((n) => n.hasBreakpoint === true);
        const replacedFlowName = await startIngressListener(
          port,
          flow,
          useDebug,
          (event) => useDebugStore.getState().handleDebugEvent(event),
          (logEvent) => useConsoleStore.getState().addLog(logEvent)
        );
        setListeningPort(port);
        if (replacedFlowName && replacedFlowName !== flow.name) {
          toast.success(
            ll.flow.ingress_listener_switched({
              oldFlow: replacedFlowName,
              port,
            })
          );
        } else {
          toast.success(ll.flow.ingress_listener_started({ port }));
        }
      }
    } catch (err) {
      toast.error(String(err));
    }
  };
  const isDatasetExecuting = useDatasetExecutionStore((s) => s.isExecuting);

  // 是否有活跃的执行/调试/压测/数据驱动
  const isAnyActive = isExecuting || isDebugging || isStressTestRunning || isDatasetExecuting;

  const handleExecute = async () => {
    if (!flow || isExecuting) return;

    // 执行前验证流程结构
    try {
      const validation = await validateFlow(flow);
      if (!validation.valid && validation.errors.length > 0) {
        setValidationErrors(validation.errors);
        setShowValidationDialog(true);
        setLocalExecuting(false);
        return;
      }
    } catch {
      // 验证调用失败（如非 Tauri 环境）继续执行，后端会在执行时再次验证
    }

    setLocalExecuting(true);
    setConfigPanelOpen(false);

    // 清空上次增量执行状态，打开底部面板并切换到运行标签
    useExecutionStore.getState().clearExecutionState();
    useUIStore.setState({ isBottomPanelExpanded: true, bottomPanelTab: 'running' });

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
      const result = await executeFlowWithChannel(
        flow,
        handleLog,
        handleExecutionEvent,
        handleWsEvent,
        undefined,
        reportDir
      );

      if (result.flow_report) {
        setReport(result.flow_report);
        const report = result.flow_report;
        if (result.success) {
          toast.success(ll.execution.toast.flow_completed(), {
            description: `${ll.execution.report.total_duration()}: ${report.duration_ms ?? 0}ms | ${ll.execution.toast.success_nodes()}: ${report.statistics?.success_count ?? 0}/${report.total_nodes ?? 0}`,
            duration: 10000,
            action: {
              label: ll.execution.toast.view_report(),
              onClick: () => useExecutionStore.getState().openReportDialog(),
            },
          });
        } else {
          setError(result.error || ll.execution.viewers.execution_failed());
          toast.error(ll.execution.toast.flow_failed(), {
            description: `${ll.execution.report.total_duration()}: ${report.duration_ms ?? 0}ms | ${ll.execution.toast.success_nodes()}: ${report.statistics?.success_count ?? 0}/${report.total_nodes ?? 0}`,
            action: {
              label: ll.execution.toast.view_report(),
              onClick: () => useExecutionStore.getState().openReportDialog(),
            },
            duration: 10000,
          });
        }
      } else {
        setError(result.error || ll.execution.viewers.execution_failed());
        toast.error(ll.execution.toast.flow_failed(), {
          description: result.error || ll.execution.toast.unknown_error(),
          duration: 10000,
        });
      }
    } catch (error) {
      const errorMsg = (error as Error).message;
      setError(errorMsg);
      notifyError(ll.execution.toast.flow_exception(), error, {
        category: 'Execution',
        flowUuid: flow?.uuid,
      });
    } finally {
      useConsoleStore.getState().flushLogs();
      setLocalExecuting(false);
    }
  };

  const handleStartDebug = async () => {
    if (!flow) return;

    // 找到入口节点
    const entryNode = flow.nodes.find((n) => isEntryNodeType(n.type));
    if (!entryNode) return;

    // 收集所有三阶段中有断点的节点
    const allNodes = getAllNodes(flow);
    const breakpointUuids = allNodes
      .filter((n) => n.hasBreakpoint === true)
      .map((n) => n.uuid);

    // 计算调试起始停靠节点：
    // 若 setup 存在且有节点，调试从 setup 起始节点开始执行，默认应停留在 setup 起始节点；
    // 否则停留在 main 的 entry 节点。
    const setupNodes = flow.setup?.nodes ?? [];
    const setupConnections = flow.setup?.connections ?? [];
    let initialStartNodeUuid: string = entryNode.uuid;

    if (setupNodes.length > 0) {
      const targetUuids = new Set(setupConnections.map((c) => c.target_node_uuid));
      const rootNodes = setupNodes.filter((n) => !targetUuids.has(n.uuid));
      const setupStartNode = rootNodes[0] ?? setupNodes[0];
      if (setupStartNode) {
        initialStartNodeUuid = setupStartNode.uuid;
        // 自动将画布切到 Setup 阶段，让用户立刻看到起始断点
        useFlowStore.getState().setActiveStage('setup');
      }
    }

    const breakpointsSet = new Set(breakpointUuids);
    breakpointsSet.add(entryNode.uuid);
    breakpointsSet.add(initialStartNodeUuid);
    const breakpoints = Array.from(breakpointsSet);

    // 启动调试时自动打开右侧面板以显示调试面板
    setConfigPanelOpen(true);
    await startDebugSession(flow, breakpoints);
  };

  const nodes = useFlowStore(selectNodes);
  const hasValidFlow = nodes.length > 0;
  const hasEntryNode = nodes.some((n) => isEntryNodeType(n.type));
  // 压测运行时禁用执行和调试
  const isDisabled = !hasValidFlow || !hasEntryNode || isExecuting || isStressTestRunning || isDatasetExecuting;

  // 调试按钮不再依赖用户手动设置断点（入口节点默认作为断点）
  // 压测运行时禁用调试按钮
  const canStartDebug = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning && !isDatasetExecuting;
  // 执行或调试时禁用压测按钮
  const canStartStressTest = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning && !isDatasetExecuting;
  // 数据驱动按钮：执行/调试/压测时禁用
  const canStartDatasetExecution = hasValidFlow && hasEntryNode && !isExecuting && !isDebugging && !isStressTestRunning && !isDatasetExecuting;

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
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10 flex items-center gap-2">
      {/* 主工具栏 */}
      <div className="flex items-center gap-3 bg-background/90 backdrop-blur-sm rounded-full px-3 py-2 shadow-lg border border-border/50 dark:border-white/20">
        {/* 环境选择器 */}
        {!isAnyActive && <EnvironmentSelector />}

        {/* 分隔线 */}
        {!isAnyActive && hasEnvironments && (
          <div className="w-px h-5 bg-border" />
        )}

        {/* 添加节点按钮 */}
        {!isAnyActive && (
          <>
            <button
              type="button"
              onClick={() => useUIStore.getState().openNodePalette()}
              title={getLL().flow.add_node_tooltip()}
              className={cn(
                'flex items-center justify-center w-9 h-9 rounded-lg',
                'text-foreground',
                'hover:bg-accent hover:scale-110',
                'active:scale-95',
                'transition-all duration-150 cursor-pointer',
              )}
            >
              <Plus className="w-4 h-4" />
            </button>
            <div className="w-px h-5 bg-border" />
          </>
        )}

        {/* 撤销/重做按钮 */}
        <button
          type="button"
          onClick={undo}
          disabled={!canUndo}
          title={getLL().flow.undo()}
          className={cn(
            'flex items-center justify-center transition-all duration-150',
            canUndo && 'text-muted-foreground hover:text-foreground hover:scale-110 cursor-pointer',
            !canUndo && 'text-muted-foreground/30 cursor-not-allowed'
          )}
        >
          <Undo2 className="w-4 h-4" />
        </button>
        <button
          type="button"
          onClick={redo}
          disabled={!canRedo}
          title={getLL().flow.redo()}
          className={cn(
            'flex items-center justify-center transition-all duration-150',
            canRedo && 'text-muted-foreground hover:text-foreground hover:scale-110 cursor-pointer',
            !canRedo && 'text-muted-foreground/30 cursor-not-allowed'
          )}
        >
          <Redo2 className="w-4 h-4" />
        </button>

        {/* 分隔线 */}
        <div className="w-px h-5 bg-border" />

        {/* 执行/停止按钮 */}
        {isExecuting ? (
          <button
            type="button"
            data-stop-btn
            onClick={() => useExecutionStore.getState().cancelExecution()}
            title={getLL().flow.stop_execution()}
            className="flex items-center justify-center transition-all duration-150 text-red-600 hover:text-red-700 hover:scale-110 cursor-pointer"
          >
            <Square className="w-6 h-6" fill="currentColor" />
          </button>
        ) : (
          <button
            type="button"
            data-execute-btn
            onClick={handleExecute}
            disabled={isDisabled}
            title={`${getLL().flow.execute_flow()} (${hotkeyLabel(['Ctrl+Enter'])})`}
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
              ? getLL().flow.debug_unavailable_executing()
              : isStressTestRunning
              ? getLL().flow.debug_unavailable_stress_test()
              : getLL().flow.start_debug()
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

        {/* Ingress Mock 服务按钮（仅当流程包含 http:ingress 节点时展示） */}
        {ingressNode && (
          <button
            type="button"
            onClick={handleToggleIngress}
            title={
              listeningPort !== null
                ? ll.flow.ingress_listener_stop_hint({ port: listeningPort })
                : ll.flow.ingress_listener_start_hint()
            }
            className={cn(
              'flex items-center justify-center p-1 rounded-lg transition-all duration-150',
              listeningPort !== null
                ? 'text-emerald-500 bg-emerald-500/10 hover:bg-emerald-500/20'
                : 'text-indigo-500 hover:text-indigo-600 hover:scale-110 cursor-pointer'
            )}
          >
            <Radio className={cn('w-5 h-5', listeningPort !== null && 'animate-pulse')} />
          </button>
        )}

        {/* 压测按钮 */}
        <button
          type="button"
          onClick={openStressTestConfigDialog}
          disabled={!canStartStressTest}
          title={
            isExecuting
              ? getLL().flow.stress_test_unavailable_executing()
              : isDebugging
              ? getLL().flow.stress_test_unavailable_debugging()
              : isStressTestRunning
              ? getLL().flow.stress_test_running()
              : getLL().flow.stress_test()
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

        {/* 数据驱动按钮 */}
        <button
          type="button"
          onClick={isDatasetExecuting ? undefined : openDatasetExecutionPanel}
          disabled={!canStartDatasetExecution && !isDatasetExecuting}
          title={
            isExecuting
              ? getLL().flow.dataset_unavailable_executing()
              : isDebugging
              ? getLL().flow.dataset_unavailable_debugging()
              : isStressTestRunning
              ? getLL().flow.dataset_unavailable_stress_test()
              : isDatasetExecuting
              ? getLL().flow.dataset_running()
              : getLL().flow.dataset()
          }
          className={cn(
            'flex items-center justify-center',
            'transition-all duration-150',
            (canStartDatasetExecution || isDatasetExecuting) && [
              isDatasetExecuting
                ? 'text-violet-500 animate-pulse'
                : 'text-violet-500 hover:text-violet-600',
              'hover:scale-110',
              'cursor-pointer',
            ],
            !canStartDatasetExecution && !isDatasetExecuting && [
              'text-muted-foreground cursor-not-allowed',
            ]
          )}
        >
          <Database className="w-6 h-6" />
        </button>

        {/* 变量按钮 - 工具栏内 */}
        {hasVariables && !isAnyActive && (
          <>
            <div className="w-px h-5 bg-border" />
            <div className="relative">
              <Popover.Root open={isVariableDrawerOpen} onOpenChange={(open) => { if (!open) toggleVariableDrawer(); }}>
                <Popover.Trigger asChild>
                  <button
                    type="button"
                    onClick={toggleVariableDrawer}
                    className={cn(
                      'flex flex-col items-center justify-center',
                      'w-9 h-9 rounded-lg',
                      'text-foreground',
                      'hover:bg-accent hover:scale-105',
                      'active:scale-95',
                      'transition-all duration-100 cursor-pointer',
                      'select-none',
                      isVariableDrawerOpen && 'bg-accent text-violet-600',
                    )}
                    title={isVariableDrawerOpen ? getLL().flow.collapse_variable_panel() : getLL().flow.variable_count({count: totalVariables})}
                  >
                    <div className="relative">
                      <VariableIcon className="w-3.5 h-3.5" />
                      <span className="absolute -top-1 -right-1.5 min-w-[10px] h-[10px] flex items-center justify-center rounded-full bg-violet-100 text-violet-600 dark:bg-violet-900/40 dark:text-violet-400 text-[7px] font-bold leading-none px-px">
                        {totalVariables}
                      </span>
                    </div>
                  </button>
                </Popover.Trigger>

                <Popover.Portal>
                  <Popover.Content
                    side="bottom"
                    sideOffset={8}
                    align="center"
                    className={cn(
                      'z-50',
                      'w-72 max-h-[calc(100vh-200px)]',
                      'flex flex-col',
                      'bg-background/95 backdrop-blur-xl rounded-xl',
                      'shadow-xl border border-border/50 dark:border-white/20',
                      'overflow-hidden',
                      'animate-in fade-in-0 zoom-in-95',
                      'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
                    )}
                  >
                    {/* 标题栏 + 关闭按钮 */}
                    <div className="flex items-center justify-between px-3 py-2 border-b border-border shrink-0">
                      <div className="flex items-center gap-2">
                        <VariableIcon className="w-4 h-4 text-violet-600" />
                        <span className="text-sm font-semibold text-foreground">{getLL().flow.variable_panel()}</span>
                        <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
                          {totalVariables}
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={toggleVariableDrawer}
                        className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                        title={getLL().flow.toggle_variable_panel()}
                      >
                        <X className="w-3.5 h-3.5" />
                      </button>
                    </div>
                    {/* 变量面板内容 */}
                    <div className="flex-1 overflow-y-auto min-h-0">
                      <VariablePanel flow={flow!} nodes={flow!.nodes} showHeader={false} />
                    </div>
                    <Popover.Arrow className="fill-background drop-shadow-sm" />
                  </Popover.Content>
                </Popover.Portal>
              </Popover.Root>
            </div>
          </>
        )}

      </div>
    </div>
  );

  return (
    <>
      {toolbarContent}
      
      {/* 压力测试配置对话框（按需懒加载） */}
      {showStressTestConfigDialog && (
        <Suspense fallback={null}>
          <StressTestConfigDialog
            open={showStressTestConfigDialog}
            onOpenChange={(open) => !open && closeStressTestConfigDialog()}
          />
        </Suspense>
      )}

      {/* 数据驱动执行面板（按需懒加载） */}
      {showDatasetExecutionPanel && (
        <Suspense fallback={null}>
          <DatasetExecutionPanel
            open={showDatasetExecutionPanel}
            onOpenChange={(open) => !open && closeDatasetExecutionPanel()}
          />
        </Suspense>
      )}
      {/* 验证错误对话框 */}
      <ValidationErrorDialog
        open={showValidationDialog}
        type="validation"
        errors={validationErrors}
        onClose={() => setShowValidationDialog(false)}
      />
    </>
  );
}
