/**
 * LeftSidebarPanel - 左侧面板容器
 * 管理 VSCode 风格的面板布局：顶部固定 + 底部可折叠面板区
 */
import { useState, useCallback, useRef, useEffect, memo } from 'react';
import { Plus, FolderPlus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CollapsiblePanel } from '@/components/ui/CollapsiblePanel';
import { FlowListPanel } from '@/components/sidebar';
import { DataSourcePanel } from '@/components/sidebar/DataSourcePanel';
import { FlowNodeList } from '@/components/panels/FlowNodeList';
import { useUIStore, useDebugStore, useFlowStore } from '@/stores';
import { selectIsDebugging } from '@/stores/debug';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
/** 面板标题栏高度 */
const HEADER_HEIGHT = 32;

/** 流程列表折叠状态下的内容高度 */
const FLOW_LIST_COLLAPSED_HEIGHT = HEADER_HEIGHT;

/** 流程列表最小高度（保证不会被完全挤掉） */
const FLOW_LIST_MIN_HEIGHT = 100;

/** 数据源面板默认高度 */
const DATASOURCE_PANEL_DEFAULT_HEIGHT = 280;
/** 数据源面板最小高度 */
const DATASOURCE_PANEL_MIN_HEIGHT = 100;

/** 节点大纲默认高度 */
const NODE_OUTLINE_DEFAULT_HEIGHT = 220;

