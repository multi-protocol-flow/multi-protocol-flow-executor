/**
 * WebSocket 消息流面板组件
 * 显示 WebSocket 连接的消息流，支持虚拟化列表
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { FixedSizeList as List, type ListChildComponentProps } from 'react-window';
import { useDebugStore } from '@/stores/debug';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  ArrowDown,
  ArrowUp,
  Wifi,
  WifiOff,
  ChevronDown,
  ChevronRight,
  ArrowDownToLine,
  ScrollText,
} from 'lucide-react';
import type { WsMessage, WsStreamSegment, WsConnectionStream } from '@/types/ws_stream';

// ============================================================================
// 常量
// ============================================================================

/** 消息项高度 */
const MESSAGE_ITEM_HEIGHT = 40;
/** 文本消息预览最大长度 */
const TEXT_PREVIEW_MAX_LENGTH = 200;
/** 二进制消息预览最大长度 */
const BINARY_PREVIEW_MAX_LENGTH = 100;

// ============================================================================
// 工具函数
// ============================================================================

/** 格式化时间戳 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const timeStr = date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
  const ms = String(date.getMilliseconds()).padStart(3, '0');
  return `${timeStr}.${ms}`;
}

/** 格式化字节大小 */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)}MB`;
}

/** 获取消息预览内容 */
function getMessagePreview(message: WsMessage): string {
  if (message.is_binary) {
    const content = message.content.type === 'binary' ? message.content.content : '';
    const truncated = content.length > BINARY_PREVIEW_MAX_LENGTH
      ? content.slice(0, BINARY_PREVIEW_MAX_LENGTH) + '...'
      : content;
    return `[Binary] ${truncated}`;
  }
  const content = message.content.type === 'text' ? message.content.content : '';
  return content.length > TEXT_PREVIEW_MAX_LENGTH
    ? content.slice(0, TEXT_PREVIEW_MAX_LENGTH) + '...'
    : content;
}

/** 计算连接统计 */
function calculateStats(stream: WsConnectionStream) {
  let sent = 0;
  let received = 0;
  let dropped = 0;

  for (const segment of stream.segments) {
    dropped += segment.dropped_message_count;
    for (const msg of segment.messages) {
      // 安全检查：防止 undefined 消息导致崩溃
      if (!msg) continue;
      if (msg.direction === 'sent') sent++;
      else received++;
    }
  }

  return { sent, received, dropped };
}

// ============================================================================
// 列表项类型
// ============================================================================

/** 虚拟列表项类型 */
type ListItem =
  | { type: 'segment-header'; segment: WsStreamSegment; index: number }
  | { type: 'message'; message: WsMessage; segmentIndex: number; messageIndex: number };

/** 列表行属性 */
interface RowData {
  items: ListItem[];
  expandedSegments: Set<string>;
  onToggleSegment: (nodeUuid: string) => void;
  onMessageClick: (message: WsMessage) => void;
}

// ============================================================================
// 子组件
// ============================================================================

/** 空状态组件 */
function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
      <WifiOff className="h-8 w-8 mb-2 opacity-50" />
      <span className="text-sm">暂无 WebSocket 连接</span>
      <span className="text-xs mt-1 opacity-70">调试时 WebSocket 消息将在此显示</span>
    </div>
  );
}

/** 连接头部组件 */
interface ConnectionHeaderProps {
  stream: WsConnectionStream;
  onToggleAutoScroll: () => void;
  autoScroll: boolean;
}

function ConnectionHeader({ stream, onToggleAutoScroll, autoScroll }: ConnectionHeaderProps) {
  const stats = calculateStats(stream);

  return (
    <div className="border-b border-border p-3 space-y-2">
      {/* 第一行：连接信息 + ID */}
      <div className="flex items-center gap-2 flex-wrap">
        <Badge variant={stream.is_closed ? 'secondary' : 'default'} className="text-xs">
          {stream.is_closed ? (
            <>
              <WifiOff className="h-3 w-3 mr-1" />
              已关闭
            </>
          ) : (
            <>
              <Wifi className="h-3 w-3 mr-1" />
              已连接
            </>
          )}
        </Badge>
        <span className="text-xs text-muted-foreground">
          ID: {stream.connection_id.slice(0, 8)}...
        </span>
      </div>

      {/* 第二行：URL */}
      <div className="text-xs text-muted-foreground font-mono truncate max-w-full" title={stream.url}>
        {stream.url}
      </div>

      {/* 第三行：统计和操作 */}
      <div className="flex items-center gap-3 text-xs">
        <div className="flex items-center gap-1 text-green-600">
          <ArrowUp className="h-3 w-3" />
          <span>{stats.sent}</span>
        </div>
        <div className="flex items-center gap-1 text-blue-600">
          <ArrowDown className="h-3 w-3" />
          <span>{stats.received}</span>
        </div>
        {stats.dropped > 0 && (
          <div className="flex items-center gap-1 text-orange-500">
            <span>丢弃: {stats.dropped}</span>
          </div>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto h-6 px-2 text-xs"
          onClick={onToggleAutoScroll}
        >
          {autoScroll ? (
            <>
              <ArrowDownToLine className="h-3 w-3 mr-1" />
              自动滚动
            </>
          ) : (
            <>
              <ScrollText className="h-3 w-3 mr-1" />
              手动
            </>
          )}
        </Button>
      </div>
    </div>
  );
}

/** 消息详情对话框 */
interface MessageDetailDialogProps {
  message: WsMessage | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function MessageDetailDialog({ message, open, onOpenChange }: MessageDetailDialogProps) {
  if (!message) return null;

  const content = message.is_binary
    ? message.content.type === 'binary' ? message.content.content : ''
    : message.content.type === 'text' ? message.content.content : '';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {message.direction === 'sent' ? (
              <>
                <ArrowUp className="h-4 w-4 text-green-600" />
                发送的消息
              </>
            ) : (
              <>
                <ArrowDown className="h-4 w-4 text-blue-600" />
                接收的消息
              </>
            )}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="flex gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">时间: </span>
              <span className="font-mono">{formatTimestamp(message.timestamp)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">大小: </span>
              <span>{formatSize(message.size)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">类型: </span>
              <span>{message.is_binary ? '二进制' : '文本'}</span>
            </div>
          </div>
          <div className="border rounded-md">
            <div className="bg-muted px-3 py-1.5 text-xs text-muted-foreground border-b">
              消息内容
            </div>
            <ScrollArea className="max-h-[400px]">
              <pre className="p-3 text-xs font-mono whitespace-pre-wrap break-all">
                {content}
              </pre>
            </ScrollArea>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ============================================================================
// 行组件
// ============================================================================

interface RowProps extends ListChildComponentProps<RowData> {}

function Row({ index, style, data }: RowProps) {
  const { items, expandedSegments, onToggleSegment, onMessageClick } = data;
  const item = items[index];
  if (!item) return null;

  if (item.type === 'segment-header') {
    const { segment } = item;
    const isExpanded = expandedSegments.has(segment.node_uuid);

    return (
      <div
        style={style}
        className="flex items-center gap-2 px-3 bg-muted/50 border-b border-border cursor-pointer hover:bg-muted/70"
        onClick={() => onToggleSegment(segment.node_uuid)}
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 shrink-0" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0" />
        )}
        <span className="text-xs font-medium truncate">
          {segment.node_name || segment.node_uuid.slice(0, 8)}
        </span>
        <Badge variant="outline" className="text-[10px] h-4 px-1">
          {segment.messages.length}
        </Badge>
        {segment.dropped_message_count > 0 && (
          <Badge variant="secondary" className="text-[10px] h-4 px-1 text-orange-500">
            +{segment.dropped_message_count}
          </Badge>
        )}
        {segment.ended_at && (
          <span className="text-[10px] text-muted-foreground ml-auto">
            已结束
          </span>
        )}
      </div>
    );
  }

  // 消息项
  const { message } = item;
  const isSent = message.direction === 'sent';

  return (
    <div
      style={style}
      className={`flex items-center gap-2 px-3 text-xs hover:bg-muted/30 cursor-pointer border-b border-border/50 ${
        isSent ? 'bg-green-50/5' : 'bg-blue-50/5'
      }`}
      onClick={() => onMessageClick(message)}
    >
      <span className="text-muted-foreground shrink-0 w-20 font-mono">
        {formatTimestamp(message.timestamp)}
      </span>
      {isSent ? (
        <ArrowUp className="h-3.5 w-3.5 shrink-0 text-green-600" />
      ) : (
        <ArrowDown className="h-3.5 w-3.5 shrink-0 text-blue-600" />
      )}
      <span className={`shrink-0 ${isSent ? 'text-green-600' : 'text-blue-600'}`}>
        {isSent ? '发送' : '接收'}
      </span>
      <span className="truncate flex-1" title={getMessagePreview(message)}>
        {getMessagePreview(message)}
      </span>
      <span className="text-muted-foreground shrink-0">
        {formatSize(message.size)}
      </span>
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export function WsMessageStreamPanel() {
  const wsStreams = useDebugStore((state) => state.wsStreams);
  const activeConnectionId = useDebugStore((state) => state.activeConnectionId);

  const [autoScroll, setAutoScroll] = useState(true);
  const [expandedSegments, setExpandedSegments] = useState<Set<string>>(new Set());
  const [selectedMessage, setSelectedMessage] = useState<WsMessage | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [listHeight, setListHeight] = useState(200);
  const listRef = useRef<List<RowData>>(null);
  const listContainerRef = useRef<HTMLDivElement>(null);

  // 获取连接列表
  const connectionIds = useMemo(() => Array.from(wsStreams.keys()), [wsStreams]);

  // 当前活动连接
  const currentConnectionId = activeConnectionId || connectionIds[0] || null;
  const currentStream = currentConnectionId ? wsStreams.get(currentConnectionId) : null;

  // 监听容器高度变化
  useEffect(() => {
    const container = listContainerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setListHeight(entry.contentRect.height);
      }
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, []);

  // 构建虚拟列表项
  const listItems = useMemo((): ListItem[] => {
    if (!currentStream) return [];

    const items: ListItem[] = [];
    currentStream.segments.forEach((segment, segmentIndex) => {
      // 添加段头部
      items.push({ type: 'segment-header', segment, index: segmentIndex });
      // 如果展开，添加消息
      if (expandedSegments.has(segment.node_uuid)) {
        segment.messages.forEach((message, messageIndex) => {
          items.push({ type: 'message', message, segmentIndex, messageIndex });
        });
      }
    });
    return items;
  }, [currentStream, expandedSegments]);

  // 切换段展开状态
  const toggleSegment = useCallback((nodeUuid: string) => {
    setExpandedSegments((prev) => {
      const next = new Set(prev);
      if (next.has(nodeUuid)) {
        next.delete(nodeUuid);
      } else {
        next.add(nodeUuid);
      }
      return next;
    });
  }, []);

  // 点击消息
  const handleMessageClick = useCallback((message: WsMessage) => {
    setSelectedMessage(message);
    setDetailOpen(true);
  }, []);

  // 行数据
  const rowData = useMemo<RowData>(() => ({
    items: listItems,
    expandedSegments,
    onToggleSegment: toggleSegment,
    onMessageClick: handleMessageClick,
  }), [listItems, expandedSegments, toggleSegment, handleMessageClick]);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && listRef.current && listItems.length > 0) {
      listRef.current.scrollToItem(listItems.length - 1, 'end');
    }
  }, [listItems.length, autoScroll]);

  // 空状态
  if (connectionIds.length === 0) {
    return <EmptyState />;
  }

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* 连接选择器 */}
      {connectionIds.length > 1 && (
        <div className="flex gap-1 p-2 border-b border-border overflow-x-auto shrink-0">
          {connectionIds.map((id) => {
            const stream = wsStreams.get(id);
            if (!stream) return null;
            const isActive = id === currentConnectionId;
            return (
              <Button
                key={id}
                variant={isActive ? 'default' : 'outline'}
                size="sm"
                className="h-6 px-2 text-xs shrink-0"
                onClick={() => {
                  useDebugStore.setState({ activeConnectionId: id });
                  setExpandedSegments(new Set());
                }}
              >
                {stream.url.replace(/^wss?:\/\//, '').slice(0, 20)}
              </Button>
            );
          })}
        </div>
      )}

      {/* 连接头部 */}
      {currentStream && (
        <ConnectionHeader
          stream={currentStream}
          autoScroll={autoScroll}
          onToggleAutoScroll={() => setAutoScroll((prev) => !prev)}
        />
      )}

      {/* 连接数量标签 */}
      <div className="flex items-center justify-end px-3 py-1 border-b border-border shrink-0">
        <Badge variant="secondary" className="text-xs">
          {connectionIds.length} 个连接
        </Badge>
      </div>

      {/* 消息列表 */}
      {currentStream && listItems.length > 0 ? (
        <div ref={listContainerRef} className="flex-1 min-h-0">
          <List
            ref={listRef}
            height={listHeight}
            width="100%"
            itemCount={listItems.length}
            itemSize={MESSAGE_ITEM_HEIGHT}
            itemData={rowData}
            overscanCount={10}
            className="font-mono"
          >
            {Row}
          </List>
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          暂无消息
        </div>
      )}

      {/* 消息详情对话框 */}
      <MessageDetailDialog
        message={selectedMessage}
        open={detailOpen}
        onOpenChange={setDetailOpen}
      />
    </div>
  );
}

export default WsMessageStreamPanel;
