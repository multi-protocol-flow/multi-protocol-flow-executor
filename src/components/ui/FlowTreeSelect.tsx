/**
 * FlowTreeSelect —— 支持文件夹层级的流程选择下拉
 *
 * 与侧边栏流程列表一致的目录呈现：根目录流程置顶，文件夹可逐级展开/折叠。
 * 支持按名称搜索（搜索时自动展开匹配路径），点击外部或 Esc 关闭。
 */
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { ChevronDown, ChevronRight, FileJson, Folder, Search, X } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface FlowTreeSelectItem {
  /** 流程 UUID */
  uuid: string;
  /** 流程名称 */
  name: string;
  /** 所属文件夹相对路径（空字符串表示根目录） */
  folderPath: string;
}

interface FlowTreeSelectProps {
  /** 当前选中的流程 UUID（空字符串表示未选择） */
  value: string;
  /** 选择变更回调 */
  onValueChange: (uuid: string) => void;
  /** 可选流程列表 */
  items: FlowTreeSelectItem[];
  /** 未选择时的占位文本 */
  placeholder: string;
  /** 搜索框占位文本 */
  searchPlaceholder?: string;
  /** 列表为空时的文本 */
  emptyText?: string;
  /** 搜索无结果时的文本 */
  noResultsText?: string;
  /** 清除当前选择的按钮文案（可访问名） */
  clearLabel?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 附加类名（作用于外层容器） */
  className?: string;
}

interface FlowTree {
  rootFlows: FlowTreeSelectItem[];
  flowsByFolder: Map<string, FlowTreeSelectItem[]>;
  rootFolders: string[];
  childFolders: Map<string, string[]>;
  folderLabels: Map<string, string>;
  folderCounts: Map<string, number>;
}

const byName = (a: string, b: string) => a.localeCompare(b);

/** 依据流程的 folderPath 构建文件夹树（含各文件夹子树流程计数） */
function buildTree(items: FlowTreeSelectItem[]): FlowTree {
  const flowsByFolder = new Map<string, FlowTreeSelectItem[]>();
  const rootFlows: FlowTreeSelectItem[] = [];
  const folderLabels = new Map<string, string>();

  for (const item of items) {
    if (!item.folderPath) {
      rootFlows.push(item);
      continue;
    }
    const parts = item.folderPath.split('/');
    for (let i = 1; i <= parts.length; i += 1) {
      const path = parts.slice(0, i).join('/');
      if (!folderLabels.has(path)) folderLabels.set(path, parts[i - 1]);
    }
    const bucket = flowsByFolder.get(item.folderPath);
    if (bucket) {
      bucket.push(item);
    } else {
      flowsByFolder.set(item.folderPath, [item]);
    }
  }

  const childFolders = new Map<string, string[]>();
  const rootFolders: string[] = [];
  for (const path of folderLabels.keys()) {
    const parent = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
    if (parent && folderLabels.has(parent)) {
      const siblings = childFolders.get(parent);
      if (siblings) {
        siblings.push(path);
      } else {
        childFolders.set(parent, [path]);
      }
    } else {
      rootFolders.push(path);
    }
  }

  rootFolders.sort(byName);
  for (const siblings of childFolders.values()) siblings.sort(byName);
  rootFlows.sort((a, b) => byName(a.name, b.name));
  for (const bucket of flowsByFolder.values()) bucket.sort((a, b) => byName(a.name, b.name));

  // 子树流程计数（含嵌套子文件夹）
  const folderCounts = new Map<string, number>();
  const countOf = (path: string): number => {
    const cached = folderCounts.get(path);
    if (cached !== undefined) return cached;
    let total = flowsByFolder.get(path)?.length ?? 0;
    for (const child of childFolders.get(path) ?? []) total += countOf(child);
    folderCounts.set(path, total);
    return total;
  };
  for (const path of folderLabels.keys()) countOf(path);

  return { rootFlows, flowsByFolder, rootFolders, childFolders, folderLabels, folderCounts };
}

