/**
 * SelectionAlignmentBar - 多选节点快捷排版工具条
 * 当画布多选选中 >= 2 个节点时在画布顶部居中浮出，支持一键水平/垂直对齐与均匀分布
 */
import { memo, useCallback } from 'react';
import type { Node } from '@xyflow/react';
import {
  AlignLeft,
  AlignCenterHorizontal,
  AlignRight,
  AlignStartVertical,
  AlignCenterVertical,
  AlignEndVertical,
  AlignHorizontalSpaceBetween,
  AlignVerticalSpaceBetween,
} from 'lucide-react';
import { useFlowStore } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';
import { Button } from '@/components/ui/button';

export interface SelectionAlignmentBarProps {
  selectedNodes: Node[];
}

export const SelectionAlignmentBar = memo(function SelectionAlignmentBar({
  selectedNodes,
}: SelectionAlignmentBarProps) {
  const ll = getLL();
  const batchUpdateNodePositions = useFlowStore((s) => s.batchUpdateNodePositions);

  const getNodeWidth = (n: Node) => n.measured?.width ?? n.width ?? 180;
  const getNodeHeight = (n: Node) => n.measured?.height ?? n.height ?? 60;

  // 左对齐
  const handleAlignLeft = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const minX = Math.min(...selectedNodes.map((n) => n.position.x));
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, { x: minX, y: n.position.y });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 水平居中对齐
  const handleAlignCenterH = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const centers = selectedNodes.map((n) => n.position.x + getNodeWidth(n) / 2);
    const avgCenter = centers.reduce((a, b) => a + b, 0) / centers.length;
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, {
        x: Math.round(avgCenter - getNodeWidth(n) / 2),
        y: n.position.y,
      });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 右对齐
  const handleAlignRight = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const maxRight = Math.max(...selectedNodes.map((n) => n.position.x + getNodeWidth(n)));
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, {
        x: Math.round(maxRight - getNodeWidth(n)),
        y: n.position.y,
      });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 顶对齐
  const handleAlignTop = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const minY = Math.min(...selectedNodes.map((n) => n.position.y));
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, { x: n.position.x, y: minY });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 垂直居中对齐
  const handleAlignCenterV = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const centers = selectedNodes.map((n) => n.position.y + getNodeHeight(n) / 2);
    const avgCenter = centers.reduce((a, b) => a + b, 0) / centers.length;
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, {
        x: n.position.x,
        y: Math.round(avgCenter - getNodeHeight(n) / 2),
      });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 底对齐
  const handleAlignBottom = useCallback(() => {
    if (selectedNodes.length < 2) return;
    const maxBottom = Math.max(...selectedNodes.map((n) => n.position.y + getNodeHeight(n)));
    const positions = new Map<string, { x: number; y: number }>();
    for (const n of selectedNodes) {
      positions.set(n.id, {
        x: n.position.x,
        y: Math.round(maxBottom - getNodeHeight(n)),
      });
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 水平均匀分布（>= 3 个节点）
  const handleDistributeH = useCallback(() => {
    if (selectedNodes.length < 3) return;
    const sorted = [...selectedNodes].sort((a, b) => a.position.x - b.position.x);
    const first = sorted[0];
    const last = sorted[sorted.length - 1];
    const totalSpan = last.position.x + getNodeWidth(last) - first.position.x;
    const totalNodeWidth = sorted.reduce((acc, n) => acc + getNodeWidth(n), 0);
    const totalGap = Math.max(0, totalSpan - totalNodeWidth);
    const gap = totalGap / (sorted.length - 1);

    const positions = new Map<string, { x: number; y: number }>();
    let currentX = first.position.x;
    for (const n of sorted) {
      positions.set(n.id, { x: Math.round(currentX), y: n.position.y });
      currentX += getNodeWidth(n) + gap;
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  // 垂直均匀分布（>= 3 个节点）
  const handleDistributeV = useCallback(() => {
    if (selectedNodes.length < 3) return;
    const sorted = [...selectedNodes].sort((a, b) => a.position.y - b.position.y);
    const first = sorted[0];
    const last = sorted[sorted.length - 1];
    const totalSpan = last.position.y + getNodeHeight(last) - first.position.y;
    const totalNodeHeight = sorted.reduce((acc, n) => acc + getNodeHeight(n), 0);
    const totalGap = Math.max(0, totalSpan - totalNodeHeight);
    const gap = totalGap / (sorted.length - 1);

    const positions = new Map<string, { x: number; y: number }>();
    let currentY = first.position.y;
    for (const n of sorted) {
      positions.set(n.id, { x: n.position.x, y: Math.round(currentY) });
      currentY += getNodeHeight(n) + gap;
    }
    batchUpdateNodePositions(positions);
  }, [selectedNodes, batchUpdateNodePositions]);

  if (selectedNodes.length < 2) return null;

  return (
    <div className="absolute top-14 left-1/2 -translate-x-1/2 z-20 flex items-center gap-0.5 bg-background/95 backdrop-blur border border-border rounded-lg shadow-lg px-2 py-1 animate-in fade-in zoom-in-95 duration-150">
      <span className="text-[11px] font-semibold text-muted-foreground mr-1.5 px-1 select-none">
        {selectedNodes.length}
      </span>
      <div className="flex items-center gap-0.5">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignLeft}
          title={ll.flow.align_left()}
        >
          <AlignLeft className="w-3.5 h-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignCenterH}
          title={ll.flow.align_center_h()}
        >
          <AlignCenterHorizontal className="w-3.5 h-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignRight}
          title={ll.flow.align_right()}
        >
          <AlignRight className="w-3.5 h-3.5" />
        </Button>
      </div>

      <div className="w-[1px] h-4 bg-border mx-1" />

      <div className="flex items-center gap-0.5">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignTop}
          title={ll.flow.align_top()}
        >
          <AlignStartVertical className="w-3.5 h-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignCenterV}
          title={ll.flow.align_center_v()}
        >
          <AlignCenterVertical className="w-3.5 h-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
          onClick={handleAlignBottom}
          title={ll.flow.align_bottom()}
        >
          <AlignEndVertical className="w-3.5 h-3.5" />
        </Button>
      </div>

      {selectedNodes.length >= 3 && (
        <>
          <div className="w-[1px] h-4 bg-border mx-1" />
          <div className="flex items-center gap-0.5">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
              onClick={handleDistributeH}
              title={ll.flow.distribute_h()}
            >
              <AlignHorizontalSpaceBetween className="w-3.5 h-3.5" />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
              onClick={handleDistributeV}
              title={ll.flow.distribute_v()}
            >
              <AlignVerticalSpaceBetween className="w-3.5 h-3.5" />
            </Button>
          </div>
        </>
      )}
    </div>
  );
});
