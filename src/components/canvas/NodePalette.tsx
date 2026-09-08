/**
 * NodePalette - 双栏插件化节点选择器
 *
 * 采用双栏布局（左侧插件/内置分类导航，右侧算子列表与详情卡片），
 * 深度融合插件体系生态，支持全库模糊搜索、协议别名映射、
 * 键盘上下左右及回车操作、拖拽与点击添加、插件管理快捷跳转等。
 */
import {
  Search,
  X,
  Settings,
  Layers,
} from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { useFlowStore, useUIStore } from '@/stores';
import type { NodeType } from '@/types';
import type { NodeMetadata } from '@/types/nodeRegistry';
import {
  getNodeMetadata,
  buildCatalogGroups,
  type CatalogGroup,
  type CatalogItem,
} from '@/lib/NodeRegistry';
import { cn } from '@/lib/utils';
import { useEffect, useMemo, useState, useCallback, useRef } from 'react';

// ============================================================================
// 常量与尺寸规范
// ============================================================================

const PALETTE_WIDTH = 580;
const PALETTE_HEIGHT = 440;
const USAGE_KEY = 'node-palette-usage';

// ============================================================================
// 搜索别名映射
// ============================================================================

const SEARCH_ALIASES: Record<string, string[]> = {
  rest: ['http'],
  api: ['http'],
  post: ['http'],
  get: ['http'],
  put: ['http'],
  delete: ['http'],
  stream: ['sse_listen', 'ws_send_collect'],
  trigger: ['sse_listen'],
  realtime: ['ws_connect', 'sse_listen'],
  ws: ['ws_connect', 'ws_send_collect', 'ws_close'],
  socket: ['ws_connect', 'tcp_connect'],
  pubsub: ['graphql_subscribe', 'sse_listen'],
  graphql: ['graphql_query', 'graphql_connect', 'graphql_subscribe'],
  assertion: ['assertion'],
  assert: ['assertion'],
  check: ['assertion'],
  if: ['conditional'],
  else: ['conditional'],
  branch: ['conditional'],
  js: ['script'],
  ts: ['script'],
  code: ['script'],
  extract: ['variable_extractor'],
  jsonpath: ['variable_extractor'],
  proxy: ['proxy'],
};

function expandAliases(query: string): string[] {
  const q = query.toLowerCase().trim();
  return SEARCH_ALIASES[q] ?? [];
}

/** Levenshtein 编辑距离 */
function levenshtein(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] =
        a[i - 1] === b[j - 1]
          ? dp[i - 1][j - 1]
          : 1 + Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
    }
  }
  return dp[m][n];
}

/** 模糊匹配评分 */
function calculateMatchScore(query: string, target: string): number {
  const q = query.toLowerCase();
  const t = target.toLowerCase();
  if (t === q) return 100;
  if (t.startsWith(q)) return 85;
  if (t.includes(q)) return 70;
  const dist = levenshtein(q, t);
  if (dist <= 2) return 50 - dist * 15;
  return 0;
}

// ============================================================================
// 使用频率记录
// ============================================================================

function recordNodeUsage(type: string) {
  try {
    const raw = localStorage.getItem(USAGE_KEY);
    const counts: Record<string, number> = raw ? JSON.parse(raw) : {};
    counts[type] = (counts[type] || 0) + 1;
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 50);
    localStorage.setItem(USAGE_KEY, JSON.stringify(Object.fromEntries(sorted)));
  } catch {
    // 忽略存储异常
  }
}

function getRecentTypeIds(): string[] {
  try {
    const raw = localStorage.getItem(USAGE_KEY);
    if (!raw) return [];
    const counts: Record<string, number> = JSON.parse(raw);
    return Object.entries(counts)
      .sort((a, b) => b[1] - a[1])
      .map(([type]) => type)
      .slice(0, 8);
  } catch {
    return [];
  }
}

// ============================================================================
// 主组件
// ============================================================================

