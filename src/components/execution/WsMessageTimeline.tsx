/**
 * WebSocket 消息时间线组件
 * Postman 风格的平面化消息流视图，替代旧的嵌套手风琴和虚拟化列表
 */
import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { VariableSizeList as List, type ListChildComponentProps } from 'react-window';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './viewers/RawJsonViewer';
import { NODE_TYPE_NAMES } from './constants';
import type { WsMessage, WsConnectionStream, WsStreamSegment } from '@/types/ws_stream';
import {
  ArrowUp,
  ArrowDown,
  Wifi,
  WifiOff,
  Search,
  X,
  ArrowDownToLine,
  Copy,
  Check,
} from 'lucide-react';

// ============================================================================
// 常量
// ============================================================================

/** 消息行高度 */
const MESSAGE_ROW_HEIGHT = 40;
/** 段分隔线高度 */
const SEGMENT_DIVIDER_HEIGHT = 28;
/** 文本预览最大长度 */
const TEXT_PREVIEW_MAX = 200;
/** 二进制预览字节数 */
const BINARY_PREVIEW_MAX = 64;

// ============================================================================
// 工具函数
// ============================================================================

/** 格式化时间戳 HH:mm:ss.SSS */
function formatTimestampMs(timestamp: number): string {
  const d = new Date(timestamp);
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  const sss = String(d.getMilliseconds()).padStart(3, '0');
  return `${hh}:${mm}:${ss}.${sss}`;
}

/** 格式化字节大小 */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

/** 获取消息预览文本 */
function getMessagePreview(message: WsMessage): string {
  if (message.is_binary) {
    if (message.content.type === 'binary') {
      const hex = message.content.content
        .slice(0, BINARY_PREVIEW_MAX)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join(' ');
      return `[Binary ${message.size}B] ${hex}${message.size > BINARY_PREVIEW_MAX ? ' ...' : ''}`;
    }
    return `[Binary]`;
  }
  if (message.content.type === 'text') {
    const text = message.content.content;
    return text.length > TEXT_PREVIEW_MAX ? text.slice(0, TEXT_PREVIEW_MAX) + '...' : text;
  }
  return '';
}

/** 获取消息完整内容（文本形式） */
function getMessageText(message: WsMessage): string {
  if (message.content.type === 'text') return message.content.content;
  if (message.content.type === 'binary') {
    return message.content.content
      .map((b) => b.toString(16).padStart(2, '0'))
      .join(' ');
  }
  return '';
}

/** 格式化 hex dump */
function formatHexDump(message: WsMessage): string {
  if (message.content.type !== 'binary') {
    // 文本消息转 hex
    const encoder = new TextEncoder();
    const bytes = encoder.encode(message.content.type === 'text' ? message.content.content : '');
    return buildHexDump(Array.from(bytes));
  }
  return buildHexDump(message.content.content);
}

function buildHexDump(bytes: number[]): string {
  const lines: string[] = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const offset = i.toString(16).padStart(8, '0');
    const hexPart = chunk
      .map((b) => b.toString(16).padStart(2, '0'))
      .join(' ')
      .padEnd(48, ' ');
    const asciiPart = chunk
      .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.'))
      .join('');
    lines.push(`${offset}  ${hexPart}  |${asciiPart}|`);
  }
  return lines.join('\n');
}

/** 尝试解析 JSON */
function tryParseJson(text: string): unknown | null {
  try {
    const parsed = JSON.parse(text);
    if (typeof parsed === 'object' && parsed !== null) return parsed;
    return null;
  } catch {
    return null;
  }
}

/** 计算连接统计 */
function calculateStats(stream: WsConnectionStream) {
  let sent = 0;
  let received = 0;
  let dropped = 0;
  for (const seg of stream.segments) {
    dropped += seg.dropped_message_count;
    for (const msg of seg.messages) {
      if (!msg) continue;
      if (msg.direction === 'sent') sent++;
      else received++;
    }
  }
  return { sent, received, dropped };
}

// ============================================================================
// 类型定义
// ============================================================================

/** 时间线条目类型（平面化后的统一类型） */
type TimelineItem =
  | { type: 'segment-divider'; segment: WsStreamSegment }
  | { type: 'message'; message: WsMessage; globalIndex: number };

/** 方向过滤 */
type DirectionFilter = 'all' | 'sent' | 'received';

/** 内容格式标签 */
type FormatTab = 'text' | 'json' | 'hex';

