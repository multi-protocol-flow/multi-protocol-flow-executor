/**
 * 控制台面板组件
 * 显示实时日志，支持级别过滤和自动滚动
 */
import { useEffect, useMemo, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useConsoleStore } from '@/stores/console';
import { getLL } from '@/i18n/i18n-lazy';
import { useThrottledValue } from '@/hooks/useThrottledValue';
import { Terminal, Info, AlertTriangle, Bug, XCircle } from 'lucide-react';
import type { LogLevel } from '@/types/events';
import { useCurrentLocale } from '@/hooks/useLocale';
import { cn } from '@/lib/utils';
import { safeStringify } from '@/lib/clipboard';
const levelColors: Record<LogLevel, string> = {
  INFO: 'text-blue-500',
  DEBUG: 'text-muted-foreground',
  WARN: 'text-yellow-500',
  ERROR: 'text-red-500',
};

// 日志级别图标映射
const levelIcons: Record<LogLevel, React.ElementType> = {
  INFO: Info,
  DEBUG: Bug,
  WARN: AlertTriangle,
  ERROR: XCircle,
};

interface ConsolePanelProps {
  flowUuid?: string;
}

export function ConsolePanel({ flowUuid: _flowUuid }: ConsolePanelProps) {
  const ll = getLL();
  const locale = useCurrentLocale();
  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString(locale, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };
  const scrollRef = useRef<HTMLDivElement>(null);
  const rawLogs = useConsoleStore(useShallow((state) => state.logs));
  const logs = useThrottledValue(rawLogs);
  const filter = useConsoleStore(useShallow((state) => state.filter));

  const filteredLogs = useMemo(() => {
    return logs.filter((log) => {
      // 来源过滤 (ALL / flow / system)
      if (filter.source && filter.source !== 'ALL') {
        const logSource = log.source ?? 'flow';
        if (logSource !== filter.source) {
          return false;
        }
      }
      if (filter.level && filter.level !== 'ALL' && log.level !== filter.level) {
        return false;
      }
      if (filter.nodeUuid && filter.nodeUuid !== 'ALL' && log.nodeUuid !== filter.nodeUuid) {
        return false;
      }
      if (filter.searchKeyword) {
        const searchLower = filter.searchKeyword.toLowerCase();
        return (
          log.message.toLowerCase().includes(searchLower) ||
          (log.nodeName && log.nodeName.toLowerCase().includes(searchLower)) ||
          false
        );
      }
      return true;
    });
  }, [logs, filter]);
  // 自动滚动到底部
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [filteredLogs.length]);

  if (logs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <Terminal className="h-8 w-8 mb-2 opacity-50" />
        <span className="text-sm">{ll.viewer.console.no_logs()}</span>
      </div>
    );
  }

  return (
    <ScrollArea className="h-full">
      <div ref={scrollRef} className="p-2 space-y-1 font-mono text-xs">
        {filteredLogs.map((log) => {
          const Icon = levelIcons[log.level];
          return (
            <div
              key={log.id}
              className="flex flex-col gap-0.5 px-1 py-0.5 hover:bg-muted/50 rounded group"
            >
              <div className="flex items-start gap-2">
                <span className="text-muted-foreground shrink-0">
                  {formatTimestamp(log.timestamp)}
                </span>
                <Icon
                  className={`h-3.5 w-3.5 mt-0.5 shrink-0 ${levelColors[log.level]}`}
                />
                {log.nodeName && (
                  <span
                    className={cn(
                      'shrink-0 font-medium',
                      log.source === 'system'
                        ? 'text-amber-600 dark:text-amber-400'
                        : 'text-purple-600 dark:text-purple-400'
                    )}
                  >
                    {log.nodeName.startsWith('[') ? log.nodeName : `[${log.nodeName}]`}
                  </span>
                )}
                <span className="break-all flex-1">{log.message}</span>
              </div>
              {log.details ? (
                <div className="ml-16 mt-0.5 px-2 py-1 bg-muted/40 rounded border border-border/50 text-[11px] text-muted-foreground overflow-x-auto whitespace-pre-wrap">
                  {typeof log.details === 'object'
                    ? safeStringify(log.details, 2)
                    : String(log.details)}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );
}
