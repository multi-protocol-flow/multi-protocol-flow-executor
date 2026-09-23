/**
 * ExecutionPanel - 底部面板水平布局执行面板
 * 左右分栏：左侧节点列表（含状态），右侧选中节点详情
 * 右侧详情区域包含子标签：[输出] [WS消息] [变量] [栈]
 */
import { memo, useState, useMemo, useEffect, useRef, useCallback } from 'react';
import {
  Loader2,
  CheckCircle2,
  XCircle,
  Circle,
  Clock,
  MousePointerClick,
  Ban,
} from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import {
  useExecutionStore,
  useFlowStore,
  useDebugStore,
  useUIStore,
  getNodeStage,
  selectAllNodes,
  findNodeInFlow,
} from '@/stores';
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
import { StreamInspectorPanel } from './StreamInspectorPanel';
import { VariableTreeView } from './VariableTreeView';
import { VariableTimelineDrawer } from './VariableTimelineDrawer';
import { ExecutionStackView } from './ExecutionStackView';
import { DebugReplConsole } from './DebugReplConsole';
import { getNodeMeta } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { sendDebugCommand } from '@/services/tauri';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { ResizablePanel } from '@/components/ui/ResizablePanel';
import type { Node, NodeExecutionReport } from '@/types';

// ============================================================================
// 子组件：节点状态列表项
// ============================================================================

type NodeItemStatus = 'running' | 'success' | 'failed' | 'cancelled' | 'pending';

interface NodeStatusItemProps {
  node: Node;
  status: NodeItemStatus;
  isSelected: boolean;
  durationMs: number | null;
  onSelect: (uuid: string) => void;
}

function StatusIcon({ status }: { status: NodeItemStatus }) {
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
}

const NodeStatusItem = memo(function NodeStatusItem({
  node,
  status,
  isSelected,
  durationMs,
  onSelect,
}: NodeStatusItemProps) {
  const meta = getNodeMeta(node.data.type);
  const nodeTypeLabel = meta?.name ?? node.data.type;
  const flow = useFlowStore((s) => s.flow);
  const stage = flow ? getNodeStage(flow, node.uuid) : null;
  const stageLl = getLL().stages;
  return (
    <button
      type="button"
      onClick={() => onSelect(node.uuid)}
      className={cn(
        'w-full flex items-center gap-2 px-2.5 py-1.5 text-left transition-colors',
        'border-l-2',
        isSelected
          ? 'bg-accent border-l-primary'
          : 'border-l-transparent hover:bg-muted/50',
        status === 'running' && !isSelected && 'bg-blue-500/5',
      )}
    >
      <StatusIcon status={status} />
      <span
        className={cn(
          'text-xs truncate flex-1 min-w-0',
          status === 'pending' ? 'text-muted-foreground' : 'text-foreground',
        )}
      >
        {node.name}
      </span>
      {stage && stage !== 'main' && (
        <span
          className={cn(
            'shrink-0 rounded px-1 py-0.5 text-[9px] font-medium leading-none border',
            stage === 'setup'
              ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20'
              : 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
          )}
        >
          {stage === 'setup' ? stageLl.short_setup() : stageLl.short_teardown()}
        </span>
      )}
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
});

/**
 * 独立进度条组件：独立订阅完成计数与总数，避免计数递增导致整个面板与列表全量重渲染
 */
const ExecutionProgressBar = memo(function ExecutionProgressBar({
  isExecuting,
}: {
  isExecuting: boolean;
}) {
  const ll = getLL();
  const completedNodeCount = useExecutionStore(selectCompletedNodeCount);
  const totalNodes = useExecutionStore(selectTotalNodes);
  const progressPercent =
    totalNodes > 0 ? Math.round((completedNodeCount / totalNodes) * 100) : 0;

  return (
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
  );
});

/**
 * 独立节点列表组件
 */
interface ExecutionNodeListProps {
  orderedNodes: Node[];
  selectedNodeUuid: string | null;
  onSelectNode: (uuid: string) => void;
  getNodeExecutionStatus: (nodeUuid: string) => NodeItemStatus;
  getNodeExecutionReport: (nodeUuid: string) => NodeExecutionReport | null;
  nodeListRef: React.RefObject<HTMLDivElement | null>;
}

const ExecutionNodeList = memo(function ExecutionNodeList({
  orderedNodes,
  selectedNodeUuid,
  onSelectNode,
  getNodeExecutionStatus,
  getNodeExecutionReport,
  nodeListRef,
}: ExecutionNodeListProps) {
  return (
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
              onSelect={onSelectNode}
            />
          </div>
        );
      })}
    </div>
  );
});

