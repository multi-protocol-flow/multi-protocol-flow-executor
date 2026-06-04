/**
 * NodeVariableTooltip - 节点变量浮窗
 * 执行时/执行后，悬停在节点上可查看该节点的输入/输出变量
 */
import { useExecutionStore } from '@/stores/execution';
import { VariableTreeView } from '@/components/execution/VariableTreeView';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import type { Node } from '@xyflow/react';

interface NodeVariableTooltipProps {
  /** 当前悬停的节点（null 表示未悬停） */
  hoveredNode: Node | null;
  /** 浮窗位置 */
  position: { x: number; y: number };
}

export function NodeVariableTooltip({ hoveredNode, position }: NodeVariableTooltipProps) {
  const report = useExecutionStore(s => s.report);
  const nodeReportsMap = useExecutionStore(s => s.nodeReportsMap);

  if (!hoveredNode) return null;

  // 从增量 nodeReportsMap 或最终报告中获取节点报告
  const nodeReport = nodeReportsMap[hoveredNode.id]
    ?? report?.node_reports.find(r => r.node_uuid === hoveredNode.id);

  if (!nodeReport) return null;

  // input_data 和 output_data 可能是任意类型，仅当为对象时展示
  const inputVars = (typeof nodeReport.input_data === 'object' && nodeReport.input_data !== null)
    ? (nodeReport.input_data as Record<string, unknown>)
    : {};
  const outputVars = (typeof nodeReport.output_data === 'object' && nodeReport.output_data !== null)
    ? (nodeReport.output_data as Record<string, unknown>)
    : {};

  const hasData = Object.keys(inputVars).length > 0 || Object.keys(outputVars).length > 0;
  if (!hasData) return null;

  return (
    <div
      className="fixed z-[100] pointer-events-none"
      style={{
        left: position.x + 16,
        top: position.y - 8,
      }}
    >
      <Card className="w-72 shadow-xl border-border/50 bg-background/95 backdrop-blur-sm">
        <CardHeader className="py-2 px-3">
          <CardTitle className="text-xs font-semibold text-muted-foreground">
            {typeof hoveredNode.data?.label === 'string' ? hoveredNode.data.label : hoveredNode.id}
          </CardTitle>
        </CardHeader>
        <CardContent className="py-2 px-3 space-y-2 max-h-60 overflow-y-auto">
          {Object.keys(inputVars).length > 0 && (
            <div>
              <div className="text-[10px] font-medium text-muted-foreground mb-1 uppercase tracking-wider">
                输入
              </div>
              <VariableTreeView variables={inputVars} />
            </div>
          )}
          {Object.keys(outputVars).length > 0 && (
            <div>
              <div className="text-[10px] font-medium text-muted-foreground mb-1 uppercase tracking-wider">
                输出
              </div>
              <VariableTreeView variables={outputVars} />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
