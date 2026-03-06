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

/** 面板标题栏高度 */
const HEADER_HEIGHT = 32;

/** 流程列表折叠状态下的内容高度 */
const FLOW_LIST_COLLAPSED_HEIGHT = HEADER_HEIGHT;

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

  return (
    <div className="h-full flex flex-col bg-white">
      {/* ===== 顶部：流程列表（自动扩展填充剩余空间） ===== */}
      <CollapsiblePanel
        title="流程列表"
        isCollapsed={isFlowListCollapsed}
        onToggle={toggleFlowListCollapsed}
        collapsedHeight={FLOW_LIST_COLLAPSED_HEIGHT}
        className={isFlowListCollapsed ? "shrink-0" : "flex-1 min-h-0"}
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
      <div className="shrink-0 flex flex-col mt-auto">
        {/* 节点库 */}
        <CollapsiblePanel
          title="节点库"
          isCollapsed={isNodeLibraryCollapsed}
          onToggle={toggleNodeLibraryCollapsed}
          height={nodeLibraryHeight}
          onHeightChange={setNodeLibraryHeight}
          showDragHandle={true}
          resizable={!isNodeLibraryCollapsed}
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
            showDragHandle={true}
            resizable={!isVariablePanelCollapsed}
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