// ============================================================================
// 判断节点是否为 WebSocket 类型
// ============================================================================

function isWsNodeType(nodeType: string): boolean {
  return (
    nodeType === 'ws:connect' ||
    nodeType === 'ws:send_collect' ||
    nodeType === 'ws:close'
  );
}

/**
 * 右侧「输出」标签面板。
 *
 * 只订阅**当前选中节点自己**的状态与报告：其它节点的执行事件不会触发本组件重渲染。
 * 节点处于运行中时渲染轻量占位（不挂载重量级 Viewer），避免高频执行下反复
 * 卸载/挂载详情组件树导致的 UI 线程卡顿。
 */
const NodeOutputPane = memo(function NodeOutputPane({
  nodeUuid,
  autoTrackingRef,
}: {
  nodeUuid: string | null;
  /** 是否仍在自动跟随当前执行节点（用户手动点选其它节点后为 false） */
  autoTrackingRef: React.MutableRefObject<boolean>;
}) {
  const ll = getLL();
  // 选择器返回字符串 / 对象引用：仅当该节点自身状态或报告变化时才触发重渲染
  const status = useExecutionStore((state) =>
    nodeUuid ? state.getNodeExecutionStatus(nodeUuid) : null,
  );
  const report = useExecutionStore((state) =>
    nodeUuid ? state.getNodeExecutionReport(nodeUuid) : null,
  );
  const isExecuting = useExecutionStore(selectIsExecuting);
  const nodeName = useFlowStore((state) =>
    nodeUuid ? findNodeInFlow(state.flow, nodeUuid)?.name ?? null : null,
  );

  // ===== 冷通道按需拉取完整报文 =====
  useEffect(() => {
    // 关键优化：执行期间自动跟随模式下不预拉取大报文，避免频繁 IPC 与重渲染导致页面卡顿；
    // 只有在流程停止执行，或用户主动手动点击了某个具体节点时，才按需拉取冷通道详情
    if (!isExecuting || !autoTrackingRef.current) {
      if (nodeUuid && report?.is_detail_stripped) {
        const flowUuid = useFlowStore.getState().flow?.uuid;
        if (flowUuid) {
          void useExecutionStore.getState().fetchNodeDetail(flowUuid, nodeUuid);
        }
      }
    }
  }, [isExecuting, nodeUuid, report?.is_detail_stripped, autoTrackingRef]);

  if (!nodeUuid) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <span className="text-sm">{ll.execution.panel.select_node_hint()}</span>
      </div>
    );
  }

  if (status === 'pending') {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <span className="text-sm">{ll.execution.panel.waiting()}</span>
      </div>
    );
  }

  if (status === 'running') {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 text-muted-foreground gap-3">
        <Loader2 className="w-6 h-6 text-blue-500 animate-spin" />
        <div className="text-center">
          <p className="text-sm font-semibold text-foreground">
            {nodeName || ll.execution.panel.executing()}
          </p>
          <p className="text-xs text-blue-500 mt-1 font-medium">
            {ll.execution.panel.executing()}...
          </p>
        </div>
      </div>
    );
  }

  if (!report) return null;

  return (
    <div className="h-full overflow-y-auto scrollbar-thin">
      <div className="p-3">
        <NodeExecutionDetails report={report} />
      </div>
    </div>
  );
});

// ============================================================================
// 主组件：ExecutionPanel
// ============================================================================

/** 详情区域子标签类型 */
type DetailTab = 'output' | 'ws' | 'variables' | 'stack';

