/**
 * 控制台面板组件
 * 显示实时日志，支持级别过滤和自动滚动
 */
import { useEffect, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useConsoleStore } from '@/stores/console';
import { useVT } from '@/i18n/viewer-translations';
import { Terminal, Info, AlertTriangle, Bug, XCircle } from 'lucide-react';
import type { LogLevel } from '@/types/events';

// 日志级别颜色映射
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

// 格式化时间戳
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

interface ConsolePanelProps {
  flowUuid?: string;
}

export function ConsolePanel({ flowUuid: _flowUuid }: ConsolePanelProps) {
  const vt = useVT();
  const scrollRef = useRef<HTMLDivElement>(null);
  const logs = useConsoleStore(useShallow((state) => state.logs));
  const getFilteredLogs = useConsoleStore((state) => state.getFilteredLogs);

  const filteredLogs = getFilteredLogs();

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
        <span className="text-sm">{vt('console.no_logs')}</span>
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
              className="flex items-start gap-2 px-1 py-0.5 hover:bg-muted/50 rounded"
            >
              <span className="text-muted-foreground shrink-0">
                {formatTimestamp(log.timestamp)}
              </span>
              <Icon
                className={`h-3.5 w-3.5 mt-0.5 shrink-0 ${levelColors[log.level]}`}
              />
              {log.nodeName && (
                <span className="text-purple-500 shrink-0">[{log.nodeName}]</span>
              )}
              <span className="break-all">{log.message}</span>
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );
}
