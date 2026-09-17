/**
 * StreamInspectorPanel - 长连接实时探针控制台
 * 提供多连接切换、双向消息瀑布流、实时探针注入、Ping/Pong 保活心跳及快速断言生成
 */
import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import {
  Wifi,
  ArrowUp,
  ArrowDown,
  Send,
  Radio,
  Copy,
  Check,
  Search,
  Filter,
  Activity,
  Code,
  FileText,
  Binary,
} from 'lucide-react';
import { useWsStreamStore } from '@/stores/wsStream';
import { useDebugStore } from '@/stores/debug';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import type { WsConnectionStream, WsMessage } from '@/types/ws_stream';

interface StreamInspectorPanelProps {
  streams: WsConnectionStream[];
  live?: boolean;
}

type PayloadFormat = 'json' | 'text' | 'binary';

function formatTimestamp(timestamp: number): string {
  const d = new Date(timestamp);
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  const sss = String(d.getMilliseconds()).padStart(3, '0');
  return `${hh}:${mm}:${ss}.${sss}`;
}

function getMessageText(msg: WsMessage): string {
  if (msg.content.type === 'text') {
    return msg.content.content;
  }
  if (msg.content.type === 'binary') {
    return getLL().flow.stream_probe_binary_bytes({ count: msg.content.content.length });
  }
  return '';
}

