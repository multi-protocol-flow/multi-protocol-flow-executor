/**
 * LeftSidebarPanel - 左侧面板容器
 * 管理 VSCode 风格的面板布局：顶部固定 + 底部可折叠面板区
 */
import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CollapsiblePanel } from '@/components/ui/CollapsiblePanel';
import { FlowListPanel } from '@/components/sidebar';
import { NodeConfigPanel } from '@/components/panels';
import { useUIStore, useFlowStore, useDebugStore } from '@/stores';
import { selectIsDebugging } from '@/stores/debug';
import { useRef } from 'react';

/** 面板标题栏高度 */
const HEADER_HEIGHT = 32;

/** 流程列表折叠状态下的内容高度 */
const FLOW_LIST_COLLAPSED_HEIGHT = HEADER_HEIGHT;

/** 流程列表最小高度（保证不会被完全挤掉） */
const FLOW_LIST_MIN_HEIGHT = 100;

export function LeftSidebarPanel() {
  // 获取选中节点（用于调试模式下的只读配置显示）
  const getSelectedNode = useFlowStore((state) => state.getSelectedNode);

  // UI 状态
  const isFlowListCollapsed = useUIStore((state) => state.isFlowListCollapsed);
  
  // 调试状态
  const isDebugging = useDebugStore(selectIsDebugging);

  // Actions
  const toggleFlowListCollapsed = useUIStore((state) => state.toggleFlowListCollapsed);
  const openNewFlowDialog = useUIStore((state) => state.openNewFlowDialog);
  
  // 获取当前选中的节点
  const selectedNode = getSelectedNode();

  // 容器引用
  const containerRef = useRef<HTMLDivElement>(null);

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden">
      {/* 调试模式：整个左侧面板变成节点只读配置面板 */}
      {isDebugging ? (
        <NodeConfigPanel readOnly node={selectedNode} />
      ) : (
        <>
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
                className="h-6 w-6 rounded-md hover:bg-accent"
                title="新建流程"
              >
                <Plus className="w-3.5 h-3.5" />
              </Button>
            }
          >
            {/* 流程列表内容（不含标题栏） */}
            <FlowListPanel showHeader={false} />
          </CollapsiblePanel>
        </>
      )}
    </div>
  );
}
