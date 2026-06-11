/**
 * HexViewer - 虚拟化十六进制查看器
 * 三栏布局（Address | Hex Bytes | ASCII），支持 1MB+ 载荷渲染
 * 内置搜索、跳转到偏移量、复制功能
 */
import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { Search, ArrowRightLeft, Copy, Check } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';

// ============================================================================
// 常量
// ============================================================================

const ROW_HEIGHT = 20;
const BYTES_PER_ROW = 16;
const BUFFER_ROWS = 4;

// ============================================================================
// 工具函数
// ============================================================================

/** 判断字节是否为可打印 ASCII */
function isPrintableAscii(byte: number): boolean {
  return byte >= 0x20 && byte <= 0x7e;
}

/** 将字符串数据转为 Uint8Array（兼容 Base64 和原始文本） */
function dataToBytes(data: string): Uint8Array {
  // 尝试 Base64 解码
  try {
    const decoded = atob(data);
    // 检测是否为有效 Base64（解码后不应包含过多控制字符）
    const bytes = Uint8Array.from(decoded, c => c.charCodeAt(0));
    return bytes;
  } catch {
    // 非 Base64，按原始文本处理
    const bytes = new Uint8Array(data.length);
    for (let i = 0; i < data.length; i++) {
      bytes[i] = data.charCodeAt(i) & 0xff;
    }
    return bytes;
  }
}

/** 解析十六进制搜索字符串为字节数组 */
function parseHexSearch(hex: string): number[] | null {
  const cleaned = hex.replace(/\s+/g, '').toLowerCase();
  if (cleaned.length === 0) return null;
  if (cleaned.length % 2 !== 0) return null;
  if (!/^[0-9a-f]*$/.test(cleaned)) return null;

  const result: number[] = [];
  for (let i = 0; i < cleaned.length; i += 2) {
    result.push(parseInt(cleaned.substring(i, i + 2), 16));
  }
  return result;
}

/** 在数据中搜索字节模式，返回所有匹配起始偏移量 */
function findMatchOffsets(data: Uint8Array, pattern: number[]): number[] {
  if (pattern.length === 0 || pattern.length > data.length) return [];

  const offsets: number[] = [];
  const maxStart = data.length - pattern.length;

  for (let i = 0; i <= maxStart; i++) {
    let match = true;
    for (let j = 0; j < pattern.length; j++) {
      if (data[i + j] !== pattern[j]) {
        match = false;
        break;
      }
    }
    if (match) {
      offsets.push(i);
    }
  }
  return offsets;
}

/** 解析偏移量字符串（支持十进制和 0x 前缀十六进制） */
function parseOffset(input: string): number | null {
  const trimmed = input.trim();
  if (trimmed.length === 0) return null;

  if (trimmed.startsWith('0x') || trimmed.startsWith('0X')) {
    const val = parseInt(trimmed, 16);
    return isNaN(val) ? null : val;
  }

  const val = parseInt(trimmed, 10);
  return isNaN(val) ? null : val;
}

// ============================================================================
// Props
// ============================================================================

interface HexViewerProps {
  /** 二进制数据 */
  data: Uint8Array;
  /** 最大显示字节数（默认 unlimited） */
  maxDisplayBytes?: number;
  /** 复制回调 */
  onCopy?: (data: string) => void;
  /** 是否只读（默认 true） */
  readOnly?: boolean;
  /** 容器高度（默认 400px） */
  height?: number;
}

// ============================================================================
// ByteCell — 单字节渲染
// ============================================================================

function ByteCell({
  byte,
  isHighlighted,
}: {
  byte: number;
  isHighlighted: boolean;
}) {
  const hex = byte.toString(16).padStart(2, '0');

  let colorClass: string;
  if (byte === 0x00) {
    colorClass = 'text-gray-600';
  } else if (isPrintableAscii(byte)) {
    colorClass = 'text-sky-400';
  } else {
    colorClass = 'text-red-400';
  }

  if (isHighlighted) {
    colorClass = 'text-gray-900';
  }

  return (
    <span
      className={`${isHighlighted ? 'bg-yellow-400 rounded-sm' : ''} ${colorClass}`}
    >
      {hex}
    </span>
  );
}

// ============================================================================
// AsciiCell — 单个 ASCII 字符
// ============================================================================

function AsciiCell({
  byte,
  isHighlighted,
}: {
  byte: number;
  isHighlighted: boolean;
}) {
  const char = isPrintableAscii(byte) ? String.fromCharCode(byte) : '.';

  return (
    <span
      className={`${isHighlighted ? 'bg-yellow-400 text-gray-900 rounded-sm' : ''}`}
    >
      {char}
    </span>
  );
}

// ============================================================================
// HexRow — 单行渲染（16 字节）
// ============================================================================

