/**
 * ExecutionPanel - 底部面板水平布局执行面板
 * 左右分栏：左侧节点列表（含状态），右侧选中节点详情
 * 右侧详情区域包含子标签：[输出] [WS消息] [变量] [栈]
 */
import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import {
  Loader2,
  CheckCircle2,
  XCircle,
  Circle,
  Clock,
  MousePointerClick,
  Ban,
} from 'lucide-react';
import { useExecutionStore, useFlowStore, useDebugStore, useUIStore } from '@/stores';
import {
  selectIsExecuting,
  selectCompletedNodeCount,
  selectTotalNodes,
  selectReport,
} from '@/stores/execution';
import { selectIsDebugging, selectCurrentNodeUuid } from '@/stores/debug';
import { useWsStreamStore } from '@/stores/wsStream';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { NodeExecutionDetails } from './NodeExecutionDetails';
import { WsMessageTimeline } from './WsMessageTimeline';
import { VariableTreeView } from './VariableTreeView';
import { ExecutionStackView } from './ExecutionStackView';
import { getNodeMeta } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { sendDebugCommand } from '@/services/tauri';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { ResizablePanel } from '@/components/ui/ResizablePanel';
import type { Node } from '@/types';

// ============================================================================
// 子组件：节点状态列表项
// ============================================================================

interface NodeStatusItemProps {
  node: Node;
  status: 'running' | 'success' | 'failed' | 'cancelled' | 'pending';
  isSelected: boolean;
  durationMs: number | null;
  onClick: () => void;
}

function NodeStatusItem({
  node,
  status,
  isSelected,
  durationMs,
  onClick,
}: NodeStatusItemProps) {
  const meta = getNodeMeta(node.data.type);
  const nodeTypeLabel = meta?.name ?? node.data.type;

  const StatusIcon = () => {
    switch (status) {
      case 'running':
        return (
          <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin shrink-0" />
        );
      case 'success':
        return (
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500 shrink-0" />
        );
      case 'failed':
        return <XCircle className="w-3.5 h-3.5 text-red-500 shrink-0" />;
      case 'cancelled':
        return <Ban className="w-3.5 h-3.5 text-amber-500 shrink-0" />;
      default:
        return (
          <Circle className="w-3.5 h-3.5 text-muted-foreground/40 shrink-0" />
        );
    }
  };

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-2 px-2.5 py-1.5 text-left transition-colors',
        'border-l-2',
        isSelected
          ? 'bg-accent border-l-primary'
          : 'border-l-transparent hover:bg-muted/50',
        status === 'running' && !isSelected && 'bg-blue-500/5',
      )}
    >
      <StatusIcon />
      <span
        className={cn(
          'text-xs truncate flex-1 min-w-0',
          status === 'pending' ? 'text-muted-foreground' : 'text-foreground',
        )}
      >
        {node.name}
      </span>
      <span
        className={cn(
          'text-[10px] px-1.5 py-0.5 rounded shrink-0 font-medium leading-none',
          meta?.badge || 'bg-muted text-muted-foreground',
        )}
      >
        {nodeTypeLabel}
      </span>
      {durationMs !== null && (
        <span className="text-[10px] text-muted-foreground tabular-nums shrink-0 flex items-center gap-0.5">
          <Clock className="w-2.5 h-2.5" />
          {durationMs < 1000 ? `${durationMs}ms` : `${(durationMs / 1000).toFixed(1)}s`}
        </span>
      )}
    </button>
  );
}

// ============================================================================
// 判断节点是否为 WebSocket 类型
// ============================================================================

function isWsNodeType(nodeType: string): boolean {
  return (
    nodeType === 'ws_connect' ||
    nodeType === 'ws_send_collect' ||
    nodeType === 'ws_close'
  );
}

// ============================================================================
// 主组件：ExecutionPanel
// ============================================================================

/** 详情区域子标签类型 */
type DetailTab = 'output' | 'ws' | 'variables' | 'stack';