/** 主组件属性 */
interface WsMessageTimelineProps {
  /** WebSocket 连接流列表 */
  streams: WsConnectionStream[];
  /** 是否为实时模式（调试/执行中），自动滚动到底部 */
  live?: boolean;
  /** 最大高度（默认自适应） */
  maxHeight?: string;
}

/** 虚拟列表行数据 */
interface RowData {
  items: TimelineItem[];
  selectedGlobalIndex: number | null;
  onMessageClick: (message: WsMessage, globalIndex: number) => void;
}

// ============================================================================
// 子组件：连接选择器
// ============================================================================

interface ConnectionSelectorProps {
  streams: WsConnectionStream[];
  activeIndex: number;
  onSelect: (index: number) => void;
}

function ConnectionSelector({ streams, activeIndex, onSelect }: ConnectionSelectorProps) {
  return (
    <div className="flex gap-1 px-3 py-1.5 border-b border-border overflow-x-auto shrink-0">
      {streams.map((stream, idx) => {
        const isActive = idx === activeIndex;
        const shortUrl = stream.url.replace(/^wss?:\/\//, '').split('/')[0];
        return (
          <button
            key={stream.connection_id}
            type="button"
            onClick={() => onSelect(idx)}
            className={cn(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium transition-colors shrink-0',
              isActive
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground',
            )}
          >
            {stream.is_closed ? (
              <WifiOff className="w-3 h-3" />
            ) : (
              <Wifi className="w-3 h-3" />
            )}
            <span className="font-mono truncate max-w-32" title={stream.url}>
              {shortUrl}
            </span>
          </button>
        );
      })}
    </div>
  );
}

// ============================================================================
// 子组件：时间线头部（状态 + 统计 + 过滤 + 搜索）
// ============================================================================

interface TimelineHeaderProps {
  stream: WsConnectionStream;
  messageCount: number;
  directionFilter: DirectionFilter;
  onDirectionFilterChange: (filter: DirectionFilter) => void;
  searchQuery: string;
  onSearchQueryChange: (query: string) => void;
  searchMatchCount: number;
  live?: boolean;
}

function TimelineHeader({
  stream,
  messageCount,
  directionFilter,
  onDirectionFilterChange,
  searchQuery,
  onSearchQueryChange,
  searchMatchCount,
  live,
}: TimelineHeaderProps) {
  const stats = calculateStats(stream);
  const [searchVisible, setSearchVisible] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (searchVisible && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [searchVisible]);

  return (
    <div className="border-b border-border shrink-0">
      {/* 第一行：连接信息 */}
      <div className="flex items-center gap-2 px-3 py-1.5">
        <Badge
          variant={stream.is_closed ? 'secondary' : 'default'}
          className={cn(
            'text-[10px] h-5 gap-1 shrink-0',
            !stream.is_closed && 'bg-emerald-600/10 text-emerald-600 border-emerald-600/20',
          )}
        >
          {stream.is_closed ? (
            <>
              <WifiOff className="w-3 h-3" />
              已关闭
            </>
          ) : (
            <>
              <Wifi className="w-3 h-3" />
              已连接
            </>
          )}
        </Badge>
        <span className="text-xs font-mono text-muted-foreground truncate" title={stream.url}>
          {stream.url}
        </span>
      </div>

      {/* 第二行：统计 + 过滤 + 搜索 + 操作 */}
      <div className="flex items-center gap-2 px-3 py-1">
        {/* 方向统计 */}
        <div className="flex items-center gap-2.5 text-xs text-muted-foreground shrink-0">
          <span className="flex items-center gap-0.5 text-emerald-600">
            <ArrowUp className="w-3 h-3" />
            {stats.sent}
          </span>
          <span className="flex items-center gap-0.5 text-blue-600">
            <ArrowDown className="w-3 h-3" />
            {stats.received}
          </span>
          {stats.dropped > 0 && (
            <span className="text-amber-600">({stats.dropped} 丢弃)</span>
          )}
        </div>

        {/* 分隔 */}
        <div className="w-px h-4 bg-border shrink-0" />

        {/* 方向过滤按钮 */}
        <div className="flex items-center gap-0.5 shrink-0">
          {([
            { key: 'all' as const, label: '全部' },
            { key: 'sent' as const, label: '发送' },
            { key: 'received' as const, label: '接收' },
          ]).map(({ key, label }) => (
            <button
              key={key}
              type="button"
              onClick={() => onDirectionFilterChange(key)}
              className={cn(
                'px-2 py-0.5 rounded text-[11px] font-medium transition-colors',
                directionFilter === key
                  ? key === 'sent'
                    ? 'bg-emerald-600/15 text-emerald-600'
                    : key === 'received'
                      ? 'bg-blue-600/15 text-blue-600'
                      : 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/50',
              )}
            >
              {label}
            </button>
          ))}
        </div>

        {/* 弹性空间 */}
        <div className="flex-1" />

        {/* 搜索 */}
        {searchVisible ? (
          <div className="flex items-center gap-1 shrink-0">
            <div className="relative">
              <Search className="w-3 h-3 absolute left-1.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onChange={(e) => onSearchQueryChange(e.target.value)}
                placeholder="搜索内容..."
                className="h-6 w-40 rounded-md border border-input bg-background pl-7 pr-2 text-xs focus-visible:outline-none focus-visible:border-ring"
              />
            </div>
            {searchQuery && (
              <Badge variant="secondary" className="text-[10px] h-5 px-1.5 shrink-0">
                {searchMatchCount}
              </Badge>
            )}
            <button
              type="button"
              onClick={() => {
                onSearchQueryChange('');
                setSearchVisible(false);
              }}
              className="p-0.5 text-muted-foreground hover:text-foreground"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setSearchVisible(true)}
            className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors"
            title="搜索"
          >
            <Search className="w-3.5 h-3.5" />
          </button>
        )}


      </div>
    </div>
  );
}

