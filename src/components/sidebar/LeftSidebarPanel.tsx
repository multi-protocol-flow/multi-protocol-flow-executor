/**
 * LeftSidebarPanel - 左侧面板容器
 * 管理 VSCode 风格的面板布局：顶部固定 + 底部可折叠面板区
 */
import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CollapsiblePanel } from '@/components/ui/CollapsiblePanel';
import { FlowListPanel, NodeLibrary } from '@/components/sidebar';
import { VariablePanel } from '@/components/panels';
import { useUIStore, useFlowStore } from '@/stores';
import { useRef, useState, useEffect, useCallback } from 'react';

/** 面板标题栏高度 */
const HEADER_HEIGHT = 32;

/** 流程列表折叠状态下的内容高度 */
const FLOW_LIST_COLLAPSED_HEIGHT = HEADER_HEIGHT;

/** 流程列表最小高度（保证不会被完全挤掉） */
const FLOW_LIST_MIN_HEIGHT = 100;

/** 底部面板最小高度 */
const BOTTOM_PANEL_MIN_HEIGHT = 100;

export function LeftSidebarPanel() {
  const flow = useFlowStore((state) => state.flow);
  const nodes = flow?.nodes ?? [];

  // UI 状态
  const isFlowListCollapsed = useUIStore((state) => state.isFlowListCollapsed);
  const isNodeLibraryCollapsed = useUIStore((state) => state.isNodeLibraryCollapsed);
  const isVariablePanelCollapsed = useUIStore((state) => state.isVariablePanelCollapsed);
  const nodeLibraryHeight = useUIStore((state) => state.nodeLibraryHeight);
  const variablePanelHeight = useUIStore((state) => state.variablePanelHeight);

  // Actions
  const toggleFlowListCollapsed = useUIStore((state) => state.toggleFlowListCollapsed);
  const toggleNodeLibraryCollapsed = useUIStore((state) => state.toggleNodeLibraryCollapsed);
  const toggleVariablePanelCollapsed = useUIStore((state) => state.toggleVariablePanelCollapsed);
  const setNodeLibraryHeight = useUIStore((state) => state.setNodeLibraryHeight);
  const setVariablePanelHeight = useUIStore((state) => state.setVariablePanelHeight);
  const openNewFlowDialog = useUIStore((state) => state.openNewFlowDialog);

  // 容器引用和可用高度计算
  const containerRef = useRef<HTMLDivElement>(null);
  const [maxBottomPanelHeight, setMaxBottomPanelHeight] = useState(400);

  // 计算底部面板区的最大可用高度
  const updateMaxHeight = useCallback(() => {
    if (!containerRef.current) return;
    const containerHeight = containerRef.current.clientHeight;
    // 流程列表至少保留最小高度 + 标题栏
    const flowListReserved = isFlowListCollapsed 
      ? FLOW_LIST_COLLAPSED_HEIGHT 
      : FLOW_LIST_MIN_HEIGHT + HEADER_HEIGHT;
    const available = containerHeight - flowListReserved;
    setMaxBottomPanelHeight(Math.max(BOTTOM_PANEL_MIN_HEIGHT, available));
  }, [isFlowListCollapsed]);

  useEffect(() => {
    updateMaxHeight();
    
    // 监听窗口大小变化
    window.addEventListener('resize', updateMaxHeight);
    return () => window.removeEventListener('resize', updateMaxHeight);
  }, [updateMaxHeight]);

  // 折叠状态变化时重新计算
  useEffect(() => {
    // 延迟一帧确保 DOM 更新完成
    requestAnimationFrame(updateMaxHeight);
  }, [updateMaxHeight]);

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-white overflow-hidden">
      {/* ===== 顶部：流程列表（自动扩展填充剩余空间） ===== */}
      <CollapsiblePanel
        title="流程列表"
        isCollapsed={isFlowListCollapsed}
        onToggle={toggleFlowListCollapsed}
        collapsedHeight={FLOW_LIST_COLLAPSED_HEIGHT}
        className={isFlowListCollapsed ? "shrink-0" : "flex-1 min-h-0"}
        style={!isFlowListCollapsed ? { minHeight: FLOW_LIST_MIN_HEIGHT + HEADER_HEIGHT } : undefined}
        headerRight={
          <Button
            variant="ghost"
            size="icon"
            onClick={(e) => {
              e.stopPropagation();
              openNewFlowDialog();
            }}
            className="h-6 w-6 rounded-md hover:bg-slate-100"
            title="新建流程"
          >
            <Plus className="w-3.5 h-3.5" />
          </Button>
        }
      >
        {/* 流程列表内容（不含标题栏） */}
        <FlowListPanel showHeader={false} />
      </CollapsiblePanel>

      {/* ===== 底部面板区：节点库 + 变量面板 ===== */}
      <div className="shrink-0 flex flex-col mt-auto overflow-hidden">
        {/* 节点库 */}
        <CollapsiblePanel
          title="节点库"
          isCollapsed={isNodeLibraryCollapsed}
          onToggle={toggleNodeLibraryCollapsed}
          height={nodeLibraryHeight}
          onHeightChange={setNodeLibraryHeight}
          resizable={!isNodeLibraryCollapsed}
          maxHeight={maxBottomPanelHeight}
          className="border-t border-border"
        >
          {/* 节点库内容（不含标题栏） */}
          <NodeLibrary showHeader={false} />
        </CollapsiblePanel>

        {/* 变量面板（仅在有流程时显示） */}
        {flow && (
          <CollapsiblePanel
            title="变量面板"
            isCollapsed={isVariablePanelCollapsed}
            onToggle={toggleVariablePanelCollapsed}
            height={variablePanelHeight}
            onHeightChange={setVariablePanelHeight}
            resizable={!isVariablePanelCollapsed}
            maxHeight={maxBottomPanelHeight}
            className="border-t border-border"
          >
            {/* 变量面板内容（不含标题栏） */}
            <VariablePanel flow={flow} nodes={nodes} showHeader={false} />
          </CollapsiblePanel>
        )}
      </div>
    </div>
  );
}
