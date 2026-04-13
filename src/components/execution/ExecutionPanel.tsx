/**
 * ExecutionPanel - 执行进度侧边栏面板
 * 在流程执行期间实时显示每个节点的完成状态、进度和详情
 */
import { useState, useMemo } from 'react';
import { X, Loader2, CheckCircle2 } from 'lucide-react';
import { useExecutionStore, useUIStore } from '@/stores';
import {
  selectIsExecuting,
  selectNodeReportsMap,
  selectCompletedNodeCount,
  selectTotalNodes,
  selectReport,
} from '@/stores/execution';
import { selectIsExecutionPanelOpen } from '@/stores/ui';
import { NodeListItem } from './NodeListItem';
import { NodeExecutionDetails } from './NodeExecutionDetails';
import { cn } from '@/lib/utils';
import type { NodeExecutionReport } from '@/types';

export function ExecutionPanel() {
  const isOpen = useUIStore(selectIsExecutionPanelOpen);
  const setExecutionPanelOpen = useUIStore((s) => s.setExecutionPanelOpen);
  const nodeReportsMap = useExecutionStore(selectNodeReportsMap);
  const completedNodeCount = useExecutionStore(selectCompletedNodeCount);
  const totalNodes = useExecutionStore(selectTotalNodes);
  const isExecuting = useExecutionStore(selectIsExecuting);
  const report = useExecutionStore(selectReport);
  const getNodeExecutionReport = useExecutionStore((s) => s.getNodeExecutionReport);

  // 当前选中的节点 UUID
  const [selectedNodeUuid, setSelectedNodeUuid] = useState<string | null>(null);

  // 将 nodeReportsMap 转为有序数组（按完成顺序）
  const nodeReports = useMemo<NodeExecutionReport[]>(() => {
    const reports = Object.values(nodeReportsMap);
    // 如果有最终报告，用其顺序；否则按 map 中的顺序
    if (report) {
      const orderMap = new Map(report.node_reports.map((r, i) => [r.node_uuid, i]));
      return reports.sort((a, b) => {
        const ia = orderMap.get(a.node_uuid) ?? 0;
        const ib = orderMap.get(b.node_uuid) ?? 0;
        return ia - ib;
      });
    }
    return reports;
  }, [nodeReportsMap, report]);

  // 进度百分比
  const progressPercent = totalNodes > 0 ? Math.round((completedNodeCount / totalNodes) * 100) : 0;

  // 是否全部完成
  const isComplete = !isExecuting && completedNodeCount > 0;

  // 选中节点的报告
  const selectedReport = selectedNodeUuid ? getNodeExecutionReport(selectedNodeUuid) : null;

  if (!isOpen) return null;

  return (
    <div
      className={cn(
        'h-full flex flex-col bg-background overflow-hidden',
        'border-l border-border'
      )}
      style={{ width: 320 }}
    >
      {/* ===== 头部 ===== */}
      <div className="shrink-0 border-b border-border px-3 py-2.5">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
            {isExecuting ? (
              <Loader2 className="w-4 h-4 text-blue-500 animate-spin" />
            ) : isComplete ? (
              <CheckCircle2 className="w-4 h-4 text-emerald-500" />
            ) : (
              <div className="w-4 h-4 rounded-full border-2 border-border" />
            )}
            执行进度
          </h3>
          <button
            type="button"
            onClick={() => setExecutionPanelOpen(false)}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
            title="关闭面板"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 进度条 */}
        <div className="space-y-1">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{completedNodeCount} / {totalNodes} 节点</span>
            <span>{progressPercent}%</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div
              className={cn(
                'h-full rounded-full transition-all duration-500 ease-out',
                isComplete
                  ? 'bg-emerald-500'
                  : 'bg-blue-500'
              )}
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </div>
      </div>

      {/* ===== 节点列表 ===== */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {nodeReports.length > 0 ? (
          <div className="p-2 space-y-1">
            {nodeReports.map((nodeReport) => (
              <NodeListItem
                key={nodeReport.node_uuid}
                report={nodeReport}
                isSelected={selectedNodeUuid === nodeReport.node_uuid}
                onClick={() => setSelectedNodeUuid(
                  selectedNodeUuid === nodeReport.node_uuid ? null : nodeReport.node_uuid
                )}
              />
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground text-sm gap-2">
            {isExecuting ? (
              <>
                <Loader2 className="w-6 h-6 animate-spin" />
                <span>等待节点完成...</span>
              </>
            ) : (
              <span>暂无执行记录</span>
            )}
          </div>
        )}
      </div>

      {/* ===== 节点详情区域 ===== */}
      {selectedReport && (
        <div className="shrink-0 border-t border-border overflow-y-auto" style={{ maxHeight: '40%' }}>
          <div className="p-2">
            <NodeExecutionDetails report={selectedReport} />
          </div>
        </div>
      )}
    </div>
  );
}
