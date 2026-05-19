/**
 * LeftSidebarPanel - 左侧面板容器
 * 管理 VSCode 风格的面板布局：顶部固定 + 底部可折叠面板区
 */
import { Plus, FolderPlus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CollapsiblePanel } from '@/components/ui/CollapsiblePanel';
import { FlowListPanel } from '@/components/sidebar';
import { useUIStore, useDebugStore } from '@/stores';
import { selectIsDebugging } from '@/stores/debug';
import { useRef, useCallback } from 'react';
import { getLL } from '@/i18n/i18n-lazy';

/** 面板标题栏高度 */
const HEADER_HEIGHT = 32;

/** 流程列表折叠状态下的内容高度 */
const FLOW_LIST_COLLAPSED_HEIGHT = HEADER_HEIGHT;

/** 流程列表最小高度（保证不会被完全挤掉） */
const FLOW_LIST_MIN_HEIGHT = 100;

export function LeftSidebarPanel() {
  const LL = getLL();
  // UI 状态
  const isFlowListCollapsed = useUIStore((state) => state.isFlowListCollapsed);
  
  // 调试状态
  const isDebugging = useDebugStore(selectIsDebugging);
  const debugSessionId = useDebugStore((state) => state.sessionId);

  // Actions
  const toggleFlowListCollapsed = useUIStore((state) => state.toggleFlowListCollapsed);
  const openNewFlowDialog = useUIStore((state) => state.openNewFlowDialog);

  // 容器引用
  const containerRef = useRef<HTMLDivElement>(null);

  // 新建文件夹回调引用
  const createFolderTriggerRef = useRef<(() => void) | null>(null);

  const handleCreateFolderClick = useCallback(() => {
    createFolderTriggerRef.current?.();
  }, []);

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-background overflow-hidden">
      {/* ===== 顶部：流程列表（自动扩展填充剩余空间） ===== */}
      <CollapsiblePanel
        title="流程列表"
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
    </div>
  );
}