export function FlowTreeSelect({
  value,
  onValueChange,
  items,
  placeholder,
  searchPlaceholder = 'Search...',
  emptyText = 'No options available',
  noResultsText = 'No matching results',
  clearLabel = 'Clear',
  disabled = false,
  className,
}: FlowTreeSelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const tree = useMemo(() => buildTree(items), [items]);
  const selected = items.find((item) => item.uuid === value);

  /** 搜索命中的流程 UUID 集合；无搜索词时为 null（表示不过滤） */
  const matchedUuids = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    if (!keyword) return null;
    const matched = new Set<string>();
    for (const item of items) {
      if (item.name.toLowerCase().includes(keyword)) matched.add(item.uuid);
    }
    return matched;
  }, [items, search]);

  const visibleFlows = useCallback(
    (folderPath: string) =>
      (tree.flowsByFolder.get(folderPath) ?? []).filter(
        (flow) => !matchedUuids || matchedUuids.has(flow.uuid)
      ),
    [tree, matchedUuids]
  );

  /** 文件夹是否包含可见流程（含子文件夹） */
  const hasVisibleFlows = useCallback(
    (folderPath: string): boolean =>
      visibleFlows(folderPath).length > 0 ||
      (tree.childFolders.get(folderPath) ?? []).some(hasVisibleFlows),
    [tree, visibleFlows]
  );

  useEffect(() => {
    if (!open) return;
    const timer = setTimeout(() => searchInputRef.current?.focus(), 30);
    return () => clearTimeout(timer);
  }, [open]);

  useEffect(() => {
    if (!open) {
      setSearch('');
      return;
    }
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [open]);

  /** 展开时自动展开选中流程所在的文件夹链路 */
  useEffect(() => {
    if (!open || !selected?.folderPath) return;
    const parts = selected.folderPath.split('/');
    const chain = new Set<string>();
    for (let i = 1; i <= parts.length; i += 1) chain.add(parts.slice(0, i).join('/'));
    setExpandedFolders((prev) => new Set([...prev, ...chain]));
  }, [open, selected?.folderPath]);

  const toggleFolder = (path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const selectFlow = (uuid: string) => {
    onValueChange(uuid);
    setOpen(false);
  };

  const renderFlowRow = (flow: FlowTreeSelectItem, depth: number): ReactNode => (
    <button
      key={flow.uuid}
      type="button"
      title={flow.name}
      onClick={() => selectFlow(flow.uuid)}
      style={{ paddingLeft: `${depth * 14 + 20}px` }}
      className={cn(
        'flex w-full items-center gap-2 py-1.5 pr-2 text-left text-xs transition-colors hover:bg-accent',
        flow.uuid === value && 'bg-accent font-medium'
      )}
    >
      <FileJson className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
      <span className="truncate">{flow.name}</span>
    </button>
  );

  const renderFolderRow = (paths: string[], depth: number): ReactNode[] => {
    const rows: ReactNode[] = [];
    for (const path of paths) {
      if (!hasVisibleFlows(path)) continue;
      const children = tree.childFolders.get(path) ?? [];
      const isExpanded = matchedUuids ? true : expandedFolders.has(path);

      rows.push(
        <div
          key={`folder-${path}`}
          role="button"
          tabIndex={0}
          onClick={() => toggleFolder(path)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              toggleFolder(path);
            }
          }}
          style={{ paddingLeft: `${depth * 14 + 4}px` }}
          className="flex items-center gap-1 py-1.5 pr-2 text-xs cursor-pointer hover:bg-accent transition-colors"
        >
          <span className="shrink-0 text-muted-foreground">
            {isExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
          </span>
          <Folder className="w-3.5 h-3.5 shrink-0 text-amber-500" />
          <span className="truncate font-medium">{tree.folderLabels.get(path) ?? path}</span>
          <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground tabular-nums">
            {tree.folderCounts.get(path) ?? 0}
          </span>
        </div>
      );

      if (isExpanded) {
        rows.push(...renderFolderRow(children, depth + 1));
        for (const flow of visibleFlows(path)) rows.push(renderFlowRow(flow, depth + 1));
      }
    }
    return rows;
  };

  const visibleRootFlows = tree.rootFlows.filter(
    (flow) => !matchedUuids || matchedUuids.has(flow.uuid)
  );
  const isEmptyList = items.length === 0;
  const isNoMatch = !isEmptyList && visibleRootFlows.length === 0 && !tree.rootFolders.some(hasVisibleFlows);

  return (
    <div ref={containerRef} className={cn('relative', className)}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((prev) => !prev)}
        className={cn(
          'flex h-9 w-full min-w-0 items-center justify-between gap-2 rounded-md border border-input bg-background px-3 py-2 text-xs text-left',
          'focus:outline-none focus:border-[3px] focus:border-ring disabled:cursor-not-allowed disabled:opacity-50',
          !selected && 'text-muted-foreground'
        )}
      >
        <span className="flex min-w-0 items-center gap-1.5">
          {selected ? (
            <>
              <FileJson className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
              <span className="truncate" title={selected.name}>
                {selected.name}
              </span>
              {selected.folderPath && (
                <span className="shrink-0 truncate max-w-[110px] text-[10px] text-muted-foreground">
                  {selected.folderPath}
                </span>
              )}
            </>
          ) : (
            <span className="truncate">{placeholder}</span>
          )}
        </span>
        <span className="flex shrink-0 items-center gap-1">
          {selected && !disabled && (
            <span
              role="button"
              aria-label={clearLabel}
              title={clearLabel}
              onClick={(e) => {
                e.stopPropagation();
                onValueChange('');
              }}
            >
              <X className="h-3.5 w-3.5 opacity-50 hover:opacity-100" />
            </span>
          )}
          <ChevronDown
            className={cn('h-4 w-4 opacity-50 transition-transform', open && 'rotate-180')}
          />
        </span>
      </button>

      {open && (
        <div className="absolute z-50 mt-1 w-full min-w-[14rem] rounded-md border border-border bg-popover text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95">
          <div className="flex items-center border-b border-border px-3">
            <Search className="mr-2 h-3.5 w-3.5 shrink-0 opacity-50" />
            <input
              ref={searchInputRef}
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={searchPlaceholder}
              className="flex h-8 w-full bg-transparent py-2 text-xs outline-none placeholder:text-muted-foreground"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch('')}
                className="shrink-0 opacity-50 hover:opacity-100"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>

          {isEmptyList ? (
            <div className="px-3 py-6 text-center text-xs text-muted-foreground">{emptyText}</div>
          ) : isNoMatch ? (
            <div className="px-3 py-6 text-center text-xs text-muted-foreground">
              {noResultsText}
            </div>
          ) : (
            <div className="max-h-64 overflow-y-auto py-1">
              {/* 与侧边栏流程列表一致：文件夹（含嵌套子文件夹）优先，根目录流程排在最后 */}
              {renderFolderRow(tree.rootFolders, 0)}
              {visibleRootFlows.map((flow) => renderFlowRow(flow, 0))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