// ============================================================================
// 子组件：段分隔线
// ============================================================================

interface SegmentDividerProps {
  segment: WsStreamSegment;
}

function SegmentDivider({ segment }: SegmentDividerProps) {
  const typeLabel = NODE_TYPE_NAMES[segment.node_type] || segment.node_type;

  return (
    <div className="flex items-center gap-2 px-3 h-full">
      <div className="flex-1 h-px bg-border" />
      <span className="text-[10px] text-muted-foreground font-medium shrink-0">
        {segment.node_name}
      </span>
      <Badge variant="outline" className="text-[10px] h-4 px-1.5 py-0 border-border/50 shrink-0">
        {typeLabel}
      </Badge>
      <div className="flex-1 h-px bg-border" />
    </div>
  );
}

// ============================================================================
// 子组件：消息行
// ============================================================================

interface MessageRowProps {
  message: WsMessage;
  isSelected: boolean;
  onClick: () => void;
}

function MessageRow({ message, isSelected, onClick }: Omit<MessageRowProps, 'globalIndex'>) {
  const isSent = message.direction === 'sent';
  const preview = getMessagePreview(message);

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex items-center gap-2 px-3 h-full cursor-pointer transition-colors border-b border-border/30',
        isSelected
          ? isSent
            ? 'bg-emerald-600/5 border-l-2 border-l-emerald-600'
            : 'bg-blue-600/5 border-l-2 border-l-blue-600'
          : isSent
            ? 'hover:bg-emerald-600/5'
            : 'hover:bg-blue-600/5',
      )}
      title={preview}
    >
      {/* 方向图标 */}
      {isSent ? (
        <ArrowUp className="w-3.5 h-3.5 shrink-0 text-emerald-600" />
      ) : (
        <ArrowDown className="w-3.5 h-3.5 shrink-0 text-blue-600" />
      )}

      {/* 时间戳 */}
      <span className="text-[11px] font-mono text-muted-foreground shrink-0 w-[80px] tabular-nums">
        {formatTimestampMs(message.timestamp)}
      </span>

      {/* 方向标签 */}
      <span
        className={cn(
          'text-[11px] font-medium shrink-0 w-6',
          isSent ? 'text-emerald-600' : 'text-blue-600',
        )}
      >
        {isSent ? '发送' : '接收'}
      </span>

      {/* 预览内容 */}
      <span className="text-xs font-mono text-foreground/80 truncate flex-1 min-w-0">
        {preview}
      </span>

      {/* 大小标签 */}
      <span className="text-[10px] text-muted-foreground shrink-0 tabular-nums">
        {formatSize(message.size)}
      </span>
    </button>
  );
}

// ============================================================================
// 子组件：消息详情面板
// ============================================================================

interface MessageDetailPanelProps {
  message: WsMessage;
  onClose: () => void;
}