function HexRow({
  rowOffset,
  data,
  dataLength,
  highlightedSet,
}: {
  rowOffset: number;
  data: Uint8Array;
  dataLength: number;
  highlightedSet: Set<number>;
}) {
  const address = rowOffset.toString(16).padStart(8, '0');

  // Hex 部分：16 字节，8 字节一组
  const hexCells: React.ReactNode[] = [];
  for (let i = 0; i < BYTES_PER_ROW; i++) {
    const offset = rowOffset + i;
    if (offset < dataLength) {
      hexCells.push(
        <ByteCell
          key={i}
          byte={data[offset]}
          isHighlighted={highlightedSet.has(offset)}
        />
      );
    } else {
      hexCells.push(<span key={i}>{'  '}</span>);
    }

    // 第 8 字节后加额外间距
    if (i === 7) {
      hexCells.push(<span key="sep">{'  '}</span>);
    }
  }

  // ASCII 部分
  const asciiCells: React.ReactNode[] = [];
  for (let i = 0; i < BYTES_PER_ROW; i++) {
    const offset = rowOffset + i;
    if (offset < dataLength) {
      asciiCells.push(
        <AsciiCell
          key={i}
          byte={data[offset]}
          isHighlighted={highlightedSet.has(offset)}
        />
      );
    }
  }

  return (
    <div
      className="flex items-center whitespace-pre"
      style={{ height: ROW_HEIGHT }}
    >
      {/* Address */}
      <span className="text-gray-500 w-[72px] shrink-0 select-none">{address}</span>
      {/* Separator */}
      <span className="text-gray-700 w-[16px] shrink-0 select-none">{'  '}</span>
      {/* Hex bytes */}
      <span className="tracking-wider">{hexCells}</span>
      {/* Separator */}
      <span className="text-gray-700 w-[16px] shrink-0 select-none">{'  '}</span>
      {/* ASCII */}
      <span className="text-gray-400">{asciiCells}</span>
    </div>
  );
}

// ============================================================================
// HeaderRow — 列头
// ============================================================================

function HeaderRow() {
  return (
    <div
      className="flex items-center whitespace-pre text-gray-500 bg-gray-800/80 border-b border-gray-700/50"
      style={{ height: ROW_HEIGHT }}
    >
      <span className="w-[72px] shrink-0 select-none text-[10px]">Offset</span>
      <span className="w-[16px] shrink-0 select-none" />
      <span className="text-[10px] tracking-wider">
        {'00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f'}
      </span>
      <span className="w-[16px] shrink-0 select-none" />
      <span className="text-[10px]">0123456789abcdef</span>
    </div>
  );
}

// ============================================================================
// HexViewer 主组件
// ============================================================================

