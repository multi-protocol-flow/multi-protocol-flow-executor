/**
 * VariableTimelineDrawer - 变量时空追踪抽屉
 * 呈现特定变量在流程执行历史中随节点的演进过程
 */
import { useMemo } from 'react';
import { X, History, MapPin, Code } from 'lucide-react';
import { useDebugStore, useFlowStore } from '@/stores';
import { getNodeMeta } from '@/lib/nodeMeta';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface VariableTimelineDrawerProps {
  /** 目标变量名，为 null 时不显示 */
  varName: string | null;
  /** 关闭回调 */
  onClose: () => void;
}

interface TimelineItem {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  timestamp: number;
  oldValue?: unknown;
  newValue: unknown;
  isInitial: boolean;
}

function formatTime(timestamp: number): string {
  if (!timestamp) return '';
  const d = new Date(timestamp);
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  const sss = String(d.getMilliseconds()).padStart(3, '0');
  return `${hh}:${mm}:${ss}.${sss}`;
}

export function VariableTimelineDrawer({ varName, onClose }: VariableTimelineDrawerProps) {
  const ll = getLL();
  const executionStack = useDebugStore((s) => s.executionStack);

  const timelineItems = useMemo<TimelineItem[]>(() => {
    if (!varName) return [];
    const items: TimelineItem[] = [];

    for (const entry of executionStack) {
      if (!entry.variable_changes || entry.variable_changes.length === 0) continue;
      for (const change of entry.variable_changes) {
        const changeKey = change.key;
        if (changeKey === varName) {
          const isInitial = change.old_value === undefined || change.old_value === null;
          items.push({
            nodeUuid: entry.node_uuid,
            nodeName: entry.node_name,
            nodeType: entry.node_type,
            timestamp: entry.started_at,
            oldValue: change.old_value,
            newValue: change.new_value,
            isInitial,
          });
        }
      }
    }

    return items;
  }, [varName, executionStack]);

  if (!varName) return null;

  const handleLocateNode = (nodeUuid: string) => {
    useFlowStore.getState().centerOnNode?.(nodeUuid);
  };

  return (
    <div className="absolute inset-y-0 right-0 w-80 sm:w-96 bg-background/95 backdrop-blur border-l border-border shadow-2xl z-30 flex flex-col animate-in slide-in-from-right duration-200">
      {/* 顶部标题栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-muted/40">
        <div className="flex items-center gap-2 min-w-0">
          <History className="w-4 h-4 text-teal-500 shrink-0" />
          <h3 className="font-semibold text-sm truncate" title={varName}>
            {ll.flow.variable_timeline_title({ name: varName })}
          </h3>
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
          title={ll.flow.cancel()}
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* 内容区域 */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {timelineItems.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center text-muted-foreground p-6">
            <History className="w-8 h-8 stroke-1 text-muted-foreground/50 mb-2" />
            <p className="text-xs">{ll.flow.variable_timeline_empty()}</p>
          </div>
        ) : (
          <div className="relative pl-6 space-y-6 before:absolute before:left-2.5 before:top-2 before:bottom-2 before:w-0.5 before:bg-border">
            {timelineItems.map((item, idx) => {
              const meta = getNodeMeta(item.nodeType);
              const NodeIcon = meta?.icon ?? Code;
              const iconBgClass = meta?.iconBg ?? 'bg-muted text-foreground';

              return (
                <div key={`${item.nodeUuid}-${idx}`} className="relative group">
                  {/* 时间线圆点 */}
                  <div
                    className={cn(
                      'absolute -left-6 top-1 w-3 h-3 rounded-full border-2 border-background transition-transform group-hover:scale-125',
                      item.isInitial ? 'bg-green-500' : 'bg-amber-500'
                    )}
                  />

                  {/* 节点卡片 */}
                  <div className="bg-card border border-border rounded-lg p-3 shadow-xs hover:border-primary/50 transition-colors">
                    <div className="flex items-center justify-between gap-2 mb-2">
                      <div className="flex items-center gap-1.5 min-w-0">
                        <div
                          className={cn('w-5 h-5 rounded flex items-center justify-center shrink-0 text-white', iconBgClass)}
                        >
                          <NodeIcon className="w-3 h-3" />
                        </div>
                        <span className="font-medium text-xs truncate" title={item.nodeName}>
                          {item.nodeName}
                        </span>
                      </div>

                      <button
                        onClick={() => handleLocateNode(item.nodeUuid)}
                        className="opacity-0 group-hover:opacity-100 flex items-center gap-1 text-[11px] text-primary hover:underline transition-opacity shrink-0"
                        title={ll.flow.variable_timeline_locate()}
                      >
                        <MapPin className="w-3 h-3" />
                        <span>{ll.flow.variable_timeline_locate()}</span>
                      </button>
                    </div>

                    <div className="flex items-center justify-between text-[11px] text-muted-foreground mb-2">
                      <span>{formatTime(item.timestamp)}</span>
                      <span
                        className={cn(
                          'px-1.5 py-0.5 rounded text-[10px] font-medium',
                          item.isInitial
                            ? 'bg-green-500/10 text-green-700 dark:text-green-300'
                            : 'bg-amber-500/10 text-amber-700 dark:text-amber-300'
                        )}
                      >
                        {item.isInitial
                          ? ll.flow.variable_timeline_created()
                          : ll.flow.variable_timeline_updated()}
                      </span>
                    </div>

                    {/* 变动数值呈现 */}
                    <div className="text-xs font-mono bg-muted/40 rounded p-2 overflow-x-auto">
                      {!item.isInitial && (
                        <div className="mb-1 text-muted-foreground line-through opacity-80 break-all">
                          <span className="text-[10px] uppercase font-sans tracking-wide text-muted-foreground/70 block">
                            {ll.flow.variable_timeline_old_val()}:
                          </span>
                          {JSON.stringify(item.oldValue)}
                        </div>
                      )}
                      <div className="text-foreground font-semibold break-all">
                        {!item.isInitial && (
                          <span className="text-[10px] uppercase font-sans tracking-wide text-muted-foreground/70 block">
                            {ll.flow.variable_timeline_new_val()}:
                          </span>
                        )}
                        {JSON.stringify(item.newValue)}
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