export function StreamInspectorPanel({ streams, live = false }: StreamInspectorPanelProps) {
  const ll = getLL();
  const sendLiveFrame = useWsStreamStore((s) => s.sendLiveFrame);
  const isPaused = useDebugStore((s) => s.status === 'paused');

  // 当前选中连接
  const [selectedConnId, setSelectedConnId] = useState<string>(() => {
    return streams[0]?.connection_id ?? '';
  });

  useEffect(() => {
    if (!selectedConnId && streams.length > 0) {
      setSelectedConnId(streams[0].connection_id);
    }
  }, [streams, selectedConnId]);

  const currentStream = useMemo(() => {
    return streams.find((s) => s.connection_id === selectedConnId) ?? streams[0];
  }, [streams, selectedConnId]);

  // 控制台发送状态
  const [format, setFormat] = useState<PayloadFormat>('json');
  const [inputPayload, setInputPayload] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [keepAlive, setKeepAlive] = useState(false);
  const [copiedMsgId, setCopiedMsgId] = useState<string | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // 提取所有消息列表
  const allMessages = useMemo<WsMessage[]>(() => {
    if (!currentStream) return [];
    return currentStream.segments.flatMap((seg) => seg.messages);
  }, [currentStream]);

  // 过滤消息
  const filteredMessages = useMemo(() => {
    if (!searchQuery.trim()) return allMessages;
    const q = searchQuery.toLowerCase();
    return allMessages.filter((m) => getMessageText(m).toLowerCase().includes(q));
  }, [allMessages, searchQuery]);

  // 自动滚动到最新消息
  useEffect(() => {
    if (messagesEndRef.current && typeof messagesEndRef.current.scrollIntoView === 'function') {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [filteredMessages.length]);

  // 15s 自动保活心跳
  useEffect(() => {
    if (!keepAlive || !currentStream || currentStream.is_closed) return;
    const interval = setInterval(() => {
      sendLiveFrame(currentStream.connection_id, JSON.stringify({ type: 'ping', timestamp: Date.now() })).catch(() => {});
    }, 15000);

    return () => clearInterval(interval);
  }, [keepAlive, currentStream, sendLiveFrame]);

  // 发送数据帧
  const handleSend = useCallback(async () => {
    if (!currentStream || !inputPayload.trim()) return;

    try {
      await sendLiveFrame(
        currentStream.connection_id,
        inputPayload.trim(),
        format === 'binary'
      );
      toast.success(ll.flow.stream_probe_send());
      setInputPayload('');
    } catch (err) {
      toast.error(String(err));
    }
  }, [currentStream, inputPayload, format, sendLiveFrame, ll]);

  // 快捷发送 Ping
  const handleSendPing = async () => {
    if (!currentStream) return;
    try {
      const pingBody = format === 'json' ? JSON.stringify({ type: 'ping', time: Date.now() }) : 'ping';
      await sendLiveFrame(currentStream.connection_id, pingBody, false);
      toast.success(ll.flow.stream_probe_ping());
    } catch (err) {
      toast.error(String(err));
    }
  };

  // 快捷发送 Pong
  const handleSendPong = async () => {
    if (!currentStream) return;
    try {
      const pongBody = format === 'json' ? JSON.stringify({ type: 'pong', time: Date.now() }) : 'pong';
      await sendLiveFrame(currentStream.connection_id, pongBody, false);
      toast.success(ll.flow.stream_probe_pong());
    } catch (err) {
      toast.error(String(err));
    }
  };

  // 快捷复制为断言表达式
  const handleGenerateAssertion = async (msg: WsMessage, id: string) => {
    const text = getMessageText(msg);
    let assertionExpr = '$.status === 200';

    try {
      const parsed = JSON.parse(text);
      if (typeof parsed === 'object' && parsed !== null) {
        const keys = Object.keys(parsed);
        if (keys.includes('type')) {
          assertionExpr = `$.type === "${parsed.type}"`;
        } else if (keys.includes('code')) {
          assertionExpr = `$.code === ${JSON.stringify(parsed.code)}`;
        } else if (keys.includes('status')) {
          assertionExpr = `$.status === ${JSON.stringify(parsed.status)}`;
        } else if (keys.length > 0) {
          assertionExpr = `$.${keys[0]} !== undefined`;
        }
      }
    } catch {
      assertionExpr = `$.includes("${text.slice(0, 20)}")`;
    }

    try {
      await navigator.clipboard.writeText(assertionExpr);
      setCopiedMsgId(id);
      setTimeout(() => setCopiedMsgId(null), 2000);
      toast.success(ll.flow.stream_probe_copy_filter_success({ expr: assertionExpr }));
    } catch {
      toast.error(ll.execution.report.copy_failed());
    }
  };

  if (!currentStream) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground p-6">
        <Wifi className="w-8 h-8 stroke-1 text-muted-foreground/50 mb-2" />
        <span className="text-xs">{ll.flow.stream_probe_no_conns()}</span>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-background text-foreground overflow-hidden">
      {/* 顶部工具栏：连接选择器 + 状态 + 搜索 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border bg-card/60 gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <Wifi className={cn('w-4 h-4 shrink-0', currentStream.is_closed ? 'text-muted-foreground' : 'text-emerald-500')} />
          <select
            value={currentStream.connection_id}
            onChange={(e) => setSelectedConnId(e.target.value)}
            className="text-xs font-medium bg-background border border-border rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-primary truncate max-w-[200px]"
          >
            {streams.map((s) => (
              <option key={s.connection_id} value={s.connection_id}>
                {s.connection_id} ({s.url})
              </option>
            ))}
          </select>

          <span
            className={cn(
              'px-1.5 py-0.5 rounded text-[10px] font-mono shrink-0',
              currentStream.is_closed
                ? 'bg-muted text-muted-foreground'
                : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-300 dark:border-emerald-800'
            )}
          >
            {currentStream.is_closed
              ? ll.flow.stream_probe_status_closed()
              : ll.flow.stream_probe_status_connected()}
          </span>

          {live && (
            <span className="flex items-center gap-1 text-[10px] text-emerald-600 dark:text-emerald-400 font-mono">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
              {ll.flow.stream_probe_live_badge()}
            </span>
          )}
          {isPaused && (
            <span className="text-[10px] text-amber-600 dark:text-amber-400 font-mono px-1 rounded bg-amber-500/10 border border-amber-300 dark:border-amber-800 hidden sm:inline">
              {ll.flow.stream_probe_active_badge()}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {/* 自动心跳保活 */}
          <label className="flex items-center gap-1.5 text-xs text-muted-foreground select-none cursor-pointer">
            <input
              type="checkbox"
              checked={keepAlive}
              onChange={(e) => setKeepAlive(e.target.checked)}
              className="rounded border-border"
            />
            <Activity className="w-3 h-3 text-amber-500" />
            <span className="text-[11px] hidden sm:inline">{ll.flow.stream_probe_keepalive()}</span>
          </label>

          {/* 搜索框 */}
          <div className="relative">
            <Search className="w-3 h-3 absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={ll.flow.stream_probe_filter_placeholder()}
              className="h-6 pl-6 pr-2 text-xs rounded border border-border bg-background w-28 sm:w-36 focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
        </div>
      </div>

      {/* 中部：双向 Message 瀑布流 */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2 font-mono text-xs">
        {filteredMessages.length === 0 ? (
          <div className="h-full flex items-center justify-center text-muted-foreground/60 text-xs">
            {ll.flow.stream_probe_no_frames()}
          </div>
        ) : (
          filteredMessages.map((msg, idx) => {
            const isSent = msg.direction === 'sent';
            const msgText = getMessageText(msg);
            const msgId = `${msg.timestamp}-${idx}`;

            return (
              <div
                key={msgId}
                className={cn(
                  'rounded border p-2 text-xs space-y-1 transition-colors shadow-2xs group',
                  isSent
                    ? 'border-emerald-500/30 bg-emerald-50/20 dark:bg-emerald-950/15'
                    : 'border-blue-500/30 bg-blue-50/20 dark:bg-blue-950/15'
                )}
              >
                {/* 帧头信息 */}
                <div className="flex items-center justify-between text-muted-foreground text-[11px]">
                  <div className="flex items-center gap-1.5 font-semibold">
                    {isSent ? (
                      <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
                        <ArrowUp className="w-3 h-3" />
                        <span>OUT</span>
                      </span>
                    ) : (
                      <span className="flex items-center gap-1 text-sky-600 dark:text-sky-400">
                        <ArrowDown className="w-3 h-3" />
                        <span>IN</span>
                      </span>
                    )}
                    <span className="text-muted-foreground/80 font-normal">{formatTimestamp(msg.timestamp)}</span>
                  </div>

                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    {/* 复制文本 */}
                    <button
                      onClick={async () => {
                        await navigator.clipboard.writeText(msgText);
                        toast.success(ll.flow.stream_probe_copy_frame_success());
                      }}
                      className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground"
                      title={ll.flow.stream_probe_copy_frame()}
                    >
                      <Copy className="w-3 h-3" />
                    </button>

                    {/* 以此生成断言表达式 */}
                    {!isSent && (
                      <button
                        onClick={() => handleGenerateAssertion(msg, msgId)}
                        className="px-1.5 py-0.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground text-[10px] flex items-center gap-1"
                        title={ll.flow.stream_probe_copy_filter()}
                      >
                        {copiedMsgId === msgId ? (
                          <Check className="w-3 h-3 text-green-500" />
                        ) : (
                          <Filter className="w-3 h-3 text-teal-500" />
                        )}
                        <span>{ll.flow.stream_probe_copy_filter()}</span>
                      </button>
                    )}
                  </div>
                </div>

                {/* 消息体 */}
                <pre className="text-foreground text-xs whitespace-pre-wrap break-all max-h-40 overflow-y-auto bg-muted/40 rounded p-1.5 font-mono">
                  {msgText}
                </pre>
              </div>
            );
          })
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* 底部：交互控制台 */}
      <div className="border-t border-border bg-card p-2.5 space-y-2">
        <div className="flex items-center justify-between gap-2">
          {/* 格式切换 */}
          <div className="flex rounded border border-border bg-muted/50 p-0.5 shrink-0">
            <button
              type="button"
              onClick={() => setFormat('json')}
              className={cn(
                'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                format === 'json' ? 'bg-background shadow-xs text-foreground' : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <Code className="w-3 h-3" />
              <span>{ll.flow.stream_probe_format_json()}</span>
            </button>
            <button
              type="button"
              onClick={() => setFormat('text')}
              className={cn(
                'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                format === 'text' ? 'bg-background shadow-xs text-foreground' : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <FileText className="w-3 h-3" />
              <span>{ll.flow.stream_probe_format_text()}</span>
            </button>
            <button
              type="button"
              onClick={() => setFormat('binary')}
              className={cn(
                'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                format === 'binary' ? 'bg-background shadow-xs text-foreground' : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <Binary className="w-3 h-3" />
              <span>{ll.flow.stream_probe_format_bin()}</span>
            </button>
          </div>

          {/* 快捷 Ping / Pong */}
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={handleSendPing}
              disabled={currentStream.is_closed}
              className="px-2 py-0.5 rounded border border-border text-xs hover:bg-accent text-muted-foreground hover:text-foreground disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1"
            >
              <Radio className="w-3 h-3 text-amber-500" />
              <span>{ll.flow.stream_probe_ping()}</span>
            </button>
            <button
              type="button"
              onClick={handleSendPong}
              disabled={currentStream.is_closed}
              className="px-2 py-0.5 rounded border border-border text-xs hover:bg-accent text-muted-foreground hover:text-foreground disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1"
            >
              <Radio className="w-3 h-3 text-emerald-500" />
              <span>{ll.flow.stream_probe_pong()}</span>
            </button>
          </div>
        </div>

        {/* 输入框与发送按钮 */}
        <div className="flex items-end gap-2">
          <textarea
            value={inputPayload}
            onChange={(e) => setInputPayload(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder={ll.flow.stream_probe_placeholder()}
            rows={2}
            className="flex-1 px-2.5 py-1.5 text-xs font-mono rounded border border-border bg-background placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-primary resize-none"
          />

          <button
            type="button"
            onClick={handleSend}
            disabled={!inputPayload.trim() || currentStream.is_closed}
            className="h-14 px-4 rounded bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 disabled:opacity-40 disabled:cursor-not-allowed flex flex-col items-center justify-center gap-1 shrink-0 shadow-xs"
          >
            <Send className="w-3.5 h-3.5" />
            <span>{ll.flow.stream_probe_send()}</span>
          </button>
        </div>
      </div>
    </div>
  );
}