const EMPTY_NODES: Node[] = [];

export function ExecutionPanel() {
  const ll = getLL();
  // ===== Store 数据 =====
  const nodes = useFlowStore((state) => state.flow?.nodes ?? EMPTY_NODES);
  const isExecuting = useExecutionStore(selectIsExecuting);
  const currentNodeUuid = useExecutionStore((state) => state.currentNodeUuid);
  const completedNodeCount = useExecutionStore(selectCompletedNodeCount);
  const totalNodes = useExecutionStore(selectTotalNodes);
  const report = useExecutionStore(selectReport);
  const getNodeExecutionStatus = useExecutionStore(
    (state) => state.getNodeExecutionStatus,
  );
  const getNodeExecutionReport = useExecutionStore(
    (state) => state.getNodeExecutionReport,
  );
  const wsStreams = useWsStreamStore((state) => state.wsStreams);
  const nodeReportsMap = useExecutionStore((state) => state.nodeReportsMap);

  // ===== 调试状态 =====
  const isDebugging = useDebugStore(selectIsDebugging);
  const debugCurrentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const variables = useDebugStore((state) => state.variables);
  const executionStack = useDebugStore((state) => state.executionStack);
  const focusOnNode = useFlowStore((state) => state.focusOnNode);

  const showDebugTabs = isDebugging;

  // ===== 本地状态 =====
  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>('output');
  const nodeListRef = useRef<HTMLDivElement>(null);
  const prevCompletedCountRef = useRef(0);

  // ===== 调试开始时自动打开底部面板 =====
  useEffect(() => {
    if (isDebugging) {
      useUIStore.getState().setExecutionPanelOpen(true);
    }
  }, [isDebugging]);

  // ===== 有序节点列表（仅显示实际执行路径上的节点） =====
  const orderedNodes = useMemo(() => {
    const nodeMap = new Map(nodes.map((n) => [n.uuid, n]));

    // 执行完成：仅显示报告中的节点（实际执行路径）
    if (report && report.node_reports.length) {
      return report.node_reports
        .map((r) => nodeMap.get(r.node_uuid))
        .filter((n): n is Node => n !== undefined);
    }

    // 执行中：显示已完成节点 + 当前正在执行的节点（保持执行顺序）
    if (isExecuting) {
      const completedUuids = Object.keys(nodeReportsMap);
      const uuids = [...completedUuids];
      if (currentNodeUuid && !uuids.includes(currentNodeUuid)) {
        uuids.push(currentNodeUuid);
      }
      return uuids
        .map((uuid) => nodeMap.get(uuid))
        .filter((n): n is Node => n !== undefined);
    }

    // 未执行：空列表（空状态由 hasAnyExecution 守卫处理）
    return [];
  }, [nodes, report, isExecuting, nodeReportsMap, currentNodeUuid]);

  // ===== 自动选中正在运行的节点 =====
  useEffect(() => {
    if (currentNodeUuid) {
      setSelectedNodeUuid(currentNodeUuid);
    }
  }, [currentNodeUuid]);

  // ===== 自动滚动节点列表 =====
  useEffect(() => {
    if (
      completedNodeCount > prevCompletedCountRef.current ||
      currentNodeUuid
    ) {
      requestAnimationFrame(() => {
        const activeEl = nodeListRef.current?.querySelector(
          '[data-status="running"], [data-status="success"]:last-of-type',
        );
        activeEl?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      });
    }
    prevCompletedCountRef.current = completedNodeCount;
  }, [completedNodeCount, currentNodeUuid]);

  // ===== 进度 =====
  const progressPercent =
    totalNodes > 0 ? Math.round((completedNodeCount / totalNodes) * 100) : 0;

  // ===== 选中节点信息 =====
  const selectedStatus = selectedNodeUuid
    ? getNodeExecutionStatus(selectedNodeUuid)
    : null;
  const selectedReport = selectedNodeUuid
    ? getNodeExecutionReport(selectedNodeUuid)
    : null;

  // ===== 选中节点的 WS 流 =====
  const selectedWsStreams = useMemo(() => {
    if (!selectedNodeUuid) return [];
    const selectedNode = nodes.find((n) => n.uuid === selectedNodeUuid);
    if (!selectedNode || !isWsNodeType(selectedNode.data.type)) return [];

    return Array.from(wsStreams.values());
  }, [selectedNodeUuid, nodes, wsStreams]);

  // ===== 变量编辑回调 =====
  const canEditVariables = useDebugStore((state) => state.status === 'paused');
  const handleModifyVariable = useCallback(async (name: string, value: unknown) => {
    try {
      await sendDebugCommand({
        ModifyVariable: { name, value },
      });
    } catch (error) {
      toast.error(ll.execution.toast.variable_modify_failed() + ': ' + (error as Error).message);
    }
  }, []);

  // ===== 是否存在 WS 节点 =====
  const hasWsNodes = useMemo(
    () => nodes.some((n) => isWsNodeType(n.data.type)),
    [nodes],
  );

  // ===== 新执行开始时重置标签到「输出」 =====
  useEffect(() => {
    if (isExecuting) {
      setDetailTab('output');
    }
  }, [isExecuting]);

  // ===== 流程无 WS 节点时回退到「输出」标签 =====
  useEffect(() => {
    if (detailTab === 'ws' && !hasWsNodes) {
      setDetailTab('output');
    }
  }, [hasWsNodes, detailTab]);

  // ===== 空状态（尚未执行） =====
  const hasAnyExecution =
    isExecuting || completedNodeCount > 0 || Object.keys(nodeReportsMap).length > 0;

  if (!hasAnyExecution && !showDebugTabs) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="flex flex-col items-center gap-2">
          <MousePointerClick className="w-6 h-6 opacity-40" />
          <span className="text-sm">{ll.execution.panel.click_to_run()}</span>
        </div>
      </div>
    );
  }