/** 节点大纲最小高度 */
const NODE_OUTLINE_MIN_HEIGHT = 80;
export const LeftSidebarPanel = memo(function LeftSidebarPanel() {
  const LL = getLL();
  // UI 状态
  const isFlowListCollapsed = useUIStore((state) => state.isFlowListCollapsed);

  // 数据源面板折叠状态（默认折叠）
  const [isDatasourceCollapsed, setIsDatasourceCollapsed] = useState(true);
  // 数据源面板高度（支持拖拽调整）
  const [datasourceHeight, setDatasourceHeight] = useState(DATASOURCE_PANEL_DEFAULT_HEIGHT);
  // 流程节点大纲面板状态（默认展开）
  const [isNodeOutlineCollapsed, setIsNodeOutlineCollapsed] = useState(false);
  const [nodeOutlineHeight, setNodeOutlineHeight] = useState(NODE_OUTLINE_DEFAULT_HEIGHT);

  const flow = useFlowStore((state) => state.flow);
  const nodesCount = flow?.nodes?.length ?? 0;
  const isDebugging = useDebugStore(selectIsDebugging);
  const debugSessionId = useDebugStore((state) => state.sessionId);
  // Actions
  const toggleFlowListCollapsed = useUIStore((state) => state.toggleFlowListCollapsed);
  const openNewFlowDialog = useUIStore((state) => state.openNewFlowDialog);

  // 容器引用与高度监听
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerHeight, setContainerHeight] = useState(600);

  useEffect(() => {
    if (!containerRef.current) return;
    let animationFrameId: number | null = null;
    const observer = new ResizeObserver((entries) => {
      const updateHeight = () => {
        for (const entry of entries) {
          const height = entry.contentRect.height;
          if (height > 0) {
            setContainerHeight((prev) => (Math.abs(prev - height) > 1 ? height : prev));
          }
        }
      };

      if (typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
        if (animationFrameId !== null) {
          window.cancelAnimationFrame(animationFrameId);
        }
        animationFrameId = window.requestAnimationFrame(updateHeight);
      } else {
        updateHeight();
      }
    });
    observer.observe(containerRef.current);
    return () => {
      if (animationFrameId !== null && typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
        window.cancelAnimationFrame(animationFrameId);
      }
      observer.disconnect();
    };
  }, []);

  // 新建文件夹回调引用
  const createFolderTriggerRef = useRef<(() => void) | null>(null);

  const handleCreateFolderClick = useCallback(() => {
    createFolderTriggerRef.current?.();
  }, []);

  // 切换数据源折叠
  const handleToggleDatasource = useCallback(() => {
    setIsDatasourceCollapsed((prev) => !prev);
  }, []);

  // 切换节点大纲折叠
  const handleToggleNodeOutline = useCallback(() => {
    setIsNodeOutlineCollapsed((prev) => !prev);
  }, []);

  const maxNodeOutlineHeight = Math.max(
    NODE_OUTLINE_MIN_HEIGHT + 40,
    containerHeight - (FLOW_LIST_MIN_HEIGHT + HEADER_HEIGHT * 2)
  );

  const maxDatasourceHeight = Math.max(
    DATASOURCE_PANEL_MIN_HEIGHT + 50,
    containerHeight - (FLOW_LIST_MIN_HEIGHT + HEADER_HEIGHT * (flow ? 2 : 1))
  );
  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden" data-testid="sidebar-flow-list">
      {/* ===== 顶部：流程列表（自动扩展填充剩余空间） ===== */}
      <CollapsiblePanel
        title={LL.flow.flow_list_title()}
        isCollapsed={isFlowListCollapsed}
        onToggle={toggleFlowListCollapsed}
        collapsedHeight={FLOW_LIST_COLLAPSED_HEIGHT}
        className={isFlowListCollapsed ? "shrink-0" : "flex-1 min-h-0"}
        style={!isFlowListCollapsed ? { minHeight: FLOW_LIST_MIN_HEIGHT + HEADER_HEIGHT } : undefined}
        headerRight={
          <div className="flex items-center gap-0.5">
            <Button
              variant="ghost"
              size="icon"
              onClick={(e) => {
                e.stopPropagation();
                if (!isDebugging) {
                  handleCreateFolderClick();
                }
              }}
              disabled={isDebugging}
              className="h-6 w-6 rounded-md hover:bg-accent disabled:opacity-50"
              title={isDebugging ? LL.flow.debug_no_create_folder() : LL.flow.new_folder()}
            >
              <FolderPlus className="w-3.5 h-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={(e) => {
                e.stopPropagation();
                if (!isDebugging) {
                  openNewFlowDialog();
                }
              }}
              disabled={isDebugging}
              className="h-6 w-6 rounded-md hover:bg-accent disabled:opacity-50"
              title={isDebugging ? LL.flow.debug_no_new_flow() : LL.flow.new_flow()}
              data-guide="new-flow-btn"
            >
              <Plus className="w-3.5 h-3.5" />
            </Button>
          </div>
        }
      >
        {/* 流程列表内容（不含标题栏） */}
        <FlowListPanel
          showHeader={false}
          disabled={isDebugging}
          debugFlowUuid={debugSessionId}
          onCreateFolderTrigger={(fn) => { createFolderTriggerRef.current = fn; }}
        />
      </CollapsiblePanel>

      {/* ===== 中间：流程节点大纲（仅在当前打开流程时显示） ===== */}
      {flow && (
        <CollapsiblePanel
          title={LL.panels.flowNodeList.title()}
          isCollapsed={isNodeOutlineCollapsed}
          onToggle={handleToggleNodeOutline}
          collapsedHeight={HEADER_HEIGHT}
          height={
            isNodeOutlineCollapsed
              ? undefined
              : isFlowListCollapsed
              ? undefined
              : nodeOutlineHeight
          }
          onHeightChange={setNodeOutlineHeight}
          resizable={!isFlowListCollapsed && !isNodeOutlineCollapsed}
          minHeight={NODE_OUTLINE_MIN_HEIGHT}
          maxHeight={maxNodeOutlineHeight}
          className={cn(
            isNodeOutlineCollapsed
              ? 'shrink-0 border-t border-border'
              : isFlowListCollapsed
              ? 'flex-1 min-h-0 border-t border-border'
              : 'shrink-0 border-t border-border'
          )}
          headerRight={
            <span className="text-xs text-muted-foreground font-normal mr-1">
              ({nodesCount})
            </span>
          }
        >
          <FlowNodeList showHeader={false} readOnly={isDebugging} />
        </CollapsiblePanel>
      )}

      {/* ===== 底部：数据源面板（可折叠、可拖拽调高、折叠置底） ===== */}
      <CollapsiblePanel
        title={LL.datasource.title()}
        isCollapsed={isDatasourceCollapsed}
        onToggle={handleToggleDatasource}
        collapsedHeight={HEADER_HEIGHT}
        height={
          isDatasourceCollapsed
            ? undefined
            : isFlowListCollapsed
            ? undefined
            : datasourceHeight
        }
        onHeightChange={setDatasourceHeight}
        resizable={!isFlowListCollapsed && !isDatasourceCollapsed}
        minHeight={DATASOURCE_PANEL_MIN_HEIGHT}
        maxHeight={maxDatasourceHeight}
        className={cn(
          isDatasourceCollapsed
            ? 'mt-auto shrink-0 border-t border-border'
            : isFlowListCollapsed
            ? 'flex-1 min-h-0'
            : 'shrink-0 border-t border-border'
        )}
      >
        <DataSourcePanel />
      </CollapsiblePanel>
    </div>
  );
});
