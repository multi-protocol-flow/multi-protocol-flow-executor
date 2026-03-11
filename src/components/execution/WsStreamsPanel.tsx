/**
 * WebSocket 消息流面板组件
 */
import { Wifi, ArrowUpRight, ArrowDownLeft, Trash2, Layers, ChevronRight, ChevronDown, Clock } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { NODE_TYPE_NAMES, NODE_TYPE_COLORS } from './constants';
import type { WsConnectionStream, WsStreamSegment, WsMessage } from '@/types';

interface WsStreamsPanelProps {
  wsStreams: WsConnectionStream[];
}

function formatTimestamp(ms: number): string {
  const date = new Date(ms);
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}:${String(date.getSeconds()).padStart(2, '0')}`;
}

function truncateUrl(url: string, max: number): string {
  return url.length > max ? url.substring(0, max - 3) + '...' : url;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatContent(content: WsMessage['content']): string {
  if (content.type === 'text') {
    return content.content.length > 100 ? content.content.substring(0, 97) + '...' : content.content;
  }
  // content.type === 'binary'
  const bytes: number[] = content.content;
  const hex = bytes.slice(0, 16).map(b => b.toString(16).padStart(2, '0')).join('');
  return `[${bytes.length}字节] ${hex}...`;
}

function calculateStats(stream: WsConnectionStream) {
  let sent = 0, received = 0, dropped = 0;
  stream.segments.forEach(seg => {
    seg.messages.forEach(msg => { if (msg.direction === 'sent') sent++; else received++; });
    dropped += seg.dropped_message_count;
  });
  return { sent, received, dropped };
}

function WsMessageItem({ message }: { message: WsMessage }) {
  const isSent = message.direction === 'sent';
  return (
    <div className={cn('flex items-center gap-3 p-2 rounded-lg hover:bg-muted/50', isSent ? 'bg-emerald-50/50' : 'bg-blue-50/50')}>
      <div className={cn('w-5 h-5 flex items-center justify-center rounded-full shrink-0', isSent ? 'text-emerald-600' : 'text-blue-600')}>
        {isSent ? <ArrowUpRight className="w-3 h-3" /> : <ArrowDownLeft className="w-3 h-3" />}
      </div>
      <span className={cn('text-xs font-medium shrink-0', isSent ? 'text-emerald-600' : 'text-blue-600')}>
        {isSent ? '发送' : '接收'}
      </span>
      <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
        <Clock className="w-3 h-3" /><span>{formatTimestamp(message.timestamp)}</span>
      </div>
      <div className="text-xs text-muted-foreground shrink-0">{formatSize(message.size)}</div>
      <div className="flex-1 min-w-0 text-sm truncate" title={formatContent(message.content)}>{formatContent(message.content)}</div>
    </div>
  );
}

function WsSegmentItem({ segment, index, expanded, onToggle }: { segment: WsStreamSegment; index: number; expanded: boolean; onToggle: () => void }) {
  const typeLabel = NODE_TYPE_NAMES[segment.node_type] || segment.node_type;
  const typeColor = NODE_TYPE_COLORS[segment.node_type] || '';
  
  return (
    <div>
      <button type="button" onClick={onToggle} className={cn('w-full flex items-center justify-between px-4 py-2.5', expanded ? 'bg-muted/30' : 'hover:bg-muted/20')}>
        <div className="flex items-center gap-2.5 min-w-0">
          <span className={cn('w-5 h-5 flex items-center justify-center text-xs font-medium rounded-full shrink-0', expanded ? 'bg-violet-100 text-violet-700' : 'bg-muted/50 text-muted-foreground')}>{index + 1}</span>
          <div className="text-muted-foreground shrink-0">{expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}</div>
          <span className="text-sm truncate">{segment.node_name}</span>
          <Badge variant="outline" className={cn('text-xs font-normal border-0 shrink-0', typeColor)}>{typeLabel}</Badge>
        </div>
        <div className="flex items-center gap-1.5 text-sm text-muted-foreground shrink-0">
          <Layers className="w-3.5 h-3.5" /><span>{segment.messages.length} 条消息</span>
        </div>
      </button>
      {expanded && (
        <div className="border-t border-border/30 bg-muted/10 px-4 py-2 max-h-[250px] overflow-y-auto">
          {segment.messages.length === 0 ? <div className="text-sm text-muted-foreground text-center py-2">无消息记录</div> : segment.messages.map((m, i) => <WsMessageItem key={i} message={m} />)}
        </div>
      )}
    </div>
  );
}

function WsStreamCard({ stream }: { stream: WsConnectionStream }) {
  const [expanded, setExpanded] = useState(false);
  const [segIdx, setSegIdx] = useState<number | null>(null);
  const { sent, received, dropped } = calculateStats(stream);
  
  return (
    <div className="rounded-xl border border-border/50 bg-gradient-to-br from-violet-50/30 to-background overflow-hidden">
      <button type="button" onClick={() => setExpanded(!expanded)} className="w-full flex items-center justify-between p-3 hover:bg-muted/30 text-left">
        <div className="flex items-center gap-2.5 min-w-0">
          <div className="text-muted-foreground shrink-0">{expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}</div>
          <Wifi className="w-4 h-4 text-violet-500 shrink-0" />
          <span className="font-mono text-xs text-muted-foreground shrink-0">{stream.connection_id.substring(0, 8)}</span>
          <span className="text-sm truncate" title={stream.url}>{truncateUrl(stream.url, 50)}</span>
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <div className="flex items-center gap-1 text-xs text-emerald-600"><ArrowUpRight className="w-3 h-3" /><span>{sent}</span></div>
          <div className="flex items-center gap-1 text-xs text-blue-600"><ArrowDownLeft className="w-3 h-3" /><span>{received}</span></div>
          {dropped > 0 && <div className="flex items-center gap-1 text-xs text-amber-600"><Trash2 className="w-3 h-3" /><span>{dropped}</span></div>}
        </div>
      </button>
      {expanded && (
        <div className="border-t border-border/50 bg-muted/10">
          {stream.segments.length === 0 ? <div className="p-3 text-sm text-muted-foreground text-center">无消息流段</div> : (
            <div className="divide-y divide-border/30">
              {stream.segments.map((seg, i) => <WsSegmentItem key={seg.node_uuid} segment={seg} index={i} expanded={segIdx === i} onToggle={() => setSegIdx(segIdx === i ? null : i)} />)}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function WsStreamsPanel({ wsStreams }: WsStreamsPanelProps) {
  if (wsStreams.length === 0) return null;
  let sent = 0, received = 0, dropped = 0;
  wsStreams.forEach(s => { const st = calculateStats(s); sent += st.sent; received += st.received; dropped += st.dropped; });
  
  return (
    <div className="space-y-3 max-h-[400px] overflow-y-auto">
      <div className="flex items-center justify-between sticky top-0 bg-background/95 backdrop-blur-sm py-1 z-10">
        <h3 className="text-sm font-semibold flex items-center gap-2"><Wifi className="w-4 h-4 text-violet-500" />WebSocket 消息流</h3>
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span className="flex items-center gap-1"><ArrowUpRight className="w-3 h-3 text-emerald-500" />发送 {sent}</span>
          <span className="flex items-center gap-1"><ArrowDownLeft className="w-3 h-3 text-blue-500" />接收 {received}</span>
          {dropped > 0 && <span className="flex items-center gap-1 text-amber-600"><Trash2 className="w-3 h-3" />丢弃 {dropped}</span>}
          <Badge variant="outline" className="text-xs font-normal">{wsStreams.length} 个连接</Badge>
        </div>
      </div>
      <div className="space-y-2">{wsStreams.map(s => <WsStreamCard key={s.connection_id} stream={s} />)}</div>
    </div>
  );
}
