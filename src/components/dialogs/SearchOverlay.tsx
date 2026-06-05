/**
 * SearchOverlay - 全流程搜索面板
 * Ctrl+Shift+F 触发，搜索当前流程所有节点名称、配置、变量引用
 * 结果按节点分组，点击跳转并高亮
 */
import { useState, useMemo, useCallback } from 'react';
import { Search, FileSearch, ArrowRight } from 'lucide-react';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useFlowStore } from '@/stores';
import { searchFlow, type GroupedSearchResult } from '@/lib/flowSearch';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface SearchOverlayProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SearchOverlay({ open, onOpenChange }: SearchOverlayProps) {
  const flow = useFlowStore((s) => s.flow);
  const focusOnNode = useFlowStore((s) => s.focusOnNode);
  const [query, setQuery] = useState('');

  // 搜索结果
  const results = useMemo(() => {
    if (!query.trim() || !flow) return [];
    return searchFlow(flow, query);
  }, [flow, query]);

  // 跳转到节点
  const handleJumpToNode = useCallback(
    (nodeUuid: string) => {
      focusOnNode(nodeUuid);
      onOpenChange(false);
      setQuery('');
    },
    [focusOnNode, onOpenChange]
  );

  // 匹配类型对应的标签样式
  const matchTypeStyles: Record<string, string> = {
    name: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300',
    config: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
    variable: 'bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-300',
  };

  const ll = getLL();
  const matchTypeLabels: Record<string, string> = {
    name: ll.execution.report.search_match_name(),
    config: ll.execution.report.search_match_config(),
    variable: ll.execution.report.search_match_variable(),
  };

  // 重置搜索
  const handleClose = (open: boolean) => {
    if (!open) {
      setQuery('');
    }
    onOpenChange(open);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent
        className="sm:max-w-[550px] h-[70vh] !gap-0 p-0 flex flex-col overflow-hidden"
        hideCloseButton
      >
        {/* 标题 */}
        <div className="flex items-center gap-2 px-6 pt-5 pb-3 shrink-0">
          <FileSearch className="w-5 h-5 text-muted-foreground" />
          <h2 className="text-base font-semibold text-foreground">{getLL().flow.search_flow()}</h2>
        </div>

        {/* 搜索框 */}
        <div className="px-6 pb-3 shrink-0">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              type="text"
              placeholder={getLL().flow.search_placeholder()}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="pl-9 h-9 text-sm"
              autoFocus
            />
          </div>
        </div>

        {/* 搜索结果 */}
        <ScrollArea className="flex-1 px-6 min-h-0">
          {results.length > 0 ? (
            <div className="space-y-3 pb-4">
              {results.map((group: GroupedSearchResult) => (
                <SearchResultGroup
                  key={group.nodeUuid}
                  group={group}
                  query={query}
                  onJump={handleJumpToNode}
                  matchTypeStyles={matchTypeStyles}
                  matchTypeLabels={matchTypeLabels}
                />
              ))}
            </div>
          ) : query.trim() ? (
            <div className="py-12 text-center">
              <Search className="w-8 h-8 mx-auto text-muted-foreground/40 mb-2" />
              <p className="text-sm text-muted-foreground">{getLL().flow.search_no_results()}</p>
              <p className="text-xs text-muted-foreground/60 mt-1">{getLL().flow.search_try_other()}</p>
            </div>
          ) : (
            <div className="py-12 text-center">
              <FileSearch className="w-8 h-8 mx-auto text-muted-foreground/40 mb-2" />
              <p className="text-sm text-muted-foreground">{getLL().flow.search_hint()}</p>
              <p className="text-xs text-muted-foreground/60 mt-1">
                {getLL().flow.search_hint_detail()}
              </p>
            </div>
          )}
        </ScrollArea>

        {/* 底部提示 */}
        <div className="text-[11px] text-muted-foreground/60 text-center px-6 py-2 border-t border-border shrink-0">
          {getLL().flow.search_close_hint()}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** 搜索结果分组组件 */
function SearchResultGroup({
  group,
  query,
  onJump,
  matchTypeStyles,
  matchTypeLabels,
}: {
  group: GroupedSearchResult;
  query: string;
  onJump: (nodeUuid: string) => void;
  matchTypeStyles: Record<string, string>;
  matchTypeLabels: Record<string, string>;
}) {
  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* 分组标题（节点名） */}
      <button
        type="button"
        onClick={() => onJump(group.nodeUuid)}
        className="w-full flex items-center gap-2 px-3 py-2 bg-muted/30 hover:bg-muted/50 transition-colors text-left cursor-pointer"
      >
        <span className="text-sm font-medium text-foreground flex-1 truncate">
          {group.nodeName}
        </span>
        <span className="text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
          {getLL().flow.search_match_count({ count: group.matches.length })}
        </span>
        <ArrowRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
      </button>

      {/* 匹配项列表 */}
      <div className="divide-y divide-border/50">
        {group.matches.slice(0, 8).map((match, i) => (
          <button
            key={i}
            type="button"
            onClick={() => onJump(group.nodeUuid)}
            className="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-accent/50 transition-colors text-left cursor-pointer"
          >
            <span
              className={cn(
                'text-[10px] px-1 py-0.5 rounded font-medium shrink-0',
                matchTypeStyles[match.matchType]
              )}
            >
              {matchTypeLabels[match.matchType]}
            </span>
            <span className="text-[11px] text-muted-foreground font-mono shrink-0">
              {match.field}
            </span>
            <span className="text-xs text-foreground truncate flex-1">
              {highlightMatch(match.value, query)}
            </span>
          </button>
        ))}
        {group.matches.length > 8 && (
          <div className="px-3 py-1 text-[11px] text-muted-foreground text-center">
            {getLL().flow.search_more_matches({ count: group.matches.length - 8 })}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * 高亮匹配的文本
 * 注意：返回 JSX 用于文本片段，简单实现不涉及 XSS 风险（输入来自 flow 数据）
 */
function highlightMatch(text: string, query: string): React.ReactNode {
  if (!query.trim()) return text;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const idx = lowerText.indexOf(lowerQuery);
  if (idx === -1) return text;

  return (
    <>
      {text.slice(0, idx)}
      <mark className="bg-amber-200 dark:bg-amber-800/60 text-foreground rounded-sm px-0.5">
        {text.slice(idx, idx + query.length)}
      </mark>
      {text.slice(idx + query.length)}
    </>
  );
}

export default SearchOverlay;
