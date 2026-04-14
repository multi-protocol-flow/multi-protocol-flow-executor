/**
 * ExecutionPanel - 底部面板水平布局执行面板
 * 左右分栏：左侧节点列表（含状态），右侧选中节点详情或 WS 消息时间线
 */
import { useState, useMemo, useEffect, useRef } from 'react';
import {
  Loader2,
  CheckCircle2,
  XCircle,
  Circle,
  Clock,
  MousePointerClick,
  Ban,
} from 'lucide-react';
import { useExecutionStore, useFlowStore } from '@/stores';
import {
  selectIsExecuting,
  selectCompletedNodeCount,
  selectTotalNodes,
  selectReport,
} from '@/stores/execution';
import { NodeExecutionDetails } from './NodeExecutionDetails';
import { WsMessageTimeline } from './WsMessageTimeline';
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import { cn } from '@/lib/utils';
import type { Node } from '@/types';

// ============================================================================
// 子组件：节点状态列表项
// ============================================================================

interface NodeStatusItemProps {
  /** 节点信息 */
  node: Node;
  /** 执行状态: running / success / failed / cancelled / pending */
  status: 'running' | 'success' | 'failed' | 'cancelled' | 'pending';
  /** 是否选中 */
  isSelected: boolean;
  /** 执行耗时（ms），仅 completed 节点有值 */
  durationMs: number | null;
  /** 点击回调 */
  onClick: () => void;
}

function NodeStatusItem({
  node,
  status,
  isSelected,
  durationMs,
  onClick,
}: NodeStatusItemProps) {
  const nodeTypeLabel = NODE_TYPE_NAMES[node.data.type] || node.data.type;
  const nodeTypeColor = NODE_TYPE_COLORS[node.data.type] || '';

  // 状态图标
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
      {/* 状态图标 */}
      <StatusIcon />

      {/* 节点名称 */}
      <span
        className={cn(
          'text-xs truncate flex-1 min-w-0',
          status === 'pending' ? 'text-muted-foreground' : 'text-foreground',
        )}
      >
        {node.name}
      </span>

      {/* 类型徽标 */}
      <span
        className={cn(
          'text-[10px] px-1.5 py-0 rounded-sm shrink-0',
          nodeTypeColor || 'bg-muted text-muted-foreground',
        )}
      >
        {nodeTypeLabel}
      </span>

      {/* 耗时 */}
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

export function ExecutionPanel() {
  // ===== Store 数据 =====
  const nodes = useFlowStore((state) => state.flow?.nodes ?? []);
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
  const wsStreams = useExecutionStore((state) => state.wsStreams);
  const nodeReportsMap = useExecutionStore((state) => state.nodeReportsMap);

  // ===== 本地状态 =====
  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);
  const nodeListRef = useRef<HTMLDivElement>(null);
  const prevCompletedCountRef = useRef(0);

  // ===== 有序节点列表 =====
  // 使用 flow.nodes 的顺序，若最终报告有顺序则按报告排序
  const orderedNodes = useMemo(() => {
    if (!report || !report.node_reports.length) return nodes;

    const orderMap = new Map(
      report.node_reports.map((r, i) => [r.node_uuid, i]),
    );
    const nodeMap = new Map(nodes.map((n) => [n.uuid, n]));

    // 先按报告顺序排
    const ordered: Node[] = report.node_reports
      .map((r) => nodeMap.get(r.node_uuid))
      .filter((n): n is Node => n !== undefined);

    // 追加报告中未出现但存在于 flow 中的节点
    for (const node of nodes) {
      if (!orderMap.has(node.uuid)) {
        ordered.push(node);
      }
    }

    return ordered;
  }, [nodes, report]);

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
      // 新节点完成或有节点开始执行，滚动到可见区域
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

    return Array.from(wsStreams.values()).filter((stream) =>
      stream.segments.some((seg) => seg.node_uuid === selectedNodeUuid),
    );
  }, [selectedNodeUuid, nodes, wsStreams]);

  // ===== 空状态（尚未执行） =====
  const hasAnyExecution =
    isExecuting || completedNodeCount > 0 || Object.keys(nodeReportsMap).length > 0;

  if (!hasAnyExecution) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="flex flex-col items-center gap-2">
          <MousePointerClick className="w-6 h-6 opacity-40" />
          <span className="text-sm">点击「执行」开始运行流程</span>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex">
      {/* ===== 左侧：节点列表 (~35%) ===== */}
      <div className="w-[35%] min-w-[200px] max-w-[360px] border-r border-border flex flex-col">
        {/* 进度条 */}
        <div className="px-3 py-1.5 border-b border-border shrink-0 flex items-center gap-2">
          <span className="text-xs text-muted-foreground tabular-nums shrink-0">
            {completedNodeCount}/{totalNodes} 节点
          </span>
          <span className="text-xs text-muted-foreground/60 shrink-0">·</span>
          <span className="text-xs text-muted-foreground tabular-nums shrink-0">
            {progressPercent}%
          </span>
          {/* 微型进度条 */}
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
          className="flex-1 min-h-0 overflow-y-auto"
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
                  onClick={() =>
                    setSelectedNodeUuid(
                      selectedNodeUuid === node.uuid ? null : node.uuid,
                    )
                  }
                />
              </div>
            );
          })}
        </div>
      </div>

      {/* ===== 右侧：详情区域 (~65%) ===== */}
      <div className="flex-1 min-w-0 overflow-hidden flex flex-col">
        {/* 未选中节点 */}
        {!selectedNodeUuid && (
          <div className="h-full flex items-center justify-center text-muted-foreground">
            <span className="text-sm">选择节点查看详情</span>
          </div>
        )}

        {/* 选中节点 - pending */}
        {selectedNodeUuid && selectedStatus === 'pending' && (
          <div className="h-full flex items-center justify-center text-muted-foreground">
            <span className="text-sm">等待执行</span>
          </div>
        )}

        {/* 选中节点 - running */}
        {selectedNodeUuid && selectedStatus === 'running' && (
          <div className="h-full flex flex-col overflow-hidden">
            {/* 加载指示 */}
            <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border shrink-0">
              <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin" />
              <span className="text-xs text-muted-foreground">执行中...</span>
            </div>
            {/* WS 实时消息 */}
            {selectedWsStreams.length > 0 && (
              <div className="flex-1 min-h-0 overflow-hidden">
                <WsMessageTimeline
                  streams={selectedWsStreams}
                  live={isExecuting}
                />
              </div>
            )}
          </div>
        )}

        {/* 选中节点 - success / failed */}
        {selectedNodeUuid &&
          (selectedStatus === 'success' || selectedStatus === 'failed') && (
            <div className="flex-1 min-h-0 overflow-y-auto">
              <div className="p-3">
                {/* WS 消息时间线（如果有） */}
                {selectedWsStreams.length > 0 && (
                  <div className="mb-3">
                    <WsMessageTimeline
                      streams={selectedWsStreams}
                      live={isExecuting}
                    />
                  </div>
                )}
                {/* 节点执行详情 */}
                {selectedReport && (
                  <NodeExecutionDetails report={selectedReport} />
                )}
              </div>
            </div>
          )}
      </div>
    </div>
  );
}