function MessageDetailPanel({ message, onClose }: MessageDetailPanelProps) {
  const isSent = message.direction === 'sent';
  const [formatTab, setFormatTab] = useState<FormatTab>('text');
  const [copied, setCopied] = useState(false);
  const messageText = getMessageText(message);
  const isJson = !message.is_binary && tryParseJson(messageText) !== null;

  // 切换消息时重置格式
  useEffect(() => {
    if (message.is_binary) {
      setFormatTab('hex');
    } else if (isJson) {
      setFormatTab('json');
    } else {
      setFormatTab('text');
    }
  }, [message, isJson]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(messageText).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [messageText]);

  return (
    <div className="border-t border-border bg-background flex flex-col shrink-0 w-full overflow-hidden">
      {/* 详情头部 */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border/50 shrink-0">
        <div className="flex items-center gap-3 text-xs min-w-0">
          <span className={cn('font-medium shrink-0', isSent ? 'text-emerald-600' : 'text-blue-600')}>
            {isSent ? '发送' : '接收'}
          </span>
          <span className="font-mono text-muted-foreground tabular-nums shrink-0">
            {formatTimestampMs(message.timestamp)}
          </span>
          <span className="text-muted-foreground shrink-0">{formatSize(message.size)}</span>
          <span className="text-muted-foreground shrink-0">
            {message.is_binary ? '二进制' : '文本'}
          </span>
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <button
            type="button"
            onClick={handleCopy}
            className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors"
            title="复制内容"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-600" /> : <Copy className="w-3.5 h-3.5" />}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors"
            title="关闭"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* 格式标签页 */}
      <Tabs value={formatTab} onValueChange={(v) => setFormatTab(v as FormatTab)}>
        <div className="px-3 pt-1 shrink-0">
          <TabsList className="h-7 p-0.5">
            <TabsTrigger value="text" className="text-[11px] h-6 px-2.5">
              文本
            </TabsTrigger>
            <TabsTrigger
              value="json"
              className="text-[11px] h-6 px-2.5"
              disabled={message.is_binary || !isJson}
            >
              JSON
            </TabsTrigger>
            <TabsTrigger value="hex" className="text-[11px] h-6 px-2.5">
              Hex
            </TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="text" className="mt-0 overflow-x-auto">
            <pre className="px-3 py-2 text-xs font-mono whitespace-pre-wrap break-all text-foreground/90 leading-relaxed">
              {messageText}
            </pre>
        </TabsContent>

        <TabsContent value="json" className="mt-0 overflow-x-auto">
            <div className="px-3 py-2">
              <RawJsonViewer data={tryParseJson(messageText)} />
            </div>
        </TabsContent>

        <TabsContent value="hex" className="mt-0 overflow-x-auto">
            <pre className="px-3 py-2 text-[11px] font-mono whitespace-pre leading-relaxed text-foreground/80">
              {formatHexDump(message)}
            </pre>
        </TabsContent>
      </Tabs>
    </div>
  );
}

// ============================================================================
// 虚拟列表行渲染器
// ============================================================================

function TimelineRow({ index, style, data }: ListChildComponentProps<RowData>) {
  const { items, selectedGlobalIndex, onMessageClick } = data;
  const item = items[index];
  if (!item) return null;

  if (item.type === 'segment-divider') {
    return (
      <div style={style}>
        <SegmentDivider segment={item.segment} />
      </div>
    );
  }

  const { message, globalIndex } = item;
  return (
    <div style={style}>
      <MessageRow
        message={message}
        isSelected={selectedGlobalIndex === globalIndex}
        onClick={() => onMessageClick(message, globalIndex)}
      />
    </div>
  );
}

// ============================================================================
// 空状态
// ============================================================================

function EmptyState({ hasConnections }: { hasConnections: boolean }) {
  return (
    <div className="flex flex-col items-center justify-center h-full text-muted-foreground py-12">
      <WifiOff className="h-8 w-8 mb-2 opacity-40" />
      <span className="text-sm">
        {hasConnections ? '暂无消息' : '暂无 WebSocket 连接'}
      </span>
      <span className="text-xs mt-1 opacity-60">
        {hasConnections
          ? '连接成功后消息将在此显示'
          : '调试或执行时 WebSocket 消息将在此显示'}
      </span>
    </div>
  );
}

// ============================================================================
// 主组件：WsMessageTimeline
// ============================================================================