return (
    <ResizablePanel
      defaultWidth={280}
      minWidth={200}
      maxWidth={360}
      leftPanel={
        <div className="h-full flex flex-col">
          {/* 进度条 */}
          <div className="px-3 py-1.5 border-b border-border shrink-0 flex items-center gap-2">
            <span className="text-xs text-muted-foreground tabular-nums shrink-0">
              {ll.execution.panel.node_count({ completed: completedNodeCount, total: totalNodes })}
            </span>
            <span className="text-xs text-muted-foreground/60 shrink-0">·</span>
            <span className="text-xs text-muted-foreground tabular-nums shrink-0">
              {progressPercent}%
            </span>
            <div className="flex-1 h-1 bg-muted rounded-full overflow-hidden min-w-[40px]">
              <div
                className={cn(
                  'h-full rounded-full transition-all duration-500 ease-out',
                  !isExecuting && completedNodeCount > 0
                    ? 'bg-emerald-500'
                    : 'bg-blue-500',
                )}
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          </div>

          {/* 节点列表 */}
          <div
            ref={nodeListRef}
            className="flex-1 min-h-0 overflow-y-auto scrollbar-thin"
          >
            {orderedNodes.map((node) => {
              const status = getNodeExecutionStatus(node.uuid);
              const nodeReport = getNodeExecutionReport(node.uuid);
              const durationMs = nodeReport?.duration_ms ?? null;

              return (
                <div key={node.uuid} data-status={status}>
                  <NodeStatusItem
                    node={node}
                    status={status}
                    isSelected={selectedNodeUuid === node.uuid}
                    durationMs={durationMs}
                    onClick={() => {
                      const newUuid = selectedNodeUuid === node.uuid ? null : node.uuid;
                      setSelectedNodeUuid(newUuid);
                      if (newUuid) {
                        useFlowStore.getState().selectNode(newUuid);
                        useFlowStore.getState().focusOnNode(newUuid);
                      }
                    }}
                  />
                </div>
              );
            })}
          </div>
        </div>
      }
      rightPanel={
        <div className="h-full flex flex-col">
          {/* 子标签栏 */}
          <div className="border-b border-border shrink-0">
            <Tabs value={detailTab} onValueChange={(v) => setDetailTab(v as DetailTab)}>
              <TabsList className="h-7 p-0.5 bg-transparent">
<TabsTrigger value="output" className="h-6 px-3 text-xs">
                   {ll.execution.panel.tab_output()}
                  </TabsTrigger>
                {hasWsNodes && (
                  <TabsTrigger value="ws" className="h-6 px-3 text-xs">
                    {ll.execution.panel.tab_ws()}
                  </TabsTrigger>
                )}
                {showDebugTabs && (
                  <>
<TabsTrigger value="variables" className="h-6 px-3 text-xs">
                       {ll.execution.panel.tab_variables()}
                      </TabsTrigger>
<TabsTrigger value="stack" className="h-6 px-3 text-xs">
                       {ll.execution.panel.tab_stack()}
                      </TabsTrigger>
                  </>
                )}
              </TabsList>
            </Tabs>
          </div>

          {/* 子标签内容 */}
          <Tabs value={detailTab} className="flex-1 min-h-0 overflow-hidden">
            <TabsContent value="output" className="m-0 h-full overflow-hidden">
              {/* 输出标签页 - 原有逻辑 */}
              {!selectedNodeUuid && (
                <div className="h-full flex items-center justify-center text-muted-foreground">
                  <span className="text-sm">{ll.execution.panel.select_node_hint()}</span>
                </div>
              )}

              {selectedNodeUuid && selectedStatus === 'pending' && (
                <div className="h-full flex items-center justify-center text-muted-foreground">
                  <span className="text-sm">{ll.execution.panel.waiting()}</span>
                </div>
              )}

              {selectedNodeUuid && selectedStatus === 'running' && (
                <div className="h-full flex flex-col overflow-hidden">
                  <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border shrink-0">
                    <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin" />
                    <span className="text-xs text-muted-foreground">{ll.execution.panel.executing()}</span>
                  </div>
                  {selectedReport && (
                    <div className="flex-1 min-h-0 overflow-y-auto scrollbar-thin p-3">
                      <NodeExecutionDetails report={selectedReport} />
                    </div>
                  )}
                </div>
              )}

              {selectedNodeUuid &&
                (selectedStatus === 'success' || selectedStatus === 'failed') && (
                  <div className="h-full overflow-y-auto scrollbar-thin">
                    <div className="p-3">
                      {selectedReport && (
                        <NodeExecutionDetails report={selectedReport} />
                      )}
                    </div>
                  </div>
                )}
            </TabsContent>

            {hasWsNodes && (
              <TabsContent value="ws" className="m-0 h-full overflow-hidden">
                {selectedWsStreams.length > 0 ? (
                  <WsMessageTimeline
                    streams={selectedWsStreams}
                    live={isExecuting}
                  />
                ) : (
                  <div className="h-full flex items-center justify-center text-muted-foreground">
                    <span className="text-sm">{ll.execution.panel.select_ws_node()}</span>
                  </div>
                )}
              </TabsContent>
            )}

            {showDebugTabs && (
              <>
                <TabsContent value="variables" className="m-0 h-full overflow-hidden">
                  <VariableTreeView
                    variables={variables}
                    canEdit={canEditVariables}
                    onEdit={handleModifyVariable}
                  />
                </TabsContent>

                <TabsContent value="stack" className="m-0 h-full overflow-hidden">
                  <ExecutionStackView
                    executionStack={executionStack}
                    currentNodeUuid={debugCurrentNodeUuid}
                    isReadOnly={false}
                    onNavigateToNode={focusOnNode}
                  />
                </TabsContent>
              </>
            )}
          </Tabs>
        </div>
      }
    />
  );
}