export function NodePalette() {
  const LL = getLL();
  const t = LL.nodes.palette;
  const setPendingAddNodeType = useFlowStore((s) => s.setPendingAddNodeType);
  const flow = useFlowStore((s) => s.flow);
  const isOpen = useUIStore((s) => s.isNodePaletteOpen);
  const palettePosition = useUIStore((s) => s.nodePalettePosition);
  const closePalette = useUIStore((s) => s.closeNodePalette);
  const openPluginSettingsDialog = useUIStore((s) => s.openPluginSettingsDialog);

  // 状态
  const [metadata, setMetadata] = useState<NodeMetadata[]>([]);
  const [selectedGroupId, setSelectedGroupId] = useState<string>('all');
  const [selectedItemIndex, setSelectedItemIndex] = useState<number>(0);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [hoveredItem, setHoveredItem] = useState<CatalogItem | null>(null);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const itemListRef = useRef<HTMLDivElement>(null);

  // 加载元数据
  useEffect(() => {
    getNodeMetadata().then(setMetadata);
  }, [isOpen]);

  // 构建目录分组
  const recentTypeIds = useMemo(() => (isOpen ? getRecentTypeIds() : []), [isOpen]);
  const catalogGroups = useMemo(
    () => buildCatalogGroups(metadata, recentTypeIds),
    [metadata, recentTypeIds]
  );

  // 搜索评分与过滤
  const isSearching = searchQuery.trim().length > 0;
  const filteredGroups = useMemo(() => {
    if (!isSearching) return catalogGroups;

    const query = searchQuery.toLowerCase().trim();
    const aliasTypes = expandAliases(query);

    return catalogGroups
      .map((group) => {
        if (group.id === 'recent') return null; // 搜索时隐藏最近使用分类，聚焦匹配结果

        const scoredItems = group.items
          .map((item) => {
            let score = 0;
            const nameScore = calculateMatchScore(query, item.name);
            const typeScore = calculateMatchScore(query, item.typeId);
            const descScore = calculateMatchScore(query, item.description);
            const pluginScore = item.pluginName ? calculateMatchScore(query, item.pluginName) : 0;

            score = Math.max(nameScore, typeScore, descScore * 0.7, pluginScore * 0.8);

            if (aliasTypes.includes(item.typeId)) {
              score = Math.max(score, 90);
            }

            return { item, score };
          })
          .filter((res) => res.score > 0)
          .sort((a, b) => b.score - a.score)
          .map((res) => res.item);

        if (scoredItems.length === 0) return null;

        return {
          ...group,
          count: scoredItems.length,
          items: scoredItems,
        };
      })
      .filter((g): g is CatalogGroup => g !== null);
  }, [catalogGroups, isSearching, searchQuery]);

  // 确保选中的分组有效
  useEffect(() => {
    if (filteredGroups.length > 0) {
      const exists = filteredGroups.some((g) => g.id === selectedGroupId);
      if (!exists) {
        setSelectedGroupId(filteredGroups[0].id);
      }
    }
  }, [filteredGroups, selectedGroupId]);

  // 打开时重置状态
  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setSelectedGroupId('all');
      setSelectedItemIndex(0);
      setHoveredItem(null);
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 50);
    }
  }, [isOpen]);

  // 当前选中分组下的节点列表
  const currentGroup = useMemo(
    () => filteredGroups.find((g) => g.id === selectedGroupId) ?? filteredGroups[0],
    [filteredGroups, selectedGroupId]
  );

  const displayItems = useMemo(() => currentGroup?.items ?? [], [currentGroup]);

  // 当前高亮/选中的节点
  const activeItem = useMemo(() => {
    if (hoveredItem) return hoveredItem;
    return displayItems[selectedItemIndex] ?? displayItems[0] ?? null;
  }, [hoveredItem, displayItems, selectedItemIndex]);

  // 保证选中索引在有效范围内
  useEffect(() => {
    if (selectedItemIndex >= displayItems.length) {
      setSelectedItemIndex(Math.max(0, displayItems.length - 1));
    }
  }, [displayItems, selectedItemIndex]);

  // 添加节点动作
  const handleSelectNode = useCallback(
    (typeId: string) => {
      if (!flow) return;
      recordNodeUsage(typeId);
      setPendingAddNodeType(typeId as NodeType);
      closePalette();
    },
    [flow, setPendingAddNodeType, closePalette]
  );

  // 拖拽动作
  const handleDragStart = useCallback(
    (e: React.DragEvent, typeId: string) => {
      e.dataTransfer.setData('application/reactflow/nodeType', typeId);
      e.dataTransfer.effectAllowed = 'move';
      recordNodeUsage(typeId);
      requestAnimationFrame(() => {
        closePalette();
      });
    },
    [closePalette]
  );

  // 键盘导航
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (searchQuery) {
          setSearchQuery('');
        } else {
          closePalette();
        }
        return;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setHoveredItem(null);
        setSelectedItemIndex((prev) => {
          const next = prev + 1;
          return next < displayItems.length ? next : prev;
        });
        return;
      }

      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setHoveredItem(null);
        setSelectedItemIndex((prev) => {
          const next = prev - 1;
          return next >= 0 ? next : 0;
        });
        return;
      }

      if (e.key === 'Tab') {
        e.preventDefault();
        setHoveredItem(null);
        // Tab 在左侧分组之间切换
        const currentGroupIdx = filteredGroups.findIndex((g) => g.id === selectedGroupId);
        if (currentGroupIdx !== -1) {
          const nextGroupIdx = (currentGroupIdx + 1) % filteredGroups.length;
          setSelectedGroupId(filteredGroups[nextGroupIdx].id);
          setSelectedItemIndex(0);
        }
        return;
      }

      if (e.key === 'Enter') {
        e.preventDefault();
        if (activeItem) {
          handleSelectNode(activeItem.typeId);
        }
      }
    },
    [
      searchQuery,
      closePalette,
      displayItems.length,
      filteredGroups,
      selectedGroupId,
      activeItem,
      handleSelectNode,
    ]
  );

  // 计算视口防溢出坐标
  const popupStyle: React.CSSProperties = useMemo(() => {
    if (palettePosition) {
      const left = Math.min(palettePosition.x + 8, window.innerWidth - PALETTE_WIDTH - 24);
      const top = Math.min(palettePosition.y + 8, window.innerHeight - PALETTE_HEIGHT - 24);
      return {
        position: 'fixed',
        left: `${Math.max(left, 20)}px`,
        top: `${Math.max(top, 20)}px`,
        width: `${PALETTE_WIDTH}px`,
        height: `${PALETTE_HEIGHT}px`,
      };
    }
    return {
      position: 'fixed',
      left: '50%',
      top: '50%',
      transform: 'translate(-50%, -50%)',
      width: `${PALETTE_WIDTH}px`,
      height: `${PALETTE_HEIGHT}px`,
    };
  }, [palettePosition]);

  if (!isOpen) return null;

  return (
    <>
      {/* 半透明遮罩 */}
      <div
        className="fixed inset-0 z-[60] bg-black/20 cursor-default"
        onClick={closePalette}
      />

      {/* 弹窗面板 */}
      <div
        style={popupStyle}
        onKeyDown={handleKeyDown}
        className={cn(
          'z-[70] flex flex-col',
          'bg-background rounded-2xl',
          'shadow-2xl border border-border/60',
          'animate-in fade-in-0 zoom-in-95 duration-150',
          'overflow-hidden select-none',
        )}
        onClick={(e) => e.stopPropagation()}
      >
        {/* 顶部搜索栏 */}
        <div className="flex items-center gap-2 px-3.5 py-2.5 border-b border-border/50 bg-muted/20 shrink-0">
          <Search className="w-4 h-4 text-muted-foreground shrink-0" />
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setSelectedItemIndex(0);
            }}
            placeholder={t.search_placeholder()}
            className={cn(
              'flex-1 text-sm bg-transparent border-none outline-none',
              'text-foreground placeholder:text-muted-foreground/60',
            )}
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => {
                setSearchQuery('');
                searchInputRef.current?.focus();
              }}
              className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          )}
          <div className="flex items-center gap-1 pl-1 border-l border-border/40 text-[11px] text-muted-foreground/70">
            <kbd className="px-1.5 py-0.5 rounded bg-muted/60 border border-border/50 font-mono">
              Esc
            </kbd>
          </div>
        </div>

        {/* 双栏主体 */}
        <div className="flex-1 min-h-0 flex">
          {/* 左栏：分组与插件列表 */}
          <div className="w-[190px] border-r border-border/40 flex flex-col bg-muted/10 shrink-0">
            <div className="flex-1 overflow-y-auto p-1.5 space-y-0.5 scrollbar-thin">
              {filteredGroups.map((group) => {
                const isSelected = group.id === selectedGroupId;
                const GroupIcon = group.icon;
                return (
                  <button
                    key={group.id}
                    type="button"
                    onClick={() => {
                      setSelectedGroupId(group.id);
                      setSelectedItemIndex(0);
                      setHoveredItem(null);
                    }}
                    className={cn(
                      'w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all text-left cursor-pointer',
                      isSelected
                        ? 'bg-accent text-accent-foreground font-semibold shadow-xs'
                        : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground',
                    )}
                  >
                    <div className="flex items-center gap-2 truncate">
                      <GroupIcon
                        className={cn(
                          'w-3.5 h-3.5 shrink-0',
                          isSelected ? 'text-primary' : 'text-muted-foreground',
                        )}
                      />
                      <span className="truncate">{group.name}</span>
                    </div>
                    <span
                      className={cn(
                        'text-[10px] px-1.5 py-0.2 rounded-full shrink-0 ml-1 font-mono',
                        isSelected
                          ? 'bg-primary/15 text-primary font-bold'
                          : 'bg-muted text-muted-foreground/80',
                      )}
                    >
                      {group.count}
                    </span>
                  </button>
                );
              })}
            </div>

            {/* 插件管理快捷入口 */}
            <div className="p-1.5 border-t border-border/40 bg-muted/20 shrink-0">
              <button
                type="button"
                onClick={() => {
                  closePalette();
                  openPluginSettingsDialog();
                }}
                className={cn(
                  'w-full flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-xs',
                  'text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors',
                )}
              >
                <Settings className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                <span className="truncate">{t.plugin_manage()}</span>
              </button>
            </div>
          </div>

          {/* 右栏：节点算子列表 + 下方详情卡片 */}
          <div className="flex-1 flex flex-col min-w-0 bg-background/50">
            {/* 上部：节点算子列表 */}
            <div
              ref={itemListRef}
              className="flex-1 overflow-y-auto p-2 space-y-1 scrollbar-thin min-h-[220px]"
            >
              {displayItems.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-muted-foreground py-10">
                  <Layers className="w-8 h-8 text-muted-foreground/40 mb-2" />
                  <p className="text-xs">{t.no_results()}</p>
                </div>
              ) : (
                displayItems.map((item, idx) => {
                  const isSelected = selectedItemIndex === idx && !hoveredItem;
                  const isHovered = hoveredItem?.typeId === item.typeId;
                  const isHighlighted = isSelected || isHovered;
                  const ItemIcon = item.icon;

                  return (
                    <div
                      key={item.typeId}
                      draggable
                      onDragStart={(e) => handleDragStart(e, item.typeId)}
                      onClick={() => handleSelectNode(item.typeId)}
                      onMouseEnter={() => setHoveredItem(item)}
                      onMouseLeave={() => setHoveredItem(null)}
                      className={cn(
                        'flex items-center justify-between px-3 py-2 rounded-xl text-xs cursor-pointer transition-all',
                        isHighlighted
                          ? 'bg-primary/10 border-primary/30 text-foreground shadow-xs'
                          : 'hover:bg-accent/60 text-muted-foreground hover:text-foreground border border-transparent',
                      )}
                    >
                      <div className="flex items-center gap-2.5 min-w-0">
                        <div
                          className={cn(
                            'p-1.5 rounded-lg shrink-0 transition-colors',
                            isHighlighted ? 'bg-primary/20 text-primary' : 'bg-muted text-muted-foreground',
                          )}
                        >
                          <ItemIcon className="w-4 h-4" />
                        </div>
                        <div className="flex flex-col min-w-0">
                          <span className="font-medium text-foreground truncate text-[12.5px]">
                            {item.name}
                          </span>
                          <span className="text-[10.5px] text-muted-foreground/70 font-mono truncate">
                            {item.typeId}
                          </span>
                        </div>
                      </div>

                      <div className="flex items-center gap-1.5 shrink-0 ml-2">
                        {item.isPlugin && (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20 font-medium">
                            {item.pluginName || 'Plugin'}
                          </span>
                        )}
                        {isHighlighted && (
                          <span className="text-[10px] text-muted-foreground/60 flex items-center gap-0.5">
                            ↵ {t.insert_hint()}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>

            {/* 下部：算子微型详情卡片 */}
            {activeItem && (
              <div className="p-3 border-t border-border/40 bg-muted/25 flex flex-col gap-1.5 shrink-0">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <span className="text-xs font-semibold text-foreground">
                      {activeItem.name}
                    </span>
                    <span className="text-[10px] text-muted-foreground/70 font-mono">
                      ({activeItem.typeId})
                    </span>
                  </div>
                  {activeItem.pluginName && (
                    <span className="text-[10px] text-muted-foreground">
                      {t.source_prefix({ name: activeItem.pluginName })}
                    </span>
                  )}
                </div>

                <p className="text-[11.5px] text-muted-foreground leading-relaxed line-clamp-2">
                  {activeItem.description}
                </p>

                {activeItem.ports && activeItem.ports.length > 0 && (
                  <div className="flex items-center gap-1.5 text-[10.5px] text-muted-foreground/80 pt-0.5">
                    <span className="text-muted-foreground/60">{t.ports_label()}</span>
                    <div className="flex items-center gap-1 flex-wrap">
                      {activeItem.ports.map((port) => (
                        <span
                          key={port.id}
                          className="px-1.5 py-0.2 rounded bg-background/80 border border-border/50 text-[10px] font-mono"
                        >
                          {port.label || port.id}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        {/* 底部快捷键与操作栏 */}
        <div className="px-3.5 py-1.5 border-t border-border/40 bg-muted/30 text-[11px] text-muted-foreground/70 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <span>{t.key_navigate()}</span>
            <span>{t.key_tab()}</span>
            <span>{t.key_insert()}</span>
            <span>{t.key_drag()}</span>
          </div>
          <span>{t.key_esc()}</span>
        </div>
      </div>
    </>
  );
}