export const ExecutionPanel = memo(function ExecutionPanel() {
  const ll = getLL();
  // ===== Store 数据 =====
  const nodes = useFlowStore(selectAllNodes);
  const isExecuting = useExecutionStore(selectIsExecuting);
  const currentNodeUuid = useExecutionStore((state) => state.currentNodeUuid);
  const report = useExecutionStore(selectReport);
  const getNodeExecutionStatus = useExecutionStore(
    (state) => state.getNodeExecutionStatus,
  );
  const getNodeExecutionReport = useExecutionStore(
    (state) => state.getNodeExecutionReport,
  );
  const wsStreams = useWsStreamStore(useShallow((state) => state.wsStreams));
  const nodeReportsMap = useExecutionStore(useShallow((state) => state.nodeReportsMap));
  // ===== 调试状态 =====
  const isDebugging = useDebugStore(selectIsDebugging);
  const debugCurrentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const variables = useDebugStore((state) => state.variables);
  const executionStack = useDebugStore((state) => state.executionStack);
  const focusOnNode = useFlowStore((state) => state.focusOnNode);

  const debugStatus = useDebugStore((state) => state.status);
  const hasDebugStack = useDebugStore((state) => state.executionStack.length > 0);
  const showDebugTabs = isDebugging || (debugStatus === 'ended' && hasDebugStack);

  // ===== 本地状态 =====
  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const [selectedTimelineVar, setSelectedTimelineVar] = useState<string | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>('output');
  const nodeListRef = useRef<HTMLDivElement>(null);
  /** 用户是否手动选择了非当前运行节点，此时暂停自动跟踪当前执行节点 */
  const autoTrackingRef = useRef(true);
  /** 当前选中节点镜像：供稳定回调读取，避免 useCallback 依赖 selectedNodeUuid 破坏列表 memo */
  const selectedNodeUuidRef = useRef<string | null>(null);
  selectedNodeUuidRef.current = selectedNodeUuid;

  // ===== 节点选择回调（恒等引用，保证 ExecutionNodeList 的 memo 生效） =====
  const handleSelectNode = useCallback((nodeUuid: string) => {
    const flow = useFlowStore.getState().flow;
    if (flow) {
      const stage = getNodeStage(flow, nodeUuid);
      if (stage && stage !== useFlowStore.getState().activeStage) {
        useFlowStore.getState().setActiveStage(stage);
      }
    }
    if (selectedNodeUuidRef.current === nodeUuid) {
      // 已选中：直接定位到画布，不取消选择
      useFlowStore.getState().focusOnNode(nodeUuid);
    } else {
      setSelectedNodeUuid(nodeUuid);
      useFlowStore.getState().selectNode(nodeUuid);
      useFlowStore.getState().focusOnNode(nodeUuid);
    }
    // 点击当前运行节点 → 恢复自动跟踪；点击其他节点 → 暂停自动跟踪
    if (nodeUuid === useExecutionStore.getState().currentNodeUuid) {
      autoTrackingRef.current = true;
    } else {
      autoTrackingRef.current = false;
    }
  }, []);

  // ===== 调试开始时自动打开底部面板 =====
  useEffect(() => {
    if (isDebugging) {
      useUIStore.getState().setExecutionPanelOpen(true);
    }
  }, [isDebugging]);

  // ===== 新执行开始时重置为自动跟踪模式 =====
  useEffect(() => {
    if (isExecuting) {
      autoTrackingRef.current = true;
    }
  }, [isExecuting]);

  // ===== 有序节点列表（仅显示实际执行路径上的节点） =====
  const orderedNodes = useMemo(() => {
    const nodeMap = new Map(nodes.map((n) => [n.uuid, n]));

    // 执行完成：仅显示报告中的节点（实际执行路径）
    if (report && report.node_reports.length) {
      return report.node_reports
        .map((r) => nodeMap.get(r.node_uuid))
        .filter((n): n is Node => n !== undefined);
    }

    // 执行中或已取消（有已完成节点）：显示已完成节点 + 当前节点
    if (isExecuting || Object.keys(nodeReportsMap).length > 0) {
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

  // ===== 自动选中正在运行的节点（用户手动选择后暂停跟踪） =====
  useEffect(() => {
    if (currentNodeUuid && autoTrackingRef.current) {
      setSelectedNodeUuid(currentNodeUuid);
    }
  }, [currentNodeUuid]);

  // ===== 自动滚动节点列表（用户手动选择后暂停跟踪，使用 instant auto 滚动消除动画卡顿） =====
  useEffect(() => {
    if (!autoTrackingRef.current || !currentNodeUuid) return;
    const activeEl = nodeListRef.current?.querySelector(
      '[data-status="running"], [data-status="success"]:last-of-type',
    );
    if (typeof activeEl?.scrollIntoView === 'function') {
      activeEl.scrollIntoView({ block: 'nearest', behavior: 'auto' });
    }
  }, [currentNodeUuid]);

  // ===== 选中节点的 WS 流 =====
  const selectedWsStreams = useMemo(() => {
    if (!selectedNodeUuid) return [];
    const selectedNode = nodes.find((n) => n.uuid === selectedNodeUuid);
    if (!selectedNode || !isWsNodeType(selectedNode.data.type)) return [];

    return Array.from(wsStreams.values());
  }, [selectedNodeUuid, nodes, wsStreams]);

  // 未绑定具体节点时展示全部连接流；必须 memo —— 内联构造会让下游订阅组件
  // 每次渲染都判定 streams 变化，从而不断重启保活心跳等基于 streams 的 effect。
  const allWsStreams = useMemo(() => Array.from(wsStreams.values()), [wsStreams]);
  const inspectorStreams = selectedWsStreams.length > 0 ? selectedWsStreams : allWsStreams;

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
    isExecuting || Object.keys(nodeReportsMap).length > 0 || report !== null;
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
          {/* 进度条：独立订阅计数，与列表隔离重渲染 */}
          <ExecutionProgressBar isExecuting={isExecuting} />

          {/* 节点列表 */}
          <ExecutionNodeList
            orderedNodes={orderedNodes}
            selectedNodeUuid={selectedNodeUuid}
            onSelectNode={handleSelectNode}
            getNodeExecutionStatus={getNodeExecutionStatus}
            getNodeExecutionReport={getNodeExecutionReport}
            nodeListRef={nodeListRef}
          />
        </div>
      }
      rightPanel={
        <div className="h-full flex flex-col">
          {/* 子标签栏 */}
          <div className="border-b border-border shrink-0 px-3 py-1.5 bg-muted/20">
            <Tabs value={detailTab} onValueChange={(v) => setDetailTab(v as DetailTab)}>
              <TabsList className="h-7 p-0.5 bg-muted/60 border border-border/40 rounded-lg">
                <TabsTrigger value="output" className="h-6 px-2.5 text-xs rounded-md">
                   {ll.execution.panel.tab_output()}
                </TabsTrigger>
                {hasWsNodes && (
                  <TabsTrigger value="ws" className="h-6 px-2.5 text-xs rounded-md">
                    {ll.execution.panel.tab_ws()}
                  </TabsTrigger>
                )}
                {showDebugTabs && (
                  <>
                    <TabsTrigger value="variables" className="h-6 px-2.5 text-xs rounded-md">
                       {ll.execution.panel.tab_variables()}
                    </TabsTrigger>
                    <TabsTrigger value="stack" className="h-6 px-2.5 text-xs rounded-md">
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
              <NodeOutputPane
                nodeUuid={selectedNodeUuid}
                autoTrackingRef={autoTrackingRef}
              />
            </TabsContent>

            {hasWsNodes && (
              <TabsContent value="ws" className="m-0 h-full overflow-hidden">
                {selectedWsStreams.length > 0 || wsStreams.size > 0 ? (
                  <StreamInspectorPanel
                    streams={inspectorStreams}
                    live={isExecuting || isDebugging}
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
                <TabsContent value="variables" className="m-0 h-full overflow-hidden relative">
                  <VariableTreeView
                    variables={variables}
                    canEdit={canEditVariables}
                    onEdit={handleModifyVariable}
                    onOpenTimeline={setSelectedTimelineVar}
                  />
                  <VariableTimelineDrawer
                    varName={selectedTimelineVar}
                    onClose={() => setSelectedTimelineVar(null)}
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

          {showDebugTabs && <DebugReplConsole />}
        </div>
      }
    />
  );
});
