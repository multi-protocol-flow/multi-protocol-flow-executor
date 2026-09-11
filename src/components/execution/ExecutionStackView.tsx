/**
 * ExecutionStackView - 执行堆栈查看器
 * 从 DebugPanel 提取的执行堆栈组件
 */
import { useState } from 'react';
import { ChevronDown, ChevronRight, Copy, Pause, CheckCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { useCurrentLocale } from '@/hooks/useLocale';
import { useDebugStore } from '@/stores';
import type { ExecutionStackEntry } from '@/types/debug';

interface ExecutionStackViewProps {
  /** 执行堆栈条目 */
  executionStack: ExecutionStackEntry[];
  /** 当前节点 UUID */
  currentNodeUuid: string | null;
  /** 是否只读模式 */
  isReadOnly?: boolean;
  /** 点击节点跳转回调 */
  onNavigateToNode?: (uuid: string) => void;
}

function StackEntryItem({
  entry,
  isCurrentNode,
  onNavigateToNode,
}: {
  entry: ExecutionStackEntry;
  isCurrentNode: boolean;
  onNavigateToNode: (uuid: string) => void;
}) {
  const ll = getLL();
  const [isExpanded, setIsExpanded] = useState(false);

  const handleCopyData = async (data: unknown, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(JSON.stringify(data, null, 2));
      toast.success(ll.execution.report.copied_to_clipboard());
    } catch {
      toast.error(ll.execution.report.copy_failed());
    }
  };

  const formatDuration = () => {
    if (!entry.ended_at || !entry.started_at) return null;
    const duration = entry.ended_at - entry.started_at;
    if (duration < 1000) return `${duration}ms`;
    return `${(duration / 1000).toFixed(2)}s`;
  };

  const hasInput = entry.input !== undefined && entry.input !== null;
  const hasOutput = entry.output !== undefined && entry.output !== null;

  return (
    <div className="border-b border-border last:border-b-0">
      <div
        className={cn(
          'flex items-center gap-2 px-3 py-2 group',
          'hover:bg-accent transition-colors',
          isCurrentNode && 'bg-blue-50 dark:bg-blue-950/50 hover:bg-blue-50 dark:hover:bg-blue-950/50'
        )}
      >
        <div
          className={cn(
            'w-5 h-5 rounded-full flex items-center justify-center text-xs',
            isCurrentNode
              ? 'bg-amber-100 dark:bg-amber-900/50 text-amber-600 dark:text-amber-400'
              : entry.success
                ? 'bg-green-100 dark:bg-green-900/50 text-green-600 dark:text-green-400'
                : 'bg-red-100 dark:bg-red-900/50 text-red-600 dark:text-red-400'
          )}
          title={isCurrentNode ? ll.execution.viewers.execution_stack_current_paused() : entry.success ? ll.execution.viewers.execution_stack_success() : ll.execution.viewers.execution_stack_failed()}
        >
          {isCurrentNode ? (
            <Pause className="w-3 h-3" />
          ) : entry.success ? (
            <CheckCircle className="w-3 h-3" />
          ) : (
            '✗'
          )}
        </div>

        {(hasInput || hasOutput) ? (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="w-4 h-4 flex items-center justify-center text-muted-foreground hover:text-accent-foreground"
          >
            {isExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
          </button>
        ) : (
          <div className="w-4 h-4" />
        )}

        <button
          onClick={() => onNavigateToNode(entry.node_uuid)}
          className="flex-1 min-w-0 text-left"
          title={ll.execution.viewers.execution_stack_click_to_jump()}
        >
          <div
            className={cn(
              'text-sm font-medium truncate',
              isCurrentNode ? 'text-blue-700 dark:text-blue-300' : 'text-foreground hover:text-blue-600 dark:hover:text-blue-400'
            )}
          >
            {entry.node_name}
          </div>
          <div className="text-xs text-muted-foreground flex items-center gap-2">
            <span>{entry.node_type}</span>
            {formatDuration() && (
              <>
                <span>·</span>
                <span>{formatDuration()}</span>
              </>
            )}
            {entry.error && (
              <>
                <span>·</span>
                <span className="text-red-500 truncate">{entry.error}</span>
              </>
            )}
          </div>
        </button>
      </div>

      {isExpanded && (hasInput || hasOutput) && (
        <div className="bg-muted px-3 pb-2 space-y-2">
          {hasInput && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-muted-foreground">{ll.execution.viewers.execution_stack_input_label()}</span>
                <button
                  onClick={(e) => handleCopyData(entry.input, e)}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-accent rounded"
                  title={ll.execution.viewers.execution_stack_copy_input()}
                >
                  <Copy className="w-3 h-3 text-muted-foreground" />
                </button>
              </div>
              <pre className="text-xs font-mono text-foreground bg-background p-2 rounded border border-border overflow-x-auto max-h-32 overflow-y-auto">
                {JSON.stringify(entry.input, null, 2)}
              </pre>
            </div>
          )}

          {hasOutput && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-muted-foreground">{ll.execution.viewers.execution_stack_output_label()}</span>
                <button
                  onClick={(e) => handleCopyData(entry.output, e)}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-accent rounded"
                  title={ll.execution.viewers.execution_stack_copy_output()}
                >
                  <Copy className="w-3 h-3 text-muted-foreground" />
                </button>
              </div>
              <pre className="text-xs font-mono text-foreground bg-background p-2 rounded border border-border overflow-x-auto max-h-32 overflow-y-auto">
                {JSON.stringify(entry.output, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function ExecutionStackView({
  executionStack,
  currentNodeUuid,
  isReadOnly,
  onNavigateToNode,
}: ExecutionStackViewProps) {
  useCurrentLocale();
  const ll = getLL();
  const navigateHandler = onNavigateToNode ?? (() => {});
  const callFrames = useDebugStore((state) => state.callFrames);
  const activeFrameIndex = useDebugStore((state) => state.activeFrameIndex);
  const setActiveFrame = useDebugStore((state) => state.setActiveFrame);

  return (
    <div className="h-full overflow-y-auto">
      {/* 分层调用栈帧展示 */}
      {callFrames.length > 1 && (
        <div className="border-b border-border bg-muted/40 p-2 space-y-1">
          <div className="text-[11px] font-semibold text-muted-foreground uppercase px-2 py-0.5 tracking-wider">
            {ll.flow.debug_call_frames_title()}
          </div>
          {callFrames.map((frame, idx) => {
            const isSelected = idx === activeFrameIndex;
            const isPausedFrame = idx === callFrames.length - 1;
            return (
              <button
                key={`${frame.flow_uuid}-${idx}`}
                onClick={() => {
                  setActiveFrame(idx);
                  navigateHandler(frame.node_uuid);
                }}
                className={cn(
                  'w-full flex items-center justify-between px-2.5 py-1.5 rounded text-xs transition-colors cursor-pointer text-left',
                  isSelected
                    ? 'bg-primary/15 text-primary font-medium'
                    : 'hover:bg-muted text-muted-foreground hover:text-foreground'
                )}
              >
                <div className="flex items-center gap-2 truncate">
                  <span className="font-mono text-[10px] px-1 py-0.5 rounded bg-muted-foreground/15">
                    #{frame.frame_id}
                  </span>
                  <span className="truncate font-medium text-foreground">
                    {frame.flow_name || (idx === 0 ? ll.flow.debug_breadcrumbs_root() : ll.flow.debug_breadcrumbs_subflow({ depth: idx }))}
                  </span>
                  <span className="text-muted-foreground truncate">
                    ({frame.node_name || frame.node_uuid})
                  </span>
                </div>
                {isPausedFrame && (
                  <span className="flex items-center gap-1 text-[10px] text-amber-500 font-semibold shrink-0">
                    <Pause className="w-2.5 h-2.5 fill-current" />
                    {ll.flow.debug_call_frames_paused()}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      )}

      {executionStack.length > 0 ? (
        executionStack.map((entry) => (
          <StackEntryItem
            key={entry.node_uuid}
            entry={entry}
            isCurrentNode={!isReadOnly && entry.node_uuid === currentNodeUuid}
            onNavigateToNode={navigateHandler}
          />
        ))
      ) : (
        <div className="p-4 text-sm text-muted-foreground text-center">
          {ll.execution.viewers.execution_stack_empty()}
        </div>
      )}
    </div>
  );
}