export function HexViewer({
  data,
  maxDisplayBytes,
  readOnly = true,
  height = 400,
}: HexViewerProps) {
  // 显示数据（截断到最大字节数）
  const displayData = useMemo(() => {
    if (maxDisplayBytes !== undefined && data.length > maxDisplayBytes) {
      return data.slice(0, maxDisplayBytes);
    }
    return data;
  }, [data, maxDisplayBytes]);

  const dataLength = displayData.length;
  const totalRows = Math.ceil(dataLength / BYTES_PER_ROW);
  const totalHeight = totalRows * ROW_HEIGHT;

  // 滚动状态
  const [scrollTop, setScrollTop] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  // 搜索状态
  const [searchInput, setSearchInput] = useState('');
  const [matchOffsets, setMatchOffsets] = useState<number[]>([]);
  const [currentMatchIndex, setCurrentMatchIndex] = useState(-1);

  // 跳转偏移量
  const [offsetInput, setOffsetInput] = useState('');

  // 复制状态
  const [copied, setCopied] = useState(false);

  // 搜索匹配的高亮字节偏移集合
  const highlightedSet = useMemo(() => {
    const set = new Set<number>();
    if (matchOffsets.length === 0) return set;
    for (const offset of matchOffsets) {
      for (let i = 0; i < (parseHexSearch(searchInput)?.length ?? 0); i++) {
        set.add(offset + i);
      }
    }
    return set;
  }, [matchOffsets, searchInput]);

  // 当前匹配高亮（更亮的颜色）
  const currentHighlightSet = useMemo(() => {
    const set = new Set<number>();
    if (currentMatchIndex < 0 || currentMatchIndex >= matchOffsets.length) return set;
    const offset = matchOffsets[currentMatchIndex];
    const patternLen = parseHexSearch(searchInput)?.length ?? 0;
    for (let i = 0; i < patternLen; i++) {
      set.add(offset + i);
    }
    return set;
  }, [currentMatchIndex, matchOffsets, searchInput]);

  // 合并高亮集
  const allHighlighted = useMemo(() => {
    const merged = new Set(highlightedSet);
    for (const v of currentHighlightSet) {
      merged.add(v);
    }
    return merged;
  }, [highlightedSet, currentHighlightSet]);

  // 搜索
  const handleSearch = useCallback(() => {
    const pattern = parseHexSearch(searchInput);
    if (!pattern || pattern.length === 0) {
      setMatchOffsets([]);
      setCurrentMatchIndex(-1);
      return;
    }
    const offsets = findMatchOffsets(displayData, pattern);
    setMatchOffsets(offsets);
    setCurrentMatchIndex(offsets.length > 0 ? 0 : -1);
  }, [searchInput, displayData]);

  // 搜索输入变化时重置
  useEffect(() => {
    if (searchInput.length === 0) {
      setMatchOffsets([]);
      setCurrentMatchIndex(-1);
    }
  }, [searchInput]);

  // 当前匹配跳转
  useEffect(() => {
    if (currentMatchIndex < 0 || matchOffsets.length === 0) return;
    const targetOffset = matchOffsets[currentMatchIndex];
    const targetRow = Math.floor(targetOffset / BYTES_PER_ROW);
    const targetScrollTop = targetRow * ROW_HEIGHT;

    if (scrollRef.current) {
      const containerH = scrollRef.current.clientHeight - ROW_HEIGHT; // 减去 header
      const currentViewStart = scrollRef.current.scrollTop;
      const currentViewEnd = currentViewStart + containerH;

      if (targetScrollTop < currentViewStart || targetScrollTop + ROW_HEIGHT > currentViewEnd) {
        scrollRef.current.scrollTop = Math.max(0, targetScrollTop - containerH / 3);
      }
    }
  }, [currentMatchIndex, matchOffsets]);

  // 下一个匹配
  const handleNextMatch = useCallback(() => {
    if (matchOffsets.length === 0) return;
    setCurrentMatchIndex(prev =>
      prev < matchOffsets.length - 1 ? prev + 1 : 0
    );
  }, [matchOffsets]);

  // 上一个匹配
  const handlePrevMatch = useCallback(() => {
    if (matchOffsets.length === 0) return;
    setCurrentMatchIndex(prev =>
      prev > 0 ? prev - 1 : matchOffsets.length - 1
    );
  }, [matchOffsets]);

  // 跳转到偏移量
  const handleGoToOffset = useCallback(() => {
    const offset = parseOffset(offsetInput);
    if (offset === null || offset < 0 || offset >= dataLength) return;

    const targetRow = Math.floor(offset / BYTES_PER_ROW);
    const targetScrollTop = targetRow * ROW_HEIGHT;

    if (scrollRef.current) {
      const containerH = scrollRef.current.clientHeight - ROW_HEIGHT;
      scrollRef.current.scrollTop = Math.max(0, targetScrollTop - containerH / 3);
    }
    setOffsetInput('');
  }, [offsetInput, dataLength]);

  // 复制
  const handleCopy = useCallback(() => {
    const hexLines: string[] = [];
    for (let offset = 0; offset < dataLength; offset += BYTES_PER_ROW) {
      const address = offset.toString(16).padStart(8, '0');
      const hexParts: string[] = [];
      for (let i = 0; i < BYTES_PER_ROW; i++) {
        if (offset + i < dataLength) {
          hexParts.push(displayData[offset + i].toString(16).padStart(2, '0'));
        }
        if (i === 7) hexParts.push('');
      }
      const hexStr = hexParts.join(' ');
      const slice = Array.from(
        displayData.slice(offset, Math.min(offset + BYTES_PER_ROW, dataLength))
      );
      const asciiStr = slice
        .map(b => (isPrintableAscii(b) ? String.fromCharCode(b) : '.'))
        .join('');
      hexLines.push(`${address}  ${hexStr}  |${asciiStr}|`);
    }

    navigator.clipboard.writeText(hexLines.join('\n')).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [displayData, dataLength]);

  // 滚动处理
  const handleScroll = useCallback(() => {
    if (scrollRef.current) {
      setScrollTop(scrollRef.current.scrollTop);
    }
  }, []);

  // 计算可见行范围
  const containerHeight = height - ROW_HEIGHT - 40; // 减去 header + toolbar
  const visibleStart = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS);
  const visibleEnd = Math.min(
    totalRows,
    Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER_ROWS
  );
  const visibleRows = useMemo(() => {
    const rows: number[] = [];
    for (let i = visibleStart; i < visibleEnd; i++) {
      rows.push(i);
    }
    return rows;
  }, [visibleStart, visibleEnd]);

  // 无数据
  if (dataLength === 0) {
    return (
      <div
        className="bg-gray-900 text-gray-100 font-mono text-xs rounded-lg flex items-center justify-center text-gray-500"
        style={{ height }}
      >
        {getLL().panels.hex_empty()}
      </div>
    );
  }

  return (
    <div className="bg-gray-900 text-gray-100 font-mono text-xs rounded-lg overflow-hidden">
      {/* 工具栏 */}
      {!readOnly && (
        <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-700/50 bg-gray-800/50">
          {/* 搜索 */}
          <div className="flex items-center gap-1 flex-1">
            <Search className="w-3.5 h-3.5 text-gray-500 shrink-0" />
            <input
              type="text"
              value={searchInput}
              onChange={e => setSearchInput(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') handleSearch();
                if (e.key === 'Escape') {
                  setSearchInput('');
                  setMatchOffsets([]);
                  setCurrentMatchIndex(-1);
                }
              }}
              placeholder={getLL().panels.hex_search_placeholder()}
              className="bg-gray-800 text-gray-200 text-xs px-2 py-1 rounded border border-gray-700
                         placeholder-gray-600 focus:outline-none focus:border-sky-500 flex-1 min-w-0"
            />
            {matchOffsets.length > 0 && (
              <span className="text-gray-500 text-[10px] shrink-0">
                {currentMatchIndex + 1}/{matchOffsets.length}
              </span>
            )}
          </div>

          {/* 跳转 */}
          <div className="flex items-center gap-1">
            <ArrowRightLeft className="w-3.5 h-3.5 text-gray-500 shrink-0" />
            <input
              type="text"
              value={offsetInput}
              onChange={e => setOffsetInput(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') handleGoToOffset();
                if (e.key === 'Escape') setOffsetInput('');
              }}
              placeholder={getLL().panels.hex_offset_placeholder()}
              className="bg-gray-800 text-gray-200 text-xs px-2 py-1 rounded border border-gray-700
                         placeholder-gray-600 focus:outline-none focus:border-sky-500 w-24"
            />
          </div>

          {/* 复制 */}
          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 rounded bg-gray-800 border border-gray-700
                       text-gray-400 hover:text-gray-200 hover:border-gray-500 transition-colors shrink-0"
            title={getLL().panels.hex_copy_title()}
          >
            {copied ? (
              <Check className="w-3.5 h-3.5 text-emerald-400" />
            ) : (
              <Copy className="w-3.5 h-3.5" />
            )}
            <span className="text-[10px]">{copied ? getLL().panels.hex_copied() : getLL().panels.hex_copy()}</span>
          </button>
        </div>
      )}

      {/* 搜索导航按钮 */}
      {matchOffsets.length > 1 && (
        <div className="flex items-center gap-1 px-3 py-1 border-b border-gray-700/30 bg-gray-800/30">
          <button
            type="button"
            onClick={handlePrevMatch}
            className="px-2 py-0.5 text-[10px] text-gray-400 hover:text-gray-200 bg-gray-800 rounded border border-gray-700 hover:border-gray-500 transition-colors"
          >
            {getLL().panels.hex_prev_match()}
          </button>
          <button
            type="button"
            onClick={handleNextMatch}
            className="px-2 py-0.5 text-[10px] text-gray-400 hover:text-gray-200 bg-gray-800 rounded border border-gray-700 hover:border-gray-500 transition-colors"
          >
            {getLL().panels.hex_next_match()}
          </button>
          <span className="text-[10px] text-gray-500 ml-2">
            {getLL().panels.hex_match_count({ count: matchOffsets.length })}
          </span>
        </div>
      )}

      {/* 截断提示 */}
      {maxDisplayBytes !== undefined && data.length > maxDisplayBytes && (
        <div className="px-3 py-1 bg-amber-950/30 border-b border-amber-800/30 text-amber-400 text-[10px]">
          {getLL().panels.hex_truncated_notice({ displayed: maxDisplayBytes.toLocaleString(), total: data.length.toLocaleString() })}
        </div>
      )}

      {/* 十六进制内容区域 */}
      <div
        ref={scrollRef}
        className="overflow-auto"
        style={{ height: height - (readOnly ? 0 : 40) - (matchOffsets.length > 1 ? 28 : 0) - (maxDisplayBytes !== undefined && data.length > maxDisplayBytes ? 24 : 0) }}
        onScroll={handleScroll}
      >
        {/* 固定表头 */}
        <div className="sticky top-0 z-10">
          <HeaderRow />
        </div>

        {/* 虚拟化行容器 */}
        <div ref={containerRef} style={{ height: totalHeight, position: 'relative' }}>
          <div
            style={{
              position: 'absolute',
              top: visibleStart * ROW_HEIGHT,
              left: 0,
              right: 0,
            }}
          >
            {visibleRows.map(rowIndex => (
              <HexRow
                key={rowIndex}
                rowOffset={rowIndex * BYTES_PER_ROW}
                data={displayData}
                dataLength={dataLength}
                highlightedSet={allHighlighted}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// 便捷 Hook: 从字符串数据创建 HexViewer 所需的 Uint8Array
// ============================================================================

export { dataToBytes };
