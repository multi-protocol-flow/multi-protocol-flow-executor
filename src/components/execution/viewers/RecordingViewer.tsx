/**
 * RecordingViewer - TCP/UDP 会话录制查看器
 * 显示录制条目的时间线视图，支持按方向过滤和展开查看二进制数据
 */
import { useState, useMemo, useCallback } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ArrowUpFromLine,
  ArrowDownToLine,
  Clock,
  Filter,
  Database,
} from 'lucide-react';
import type { TcpUdpRecording, RecordingEntry, RecordingDirection } from '@/types';
import { Badge } from '@/components/ui/badge';

import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { HexViewer } from '@/components/common/HexViewer';

// ============================================================================
// Props
// ============================================================================

interface RecordingViewerProps {
  /** 录制数据 */
  recording: TcpUdpRecording;
}

// ============================================================================
// 工具函数
// ============================================================================

/** 格式化字节大小 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const val = bytes / Math.pow(k, i);
  return `${val < 10 ? val.toFixed(1) : Math.round(val)} ${units[i]}`;
}

/** 格式化相对时间 */
function formatRelativeTime(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const sec = ms / 1000;
  if (sec < 60) return `${sec.toFixed(1)}s`;
  const min = Math.floor(sec / 60);
  const remainSec = Math.round(sec % 60);
  return `${min}m ${remainSec}s`;
}

/** Base64 解码为 Uint8Array */
function base64ToUint8Array(base64: string): Uint8Array {
  if (!base64) return new Uint8Array(0);
  const decoded = atob(base64);
  const bytes = new Uint8Array(decoded.length);
  for (let i = 0; i < decoded.length; i++) {
    bytes[i] = decoded.charCodeAt(i);
  }
  return bytes;
}

/** 获取文本预览（尝试 UTF-8 解码，截断至 80 字符） */
function getTextPreview(entry: RecordingEntry): string {
  if (!entry.data_base64 || entry.data_size === 0) return `(${getLL().execution.viewers.no_data()})`;
  try {
    const bytes = base64ToUint8Array(entry.data_base64);
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return text.length > 80 ? text.slice(0, 80) + '…' : text;
  } catch {
    return `(${entry.data_size} ${getLL().execution.viewers.recording_receive()})`;
  }
}

// ============================================================================
// 过滤类型
// ============================================================================

type FilterMode = 'all' | 'send' | 'receive';

// ============================================================================
// 方向徽章
// ============================================================================

function DirectionBadge({ direction }: { direction: RecordingDirection }) {
  const isSend = direction === 'send';
  return (
    <Badge
      className={cn(
        'font-mono text-xs px-1.5 py-0.5',
        isSend
          ? 'text-sky-600 dark:text-sky-400 bg-sky-50 dark:bg-sky-950/50'
          : 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
      )}
    >
      {isSend ? getLL().execution.viewers.recording_send() : getLL().execution.viewers.recording_receive()}
    </Badge>
  );
}

// ============================================================================
// 录制条目行
// ============================================================================