export function WsMessageTimeline({
  streams,
  live = false,
  maxHeight,
}: WsMessageTimelineProps) {
  // 连接选择
  const [activeStreamIndex, setActiveStreamIndex] = useState(0);
  const activeStream = streams[activeStreamIndex] ?? null;

  // 过滤和搜索
  const [directionFilter, setDirectionFilter] = useState<DirectionFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // 选中消息
  const [selectedGlobalIndex, setSelectedGlobalIndex] = useState<number | null>(null);
  const [selectedMessage, setSelectedMessage] = useState<WsMessage | null>(null);

  // 自动滚动
  const [autoScroll, setAutoScroll] = useState(live);
  const [userScrolled, setUserScrolled] = useState(false);

  // 列表引用
  const listRef = useRef<List<RowData>>(null);
  const outerRef = useRef<HTMLDivElement>(null);
  const [outerHeight, setOuterHeight] = useState(300);

  // 切换连接时重置状态
  const handleStreamSelect = useCallback((idx: number) => {
    setActiveStreamIndex(idx);
    setSelectedGlobalIndex(null);
    setSelectedMessage(null);
    setDirectionFilter('all');
    setSearchQuery('');
  }, []);

  // 构建平面时间线（按时间排序，插入段分隔线）
  const timelineItems = useMemo((): TimelineItem[] => {
    if (!activeStream) return [];

    const items: TimelineItem[] = [];

    for (const segment of activeStream.segments) {
      // 插入段分隔线
      items.push({ type: 'segment-divider', segment });

      // 收集该段的消息
      for (const msg of segment.messages) {
        if (!msg) continue;

        // 方向过滤
        if (directionFilter !== 'all' && msg.direction !== directionFilter) continue;

        // 搜索过滤
        if (searchQuery) {
          const text = getMessagePreview(msg).toLowerCase();
          if (!text.includes(searchQuery.toLowerCase())) continue;
        }

        items.push({ type: 'message', message: msg, globalIndex: items.length });
      }
    }

    return items;
  }, [activeStream, directionFilter, searchQuery]);

  // 搜索匹配数
  const searchMatchCount = useMemo(() => {
    if (!searchQuery || !activeStream) return 0;
    let count = 0;
    for (const seg of activeStream.segments) {
      for (const msg of seg.messages) {
        if (!msg) continue;
        if (getMessagePreview(msg).toLowerCase().includes(searchQuery.toLowerCase())) {
          count++;
        }
      }
    }
    return count;
  }, [activeStream, searchQuery]);

  // 点击消息（二次点击同一条消息则收起详情）
  const handleMessageClick = useCallback((message: WsMessage, globalIndex: number) => {
    if (selectedGlobalIndex === globalIndex) {
      setSelectedGlobalIndex(null);
      setSelectedMessage(null);
    } else {
      setSelectedGlobalIndex(globalIndex);
      setSelectedMessage(message);
    }
  }, [selectedGlobalIndex]);

  // 关闭详情
  const handleCloseDetail = useCallback(() => {
    setSelectedGlobalIndex(null);
    setSelectedMessage(null);
  }, []);

  // 虚拟列表行数据
  const rowData = useMemo<RowData>(
    () => ({
      items: timelineItems,
      selectedGlobalIndex,
      onMessageClick: handleMessageClick,
    }),
    [timelineItems, selectedGlobalIndex, handleMessageClick],
  );

  // 监听外层容器高度
  useEffect(() => {
    const outer = outerRef.current;
    if (!outer) return;

    const updateHeight = () => {
      if (outer) setOuterHeight(outer.clientHeight);
    };

    const observer = new ResizeObserver(updateHeight);
    observer.observe(outer);
    updateHeight();
    return () => observer.disconnect();
  }, []);

  // 自动滚动
  useEffect(() => {
    if (autoScroll && listRef.current && timelineItems.length > 0) {
      listRef.current.scrollToItem(timelineItems.length - 1, 'end');
    }
  }, [timelineItems.length, autoScroll]);

  // 列表滚动检测
  const handleListScroll = useCallback(
    ({ scrollDirection }: { scrollDirection: 'forward' | 'backward' }) => {
      if (scrollDirection === 'backward' && live && autoScroll) {
        setUserScrolled(true);
        setAutoScroll(false);
      }
    },
    [live, autoScroll],
  );

  // 总消息数（未过滤）
  const totalMessageCount = useMemo(() => {
    if (!activeStream) return 0;
    let count = 0;
    for (const seg of activeStream.segments) {
      count += seg.messages.length;
    }
    return count;
  }, [activeStream]);

  // ========================================================================
  // 渲染
  // ========================================================================

  // 是否展开详情模式
  const isDetailOpen = selectedMessage !== null;

  // 空流列表
  if (streams.length === 0) {
    return (
      <div style={maxHeight ? { height: maxHeight } : undefined} className="h-full">
        <EmptyState hasConnections={false} />
      </div>
    );
  }

  // 公共头部（连接选择器 + 时间线头部）
  const headerSection = (
    <>
      {streams.length > 1 && (
        <ConnectionSelector
          streams={streams}
          activeIndex={activeStreamIndex}
          onSelect={handleStreamSelect}
        />
      )}
      {activeStream && (
        <TimelineHeader
          stream={activeStream}
          messageCount={totalMessageCount}
          directionFilter={directionFilter}
          onDirectionFilterChange={setDirectionFilter}
          searchQuery={searchQuery}
          onSearchQueryChange={setSearchQuery}
          searchMatchCount={searchMatchCount}
          live={live}
        />
      )}
    </>
  );

  // ------------------------------------------------------------------
  // 报告模式（非实时）：直接渲染，详情内联展开，撑高容器由父级滚动
  // ------------------------------------------------------------------
  if (!live) {
    return (
      <div
        style={!isDetailOpen && maxHeight ? { height: maxHeight } : undefined}
        className={cn('flex flex-col bg-background w-full', !isDetailOpen && 'overflow-hidden')}
      >
        {headerSection}

        {/* 消息列表 */}
        <div className={!isDetailOpen ? 'flex-1 min-h-0 overflow-y-auto' : ''}>
          {timelineItems.length > 0 ? (
            timelineItems.map((item, idx) => {
              const isMessage = item.type === 'message';
              const isSelected = isMessage && selectedGlobalIndex === item.globalIndex;
              return (
                <Fragment key={`${item.type}-${idx}`}>
                  {item.type === 'segment-divider' ? (
                    <div style={{ height: SEGMENT_DIVIDER_HEIGHT }} className="flex items-center">
                      <SegmentDivider segment={item.segment} />
                    </div>
                  ) : (
                    <div style={{ height: MESSAGE_ROW_HEIGHT }}>
                      <MessageRow
                        message={item.message}
                        isSelected={isSelected}
                        onClick={() => handleMessageClick(item.message, item.globalIndex)}
                      />
                    </div>
                  )}
                  {/* 选中消息的详情面板，内联展示在消息下方 */}
                  {isSelected && selectedMessage && (
                    <MessageDetailPanel
                      message={selectedMessage}
                      onClose={handleCloseDetail}
                    />
                  )}
                </Fragment>
              );
            })
          ) : (
            <EmptyState hasConnections />
          )}
        </div>
      </div>
    );
  }

  // ------------------------------------------------------------------
  // 实时模式：虚拟化列表
  // ------------------------------------------------------------------
  return (
    <div
      style={maxHeight ? { height: maxHeight } : undefined}
      className="flex flex-col overflow-hidden bg-background"
    >
      {headerSection}

      <div ref={outerRef} className="flex-1 min-h-0 relative overflow-hidden">
        {timelineItems.length > 0 ? (
          <>
            <List
              ref={listRef}
              height={outerHeight}
              width="100%"
              itemCount={timelineItems.length}
              itemSize={(index: number): number =>
                timelineItems[index]?.type === 'segment-divider'
                  ? SEGMENT_DIVIDER_HEIGHT
                  : MESSAGE_ROW_HEIGHT
              }
              itemData={rowData}
              overscanCount={15}
              onScroll={handleListScroll}
              className="font-mono"
            >
              {TimelineRow}
            </List>

            {live && !autoScroll && userScrolled && (
              <button
                type="button"
                onClick={() => {
                  setAutoScroll(true);
                  setUserScrolled(false);
                }}
                className="absolute bottom-2 right-3 z-10 flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-background border border-border shadow-md text-xs text-muted-foreground hover:text-foreground hover:border-foreground/20 transition-colors"
              >
                <ArrowDownToLine className="w-3 h-3" />
                自动滚动
              </button>
            )}
          </>
        ) : (
          <EmptyState hasConnections />
        )}
      </div>
    </div>
  );
}

export default WsMessageTimeline;
