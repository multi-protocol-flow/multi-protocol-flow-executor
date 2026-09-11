/**
 * DebugBreadcrumbs - 调试面包屑导航
 * 在多层子流程嵌套调试时浮动于调试菜单工具栏正下方，支持层级穿透检视与快速定位
 */
import { Fragment } from 'react';
import { ChevronRight, Pause, LocateFixed } from 'lucide-react';
import {
  useDebugStore,
  useFlowStore,
  selectIsDebugging,
  selectIsPaused,
  selectCallFrames,
  selectActiveFrameIndex,
} from '@/stores';
import { cn } from '@/lib/utils';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
export function DebugBreadcrumbs() {
  useCurrentLocale();
  const ll = getLL();
  const isDebugging = useDebugStore(selectIsDebugging);
  const isPaused = useDebugStore(selectIsPaused);
  const callFrames = useDebugStore(selectCallFrames);
  const activeFrameIndex = useDebugStore(selectActiveFrameIndex);
  const setActiveFrame = useDebugStore((state) => state.setActiveFrame);
  // 仅在调试中且栈帧深度大于 1（进入子流程）时显现
  if (!isDebugging || callFrames.length <= 1) {
    return null;
  }

  const currentPausedFrame = callFrames[callFrames.length - 1];

  return (
    <div
      data-testid="debug-breadcrumbs"
      className="absolute top-[68px] left-1/2 -translate-x-1/2 z-10 flex items-center gap-1.5 bg-background/95 backdrop-blur border border-border rounded-lg px-3 py-1.5 shadow-md text-xs select-none max-w-[90vw] overflow-x-auto whitespace-nowrap"
    >
      {callFrames.map((frame, idx) => {
        const isCurrentActive = idx === activeFrameIndex;
        return (
          <Fragment key={`${frame.flow_uuid}-${idx}`}>
            {idx > 0 && <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />}
            <button
              onClick={() => setActiveFrame(idx)}
              className={cn(
                'flex items-center gap-1 px-2 py-1 rounded transition-colors font-medium cursor-pointer',
                isCurrentActive
                  ? 'bg-primary/10 text-primary hover:bg-primary/20'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
              )}
            >
              <span className="truncate max-w-[120px]">
                {frame.flow_name || (idx === 0 ? ll.flow.debug_breadcrumbs_root() : ll.flow.debug_breadcrumbs_subflow({ depth: idx }))}
              </span>
              <span className="text-[10px] opacity-70">
                ({idx === 0 ? ll.flow.debug_breadcrumbs_root() : ll.flow.debug_breadcrumbs_depth({ depth: idx })})
              </span>
            </button>
          </Fragment>
        );
      })}

      {isPaused && currentPausedFrame && (
        <div className="flex items-center gap-1 ml-2 pl-2 border-l border-border text-amber-500 font-medium shrink-0">
          <Pause className="w-3 h-3 fill-current" />
          <span>{ll.flow.debug_breadcrumbs_paused_at()}</span>
          <span className="font-semibold text-foreground truncate max-w-[120px]">
            {currentPausedFrame.node_name || currentPausedFrame.node_uuid}
          </span>
          <button
            onClick={() => {
              setActiveFrame(callFrames.length - 1);
              useFlowStore.getState().centerOnNode(currentPausedFrame.node_uuid);
            }}
            title={ll.flow.debug_breadcrumbs_locate()}
            className="ml-1 p-1 hover:bg-muted rounded text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <LocateFixed className="w-3.5 h-3.5" />
          </button>
        </div>
      )}
    </div>
  );
}