function RecordingEntryRow({ entry, index }: { entry: RecordingEntry; index: number }) {
  const [expanded, setExpanded] = useState(false);
  const isSend = entry.direction === 'send';

  const handleToggle = useCallback(() => {
    setExpanded(prev => !prev);
  }, []);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      {/* 条目摘要行 */}
      <button
        type="button"
        onClick={handleToggle}
        className="w-full flex items-center gap-3 p-2.5 bg-muted/20 hover:bg-muted/40 transition-colors text-left"
        aria-expanded={expanded}
      >
        {/* 展开图标 */}
        <div className="text-muted-foreground shrink-0">
          {expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        </div>

        {/* 序号 */}
        <span className="text-xs text-muted-foreground font-mono w-6 shrink-0 text-right">
          #{index + 1}
        </span>

        {/* 方向徽章 */}
        <DirectionBadge direction={entry.direction} />

        {/* 时间戳 */}
        <span className="text-xs font-mono text-muted-foreground shrink-0">
          +{formatRelativeTime(entry.timestamp_ms)}
        </span>

        {/* 数据大小 */}
        <Badge variant="outline" className="font-mono text-xs shrink-0">
          {formatBytes(entry.data_size)}
        </Badge>

        {/* 文本预览 */}
        <span className="text-xs text-foreground/70 truncate flex-1 min-w-0">
          {getTextPreview(entry)}
        </span>

        {/* 方向图标 */}
        <div className={cn('shrink-0', isSend ? 'text-sky-500' : 'text-emerald-500')}>
          {isSend ? <ArrowUpFromLine className="w-4 h-4" /> : <ArrowDownToLine className="w-4 h-4" />}
        </div>
      </button>

      {/* 展开的详细视图 */}
      {expanded && (
        <div className="border-t border-border bg-background p-3">
          {/* 元信息 */}
          <div className="flex flex-wrap gap-2 mb-3">
            <Badge variant="outline" className="font-mono text-xs">
              {getLL().viewer.recording.label_connection()}: {entry.connection_id}
            </Badge>
            <Badge variant="outline" className="font-mono text-xs">
              {entry.data_size} bytes
            </Badge>
            <Badge variant="outline" className="font-mono text-xs">
              +{entry.timestamp_ms}ms
            </Badge>
          </div>

          {/* Hex 查看器 */}
          {entry.data_size > 0 && entry.data_base64 ? (
            <HexViewer data={base64ToUint8Array(entry.data_base64)} height={300} />
          ) : (
            <p className="text-sm text-muted-foreground italic">{getLL().execution.viewers.no_data()}</p>
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 录制元数据卡片
// ============================================================================

function RecordingMetaCard({ recording }: { recording: TcpUdpRecording }) {
  const ll = getLL();
  const totalSent = recording.entries.filter(e => e.direction === 'send').reduce((sum, e) => sum + e.data_size, 0);
  const totalReceived = recording.entries.filter(e => e.direction === 'receive').reduce((sum, e) => sum + e.data_size, 0);

  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 bg-muted/30 rounded-lg border border-border">
      <div className="space-y-1">
        <div className="text-xs text-muted-foreground">{ll.execution.viewers.recording_name()}</div>
        <div className="text-sm font-medium text-foreground truncate">{recording.name}</div>
      </div>
      <div className="space-y-1">
        <div className="text-xs text-muted-foreground">{ll.execution.viewers.recording_duration()}</div>
        <div className="text-sm font-mono text-foreground">{formatRelativeTime(recording.duration_ms)}</div>
      </div>
      <div className="space-y-1">
        <div className="text-xs text-muted-foreground flex items-center gap-1">
          <ArrowUpFromLine className="w-3 h-3" /> {ll.execution.viewers.recording_send()}
        </div>
        <div className="text-sm font-mono text-sky-600 dark:text-sky-400">
          {ll.viewer.recording.send_summary({count: recording.entries.filter(e => e.direction === 'send').length, size: formatBytes(totalSent)})}
        </div>
      </div>
      <div className="space-y-1">
        <div className="text-xs text-muted-foreground flex items-center gap-1">
          <ArrowDownToLine className="w-3 h-3" /> {ll.execution.viewers.recording_receive()}
        </div>
        <div className="text-sm font-mono text-emerald-600 dark:text-emerald-400">
          {ll.viewer.recording.receive_summary({count: recording.entries.filter(e => e.direction === 'receive').length, size: formatBytes(totalReceived)})}
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// RecordingViewer 主组件
// ============================================================================

export function RecordingViewer({ recording }: RecordingViewerProps) {
  const ll = getLL();
  const [filter, setFilter] = useState<FilterMode>('all');

  const filteredEntries = useMemo(() => {
    if (filter === 'all') return recording.entries;
    return recording.entries.filter(e => e.direction === filter);
  }, [recording.entries, filter]);

  const filterCounts = useMemo(() => ({
    all: recording.entries.length,
    send: recording.entries.filter(e => e.direction === 'send').length,
    receive: recording.entries.filter(e => e.direction === 'receive').length,
  }), [recording.entries]);

  return (
    <div className="space-y-3">
      {/* 录制元数据 */}
      <RecordingMetaCard recording={recording} />

      {/* 过滤器 */}
      <div className="flex items-center gap-2">
        <Filter className="w-4 h-4 text-muted-foreground" />
        {(
          [
            { key: 'all' as const, label: ll.execution.viewers.filter_all() },
            { key: 'send' as const, label: ll.execution.viewers.filter_send() },
            { key: 'receive' as const, label: ll.execution.viewers.filter_receive() },
          ]
        ).map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => setFilter(key)}
            className={cn(
              'px-2.5 py-1 text-xs rounded-md border transition-colors',
              filter === key
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-background text-muted-foreground border-border hover:bg-muted/50'
            )}
          >
            {label} ({filterCounts[key]})
          </button>
        ))}
      </div>

      {/* 条目列表 */}
      {filteredEntries.length === 0 ? (
        <div className="text-sm text-muted-foreground italic p-4 text-center">
          {ll.execution.viewers.no_entries()}
        </div>
      ) : (
        <div className="space-y-2 max-h-[600px] overflow-y-auto">
          {filteredEntries.map((entry, idx) => (
            <RecordingEntryRow key={`${entry.timestamp_ms}-${entry.direction}-${idx}`} entry={entry} index={idx} />
          ))}
        </div>
      )}

      {/* 底部汇总 */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground pt-1">
        <Database className="w-3.5 h-3.5" />
        <span>
          {ll.execution.viewers.total_data({ count: recording.entries.length, bytes: formatBytes(recording.entries.reduce((sum, e) => sum + e.data_size, 0)) })}
        </span>
        <Clock className="w-3.5 h-3.5 ml-2" />
        <span>{ll.execution.viewers.duration_label()} {formatRelativeTime(recording.duration_ms)}</span>
      </div>
    </div>
  );
}
